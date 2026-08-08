use qingui::anim::{Anim, AnimProp, Easing};
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::Ui;

fn anim_to(target: qingui::ObjRef, prop: AnimProp, end: i32, dur: u32) -> Anim {
    Anim { target, prop, start: 0, end, duration_ms: dur, delay_ms: 0,
           repeat: 1, playback: false, easing: Easing::Linear, on_done: None }
}

#[test]
fn easing_bounds() {
    for e in [Easing::Linear, Easing::EaseInQuad, Easing::EaseOutQuad, Easing::EaseInOutQuad] {
        assert_eq!(e.eval(0.0), 0.0);
        assert!((e.eval(1.0) - 1.0).abs() < 1e-6);
    }
    assert!((Easing::Bounce.eval(1.0) - 1.0).abs() < 1e-6);
    assert!(Easing::Overshoot.eval(0.7) > 1.0); // overshoot passes the target in the latter part of the curve
}

#[test]
fn linear_anim_progresses_with_tick() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    ui.set_pos(o, 0, 0);
    ui.anim_start(anim_to(o, AnimProp::X, 100, 100));
    assert!(ui.anim_running());
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 50);
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 100);
    assert!(!ui.anim_running());
    // After it ends, timer_handler returns u32::MAX (no task waiting to wake up)
    assert_eq!(ui.timer_handler(), u32::MAX);
}

#[test]
fn anim_with_delay() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    let mut a = anim_to(o, AnimProp::X, 100, 100);
    a.delay_ms = 100;
    ui.anim_start(a);
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 0); // does not move during the delay
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 100);
}

#[test]
fn playback_reverses() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    let mut a = anim_to(o, AnimProp::X, 100, 100);
    a.repeat = 2;
    a.playback = true;
    ui.anim_start(a);
    ui.tick_inc(100);
    ui.timer_handler(); // end of round 1: x=100
    assert_eq!(ui.rect(o).x, 100);
    ui.tick_inc(50);
    ui.timer_handler(); // midpoint of the reverse round 2
    assert_eq!(ui.rect(o).x, 50);
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 0);
    assert!(!ui.anim_running());
}

#[test]
fn anim_stop_removes() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    ui.anim_start(anim_to(o, AnimProp::X, 100, 1000));
    ui.anim_stop(o, AnimProp::X);
    assert!(!ui.anim_running());
}

#[test]
fn on_done_callback_fires() {
    use std::cell::Cell;
    use std::rc::Rc;
    let fired = Rc::new(Cell::new(false));
    let fired2 = fired.clone();
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    let mut a = anim_to(o, AnimProp::X, 10, 10);
    a.on_done = Some(Box::new(move |_ui: &mut Ui| fired2.set(true)));
    ui.anim_start(a);
    ui.tick_inc(10);
    ui.timer_handler();
    assert!(fired.get());
}

#[test]
fn anim_value_updates_widget_and_dirty() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let s = SliderCfg::new(0, 100).build(&mut ui, scr);
    ui.take_dirty();
    ui.anim_start(anim_to(s, AnimProp::Value, 100, 100));
    // anim_start applies the start value immediately → marks dirty (animation linked with dirty rects)
    assert!(!ui.dirty_is_empty());
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.value(s), 100);
}

#[test]
fn anim_x_on_flex_child_not_reset_by_layout() {
    use qingui::layout::{Align, Flex, FlexDir};
    use qingui::layout::Layout;
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_size(c, 200, 100);
    ui.set_layout(c, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
    }));
    let k = ObjCfg::new().build(&mut ui, c);
    ui.set_size(k, 20, 10);
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 0); // layout-computed position
    // Animation writes x: set_pos does not mark layout dirty → layout does not recompute → the animated value is kept (not reset to 0 by layout)
    ui.anim_start(anim_to(k, AnimProp::X, 50, 100));
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 25);
}

#[test]
fn translate_offsets_abs_rect_and_survives_layout() {
    use qingui::layout::{Align, Flex, FlexDir};
    use qingui::layout::Layout;
    use qingui::Rect;
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_size(c, 200, 100);
    ui.set_layout(c, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
    }));
    let k = ObjCfg::new().build(&mut ui, c);
    ui.set_size(k, 20, 10);
    ui.set_translate(c, 5, 7); // parent container translated → whole subtree offsets
    ui.timer_handler();
    assert_eq!(ui.rect(k), Rect::new(0, 0, 20, 10)); // rect unchanged
    assert_eq!(ui.abs_rect(k), Rect::new(5, 7, 20, 10)); // child's abs also adds the parent translate
    ui.set_size(c, 150, 100); // triggers a layout recompute
    ui.timer_handler();
    assert_eq!(ui.abs_rect(k), Rect::new(5, 7, 20, 10)); // translate preserved
}

#[test]
fn anim_translate_x() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    ui.anim_start(anim_to(o, AnimProp::TranslateX, 100, 100));
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.abs_rect(o).x, 50);
    assert_eq!(ui.rect(o).x, 0); // layout coordinates unaffected
}
