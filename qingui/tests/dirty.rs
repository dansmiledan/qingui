use qingui::{Rect, Ui};

#[test]
fn move_obj_marks_old_and_new_area() {
    let mut ui = Ui::new(320, 240, 40);
    ui.take_dirty(); // 清掉建屏时的全屏脏
    let o = ui.create_obj(ui.screen());
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
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 10, 10);
    ui.set_size(o, 20, 20);
    ui.take_dirty();
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(qingui::Color::RED);
    ui.set_style(o, s);
    assert_eq!(ui.take_dirty(), vec![Rect::new(10, 10, 20, 20)]);
}
