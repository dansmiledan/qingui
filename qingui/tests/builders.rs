use qingui::layout::Sizing;
use qingui::style::Style;
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
    let a = ui.create_slider(ui.screen(), 0, 100);
    let b = SliderBuilder::new(0, 100).build(&mut ui, scr);
    assert_eq!(ui.rect(a), ui.rect(b));
    assert_eq!(ui.resolved_style(a), ui.resolved_style(b));
    assert_eq!(ui.value(a), ui.value(b));
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
    let r = ui.rect(s);
    assert_eq!((r.w, r.h), (140, 14));
    assert_eq!(ui.value(s), 50);
    let st = ui.resolved_style(s);
    assert_eq!(st.bg_color, Color::RED); // 覆盖生效
    assert_eq!(st.radius, 6); // 其余默认保留
    assert_eq!(st.sizing_w, Some(Sizing::GROW));
}

#[test]
fn button_builder_pressed_focused_styles() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ui.create_button(ui.screen(), "OK");
    let b = ButtonBuilder::new("OK").build(&mut ui, scr);
    assert_eq!(ui.rect(a), ui.rect(b));
    assert_eq!(ui.resolved_style(a), ui.resolved_style(b));
    // focused 样式也一致
    ui.set_state(a, qingui::node::State::FOCUSED, true);
    ui.set_state(b, qingui::node::State::FOCUSED, true);
    assert_eq!(ui.resolved_style(a), ui.resolved_style(b));
}

#[test]
fn list_builder_size_and_style() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ui.create_list(ui.screen(), &["x", "y", "z"]);
    let b = ListBuilder::new(&["x", "y", "z"]).build(&mut ui, scr);
    assert_eq!(ui.rect(a), ui.rect(b));
    assert_eq!(ui.list_len(a), ui.list_len(b));
}

#[test]
fn roller_dropdown_builders() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ui.create_roller(ui.screen(), &["A", "B"]);
    let b = RollerBuilder::new(&["A", "B"]).build(&mut ui, scr);
    assert_eq!(ui.rect(a), ui.rect(b));
    let c = ui.create_dropdown(ui.screen(), &["R", "G"]);
    let d = DropdownBuilder::new(&["R", "G"]).build(&mut ui, scr);
    assert_eq!(ui.rect(c), ui.rect(d));
    assert_eq!(ui.resolved_style(c), ui.resolved_style(d));
}

#[test]
fn msgbox_builder_structure() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let mb = MsgboxBuilder::new("Title", "Body").buttons(&["OK"]).build(&mut ui, scr);
    assert!(ui.is_valid(mb));
    assert_eq!(ui.msgbox_selected(mb), -1);
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
    ui.group_add(b);
    ui.keypad_input(qingui::input::Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}
