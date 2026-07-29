//! 共享模拟器运行时：minifb 窗口 + flush 转发 + 按键映射 + 主循环。
//! 各 example 只需实现 UI 构建函数并调用 `sim::run(build)`。

use minifb::{Key as MKey, Scale, Window, WindowOptions};
use qingui::display::Flush;
use qingui::input::Key;
use qingui::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;
pub const BUF_ROWS: u32 = 24; // 1/10 屏，验证 PFB 分块

/// flush 写入共享的全屏 u32 缓冲（0x00RRGGBB）；debug_dirty 时给 chunk 画绿色 1px 边框
struct SimFlush {
    fb: Rc<RefCell<Vec<u32>>>,
    debug_dirty: bool,
}

impl Flush for SimFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
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
        if self.debug_dirty {
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
        }
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
    let mut ui = Ui::new(WIDTH as i32, HEIGHT as i32, BUF_ROWS);
    ui.set_flush(Box::new(SimFlush { fb: fb.clone(), debug_dirty: false }));
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
