use qingui::anim::{Anim, AnimProp, Easing};
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
    assert!(Easing::Overshoot.eval(0.7) > 1.0); // overshoot 中后段冲过终点
}

#[test]
fn linear_anim_progresses_with_tick() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
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
    // 结束后 timer_handler 返回 u32::MAX（无待唤醒任务）
    assert_eq!(ui.timer_handler(), u32::MAX);
}

#[test]
fn anim_with_delay() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    let mut a = anim_to(o, AnimProp::X, 100, 100);
    a.delay_ms = 100;
    ui.anim_start(a);
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 0); // delay 期间不动
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 100);
}

#[test]
fn playback_reverses() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    let mut a = anim_to(o, AnimProp::X, 100, 100);
    a.repeat = 2;
    a.playback = true;
    ui.anim_start(a);
    ui.tick_inc(100);
    ui.timer_handler(); // 第 1 轮结束 x=100
    assert_eq!(ui.rect(o).x, 100);
    ui.tick_inc(50);
    ui.timer_handler(); // 第 2 轮反向中点
    assert_eq!(ui.rect(o).x, 50);
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 0);
    assert!(!ui.anim_running());
}

#[test]
fn anim_stop_removes() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
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
    let o = ui.create_obj(ui.screen());
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
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.take_dirty();
    ui.anim_start(anim_to(s, AnimProp::Value, 100, 100));
    // anim_start 立即应用起始值 → 标脏（动画与脏矩形联动）
    assert!(!ui.dirty_is_empty());
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.value(s), 100);
}

#[test]
fn anim_x_on_flex_child_not_reset_by_layout() {
    use qingui::layout::{Align, Flex, FlexDir};
    use qingui::style::Layout;
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, 200, 100);
    ui.set_layout(c, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
    }));
    let k = ui.create_obj(c);
    ui.set_size(k, 20, 10);
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 0); // 布局计算位置
    // 动画写 x：set_pos 不标布局脏 → 布局不重算 → 动画值保持（未被布局重置为 0）
    ui.anim_start(anim_to(k, AnimProp::X, 50, 100));
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 25);
}

#[test]
fn translate_offsets_abs_rect_and_survives_layout() {
    use qingui::layout::{Align, Flex, FlexDir};
    use qingui::style::Layout;
    use qingui::Rect;
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, 200, 100);
    ui.set_layout(c, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
    }));
    let k = ui.create_obj(c);
    ui.set_size(k, 20, 10);
    ui.set_translate(k, 5, 7);
    ui.timer_handler();
    assert_eq!(ui.rect(k), Rect::new(0, 0, 20, 10)); // rect 不变
    assert_eq!(ui.abs_rect(k), Rect::new(5, 7, 20, 10)); // abs 叠加偏移
    ui.set_size(c, 150, 100); // 触发布局重算
    ui.timer_handler();
    assert_eq!(ui.abs_rect(k), Rect::new(5, 7, 20, 10)); // translate 保留
}

#[test]
fn anim_translate_x() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    ui.anim_start(anim_to(o, AnimProp::TranslateX, 100, 100));
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.abs_rect(o).x, 50);
    assert_eq!(ui.rect(o).x, 0); // 布局坐标不受影响
}
