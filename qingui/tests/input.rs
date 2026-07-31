use qingui::input::Key;
use qingui::widgets::bar::BarBuilder;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::obj::ObjBuilder;
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
    let a = ButtonBuilder::new("A").build(&mut ui, scr);
    let b = ButtonBuilder::new("B").build(&mut ui, scr);
    a.group_add(&mut ui);
    b.group_add(&mut ui);
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
    let scr = ui.screen();
    let a = ButtonBuilder::new("A").build(&mut ui, scr);
    let b = ButtonBuilder::new("B").build(&mut ui, scr);
    a.on(&mut ui, EventKind::Defocused, Box::new(logger(&log)));
    b.on(&mut ui, EventKind::Focused, Box::new(logger(&log)));
    a.group_add(&mut ui);
    b.group_add(&mut ui);
    ui.keypad_input(Key::Next);
    assert_eq!(*log.borrow(), vec![EventKind::Defocused, EventKind::Focused]);
    assert_eq!(b.state(&ui) & qingui::node::State::FOCUSED, qingui::node::State::FOCUSED);
}

#[test]
fn enter_clicks_button() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonBuilder::new("A").build(&mut ui, scr);
    a.on(&mut ui, EventKind::Clicked, Box::new(logger(&log)));
    a.group_add(&mut ui);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}

#[test]
fn slider_edit_mode() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100).build(&mut ui, scr);
    s.on(&mut ui, EventKind::ValueChanged, Box::new(logger(&log)));
    s.group_add(&mut ui);
    ui.keypad_input(Key::Right);
    assert_eq!(s.value(&ui), 0); // 非编辑态：Right 是焦点移动（组内仅一个对象，值不变）
    ui.keypad_input(Key::Enter); // 进入编辑态
    assert!(s.state(&ui).contains(qingui::node::State::EDITED));
    ui.keypad_input(Key::Right);
    assert_eq!(s.value(&ui), 1);
    ui.keypad_input(Key::Right);
    ui.keypad_input(Key::Left);
    assert_eq!(s.value(&ui), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged, EventKind::ValueChanged, EventKind::ValueChanged]);
    ui.keypad_input(Key::Esc); // 退出编辑态
    assert!(!s.state(&ui).contains(qingui::node::State::EDITED));
}

#[test]
fn switch_toggles_on_enter() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let sw = SwitchBuilder::new().build(&mut ui, scr);
    sw.on(&mut ui, EventKind::ValueChanged, Box::new(logger(&log)));
    sw.group_add(&mut ui);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    assert_eq!(sw.value(&ui), 1); // Switch 的 value：on=1 off=0
    ui.keypad_input(Key::Enter);
    assert_eq!(sw.value(&ui), 0);
}

#[test]
fn set_value_fires_value_changed() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let b = BarBuilder::new(0, 100).build(&mut ui, scr);
    b.on(&mut ui, EventKind::ValueChanged, Box::new(logger(&log)));
    b.set_value(&mut ui, 42);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
}

#[test]
fn focus_skips_hidden_objects() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let page = ObjBuilder::new().build(&mut ui, scr);
    let a = ButtonBuilder::new("A").build(&mut ui, page); // 随 page 隐藏
    let b = ButtonBuilder::new("B").build(&mut ui, scr);
    a.group_add(&mut ui);
    b.group_add(&mut ui);
    page.set_hidden(&mut ui, true);
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
    let scr = ui.screen();
    let a = ButtonBuilder::new("A").build(&mut ui, scr);
    let dlg = ObjBuilder::new().build(&mut ui, scr);
    let ok = ButtonBuilder::new("OK").build(&mut ui, dlg);
    a.group_add(&mut ui);
    ok.group_add(&mut ui);
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
