use qingui::display::Flush;
use qingui::prelude::*;
use qingui::widgets::chart::ChartBuilder;
use qingui::Rect;
use qingui::{Color, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn builder_defaults_and_add_series() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    // 默认 0 条序列；预建 1 条（容量 4）
    let c = ChartBuilder::new().series(Color::BLUE, 4).build(&mut ui, s);
    assert_eq!(ui.chart_point_count(c, 0), 0);
    assert_eq!(ui.chart_point_count(c, 1), 0); // 越界序列 → 0
    // 运行时再挂一条，索引递增
    assert_eq!(ui.chart_add_series(c, Color::RED, 8), 1);
    assert_eq!(ui.chart_point_count(c, 1), 0);
}

#[test]
fn push_appends_and_clamps() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartBuilder::new().range(0, 100).series(Color::BLUE, 4).build(&mut ui, s);
    ui.chart_push(c, 0, -5);  // clamp 到 0
    ui.chart_push(c, 0, 150); // clamp 到 100
    ui.chart_push(c, 0, 42);
    assert_eq!(ui.chart_point_count(c, 0), 3);
    assert_eq!(ui.chart_point(c, 0, 0), Some(0));
    assert_eq!(ui.chart_point(c, 0, 1), Some(100));
    assert_eq!(ui.chart_point(c, 0, 2), Some(42));
    assert_eq!(ui.chart_point(c, 0, 3), None);
}

#[test]
fn push_evicts_oldest_when_full() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartBuilder::new().range(0, 100).series(Color::BLUE, 3).build(&mut ui, s);
    for v in [1, 2, 3, 4] {
        ui.chart_push(c, 0, v);
    }
    assert_eq!(ui.chart_point_count(c, 0), 3);
    assert_eq!(ui.chart_point(c, 0, 0), Some(2)); // 最旧的 1 被挤出
    assert_eq!(ui.chart_point(c, 0, 2), Some(4));
}

#[test]
fn set_points_replaces_and_truncates() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartBuilder::new().range(0, 100).series(Color::BLUE, 3).build(&mut ui, s);
    ui.chart_push(c, 0, 99);
    ui.chart_set_points(c, 0, &[1, 2, 3, 4, 5]); // 超容量只留最新 3 个
    assert_eq!(ui.chart_point_count(c, 0), 3);
    assert_eq!(ui.chart_point(c, 0, 0), Some(3));
    assert_eq!(ui.chart_point(c, 0, 2), Some(5));
    ui.chart_clear(c, 0);
    assert_eq!(ui.chart_point_count(c, 0), 0);
}

#[test]
fn invalid_targets_are_silent_noop() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartBuilder::new().series(Color::BLUE, 3).build(&mut ui, s);
    // 越界序列索引
    ui.chart_push(c, 99, 5);
    ui.chart_set_points(c, 99, &[1, 2]);
    ui.chart_clear(c, 99);
    assert_eq!(ui.chart_point_count(c, 99), 0);
    assert_eq!(ui.chart_point(c, 99, 0), None);
    // 已删除的对象
    ui.delete(c);
    ui.chart_push(c, 0, 5); // 不 panic
    assert_eq!(ui.chart_point_count(c, 0), 0);
}

#[test]
fn push_marks_dirty() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartBuilder::new().series(Color::BLUE, 4).build(&mut ui, s);
    ui.take_dirty();
    assert!(ui.dirty_is_empty());
    ui.chart_push(c, 0, 10);
    assert!(!ui.dirty_is_empty());
}

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}

/// Rc 不是 fundamental type，orphan rule 要求包一层本地 newtype
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

#[test]
fn renders_flat_line_at_bottom_for_min_values() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48); // 整屏一个 chunk
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let s = ui.screen();
    // chart 铺满屏幕，容量=宽 → 每列一个点；全部推 min → 折线贴底行
    let c = ChartBuilder::new()
        .range(0, 47)
        .size(64, 48)
        .series(Color::RED, 64)
        .build(&mut ui, s);
    for _ in 0..64 {
        ui.chart_push(c, 0, 0);
    }
    ui.render();
    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 1);
    let px = &chunks[0].1;
    // y = 47 底行：min 值映射到 abs.y + h - 1；线宽 2 → 底两行都有色
    assert_eq!(px[47 * 64 + 10], Color::RED);
    assert_eq!(px[47 * 64 + 60], Color::RED);
    // 顶行不应有折线
    assert_ne!(px[10], Color::RED);
}
