use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::dropdown::DropdownCfg;
use qingui::widgets::led::LedCfg;
use qingui::prelude::*;
use qingui::widgets::roller::RollerCfg;
use qingui::widgets::spinbox::SpinboxCfg;
use qingui::widgets::table::TableCfg;
use qingui::{EventKind, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Rgb888>)>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Rgb888]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

fn setup() -> (Ui, Rc<RefCell<RecFlush>>) {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui: Ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Rgb888::BLACK);
    let scr = ui.screen();
    ui.set_style(scr, bg);
    (ui, rec)
}

fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Rgb888 {
    let chunks = &rec.borrow().chunks;
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

#[test]
fn led_brightness() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let led = LedCfg::new(Rgb888::RED).build(&mut ui, scr);
    ui.set_pos(led, 10, 10);
    ui.render();
    assert_eq!(px(&rec, 18, 18), Rgb888::RED); // fully-lit center
    ui.set_value(led, 128);
    ui.render();
    let dim = px(&rec, 18, 18);
    assert!(dim.r() < 255 && dim.r() > 100, "半亮: {:?}", dim);
    assert_eq!(dim.g(), 0);
}

#[test]
fn table_cells() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let t = TableCfg::new(2, 2).build(&mut ui, scr);
    ui.set_pos(t, 10, 10);
    ui.table_set_cell(t, 0, 0, "A1");
    ui.table_set_cell(t, 1, 1, "B2");
    ui.render();
    // FONT_6X10 'A' glyph row 1 is 001000 → lit at text origin (14,14) + 2 right, 1 down
    assert_eq!(px(&rec, 14 + 2, 14 + 1), Rgb888::WHITE);
    // Grid lines
    assert_eq!(px(&rec, 10, 20), Rgb888::new(70, 70, 90));
    // Bottom grid line (should exist after the half-open interval fix)
    assert_eq!(px(&rec, 30, 41), Rgb888::new(70, 70, 90));
    // Empty cell has no text
    assert_eq!(px(&rec, 74, 14), Rgb888::BLACK);
}

#[test]
fn spinbox_digit_edit() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, _) = setup();
    let scr = ui.screen();
    let sb = SpinboxCfg::new(0, 999, 3).build(&mut ui, scr);
    let other = ButtonCfg::new("X").build(&mut ui, scr);
    ui.add_event_cb(sb, EventKind::ValueChanged, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(sb);
    ui.group_add(other);
    // Not in edit mode: arrow keys are focus navigation, value unchanged
    ui.keypad_input(Key::Up);
    assert_eq!(ui.value(sb), 0);
    assert_eq!(ui.focused(), Some(other));
    ui.keypad_input(Key::Prev);
    // Combination-lock editing: Enter starts at the most significant digit (hundreds).
    // Up → +100; Enter locks the current digit and advances to the next; on the last
    // digit Enter confirms. Keyboard users can still move the cursor freely with Left/Right.
    ui.keypad_input(Key::Enter);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.value(sb), 100);
    ui.keypad_input(Key::Enter); // lock hundreds, cursor → tens
    ui.keypad_input(Key::Left);  // cursor → hundreds (free movement)
    ui.keypad_input(Key::Down);  // -100
    assert_eq!(ui.value(sb), 0);
    assert_eq!(log.borrow().len(), 2);
    // Enter advances the cursor one digit at a time; only the last digit commits
    ui.keypad_input(Key::Enter); // lock hundreds → tens
    ui.keypad_input(Key::Enter); // lock tens → units
    assert!(ui.state(sb).contains(qingui::node::State::EDITED)); // still editing
    ui.keypad_input(Key::Enter); // units → Commit (Click + exit edit)
    assert!(!ui.state(sb).contains(qingui::node::State::EDITED));
    // Esc exits edit mode without committing; arrow keys move focus out again
    ui.keypad_input(Key::Enter); // re-enter edit
    ui.keypad_input(Key::Esc);
    assert!(!ui.state(sb).contains(qingui::node::State::EDITED));
    ui.keypad_input(Key::Right);
    assert_eq!(ui.focused(), Some(other));
}

#[test]
fn spinbox_rotary_encoder_combination_lock() {
    let (mut ui, _) = setup();
    let scr = ui.screen();
    let sb = SpinboxCfg::new(0, 999, 3).build(&mut ui, scr);
    ui.group_add(sb);
    // A rotary encoder produces only rotation (Up/Down) and Enter — no Left/Right/Esc:
    // rotation sets the current digit, Enter locks it and advances to the next digit,
    // so every digit is reachable with a single axis + one confirm button.
    ui.keypad_input(Key::Enter); // edit, cursor at hundreds
    for _ in 0..5 {ui.keypad_input(Key::Up); }
    assert_eq!(ui.value(sb), 500); // hundreds set to 5
    ui.keypad_input(Key::Enter); // lock hundreds, cursor → tens
    for _ in 0..5 {ui.keypad_input(Key::Down); }
    assert_eq!(ui.value(sb), 450); // tens set to 4
    ui.keypad_input(Key::Enter); // lock tens, cursor → units
    for _ in 0..5 {ui.keypad_input(Key::Up); }
    assert_eq!(ui.value(sb), 455); // units set to 5
    ui.keypad_input(Key::Enter); // last digit → Commit
    assert!(!ui.state(sb).contains(qingui::node::State::EDITED));
    assert_eq!(ui.value(sb), 455);
}

#[test]
fn spinbox_cursor_highlight() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let sb = SpinboxCfg::new(0, 999, 3).build(&mut ui, scr);
    ui.set_pos(sb, 10, 10);
    ui.set_value(sb, 5);
    ui.set_state(sb, qingui::node::State::EDITED, true); // the cursor highlight only shows in edit mode
    ui.render();
    // Layout (FONT_6X10: advance 6, line height 10): spinbox default 30x18, starting at (10,10);
    // digits '0','0','5' are at x=16/22/28, glyph top row y=14. Combination-lock editing starts
    // at the most significant digit, so the hundreds highlight block is (15,11,8,16):
    assert_eq!(px(&rec, 17, 12), Rgb888::new(80, 140, 255)); // hundreds digit highlighted
    assert_ne!(px(&rec, 28, 12), Rgb888::new(80, 140, 255)); // ones digit not highlighted
}

#[test]
fn roller_rapid_select_continues_from_visual_pos() {
    let (mut ui, _) = setup();
    let scr = ui.screen();
    let r = RollerCfg::new(&["One", "Two", "Three", "Four"]).build(&mut ui, scr);
    ui.group_add(r);
    ui.keypad_input(Key::Enter); // enter the inner (EDITED) mode
    ui.keypad_input(Key::Down); // 0 → 1 (animation starts)
    ui.tick_inc(50); // mid-animation (about 1/3)
    ui.keypad_input(Key::Down); // 1 → 2 (rapid press)
    // The new animation should resume from the interpolated position (0 < from < 1), not jump from 1
    let s = ui.as_roller(r).unwrap();
    let (from, _) = s.sel_from.expect("有滚动动画");
    assert!(from > 0.0 && from < 1.0, "from={}", from);
}

#[test]
fn roller_navigation_and_fx() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, _) = setup();
    let scr = ui.screen();
    let r = RollerCfg::new(&["One", "Two", "Three"]).build(&mut ui, scr);
    ui.add_event_cb(r, EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(r);
    ui.keypad_input(Key::Enter); // enter the inner (EDITED) mode
    ui.keypad_input(Key::Down);
    assert_eq!(ui.roller_selected(r), 1);
    assert_eq!(ui.timer_handler(), 0); // scroll animation active
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down); // stops at the end
    assert_eq!(ui.roller_selected(r), 2);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.roller_selected(r), 1);
    ui.tick_inc(300);
    ui.timer_handler();
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}

#[test]
fn dropdown_open_select_close() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, _) = setup();
    let scr = ui.screen();
    let dd = DropdownCfg::new(&["Red", "Green", "Blue"]).build(&mut ui, scr);
    ui.add_event_cb(dd, EventKind::ValueChanged, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(dd);
    // Enter opens the overlay list (modal)
    ui.keypad_input(Key::Enter);
    let overlay = ui.focused().expect("有焦点");
    assert_ne!(overlay, dd);
    // Down to Green, Enter selects it
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(dd), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    assert_eq!(ui.focused(), Some(dd)); // focus restored
    // Open again, Esc closes without changing the value
    ui.keypad_input(Key::Enter);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Esc);
    assert_eq!(ui.value(dd), 1);
    assert_eq!(ui.focused(), Some(dd));
}
