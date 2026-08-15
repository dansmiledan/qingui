// Regression: after rapid presses the Roller settles, and the render must match a fresh build (no ghosting/overlap)
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::roller::RollerCfg;
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
    let mut ui: Ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    ui.set_style(scr, bg);
    (ui, rec)
}

#[test]
fn repro_roller_rapid_press_ghost() {
    // Repro: rapid presses then settle
    let (mut ui, rec) = build();
    let scr = ui.screen();
    let r = RollerCfg::new(&["One", "Two", "Three", "Four", "Five"]).build(&mut ui, scr);
    ui.set_pos(r, 10, 10);
    ui.group_add(r);
    ui.tick_inc(1);
    ui.timer_handler();
    for _ in 0..3 {
        ui.keypad_input(Key::Down);
        ui.tick_inc(50); // rapid press mid-animation
    }
    // Settle
    for _ in 0..20 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    assert!(!ui.anim_running());
    let got = rec.borrow().fb.clone();

    // Reference: fresh build, directly selecting 3
    let (mut ui2, rec2) = build();
    let scr2 = ui2.screen();
    let r2 = RollerCfg::new(&["One", "Two", "Three", "Four", "Five"]).build(&mut ui2, scr2);
    ui2.set_pos(r2, 10, 10);
    ui2.set_value(r2, 3);
    ui2.group_add(r2);
    ui2.tick_inc(1);
    ui2.timer_handler();
    let reference = rec2.borrow().fb.clone();

    let mut bad = None;
    let (mut _n, mut min_x, mut max_x, mut min_y, mut max_y) = (0, i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for y in 0..120i32 {
        for x in 0..160i32 {
            let idx = (y * 160 + x) as usize;
            if got[idx] != reference[idx] {
                if bad.is_none() {
                    bad = Some((x, y, reference[idx], got[idx]));
                }
                _n += 1;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
}
