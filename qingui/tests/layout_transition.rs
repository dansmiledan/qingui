use qingui::anim::Easing;
use qingui::layout::{Align, Flex, FlexDir, Sizing};
use qingui::style::Layout;
use qingui::widgets::obj::ObjBuilder;
use qingui::{ObjRef, Ui};

fn flex(main: Align) -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main, cross: Align::Start, track: Align::Start, gap: 0,
    })
}

fn setup() -> (Ui, ObjRef, ObjRef) {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjBuilder::new().build(&mut ui, scr);
    c.set_size(&mut ui, 200, 100);
    c.set_layout(&mut ui, flex(Align::Start));
    let k = ObjBuilder::new().build(&mut ui, c);
    k.set_size(&mut ui, 20, 10);
    k.set_transition(&mut ui, Some((100, Easing::Linear)));
    (ui, c, k)
}

#[test]
fn first_layout_does_not_animate() {
    let (mut ui, c, k) = setup();
    c.set_layout(&mut ui, flex(Align::End));
    ui.timer_handler();
    // 首次布局直接到位，不起飞入动画
    assert_eq!(k.rect(&ui).x, 180);
    assert!(!ui.anim_running());
}

#[test]
fn layout_change_animates_to_target() {
    let (mut ui, c, k) = setup();
    ui.timer_handler(); // 首次布局到位（Start → x=0）
    assert_eq!(k.rect(&ui).x, 0);
    // 布局变化 → 自动过渡到目标位置
    c.set_layout(&mut ui, flex(Align::End));
    ui.timer_handler();
    assert!(ui.anim_running());
    assert!(k.rect(&ui).x < 180); // 仍在途中
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(k.rect(&ui).x, 180);
    assert!(!ui.anim_running());
}

#[test]
fn layout_resize_animates_width() {
    let (mut ui, c, k) = setup();
    k.set_sizing(&mut ui, Some(Sizing::GROW), None);
    ui.timer_handler(); // 首次：w=200
    assert_eq!(k.rect(&ui).w, 200);
    c.set_size(&mut ui, 100, 100); // 容器变小 → 目标 w=100
    ui.timer_handler();
    assert!(ui.anim_running());
    assert!(k.rect(&ui).w > 100); // 仍在途中
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(k.rect(&ui).w, 100);
}

#[test]
fn transition_converges_with_small_ticks() {
    let (mut ui, c, k) = setup();
    k.set_sizing(&mut ui, Some(Sizing::GROW), None);
    ui.timer_handler(); // 首次：w=200
    c.set_size(&mut ui, 100, 100);
    // 模拟 60fps 小步进：动画不能被布局重算反复重启，必须收敛
    for _ in 0..20 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    assert_eq!(k.rect(&ui).w, 100);
    assert!(!ui.anim_running());
}

#[test]
fn no_transition_no_animation() {
    let (mut ui, c, k) = setup();
    k.set_transition(&mut ui, None); // 关闭 transition
    ui.timer_handler();
    c.set_layout(&mut ui, flex(Align::End));
    ui.timer_handler();
    assert_eq!(k.rect(&ui).x, 180); // 瞬移
    assert!(!ui.anim_running());
}
