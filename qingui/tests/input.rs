use qingui::input::Key;
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
    let a = ui.create_button(ui.screen(), "A");
    let b = ui.create_button(ui.screen(), "B");
    ui.group_add(a);
    ui.group_add(b);
    assert_eq!(ui.focused(), Some(a)); // 首个入组自动聚焦
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(b));
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(a)); // 循环
    ui.keypad_input(Key::Prev);
    assert_eq!(ui.focused(), Some(b));
}

#[test]
fn focus_events_and_state_flag() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let a = ui.create_button(ui.screen(), "A");
    let b = ui.create_button(ui.screen(), "B");
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
    let a = ui.create_button(ui.screen(), "A");
    ui.add_event_cb(a, EventKind::Clicked, Box::new(logger(&log)));
    ui.group_add(a);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}

#[test]
fn slider_edit_mode() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.add_event_cb(s, EventKind::ValueChanged, Box::new(logger(&log)));
    ui.group_add(s);
    ui.keypad_input(Key::Right);
    assert_eq!(ui.value(s), 0); // 非编辑态：Right 是焦点移动（组内仅一个对象，值不变）
    ui.keypad_input(Key::Enter); // 进入编辑态
    assert!(ui.state(s).contains(qingui::node::State::EDITED));
    ui.keypad_input(Key::Right);
    assert_eq!(ui.value(s), 1);
    ui.keypad_input(Key::Right);
    ui.keypad_input(Key::Left);
    assert_eq!(ui.value(s), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged, EventKind::ValueChanged, EventKind::ValueChanged]);
    ui.keypad_input(Key::Esc); // 退出编辑态
    assert!(!ui.state(s).contains(qingui::node::State::EDITED));
}

#[test]
fn switch_toggles_on_enter() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let sw = ui.create_switch(ui.screen());
    ui.add_event_cb(sw, EventKind::ValueChanged, Box::new(logger(&log)));
    ui.group_add(sw);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    assert_eq!(ui.value(sw), 1); // Switch 的 value：on=1 off=0
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(sw), 0);
}

#[test]
fn set_value_fires_value_changed() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let b = ui.create_bar(ui.screen(), 0, 100);
    ui.add_event_cb(b, EventKind::ValueChanged, Box::new(logger(&log)));
    ui.set_value(b, 42);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
}

#[test]
fn focus_skips_hidden_objects() {
    let mut ui = Ui::new(160, 120, 120);
    let page = ui.create_obj(ui.screen());
    let a = ui.create_button(page, "A"); // 随 page 隐藏
    let b = ui.create_button(ui.screen(), "B");
    ui.group_add(a);
    ui.group_add(b);
    ui.set_hidden(page, true);
    // 当前焦点在 a（隐藏）→ Next 应跳到 b
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(b));
    // 循环时同样跳过 a
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(b));
    ui.keypad_input(Key::Prev);
    assert_eq!(ui.focused(), Some(b));
}

#[test]
fn modal_restricts_focus_navigation() {
    let mut ui = Ui::new(160, 120, 120);
    let a = ui.create_button(ui.screen(), "A");
    let dlg = ui.create_obj(ui.screen());
    let ok = ui.create_button(dlg, "OK");
    ui.group_add(a);
    ui.group_add(ok);
    // 设置 modal 前焦点可在 a/ok 间循环
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(ok));
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(a));
    // 设置 modal：焦点锁进 dlg 子树
    ui.set_modal(dlg);
    assert_eq!(ui.focused(), Some(ok));
    ui.keypad_input(Key::Next); // 只能在 modal 内循环，到不了 a
    assert_eq!(ui.focused(), Some(ok));
    // 清除 modal：恢复全局导航
    ui.clear_modal();
    ui.keypad_input(Key::Prev);
    assert_eq!(ui.focused(), Some(a));
}
