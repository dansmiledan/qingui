use qingui::anim::Easing;
use qingui::layout::{Align, Flex, FlexDir, Sizing};
use qingui::layout::Layout;
use qingui::widgets::obj::ObjCfg;
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
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_size(c, 200, 100);
    ui.set_layout(c, flex(Align::Start));
    let k = ObjCfg::new().build(&mut ui, c);
    ui.set_size(k, 20, 10);
    ui.set_transition(k, Some((100, Easing::Linear)));
    (ui, c, k)
}

#[test]
fn first_layout_does_not_animate() {
    let (mut ui, c, k) = setup();
    ui.set_layout(c, flex(Align::End));
    ui.timer_handler();
    // The first layout lands directly in place, no fly-in animation
    assert_eq!(ui.rect(k).x, 180);
    assert!(!ui.anim_running());
}

#[test]
fn layout_change_animates_to_target() {
    let (mut ui, c, k) = setup();
    ui.timer_handler(); // first layout lands in place (Start → x=0)
    assert_eq!(ui.rect(k).x, 0);
    // Layout change → automatically transitions to the target position
    ui.set_layout(c, flex(Align::End));
    ui.timer_handler();
    assert!(ui.anim_running());
    assert!(ui.rect(k).x < 180); // still in transit
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 180);
    assert!(!ui.anim_running());
}

#[test]
fn layout_resize_animates_width() {
    let (mut ui, c, k) = setup();
    ui.set_sizing(k, Some(Sizing::GROW), None);
    ui.timer_handler(); // first time: w=200
    assert_eq!(ui.rect(k).w, 200);
    ui.set_size(c, 100, 100); // container shrinks → target w=100
    ui.timer_handler();
    assert!(ui.anim_running());
    assert!(ui.rect(k).w > 100); // still in transit
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(k).w, 100);
}

#[test]
fn transition_converges_with_small_ticks() {
    let (mut ui, c, k) = setup();
    ui.set_sizing(k, Some(Sizing::GROW), None);
    ui.timer_handler(); // first time: w=200
    ui.set_size(c, 100, 100);
    // Simulate 60fps small steps: the animation must not be repeatedly restarted by layout recomputation; it must converge
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
    ui.set_transition(k, None); // disable transition
    ui.timer_handler();
    ui.set_layout(c, flex(Align::End));
    ui.timer_handler();
    assert_eq!(ui.rect(k).x, 180); // teleports
    assert!(!ui.anim_running());
}
