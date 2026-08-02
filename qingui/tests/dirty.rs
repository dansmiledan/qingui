use qingui::widgets::obj::ObjBuilder;
use qingui::{Rect, Ui};

#[test]
fn move_obj_marks_old_and_new_area() {
    let mut ui = Ui::new(320, 240, 40);
    ui.take_dirty(); // 清掉建屏时的全屏脏
    let scr = ui.screen();
    let o = ObjBuilder::new().build(&mut ui, scr);
    ui.set_pos(o, 10, 10);
    ui.set_size(o, 20, 20);
    ui.take_dirty();
    ui.set_pos(o, 50, 50);
    let dirty = ui.take_dirty();
    // 旧区域与新区域不相交 → 两个独立脏矩形
    assert_eq!(dirty.len(), 2);
    assert!(dirty.iter().any(|r| r.contains(qingui::Point { x: 10, y: 10 })));
    assert!(dirty.iter().any(|r| r.contains(qingui::Point { x: 60, y: 60 })));
}

#[test]
fn disjoint_areas_stay_separate_until_cap() {
    use qingui::dirty::DirtyQueue;
    let mut q = DirtyQueue::new(Rect::new(0, 0, 320, 240), 2);
    q.add(Rect::new(0, 0, 10, 10));
    q.add(Rect::new(100, 0, 10, 10));
    q.add(Rect::new(200, 0, 10, 10));
    // 超过 cap，坍缩为全屏
    assert_eq!(q.take(), vec![Rect::new(0, 0, 320, 240)]);
}

#[test]
fn area_clipped_to_screen() {
    let mut ui = Ui::new(320, 240, 40);
    ui.take_dirty();
    ui.invalidate_area(Rect::new(-50, -50, 100, 100));
    let dirty = ui.take_dirty();
    assert_eq!(dirty, vec![Rect::new(0, 0, 50, 50)]);
}

#[test]
fn style_change_invalidates_obj() {
    let mut ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let o = ObjBuilder::new().build(&mut ui, scr);
    ui.set_pos(o, 10, 10);
    ui.set_size(o, 20, 20);
    ui.take_dirty();
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(qingui::Color::RED);
    ui.set_style(o, s);
    assert_eq!(ui.take_dirty(), vec![Rect::new(10, 10, 20, 20)]);
}

#[test]
fn hidden_obj_setters_dont_dirty_but_hide_show_do() {
    use qingui::widgets::bar::BarBuilder;
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let panel = ObjBuilder::new().build(&mut ui, scr);
    ui.set_size(panel, 40, 40);
    let bar = BarBuilder::new(0, 100).build(&mut ui, panel);
    ui.take_dirty();
    // 隐藏动作本身必须标脏（擦除对象原区域）
    ui.set_hidden(panel, true);
    assert!(!ui.dirty_is_empty());
    ui.take_dirty();
    // 有效隐藏后，setter 不再产生脏区
    ui.set_value(bar, 50);   // invalidate_obj 路径
    ui.set_pos(bar, 5, 5);   // invalidate_subtree 路径
    assert!(ui.take_dirty().is_empty());
    // 重新显示必须标脏（重绘）
    ui.set_hidden(panel, false);
    assert!(!ui.dirty_is_empty());
}

#[test]
fn hidden_target_anim_does_not_dirty() {
    use qingui::anim::{Anim, AnimProp, Easing};
    use qingui::display::Flush;
    use qingui::widgets::bar::BarBuilder;
    use std::cell::RefCell;
    use std::rc::Rc;

    // timer_handler 末尾 render() 会消费脏区，故用 flush 记录实际重绘
    #[derive(Default)]
    struct RecFlush { n: usize }
    struct SharedFlush(Rc<RefCell<RecFlush>>);
    impl Flush for SharedFlush {
        fn flush(&mut self, _area: Rect, _pixels: &[qingui::Color]) {
            self.0.borrow_mut().n += 1;
        }
    }
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    let panel = ObjBuilder::new().build(&mut ui, scr);
    ui.set_size(panel, 40, 40);
    let bar = BarBuilder::new(0, 100).build(&mut ui, panel);
    // 无限值动画（demo animate 页同款）+ 位置动画（set_pos 路径）
    ui.anim_start(Anim { target: bar, prop: AnimProp::Value, start: 0, end: 100,
                         duration_ms: 1200, delay_ms: 0, repeat: -1, playback: false,
                         easing: Easing::Linear, on_done: None });
    ui.anim_start(Anim { target: bar, prop: AnimProp::X, start: 0, end: 50,
                         duration_ms: 1000, delay_ms: 0, repeat: -1, playback: false,
                         easing: Easing::Linear, on_done: None });
    ui.set_hidden(panel, true);
    ui.tick_inc(16);
    ui.timer_handler(); // 擦除帧
    rec.borrow_mut().n = 0;
    ui.tick_inc(16);
    ui.timer_handler();
    assert_eq!(rec.borrow().n, 0); // 隐藏页面上的动画不再触发重绘
    // 动画本身仍在推进（重新显示时能见到当前值）
    assert!(ui.anim_running());
    assert!(ui.value(bar) > 0);
}
