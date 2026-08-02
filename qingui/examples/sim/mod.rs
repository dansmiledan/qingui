//! 共享模拟器运行时：minifb 窗口 + flush 转发 + 按键映射 + 主循环。
//! 各 example 只需实现 UI 构建函数并调用 `sim::run(build)`。

use minifb::{Key as MKey, Scale, Window, WindowOptions};
use qingui::display::Flush;
use qingui::input::Key;
use qingui::{Color, Rect, Ui};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;
pub const BUF_ROWS: u32 = 24; // 1/10 屏，验证 PFB 分块

/// 脏矩形调试边框只保留最近 N 次 flush（过期的恢复原始内容）
pub const DEBUG_KEEP: usize = 10;

/// 一条边框记录：chunk 矩形 + 内容快照 + 序号
struct BorderRec {
    rect: Rect,
    pixels: Vec<Color>,
    seq: u64,
}

/// flush 写入共享的全屏 u32 缓冲（0x00RRGGBB）；debug 开启时给 chunk 画绿色 1px 边框
struct SimFlush {
    fb: Rc<RefCell<Vec<u32>>>,
    debug: Rc<Cell<bool>>,
    seq: u64,
    history: std::collections::VecDeque<BorderRec>,
}

impl SimFlush {
    /// 过期边框恢复原始内容
    fn expire(&mut self) {
        while let Some(front) = self.history.front() {
            if self.seq.saturating_sub(front.seq) >= DEBUG_KEEP as u64 {
                let rec = self.history.pop_front().unwrap();
                let mut fb = self.fb.borrow_mut();
                for y in 0..rec.rect.h {
                    for x in 0..rec.rect.w {
                        let sx = rec.rect.x + x;
                        let sy = rec.rect.y + y;
                        if sx >= 0 && sx < WIDTH as i32 && sy >= 0 && sy < HEIGHT as i32 {
                            let c = rec.pixels[(y * rec.rect.w + x) as usize];
                            fb[sy as usize * WIDTH + sx as usize] =
                                ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32;
                        }
                    }
                }
            } else {
                break;
            }
        }
    }
}

impl Flush for SimFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.expire();
        {
            let mut fb = self.fb.borrow_mut();
            for y in 0..area.h {
                for x in 0..area.w {
                    let sx = area.x + x;
                    let sy = area.y + y;
                    if sx >= 0 && sx < WIDTH as i32 && sy >= 0 && sy < HEIGHT as i32 {
                        let c = pixels[(y * area.w + x) as usize];
                        fb[sy as usize * WIDTH + sx as usize] =
                            ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32;
                    }
                }
            }
        }
        if self.debug.get() {
            // 同步历史快照：新 flush 覆盖的区域以新内容为准，
            // 否则过期恢复会把旧内容写回、盖住后画的控件（如 msgbox）
            for rec in self.history.iter_mut() {
                if let Some(ov) = rec.rect.intersect(&area) {
                    for y in 0..ov.h {
                        for x in 0..ov.w {
                            let src = ((ov.y - area.y + y) * area.w + (ov.x - area.x + x)) as usize;
                            let dst = ((ov.y - rec.rect.y + y) * rec.rect.w + (ov.x - rec.rect.x + x)) as usize;
                            rec.pixels[dst] = pixels[src];
                        }
                    }
                }
            }
            let mut fb = self.fb.borrow_mut();
            for x in area.x..area.right() {
                for y in [area.y, area.bottom() - 1] {
                    if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                        fb[y as usize * WIDTH + x as usize] = 0x00FF00;
                    }
                }
            }
            for y in area.y..area.bottom() {
                for x in [area.x, area.right() - 1] {
                    if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                        fb[y as usize * WIDTH + x as usize] = 0x00FF00;
                    }
                }
            }
            drop(fb);
            self.history.push_back(BorderRec { rect: area, pixels: pixels.to_vec(), seq: self.seq });
        }
        self.seq += 1;
    }
}

fn map_key(k: MKey) -> Option<Key> {
    Some(match k {
        MKey::Up => Key::Up,
        MKey::Down => Key::Down,
        MKey::Left => Key::Left,
        MKey::Right => Key::Right,
        MKey::Enter => Key::Enter,
        MKey::Escape => Key::Esc,
        MKey::Tab => Key::Next,
        MKey::Backspace => Key::Prev,
        _ => return None,
    })
}

const KEYS: [MKey; 8] = [
    MKey::Up, MKey::Down, MKey::Left, MKey::Right,
    MKey::Enter, MKey::Escape, MKey::Tab, MKey::Backspace,
];

/// 打开模拟器窗口并运行主循环。`build` 在启动时调用一次构建 UI。
#[allow(dead_code)] // 各 example 按需使用 run 或 run_with_tick
pub fn run(build: impl FnOnce(&mut Ui)) {
    run_with_tick(build, |_| {});
}

/// 同 `run`，额外提供每帧回调（驱动定时任务，如周期性重排）
#[allow(dead_code)]
pub fn run_with_tick(build: impl FnOnce(&mut Ui), mut tick: impl FnMut(&mut Ui)) {
    let mut window = Window::new(
        "qingui sim",
        WIDTH,
        HEIGHT,
        WindowOptions { scale: Scale::X2, ..Default::default() },
    )
    .expect("open window");
    window.set_target_fps(60);

    // 共享全屏缓冲：SimFlush 写 chunk，主循环整块交给 minifb
    let fb = Rc::new(RefCell::new(vec![0u32; WIDTH * HEIGHT]));
    let debug = Rc::new(Cell::new(true)); // D 键切换脏矩形调试边框
    let mut ui = Ui::new(WIDTH as i32, HEIGHT as i32, BUF_ROWS);
    ui.set_flush(Box::new(SimFlush {
        fb: fb.clone(),
        debug: debug.clone(),
        seq: 0,
        history: std::collections::VecDeque::new(),
    }));
    build(&mut ui);

    let mut last = Instant::now();
    let mut frames = 0u32;
    let mut fps_ts = Instant::now();

    while window.is_open() && !window.is_key_down(MKey::Q) {
        let now = Instant::now();
        ui.tick_inc(now.duration_since(last).as_millis().max(1) as u32);
        last = now;

        for &k in &KEYS {
            if window.is_key_pressed(k, minifb::KeyRepeat::No) {
                if let Some(key) = map_key(k) {
                    ui.keypad_input(key);
                }
            }
        }
        // D：切换脏矩形调试边框（只保留最近 DEBUG_KEEP 次 flush）
        if window.is_key_pressed(MKey::D, minifb::KeyRepeat::No) {
            debug.set(!debug.get());
        }

        tick(&mut ui);
        ui.timer_handler();
        window
            .update_with_buffer(&fb.borrow(), WIDTH, HEIGHT)
            .unwrap();

        frames += 1;
        if fps_ts.elapsed().as_secs() >= 1 {
            window.set_title(&format!("qingui sim — {} fps", frames));
            frames = 0;
            fps_ts = Instant::now();
        }
    }
}
