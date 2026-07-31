// 回归：Roller 连按后滚定，渲染应与全新构建一致（无残影/重叠）
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::roller::RollerBuilder;
use qingui::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    fb: Vec<Color>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        let mut r = self.0.borrow_mut();
        let fb = &mut r.fb;
        for y in 0..area.h {
            for x in 0..area.w {
                fb[(area.y + y) as usize * 160 + (area.x + x) as usize] = pixels[(y * area.w + x) as usize];
            }
        }
    }
}

fn build() -> (Ui, Rc<RefCell<RecFlush>>) {
    let rec = Rc::new(RefCell::new(RecFlush { fb: vec![Color::BLACK; 160 * 120] }));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    scr.set_style(&mut ui, bg);
    (ui, rec)
}

#[test]
fn repro_roller_rapid_press_ghost() {
    // 复现：连按后滚定
    let (mut ui, rec) = build();
    let scr = ui.screen();
    let r = RollerBuilder::new(&["One", "Two", "Three", "Four", "Five"]).build(&mut ui, scr);
    r.set_pos(&mut ui, 10, 10);
    r.group_add(&mut ui);
    ui.tick_inc(1);
    ui.timer_handler();
    for _ in 0..3 {
        ui.keypad_input(Key::Down);
        ui.tick_inc(50); // 动画中途连按
    }
    // 滚定
    for _ in 0..20 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    assert!(!ui.anim_running());
    let got = rec.borrow().fb.clone();

    // 参考：全新构建，直接选中 3
    let (mut ui2, rec2) = build();
    let scr2 = ui2.screen();
    let r2 = RollerBuilder::new(&["One", "Two", "Three", "Four", "Five"]).build(&mut ui2, scr2);
    r2.set_pos(&mut ui2, 10, 10);
    r2.set_value(&mut ui2, 3);
    r2.group_add(&mut ui2);
    ui2.tick_inc(1);
    ui2.timer_handler();
    let reference = rec2.borrow().fb.clone();

    let mut bad = None;
    let (mut n, mut min_x, mut max_x, mut min_y, mut max_y) = (0, i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for y in 0..120i32 {
        for x in 0..160i32 {
            let idx = (y * 160 + x) as usize;
            if got[idx] != reference[idx] {
                if bad.is_none() {
                    bad = Some((x, y, reference[idx], got[idx]));
                }
                n += 1;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
}
