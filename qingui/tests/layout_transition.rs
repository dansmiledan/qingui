use qingui::anim::Easing;
use qingui::layout::{Align, Flex, FlexDir, Sizing};
use qingui::style::Layout;
use qingui::{ObjRef, Ui};

fn flex(main: Align) -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main, cross: Align::Start, track: Align::Start, gap: 0,
    })
}

fn setup() -> (Ui, ObjRef, ObjRef) {
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, 200, 100);
    ui.set_layout(c, flex(Align::Start));
    let k = ui.create_obj(c);
    ui.set_size(k, 20, 10);
    ui.set_transition(k, Some((100, Easing::Linear)));
    (ui, c, k)
}

#[test]
fn first_layout_does_not_animate() {
    let (mut ui, c, k) = setup();
    ui.set_layout(c, flex(Align::End));
    ui.timer_handler();
    // 首次布局直接到位，不起飞入动画
    assert_eq!(ui.rect(k).x, 180);
    assert!(!ui.anim_running());
}

#[test]
fn layout_change_animates_to_target() {
    let (mut ui, c, k) = setup();
    ui.timer_handler(); // 首次布局到位（Start → x=0）
    assert_eq!(ui.rect(k).x, 0);
    // 布局变化 → 自动过渡到目标位置
    ui.set_layout(c, flex(Align::End));
    ui.timer_handler();
    assert!(ui.anim_running());
    assert!(ui.rect(k).x < 180); // 仍在途中
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 180);
    assert!(!ui.anim_running());
}

#[test]
fn layout_resize_animates_width() {
    let (mut ui, c, k) = setup();
    ui.set_sizing(k, Some(Sizing::GROW), None);
    ui.timer_handler(); // 首次：w=200
    assert_eq!(ui.rect(k).w, 200);
    ui.set_size(c, 100, 100); // 容器变小 → 目标 w=100
    ui.timer_handler();
    assert!(ui.anim_running());
    assert!(ui.rect(k).w > 100); // 仍在途中
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(k).w, 100);
}

#[test]
fn transition_converges_with_small_ticks() {
    let (mut ui, c, k) = setup();
    ui.set_sizing(k, Some(Sizing::GROW), None);
    ui.timer_handler(); // 首次：w=200
    ui.set_size(c, 100, 100);
    // 模拟 60fps 小步进：动画不能被布局重算反复重启，必须收敛
    for _ in 0..20 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    assert_eq!(ui.rect(k).w, 100);
    assert!(!ui.anim_running());
}

#[test]
fn no_transition_no_animation() {
    let (mut ui, c, k) = setup();
    ui.set_transition(k, None); // 关闭 transition
    ui.timer_handler();
    ui.set_layout(c, flex(Align::End));
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 180); // 瞬移
    assert!(!ui.anim_running());
}
