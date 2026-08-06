use qingui::input::Key;
use qingui::widgets::bar::BarBuilder;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::slider::SliderBuilder;
use qingui::widgets::switch::SwitchBuilder;
use qingui::{EventKind, Ui};
use std::cell::RefCell;
use std::rc::Rc;

type Log = Rc<RefCell<Vec<EventKind>>>;

fn logger(log: &Log) -> impl FnMut(&mut Ui, qingui::ObjRef, EventKind) + 'static {
    let l = log.clone();
    move |_ui, _t, k| l.borrow_mut().push(k)
}

#[test]
fn focus_cycles_with_next_prev() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonCfg::new("A").build(&mut ui, scr);
    let b = ButtonCfg::new("B").build(&mut ui, scr);
    ui.group_add(a);
    ui.group_add(b);
    assert_eq!(ui.focused(), Some(a)); // the first object added to the group is auto-focused
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(b));
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(a)); // wraps around
    ui.keypad_input(Key::Prev);
    assert_eq!(ui.focused(), Some(b));
}

#[test]
fn focus_events_and_state_flag() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonCfg::new("A").build(&mut ui, scr);
    let b = ButtonCfg::new("B").build(&mut ui, scr);
    ui.add_event_cb(a, EventKind::Defocused, Box::new(logger(&log)));
    ui.add_event_cb(b, EventKind::Focused, Box::new(logger(&log)));
    ui.group_add(a);
    ui.group_add(b);
    ui.keypad_input(Key::Next);
    assert_eq!(*log.borrow(), vec![EventKind::Defocused, EventKind::Focused]);
    assert_eq!(ui.state(b) & qingui::node::State::FOCUSED, qingui::node::State::FOCUSED);
}

#[test]
fn enter_clicks_button() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonCfg::new("A").build(&mut ui, scr);
    ui.add_event_cb(a, EventKind::Clicked, Box::new(logger(&log)));
    ui.group_add(a);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}

#[test]
fn slider_edit_mode() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100).build(&mut ui, scr);
    ui.add_event_cb(s, EventKind::ValueChanged, Box::new(logger(&log)));
    ui.group_add(s);
    ui.keypad_input(Key::Right);
    assert_eq!(ui.value(s), 0); // not in edit mode: Right is focus navigation (only one object in the group, value unchanged)
    ui.keypad_input(Key::Enter); // enter edit mode
    assert!(ui.state(s).contains(qingui::node::State::EDITED));
    ui.keypad_input(Key::Right);
    assert_eq!(ui.value(s), 1);
    ui.keypad_input(Key::Right);
    ui.keypad_input(Key::Left);
    assert_eq!(ui.value(s), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged, EventKind::ValueChanged, EventKind::ValueChanged]);
    ui.keypad_input(Key::Esc); // exit edit mode
    assert!(!ui.state(s).contains(qingui::node::State::EDITED));
}

#[test]
fn switch_toggles_on_enter() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let sw = SwitchBuilder::new().build(&mut ui, scr);
    ui.add_event_cb(sw, EventKind::ValueChanged, Box::new(logger(&log)));
    ui.group_add(sw);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    assert_eq!(ui.value(sw), 1); // Switch value: on=1 off=0
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(sw), 0);
}

#[test]
fn set_value_fires_value_changed() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let b = BarBuilder::new(0, 100).build(&mut ui, scr);
    ui.add_event_cb(b, EventKind::ValueChanged, Box::new(logger(&log)));
    ui.set_value(b, 42);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
}

#[test]
fn focus_skips_hidden_objects() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let page = ObjCfg::new().build(&mut ui, scr);
    let a = ButtonCfg::new("A").build(&mut ui, page); // hidden with page
    let b = ButtonCfg::new("B").build(&mut ui, scr);
    ui.group_add(a);
    ui.group_add(b);
    ui.set_hidden(page, true);
    // Current focus is on a (hidden) → Next should jump to b
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(b));
    // Wrapping also skips a
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(b));
    ui.keypad_input(Key::Prev);
    assert_eq!(ui.focused(), Some(b));
}

#[test]
fn modal_restricts_focus_navigation() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonCfg::new("A").build(&mut ui, scr);
    let dlg = ObjCfg::new().build(&mut ui, scr);
    let ok = ButtonCfg::new("OK").build(&mut ui, dlg);
    ui.group_add(a);
    ui.group_add(ok);
    // Before modal is set, focus can cycle between a/ok
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(ok));
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(a));
    // Set modal: focus locks into the dlg subtree
    ui.set_modal(dlg);
    assert_eq!(ui.focused(), Some(ok));
    ui.keypad_input(Key::Next); // can only cycle within the modal, cannot reach a
    assert_eq!(ui.focused(), Some(ok));
    // Clear modal: global navigation is restored
    ui.clear_modal();
    ui.keypad_input(Key::Prev);
    assert_eq!(ui.focused(), Some(a));
}
