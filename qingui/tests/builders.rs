use qingui::layout::{Align, Flex, FlexDir, Sizing};
use qingui::prelude::*;
use qingui::style::Style;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::dropdown::DropdownCfg;
use qingui::widgets::list::ListCfg;
use qingui::widgets::msgbox::MsgboxBuilder;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::roller::RollerCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::widgets::Layout;
use qingui::{Color, EventKind, Ui};

#[test]
fn slider_builder_defaults() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let s = SliderCfg::new(0, 100).build(&mut ui, scr);
    let r = ui.rect(s);
    assert_eq!((r.w, r.h), (100, 12)); // default size
    assert_eq!(ui.value(s), 0); // default value = min
    let st = ui.resolved_style(s); // theme_slider
    assert_eq!(st.bg_color, Some(Color::rgb(70, 70, 80)));
    assert_eq!(st.radius, 6);
    assert_eq!(st.text_color, Color::WHITE);
    assert_eq!(st.border_width, 0);
    // theme_slider_focused: white 2px border, other fields fall back to theme_slider
    ui.set_state(s, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(s);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 2);
    assert_eq!(st.bg_color, Some(Color::rgb(70, 70, 80)));
}

#[test]
fn slider_builder_overrides() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    // A flex parent so the child's GROW sizing is observable after a layout pass.
    let parent = ObjCfg::new()
        .size(160, 40)
        .layout(Layout::Flex(Flex {
            dir: FlexDir::Row, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
        }))
        .build(&mut ui, scr);
    let s = SliderCfg::new(0, 100)
        .size(140, 14)
        .value(50)
        .style_with(|s| s.bg(Color::RED))
        .sizing(Some(Sizing::GROW), None)
        .build(&mut ui, parent);
    let r = ui.rect(s);
    assert_eq!((r.w, r.h), (140, 14));
    assert_eq!(ui.value(s), 50);
    let st = ui.resolved_style(s);
    assert_eq!(st.bg_color, Some(Color::RED)); // override takes effect
    assert_eq!(st.radius, 6); // other defaults kept
    ui.layout();
    assert_eq!(ui.rect(s).w, 160); // GROW sizing fills the parent
}

#[test]
fn button_builder_focused_styles() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let b = ButtonCfg::new("OK").build(&mut ui, scr);
    // Default size = text size + (24, 12); "OK" is 2 FONT_6X10 glyphs (6x10)
    let r = ui.rect(b);
    assert_eq!((r.w, r.h), (2 * 6 + 24, 10 + 12));
    // theme_button
    let st = ui.resolved_style(b);
    assert_eq!(st.bg_color, Some(Color::rgb(60, 90, 160)));
    assert_eq!(st.radius, 6);
    assert_eq!(st.border_color, Color::rgb(90, 120, 200));
    assert_eq!(st.border_width, 1);
    assert_eq!(st.text_color, Color::WHITE);
    // theme_button_focused: white 2px border, other fields fall back to theme_button
    ui.set_state(b, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(b);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 2);
    assert_eq!(st.bg_color, Some(Color::rgb(60, 90, 160)));
}

#[test]
fn list_builder_size_and_style() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["x", "y", "z"]).build(&mut ui, scr);
    // Default size: width 120, height min(5, n)*16 + 2
    let r = ui.rect(l);
    assert_eq!((r.w, r.h), (120, 3 * 16 + 2));
    assert_eq!(ui.list_len(l), 3);
    // theme_list
    let st = ui.resolved_style(l);
    assert_eq!(st.bg_color, Some(Color::rgb(34, 34, 44)));
    assert_eq!(st.radius, 4);
    assert_eq!(st.border_color, Color::rgb(70, 70, 90));
    assert_eq!(st.border_width, 1);
    assert_eq!(st.text_color, Color::WHITE);
    // theme_list_focused: white border (width falls back to theme_list's 1px)
    ui.set_state(l, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(l);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 1);
    assert_eq!(st.bg_color, Some(Color::rgb(34, 34, 44)));
}

#[test]
fn list_edited_style_default_and_override() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    // Default: edited style derives from the focused overlay (amber accent).
    let l = ListCfg::new(&["x", "y", "z"]).build(&mut ui, scr);
    ui.set_state(l, qingui::node::State::FOCUSED | qingui::node::State::EDITED, true);
    let st = ui.resolved_style(l);
    assert_eq!(st.border_color, qingui::style::EDIT_ACCENT);
    assert_eq!(st.border_width, 1); // width falls back to theme_list's 1px
    // Explicit style_edited wins over the theme_edited default.
    let l2 = ListCfg::new(&["x"]).style_edited(Style::new().border(Color::GREEN, 3)).build(&mut ui, scr);
    ui.set_state(l2, qingui::node::State::FOCUSED | qingui::node::State::EDITED, true);
    let st = ui.resolved_style(l2);
    assert_eq!(st.border_color, Color::GREEN);
    assert_eq!(st.border_width, 3);
}

#[test]
fn roller_dropdown_builders() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    // Roller default size: 80 x (min(3, n)*16 + 8)
    let ro = RollerCfg::new(&["A", "B"]).build(&mut ui, scr);
    let r = ui.rect(ro);
    assert_eq!((r.w, r.h), (80, 2 * 16 + 8));
    let st = ui.resolved_style(ro);
    assert_eq!(st.bg_color, Some(Color::rgb(34, 34, 44)));
    assert_eq!(st.radius, 4);
    assert_eq!(st.text_color, Color::WHITE);
    // Roller focused default: white 1px border
    ui.set_state(ro, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(ro);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 1);
    assert_eq!(st.bg_color, Some(Color::rgb(34, 34, 44)));
    // Dropdown default size: 100 x 20
    let dd = DropdownCfg::new(&["R", "G"]).build(&mut ui, scr);
    let r = ui.rect(dd);
    assert_eq!((r.w, r.h), (100, 20));
    let st = ui.resolved_style(dd);
    assert_eq!(st.bg_color, Some(Color::rgb(40, 40, 52)));
    assert_eq!(st.radius, 4);
    assert_eq!(st.text_color, Color::WHITE);
    // Dropdown focused default: white 1px border
    ui.set_state(dd, qingui::node::State::FOCUSED, true);
    let st = ui.resolved_style(dd);
    assert_eq!(st.border_color, Color::WHITE);
    assert_eq!(st.border_width, 1);
    assert_eq!(st.bg_color, Some(Color::rgb(40, 40, 52)));
}

#[test]
fn msgbox_builder_structure() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let mb = MsgboxBuilder::new("Title", "Body").buttons(&["OK"]).build(&mut ui, scr);
    assert!(ui.is_valid(mb));
    assert_eq!(ui.msgbox_selected(mb), -1);
    // Modal already set: focus is inside the msgbox subtree
    assert!(ui.focused().is_some());
}

#[test]
fn generic_style_with_composes_with_prior_style() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    // .style_with(f) alone bases on the widget's default_style() (theme_button).
    let a = ButtonCfg::new("A").style_with(|s| s.bg(Color::RED)).build(&mut ui, scr);
    let st = ui.resolved_style(a);
    assert_eq!(st.bg_color, Some(Color::RED)); // f applied
    assert_eq!(st.radius, 6); // inherited from theme_button default
    assert_eq!(st.border_width, 1); // inherited from theme_button default
    // .style(s).style_with(f) composes: f(s), not f(default_style()).
    let b = ButtonCfg::new("B")
        .style(Style::new().bg(Color::GREEN).radius(9))
        .style_with(|s| s.bg(Color::RED))
        .build(&mut ui, scr);
    let st = ui.resolved_style(b);
    assert_eq!(st.bg_color, Some(Color::RED)); // f applied to s
    assert_eq!(st.radius, 9); // preserved from s, not theme_button default's 6
}

#[test]
fn builder_event_registration() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let b = ButtonCfg::new("OK")
        .on(EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)))
        .build(&mut ui, scr);
    ui.group_add(b);
    ui.keypad_input(qingui::input::Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}
