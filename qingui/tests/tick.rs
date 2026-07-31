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
    assert_eq!(ui.timer_handler(), 0); // 自转控件保持唤醒
}

#[test]
fn static_ui_sleeps_after_first_frame() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    LabelBuilder::new("hi").build(&mut ui, s);
    ui.tick_inc(16);
    ui.timer_handler(); // 首帧（渲染建屏脏区）
    assert_eq!(ui.timer_handler(), u32::MAX); // 无动画无效果 → 睡眠
}

#[test]
fn list_fx_expires_and_sleeps() {
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, s);
    l.list_select(&mut ui, 2); // 触发高亮滑动 fx（FX_DUR=200ms）
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // fx 活动
    ui.tick_inc(300); // 超过 FX_DUR
    ui.timer_handler();
    assert_eq!(ui.timer_handler(), u32::MAX); // fx 已过期 → 睡眠
}
