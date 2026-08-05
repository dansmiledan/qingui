use qingui::widgets::obj::ObjBuilder;
use qingui::{Rect, Ui};

#[test]
fn move_obj_marks_old_and_new_area() {
    let mut ui = Ui::new(320, 240, 40);
    ui.take_dirty(); // clear the full-screen dirty produced when the screen was created
    let scr = ui.screen();
    let o = ObjBuilder::new().build(&mut ui, scr);
    ui.set_pos(o, 10, 10);
    ui.set_size(o, 20, 20);
    ui.take_dirty();
    ui.set_pos(o, 50, 50);
    let dirty = ui.take_dirty();
    // The old and new areas do not intersect → two independent dirty rects
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
    // Over the cap, collapses to the full screen
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
    // The hide action itself must mark dirty (erase the object's original area)
    ui.set_hidden(panel, true);
    assert!(!ui.dirty_is_empty());
    ui.take_dirty();
    // After a real hide, setters no longer produce dirty areas
    ui.set_value(bar, 50);   // invalidate_obj path
    ui.set_pos(bar, 5, 5);   // invalidate_subtree path
    assert!(ui.take_dirty().is_empty());
    // Showing again must mark dirty (redraw)
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

    // timer_handler renders at its end, consuming dirty areas, so flush records the actual redraws
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
    // Infinite value animation (same as the demo animate page) + position animation (set_pos path)
    ui.anim_start(Anim { target: bar, prop: AnimProp::Value, start: 0, end: 100,
                         duration_ms: 1200, delay_ms: 0, repeat: -1, playback: false,
                         easing: Easing::Linear, on_done: None });
    ui.anim_start(Anim { target: bar, prop: AnimProp::X, start: 0, end: 50,
                         duration_ms: 1000, delay_ms: 0, repeat: -1, playback: false,
                         easing: Easing::Linear, on_done: None });
    ui.set_hidden(panel, true);
    ui.tick_inc(16);
    ui.timer_handler(); // erase frame
    rec.borrow_mut().n = 0;
    ui.tick_inc(16);
    ui.timer_handler();
    assert_eq!(rec.borrow().n, 0); // animations on a hidden page no longer trigger redraws
    // The animation itself still advances (the current value is visible when shown again)
    assert!(ui.anim_running());
    assert!(ui.value(bar) > 0);
}
