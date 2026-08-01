use qingui::layout::Sizing;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::dropdown::DropdownBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::msgbox::MsgboxBuilder;
use qingui::widgets::roller::RollerBuilder;
use qingui::widgets::slider::SliderBuilder;
use qingui::{Color, EventKind, Ui};

#[test]
fn slider_builder_defaults() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100).build(&mut ui, scr);
    let r = ui.rect(s);
    assert_eq!((r.w, r.h), (100, 12)); // 默认尺寸
    assert_eq!(ui.value(s), 0); // 默认 value = min
    let st = ui.resolved_style(s); // theme_slider
    assert_eq!(st.bg_color, Color::rgb(70, 70, 80));
    assert_eq!(st.radius, 6);
    assert_eq!(st.bg_opa, 255);
    assert_eq!(st.text_color, Color::WHITE);
    assert_eq!(st.border_width, 0);
    // theme_slider_focused：白边 2px，其余字段回落 theme_slider
    ui.set_state(s, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(s);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 2);
    assert_eq!(st.bg_color, Color::rgb(70, 70, 80));
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
    let b = ButtonBuilder::new("OK").build(&mut ui, scr);
    // 默认尺寸 = 文本尺寸 + (24, 12)；"OK" 为 2 个 8x8 字模
    let r = ui.rect(b);
    assert_eq!((r.w, r.h), (2 * 8 + 24, 8 + 12));
    // theme_button
    let st = ui.resolved_style(b);
    assert_eq!(st.bg_color, Color::rgb(60, 90, 160));
    assert_eq!(st.radius, 6);
    assert_eq!(st.border_color, Color::rgb(90, 120, 200));
    assert_eq!(st.border_width, 1);
    assert_eq!(st.text_color, Color::WHITE);
    // theme_button_pressed：只覆盖背景，其余回落 theme_button
    ui.set_state(b, qingui::node::State::PRESSED, true);
    let st = ui.resolved_style(b);
    assert_eq!(st.bg_color, Color::rgb(40, 60, 110));
    assert_eq!(st.radius, 6);
    assert_eq!(st.border_color, Color::rgb(90, 120, 200));
    ui.set_state(b, qingui::node::State::PRESSED, false);
    // theme_button_focused：白边 2px，其余回落 theme_button
    ui.set_state(b, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(b);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 2);
    assert_eq!(st.bg_color, Color::rgb(60, 90, 160));
}

#[test]
fn list_builder_size_and_style() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["x", "y", "z"]).build(&mut ui, scr);
    // 默认尺寸：宽 120，高 min(5, n)*16 + 2
    let r = ui.rect(l);
    assert_eq!((r.w, r.h), (120, 3 * 16 + 2));
    assert_eq!(ui.list_len(l), 3);
    // theme_list
    let st = ui.resolved_style(l);
    assert_eq!(st.bg_color, Color::rgb(34, 34, 44));
    assert_eq!(st.radius, 4);
    assert_eq!(st.border_color, Color::rgb(70, 70, 90));
    assert_eq!(st.border_width, 1);
    assert_eq!(st.text_color, Color::WHITE);
    // theme_list_focused：白边（宽度回落 theme_list 的 1px）
    ui.set_state(l, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(l);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 1);
    assert_eq!(st.bg_color, Color::rgb(34, 34, 44));
}

#[test]
fn roller_dropdown_builders() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    // Roller 默认尺寸：80 x (min(3, n)*16 + 8)
    let ro = RollerBuilder::new(&["A", "B"]).build(&mut ui, scr);
    let r = ui.rect(ro);
    assert_eq!((r.w, r.h), (80, 2 * 16 + 8));
    let st = ui.resolved_style(ro);
    assert_eq!(st.bg_color, Color::rgb(34, 34, 44));
    assert_eq!(st.radius, 4);
    assert_eq!(st.text_color, Color::WHITE);
    // Roller focused 默认：白边 1px
    ui.set_state(ro, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(ro);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 1);
    assert_eq!(st.bg_color, Color::rgb(34, 34, 44));
    // Dropdown 默认尺寸：100 x 20
    let dd = DropdownBuilder::new(&["R", "G"]).build(&mut ui, scr);
    let r = ui.rect(dd);
    assert_eq!((r.w, r.h), (100, 20));
    let st = ui.resolved_style(dd);
    assert_eq!(st.bg_color, Color::rgb(40, 40, 52));
    assert_eq!(st.radius, 4);
    assert_eq!(st.text_color, Color::WHITE);
    // Dropdown focused 默认：白边 1px
    ui.set_state(dd, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(dd);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 1);
    assert_eq!(st.bg_color, Color::rgb(40, 40, 52));
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
