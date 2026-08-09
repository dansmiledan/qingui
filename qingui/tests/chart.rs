use qingui::display::Flush;
use qingui::prelude::*;
use qingui::widgets::chart::ChartCfg;
use qingui::Rect;
use qingui::{Color, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn builder_defaults_and_add_series() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    // Default is 0 series; pre-build 1 series (capacity 4)
    let c = ChartCfg::new().series(Color::BLUE, 4).build(&mut ui, s);
    assert_eq!(ui.chart_point_count(c, 0), 0);
    assert_eq!(ui.chart_point_count(c, 1), 0); // out-of-range series → 0
    // Attach one more series at runtime, index increments
    assert_eq!(ui.chart_add_series(c, Color::RED, 8), 1);
    assert_eq!(ui.chart_point_count(c, 1), 0);
}

#[test]
fn push_appends_and_clamps() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartCfg::new().range(0, 100).series(Color::BLUE, 4).build(&mut ui, s);
    ui.chart_push(c, 0, -5);  // clamped to 0
    ui.chart_push(c, 0, 150); // clamped to 100
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
    let c = ChartCfg::new().range(0, 100).series(Color::BLUE, 3).build(&mut ui, s);
    for v in [1, 2, 3, 4] {
        ui.chart_push(c, 0, v);
    }
    assert_eq!(ui.chart_point_count(c, 0), 3);
    assert_eq!(ui.chart_point(c, 0, 0), Some(2)); // the oldest 1 was evicted
    assert_eq!(ui.chart_point(c, 0, 2), Some(4));
}

#[test]
fn set_points_replaces_and_truncates() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartCfg::new().range(0, 100).series(Color::BLUE, 3).build(&mut ui, s);
    ui.chart_push(c, 0, 99);
    ui.chart_set_points(c, 0, &[1, 2, 3, 4, 5]); // over capacity: only the newest 3 are kept
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
    let c = ChartCfg::new().series(Color::BLUE, 3).build(&mut ui, s);
    // Out-of-range series index
    ui.chart_push(c, 99, 5);
    ui.chart_set_points(c, 99, &[1, 2]);
    ui.chart_clear(c, 99);
    assert_eq!(ui.chart_point_count(c, 99), 0);
    assert_eq!(ui.chart_point(c, 99, 0), None);
    // Deleted object
    ui.delete(c);
    ui.chart_push(c, 0, 5); // no panic
    assert_eq!(ui.chart_point_count(c, 0), 0);
}

#[test]
fn push_marks_dirty() {
    let mut ui = Ui::new(160, 120, 16);
    let s = ui.screen();
    let c = ChartCfg::new().series(Color::BLUE, 4).build(&mut ui, s);
    ui.take_dirty();
    assert!(ui.dirty_is_empty());
    ui.chart_push(c, 0, 10);
    assert!(!ui.dirty_is_empty());
}

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}

/// Rc is not a fundamental type, so the orphan rule requires wrapping it in a local wrapper struct
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

#[test]
fn polyline_has_no_bright_bulge_at_joints() {
    // A colinear polyline should render with uniform per-column brightness.
    // Previously each segment was drawn as an independent capsule whose round
    // caps overlap at the joints, blending the AA fringe twice and leaving a
    // bright bulge (~+20% ink) at every data point.
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let (w, h) = (190i32, 56i32);
    let mut ui = Ui::new(w, h, h as u32); // the whole screen is one chunk
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let s = ui.screen();
    let c = ChartCfg::new().range(0, 100).size(w, h).series(Color::RED, 48).build(&mut ui, s);
    // 48 points on the straight line y = 47 - x*43/189 (x = i*189/47)
    for i in 0..48 {
        let x = i * (w - 1) / 47;
        let v = ((47 - (47 - x as i64 * 43 / (w as i64 - 1)) as i32) * 100 + 27) / 55;
        ui.chart_push(c, 0, v);
    }
    ui.render();
    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 1);
    let px = &chunks[0].1;
    // Per-column red sum (stroke ink); interior columns only (skip end caps).
    let mut max_ink = 0u32;
    for x in 4..w as usize - 4 {
        let ink: u32 = (0..h as usize).map(|y| px[y * w as usize + x].r as u32).sum();
        max_ink = max_ink.max(ink);
    }
    // Ideal is ~2.12 * 255; overlapping joint caps pushed columns to ~2.59 * 255.
    let ink = max_ink as f32 / 255.0;
    assert!(ink <= 2.45, "joint bulge: max column ink {ink:.3} > 2.45");
}

#[test]
fn renders_flat_line_at_bottom_for_min_values() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48); // the whole screen is one chunk
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let s = ui.screen();
    // chart fills the screen, capacity = width → one point per column; all pushed to min → the polyline hugs the bottom row
    let c = ChartCfg::new()
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
    // y = 47 bottom row: min value maps to abs.y + h - 1; line width 2 → the bottom two rows are both colored
    assert_eq!(px[47 * 64 + 10], Color::RED);
    assert_eq!(px[47 * 64 + 60], Color::RED);
    // The top row should not have a polyline
    assert_ne!(px[10], Color::RED);
}
