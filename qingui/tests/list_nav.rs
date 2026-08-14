use qingui::input::Key;
use qingui::prelude::*;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::list::ListCfg;
use qingui::{EventKind, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn up_down_navigates_items_not_focus() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    let btn = ButtonCfg::new("x").build(&mut ui, scr);
    ui.group_add(l);
    ui.group_add(btn);
    assert_eq!(ui.focused(), Some(l));
    ui.keypad_input(Key::Enter); // enter the inner (EDITED) mode
    ui.keypad_input(Key::Down);
    assert_eq!(ui.list_selected(l), 1);
    assert_eq!(ui.focused(), Some(l)); // focus does not move
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down); // wraps out of range
    assert_eq!(ui.list_selected(l), 0);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.list_selected(l), 2);
    ui.keypad_input(Key::Esc); // leave the inner mode
    ui.keypad_input(Key::Next); // Next still moves focus
    assert_eq!(ui.focused(), Some(btn));
}

#[test]
fn enter_on_list_fires_clicked() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.add_event_cb(l, EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(l);
    ui.keypad_input(Key::Enter); // enter the inner mode: no Click yet
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Enter); // confirm the selection: Click
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
    assert_eq!(ui.list_selected(l), 1);
}

#[test]
fn selection_keeps_visible_with_scroll() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    // 8 items, 5 rows visible (ListCfg default height cap is 5 rows = 88px)
    let scr = ui.screen();
    let l = ListCfg::new(&["0", "1", "2", "3", "4", "5", "6", "7"]).build(&mut ui, scr);
    ui.group_add(l);
    ui.keypad_input(Key::Enter); // enter the inner (EDITED) mode
    for _ in 0..7 {
        ui.keypad_input(Key::Down);
    }
    assert_eq!(ui.list_selected(l), 7);
    // scroll has scrolled down to keep row 7 visible: scroll > 0
    let scroll = ui.as_list(l).unwrap().scroll;
    assert!(scroll > 0);
}
