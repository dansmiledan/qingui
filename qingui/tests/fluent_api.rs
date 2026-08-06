use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::Sizing;
use qingui::style::Style;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::obj::ObjCfg;
use qingui::{Color, EventKind, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn style_builder_chain() {
    let s = Style::new().bg(Color::RED).bg_opa(200).border(Color::WHITE, 2).radius(4).pads(8);
    assert_eq!(s.bg_color, Some(Color::RED));
    assert_eq!(s.bg_opa, Some(200));
    assert_eq!(s.border_color, Some(Color::WHITE));
    assert_eq!(s.border_width, Some(2));
    assert_eq!(s.radius, Some(4));
    assert_eq!(s.pad_left, Some(8));
    assert_eq!(s.pad_bottom, Some(8));
}

#[test]
fn anim_builder_chain() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    ui.anim_start(
        Anim::new(o, AnimProp::X, 0, 100, 100)
            .easing(Easing::EaseInOutQuad)
            .repeat(2)
            .playback(true)
            .delay(50),
    );
    ui.tick_inc(50); // does not move during the delay
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 0);
    ui.tick_inc(100); // end of round 1
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 100);
}

#[test]
fn widget_mut_chain() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let b = ButtonBuilder::new("OK").build(&mut ui, scr);
    ui.set_pos(b, 10, 20);
    ui.set_size(b, 60, 30);
    ui.set_sizing(b, Some(Sizing::GROW), None);
    ui.set_z_index(b, 2);
    ui.group_add(b);
    ui.add_event_cb(b, EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    assert_eq!(ui.rect(b), Rect::new(10, 20, 60, 30));
    assert_eq!(ui.focused(), Some(b));
    ui.keypad_input(qingui::input::Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}
