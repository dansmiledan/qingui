use qingui::input::Key;
use qingui::prelude::*;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::scrollview::{ScrollViewCfg, STEP};
use qingui::{Rect, Ui};

/// Builds a 60px viewport + 3 40px items (content 120px)
fn build() -> (Ui, qingui::ObjRef, qingui::ObjRef) {
    let mut ui = Ui::new(160, 120, 24);
    let s = ui.screen();
    let sv = ScrollViewCfg::new().size(80, 60).build(&mut ui, s);
    let content = ui.scrollview_content(sv).unwrap();
    for _ in 0..3 {
        let item = ObjCfg::new().build(&mut ui, content);
        ui.set_size(item, 60, 40);
    }
    (ui, sv, content)
}

#[test]
fn builder_and_content_accessor() {
    let (ui, sv, content) = build();
    assert_eq!(ui.rect(sv), Rect::new(0, 0, 80, 60));
    assert_eq!(ui.children(sv), vec![content]);
    assert_eq!(ui.children(content).len(), 3);
    assert_eq!(ui.translate(content).y, 0);
    // Invalid target
    assert!(ui.scrollview_content(content).is_none());
}

#[test]
fn focused_up_down_scrolls_and_clamps() {
    let (mut ui, sv, content) = build();
    ui.group_add(sv);
    ui.group_focus(sv);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.translate(content).y, -STEP);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down); // 4 steps = 80, but clamped to -(120-60) = -60
    assert_eq!(ui.translate(content).y, -60);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.translate(content).y, -60 + STEP);
    for _ in 0..10 {
        ui.keypad_input(Key::Up); // no more movement once at the top (0)
    }
    assert_eq!(ui.translate(content).y, 0);
}

#[test]
fn short_content_never_scrolls() {
    let mut ui = Ui::new(160, 120, 24);
    let s = ui.screen();
    let sv = ScrollViewCfg::new().size(80, 60).build(&mut ui, s);
    let content = ui.scrollview_content(sv).unwrap();
    let item = ObjCfg::new().build(&mut ui, content);
    ui.set_size(item, 60, 30); // content 30 < viewport 60
    ui.group_add(sv);
    ui.group_focus(sv);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.translate(content).y, 0);
}

#[test]
fn scroll_to_programmatic() {
    let (mut ui, sv, content) = build();
    ui.scrollview_scroll_to(sv, -30);
    assert_eq!(ui.translate(content).y, -30);
    ui.scrollview_scroll_to(sv, -999); // clamped to -60
    assert_eq!(ui.translate(content).y, -60);
    ui.scrollview_scroll_to(sv, 50); // clamped to 0
    assert_eq!(ui.translate(content).y, 0);
    ui.scrollview_scroll_to(content, -10); // not a scrollview: silent no-op
    assert_eq!(ui.translate(content).y, 0);
}
