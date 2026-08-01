use qingui::input::Key;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::{EventKind, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn up_down_navigates_items_not_focus() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    let btn = ButtonBuilder::new("x").build(&mut ui, scr);
    ui.group_add(l);
    ui.group_add(btn);
    assert_eq!(ui.focused(), Some(l));
    ui.keypad_input(Key::Down);
    assert_eq!(ui.list_selected(l), 1);
    assert_eq!(ui.focused(), Some(l)); // 焦点不动
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down); // 越界环绕
    assert_eq!(ui.list_selected(l), 0);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.list_selected(l), 2);
    ui.keypad_input(Key::Next); // Next 仍移动焦点
    assert_eq!(ui.focused(), Some(btn));
}

#[test]
fn enter_on_list_fires_clicked() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.add_event_cb(l, EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(l);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
    assert_eq!(ui.list_selected(l), 1);
}

#[test]
fn selection_keeps_visible_with_scroll() {
    let mut ui = Ui::new(160, 120, 120);
    // 8 项，可见 5 行（ListBuilder 默认高度上限 5 行 = 88px）
    let scr = ui.screen();
    let l = ListBuilder::new(&["0", "1", "2", "3", "4", "5", "6", "7"]).build(&mut ui, scr);
    ui.group_add(l);
    for _ in 0..7 {
        ui.keypad_input(Key::Down);
    }
    assert_eq!(ui.list_selected(l), 7);
    // scroll 已下滚保证第 7 行可见：scroll > 0
    let scroll = ui.as_list(l).unwrap().scroll;
    assert!(scroll > 0);
}
