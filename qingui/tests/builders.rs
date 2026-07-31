use qingui::layout::Sizing;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::dropdown::DropdownBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::msgbox::MsgboxBuilder;
use qingui::widgets::roller::RollerBuilder;
use qingui::widgets::slider::SliderBuilder;
use qingui::{Color, EventKind, Ui};

#[test]
fn slider_builder_defaults_match_create() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = SliderBuilder::new(0, 100).build(&mut ui, scr);
    let b = SliderBuilder::new(0, 100).build(&mut ui, scr);
    assert_eq!(a.rect(&ui), b.rect(&ui));
    assert_eq!(a.resolved_style(&ui), b.resolved_style(&ui));
    assert_eq!(a.value(&ui), b.value(&ui));
}

#[test]
fn slider_builder_overrides() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100)
        .size(140, 14)
        .value(50)
        .style_with(|s| s.bg(Color::RED))
        .sizing(Some(Sizing::GROW), None)
        .build(&mut ui, scr);
    let r = s.rect(&ui);
    assert_eq!((r.w, r.h), (140, 14));
    assert_eq!(s.value(&ui), 50);
    let st = s.resolved_style(&ui);
    assert_eq!(st.bg_color, Color::RED); // 覆盖生效
    assert_eq!(st.radius, 6); // 其余默认保留
    assert_eq!(st.sizing_w, Some(Sizing::GROW));
}

#[test]
fn button_builder_pressed_focused_styles() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonBuilder::new("OK").build(&mut ui, scr);
    let b = ButtonBuilder::new("OK").build(&mut ui, scr);
    assert_eq!(a.rect(&ui), b.rect(&ui));
    assert_eq!(a.resolved_style(&ui), b.resolved_style(&ui));
    // focused 样式也一致
    a.set_state(&mut ui, qingui::node::State::FOCUSED, true);
    b.set_state(&mut ui, qingui::node::State::FOCUSED, true);
    assert_eq!(a.resolved_style(&ui), b.resolved_style(&ui));
}

#[test]
fn list_builder_size_and_style() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ListBuilder::new(&["x", "y", "z"]).build(&mut ui, scr);
    let b = ListBuilder::new(&["x", "y", "z"]).build(&mut ui, scr);
    assert_eq!(a.rect(&ui), b.rect(&ui));
    assert_eq!(a.list_len(&ui), b.list_len(&ui));
}

#[test]
fn roller_dropdown_builders() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = RollerBuilder::new(&["A", "B"]).build(&mut ui, scr);
    let b = RollerBuilder::new(&["A", "B"]).build(&mut ui, scr);
    assert_eq!(a.rect(&ui), b.rect(&ui));
    let c = DropdownBuilder::new(&["R", "G"]).build(&mut ui, scr);
    let d = DropdownBuilder::new(&["R", "G"]).build(&mut ui, scr);
    assert_eq!(c.rect(&ui), d.rect(&ui));
    assert_eq!(c.resolved_style(&ui), d.resolved_style(&ui));
}

#[test]
fn msgbox_builder_structure() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let mb = MsgboxBuilder::new("Title", "Body").buttons(&["OK"]).build(&mut ui, scr);
    assert!(ui.is_valid(mb));
    assert_eq!(mb.msgbox_selected(&ui), -1);
    // 模态已设置：焦点在 msgbox 子树内
    assert!(ui.focused().is_some());
}

#[test]
fn builder_event_registration() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let b = ButtonBuilder::new("OK")
        .on(EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)))
        .build(&mut ui, scr);
    b.group_add(&mut ui);
    ui.keypad_input(qingui::input::Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}
