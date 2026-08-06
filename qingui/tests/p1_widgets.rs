use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::dropdown::DropdownBuilder;
use qingui::widgets::led::LedBuilder;
use qingui::prelude::*;
use qingui::widgets::roller::RollerBuilder;
use qingui::widgets::spinbox::SpinboxCfg;
use qingui::widgets::table::TableBuilder;
use qingui::{Color, EventKind, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

fn setup() -> (Ui, Rc<RefCell<RecFlush>>) {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    ui.set_style(scr, bg);
    (ui, rec)
}

fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Color {
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
    let led = LedBuilder::new(Color::RED).build(&mut ui, scr);
    ui.set_pos(led, 10, 10);
    ui.render();
    assert_eq!(px(&rec, 18, 18), Color::RED); // fully-lit center
    ui.set_value(led, 128);
    ui.render();
    let dim = px(&rec, 18, 18);
    assert!(dim.r < 255 && dim.r > 100, "半亮: {:?}", dim);
    assert_eq!(dim.g, 0);
}

#[test]
fn table_cells() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let t = TableBuilder::new(2, 2).build(&mut ui, scr);
    ui.set_pos(t, 10, 10);
    ui.table_set_cell(t, 0, 0, "A1");
    ui.table_set_cell(t, 1, 1, "B2");
    ui.render();
    // FONT_6X10 'A' glyph row 1 is 001000 → lit at text origin (14,14) + 2 right, 1 down
    assert_eq!(px(&rec, 14 + 2, 14 + 1), Color::WHITE);
    // Grid lines
    assert_eq!(px(&rec, 10, 20), Color::rgb(70, 70, 90));
    // Bottom grid line (should exist after the half-open interval fix)
    assert_eq!(px(&rec, 30, 41), Color::rgb(70, 70, 90));
    // Empty cell has no text
    assert_eq!(px(&rec, 74, 14), Color::BLACK);
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
    // Enter enters edit mode: Up → +1; Left to tens, Up → +10; Left to hundreds, Down → clamped to min
    ui.keypad_input(Key::Enter);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.value(sb), 1);
    ui.keypad_input(Key::Left);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.value(sb), 11);
    ui.keypad_input(Key::Left);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.value(sb), 0);
    assert_eq!(log.borrow().len(), 3);
    // Esc exits edit mode, arrow keys can move focus out again
    ui.keypad_input(Key::Esc);
    ui.keypad_input(Key::Right);
    assert_eq!(ui.focused(), Some(other));
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
    // digits '0','0','5' are at x=16/22/28, glyph top row y=14; the ones-digit highlight block is (27,11,8,16)
    // The ones digit (3rd from the right) is highlighted: sample a pixel above the glyph inside the highlight block (28,12)
    assert_eq!(px(&rec, 28, 12), Color::rgb(80, 140, 255));
    // Hundreds digit has no highlight: '0' glyph row 1 is ..#... → (18,15) is text white (not the highlight color)
    assert_eq!(px(&rec, 18, 15), Color::WHITE);
}

#[test]
fn roller_rapid_select_continues_from_visual_pos() {
    let (mut ui, _) = setup();
    let scr = ui.screen();
    let r = RollerBuilder::new(&["One", "Two", "Three", "Four"]).build(&mut ui, scr);
    ui.group_add(r);
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
    let r = RollerBuilder::new(&["One", "Two", "Three"]).build(&mut ui, scr);
    ui.add_event_cb(r, EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(r);
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
    let dd = DropdownBuilder::new(&["Red", "Green", "Blue"]).build(&mut ui, scr);
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
