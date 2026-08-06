use qingui::prelude::*;
use qingui::widgets::label::LabelBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::spinner::SpinnerBuilder;
use qingui::Ui;

#[test]
fn spinner_keeps_timer_awake() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    SpinnerBuilder::new().build(&mut ui, s);
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // self-rotating widgets keep it awake
}

#[test]
fn hidden_parent_stops_spinner_dirty() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let panel = qingui::widgets::obj::ObjCfg::new().build(&mut ui, s);
    SpinnerBuilder::new().build(&mut ui, panel);
    ui.tick_inc(16);
    ui.timer_handler();
    ui.take_dirty();
    ui.set_hidden(panel, true); // hide the parent container
    ui.take_dirty(); // discard the dirty produced by the hide action itself
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), u32::MAX); // once hidden, the spinner is no longer active → sleeps
    assert!(ui.take_dirty().is_empty()); // and no dirty areas
}

#[test]
fn static_ui_sleeps_after_first_frame() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    LabelBuilder::new("hi").build(&mut ui, s);
    ui.tick_inc(16);
    ui.timer_handler(); // first frame (renders the screen-creation dirty area)
    assert_eq!(ui.timer_handler(), u32::MAX); // no animation, no effects → sleeps
}

#[test]
fn list_fx_expires_and_sleeps() {
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, s);
    ui.list_select(l, 2); // triggers the highlight-slide fx (FX_DUR=200ms)
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // fx active
    ui.tick_inc(300); // beyond FX_DUR
    ui.timer_handler();
    assert_eq!(ui.timer_handler(), u32::MAX); // fx expired → sleeps
}
