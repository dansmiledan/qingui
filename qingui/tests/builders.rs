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
    let r = s.rect(&ui);
    assert_eq!((r.w, r.h), (100, 12)); // 默认尺寸
    assert_eq!(s.value(&ui), 0); // 默认 value = min
    let st = s.resolved_style(&ui); // theme_slider
    assert_eq!(st.bg_color, Color::rgb(70, 70, 80));
    assert_eq!(st.radius, 6);
    assert_eq!(st.bg_opa, 255);
    assert_eq!(st.text_color, Color::WHITE);
    assert_eq!(st.border_width, 0);
    // theme_slider_focused：白边 2px，其余字段回落 theme_slider
    s.set_state(&mut ui, qingui::node::State::FOCUSED, true);
    let st = s.resolved_style(&ui);
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
    let b = ButtonBuilder::new("OK").build(&mut ui, scr);
    // 默认尺寸 = 文本尺寸 + (24, 12)；"OK" 为 2 个 8x8 字模
    let r = b.rect(&ui);
    assert_eq!((r.w, r.h), (2 * 8 + 24, 8 + 12));
    // theme_button
    let st = b.resolved_style(&ui);
    assert_eq!(st.bg_color, Color::rgb(60, 90, 160));
    assert_eq!(st.radius, 6);
    assert_eq!(st.border_color, Color::rgb(90, 120, 200));
    assert_eq!(st.border_width, 1);
    assert_eq!(st.text_color, Color::WHITE);
    // theme_button_pressed：只覆盖背景，其余回落 theme_button
    b.set_state(&mut ui, qingui::node::State::PRESSED, true);
    let st = b.resolved_style(&ui);
    assert_eq!(st.bg_color, Color::rgb(40, 60, 110));
    assert_eq!(st.radius, 6);
    assert_eq!(st.border_color, Color::rgb(90, 120, 200));
    b.set_state(&mut ui, qingui::node::State::PRESSED, false);
    // theme_button_focused：白边 2px，其余回落 theme_button
    b.set_state(&mut ui, qingui::node::State::FOCUSED, true);
    let st = b.resolved_style(&ui);
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
    let r = l.rect(&ui);
    assert_eq!((r.w, r.h), (120, 3 * 16 + 2));
    assert_eq!(l.list_len(&ui), 3);
    // theme_list
    let st = l.resolved_style(&ui);
    assert_eq!(st.bg_color, Color::rgb(34, 34, 44));
    assert_eq!(st.radius, 4);
    assert_eq!(st.border_color, Color::rgb(70, 70, 90));
    assert_eq!(st.border_width, 1);
    assert_eq!(st.text_color, Color::WHITE);
    // theme_list_focused：白边（宽度回落 theme_list 的 1px）
    l.set_state(&mut ui, qingui::node::State::FOCUSED, true);
    let st = l.resolved_style(&ui);
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
    let r = ro.rect(&ui);
    assert_eq!((r.w, r.h), (80, 2 * 16 + 8));
    let st = ro.resolved_style(&ui);
    assert_eq!(st.bg_color, Color::rgb(34, 34, 44));
    assert_eq!(st.radius, 4);
    assert_eq!(st.text_color, Color::WHITE);
    // Roller focused 默认：白边 1px
    ro.set_state(&mut ui, qingui::node::State::FOCUSED, true);
    let st = ro.resolved_style(&ui);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 1);
    assert_eq!(st.bg_color, Color::rgb(34, 34, 44));
    // Dropdown 默认尺寸：100 x 20
    let dd = DropdownBuilder::new(&["R", "G"]).build(&mut ui, scr);
    let r = dd.rect(&ui);
    assert_eq!((r.w, r.h), (100, 20));
    let st = dd.resolved_style(&ui);
    assert_eq!(st.bg_color, Color::rgb(40, 40, 52));
    assert_eq!(st.radius, 4);
    assert_eq!(st.text_color, Color::WHITE);
    // Dropdown focused 默认：白边 1px
    dd.set_state(&mut ui, qingui::node::State::FOCUSED, true);
    let st = dd.resolved_style(&ui);
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
