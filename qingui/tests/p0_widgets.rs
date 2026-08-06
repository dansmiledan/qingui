use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::arc::ArcBuilder;
use qingui::widgets::checkbox::CheckboxCfg;
use qingui::prelude::*;
use qingui::widgets::msgbox::MsgboxBuilder;
use qingui::widgets::spinner::SpinnerBuilder;
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
fn arc_value_and_indicator() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let a = ArcBuilder::new(0, 100).build(&mut ui, scr);
    ui.set_pos(a, 10, 10);
    ui.set_value(a, 50);
    ui.render();
    assert_eq!(ui.value(a), 50);
    // Center (40,40), r=27. START=135°(bottom-left). 50% → the indicator reaches 135+135=270°(straight up)
    // Track color sampled along the 300° direction (outside the indicator range, not on a boundary): (40+13, 40-23) = (53, 17)
    assert_eq!(px(&rec, 53, 17), Color::rgb(70, 70, 80));
    // The 50% indicator arc covers bottom-left to straight up: middle of the ring band at 180°(straight left) (40-25, 40) = (15, 40)
    assert_eq!(px(&rec, 15, 40), Color::rgb(80, 140, 255));
    // The indicator arc covers the 200° direction (40-23, 40+8) = (17, 48)
    assert_eq!(px(&rec, 17, 48), Color::rgb(80, 140, 255));
    // 90°(straight down, inside the sweep gap) has no arc: (40, 40+25) = (40, 65) is background
    assert_eq!(px(&rec, 40, 65), Color::BLACK);
}

#[test]
fn arc_edited_turns_indicator_yellow() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let a = ArcBuilder::new(0, 100).build(&mut ui, scr);
    ui.set_pos(a, 10, 10);
    ui.set_value(a, 50);
    ui.set_state(a, qingui::node::State::EDITED, true);
    ui.render();
    // Edit mode: the indicator arc turns yellow (180° direction (15,40))
    assert_eq!(px(&rec, 15, 40), Color::rgb(255, 200, 60));
}

#[test]
fn checkbox_toggles_on_enter() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let cb = CheckboxCfg::new("OK").build(&mut ui, scr);
    ui.set_pos(cb, 10, 10);
    ui.add_event_cb(cb, EventKind::ValueChanged, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(cb);
    ui.render();
    // Unchecked: the box's top edge is gray (avoiding the widget's focus border), no checkmark inside
    assert_eq!(px(&rec, 16, 12), Color::rgb(150, 150, 160)); // box top edge
    assert_ne!(px(&rec, 15, 16), Color::rgb(80, 140, 255)); // no checkmark
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(cb), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    ui.render();
    // After checking, the checkmark line passes through (17,19)
    assert_eq!(px(&rec, 17, 19), Color::rgb(80, 140, 255));
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(cb), 0);
}

#[test]
fn spinner_keeps_timer_busy_and_draws_arc() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let s = SpinnerBuilder::new().build(&mut ui, scr);
    ui.set_pos(s, 10, 10);
    ui.render();
    assert_eq!(ui.timer_handler(), 0); // self-rotating: always awake
    // There is some arc pixel somewhere
    let mut found = false;
    for y in 10..42 {
        for x in 10..42 {
            if px(&rec, x, y) == Color::rgb(80, 140, 255) {
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn msgbox_click_records_index_and_closes() {
    let (mut ui, _) = setup();
    let sel_log = Rc::new(RefCell::new(Vec::new()));
    let sl = sel_log.clone();
    let scr = ui.screen();
    let mb = MsgboxBuilder::new("Title", "Body text").buttons(&["Yes", "No"]).build(&mut ui, scr);
    ui.add_event_cb(mb, EventKind::ValueChanged, Box::new(move |ui, t, _| {
        sl.borrow_mut().push(ui.msgbox_selected(t));
    }));
    assert!(ui.is_valid(mb));
    // Focus is locked inside the msgbox: Tab should cycle between the two buttons
    let f0 = ui.focused();
    ui.keypad_input(Key::Next);
    let f1 = ui.focused();
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), f0);
    assert_ne!(f0, f1);
    // Click the currently focused button (the first one) → selected=0, msgbox is deleted
    ui.keypad_input(Key::Enter);
    assert_eq!(*sel_log.borrow(), vec![0]);
    assert!(!ui.is_valid(mb));
}

#[test]
fn msgbox_esc_closes_with_minus_one() {
    let (mut ui, _) = setup();
    let sel_log = Rc::new(RefCell::new(Vec::new()));
    let sl = sel_log.clone();
    let scr = ui.screen();
    let mb = MsgboxBuilder::new("T", "B").buttons(&["OK"]).build(&mut ui, scr);
    ui.add_event_cb(mb, EventKind::ValueChanged, Box::new(move |ui, t, _| {
        sl.borrow_mut().push(ui.msgbox_selected(t));
    }));
    ui.keypad_input(Key::Esc);
    assert_eq!(*sel_log.borrow(), vec![-1]);
    assert!(!ui.is_valid(mb));
}
