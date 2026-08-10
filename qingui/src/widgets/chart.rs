use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// One data line: a fixed-capacity ring buffer that drops the oldest point when full
pub struct Series {
    pub color: Color,
    pub capacity: usize, // ≥1 (0 is clamped to 1)
    pub points: VecDeque<i32>,
}

impl Series {
    /// Creates a series with the given color and capacity (clamped to at least 1).
    pub fn new(color: Color, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { color, capacity, points: VecDeque::with_capacity(capacity) }
    }
    /// Appends a point (the caller has already clamped it to the range); drops the oldest when full
    pub fn push(&mut self, v: i32) {
        if self.points.len() == self.capacity {
            self.points.pop_front();
        }
        self.points.push_back(v);
    }
}

/// Chart widget state.
pub struct ChartState {
    pub min: i32, // fixed Y-axis range
    pub max: i32,
    pub series: Vec<Series>,
    pub line_width: i32,
}

/// Draws only the data lines (background/border are handled by the common draw_node):
/// adjacent points are connected with lines, a single-point series draws a dot, empty series are skipped. No allocation.
pub(crate) fn draw(s: &ChartState, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
    let abs = ctx.abs;
    if abs.w < 1 || abs.h < 1 {
        return;
    }
    let (min, max) = (s.min, s.max);
    for ser in &s.series {
        let n = ser.points.len();
        if n == 0 {
            continue;
        }
        let mut prev: Option<Point> = None;
        for (i, &v) in ser.points.iter().enumerate() {
            // X: positioned by capacity, filled from the left; with capacity==1 the single point is drawn at the horizontal center
            let x = if ser.capacity > 1 {
                abs.x + i as i32 * (abs.w - 1) / (ser.capacity as i32 - 1)
            } else {
                abs.x + abs.w / 2
            };
            // Y: clamp to [min,max] then map linearly; min==max draws a horizontal midline (avoids division by zero)
            let y = if max > min {
                let frac = (v.clamp(min, max) - min) as i64 * (abs.h - 1) as i64 / (max - min) as i64;
                abs.y + abs.h - 1 - frac as i32
            } else {
                abs.y + abs.h / 2
            };
            let p = Point { x, y };
            match prev {
                Some(q) => {
                    // Clip the segment to its half-open x-strip [q.x, p.x): adjacent
                    // capsules would otherwise overlap at the joint and blend the AA
                    // fringe twice, leaving a bright bulge at every data point. The
                    // last segment keeps its end cap (strip extends to the clip edge).
                    let seg_clip = if p.x > q.x {
                        let right = if i + 1 == n { clip.right() } else { p.x };
                        Rect::new(q.x, clip.y, right - q.x, clip.h).intersect(&clip)
                    } else {
                        // Same-column points (capacity > width): no strip to claim.
                        Some(clip)
                    };
                    if let Some(sc) = seg_clip {
                        d.draw_line(q, p, s.line_width, ser.color, ctx.ap(255), sc);
                    }
                }
                None if n == 1 => d.fill_circle(p, 1, ser.color, ctx.ap(255), clip),
                None => {}
            }
            prev = Some(p);
        }
    }
}

/// Builder for the Chart widget.
pub type ChartBuilder = WidgetBuilder<ChartCfg>;

/// Chart configuration: fixed Y-axis range and the initial series.
pub struct ChartCfg {
    min: i32,
    max: i32,
    series: Vec<(Color, usize)>,
    line_width: i32,
}

impl ChartCfg {
    /// Creates a builder with the default range 0..100.
    pub fn new() -> WidgetBuilder<ChartCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: ChartCfg { min: 0, max: 100, series: Vec::new(), line_width: 2 },
        }
    }
}

impl WidgetBuilder<ChartCfg> {
    /// Sets the fixed Y-axis range.
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.cfg.min = min;
        self.cfg.max = max;
        self
    }
    /// Pre-creates one series (may be called multiple times)
    pub fn series(mut self, color: Color, capacity: usize) -> Self {
        self.cfg.series.push((color, capacity));
        self
    }
    /// Sets the data line width in pixels (default 2).
    pub fn line_width(mut self, w: i32) -> Self {
        self.cfg.line_width = w;
        self
    }
}

impl WidgetCfg for ChartCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((120, 60));
        let state = ChartState {
            min: self.min,
            max: self.max,
            series: self.series.into_iter().map(|(c, cap)| Series::new(c, cap)).collect(),
            line_width: self.line_width,
        };
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(state));
        ui.set_style(r, common.style.take().unwrap_or_default());
        common.apply_tail(ui, r);
        r
    }
}

impl super::Widget for ChartState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(self, ctx, c, clip) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}

/// Chart data API (brought in via prelude or an explicit use)
pub trait UiChartExt {
    /// Adds a new series and returns its index.
    fn chart_add_series(&mut self, c: ObjRef, color: Color, capacity: usize) -> usize;
    /// Appends a point to a series (clamped to the chart's Y range).
    fn chart_push(&mut self, c: ObjRef, series: usize, v: i32);
    /// Replaces a series' points (keeps the last `capacity` points, clamped to the Y range).
    fn chart_set_points(&mut self, c: ObjRef, series: usize, points: &[i32]);
    /// Removes all points of a series.
    fn chart_clear(&mut self, c: ObjRef, series: usize);
    /// Returns the number of points in a series (0 if the series doesn't exist).
    fn chart_point_count(&self, c: ObjRef, series: usize) -> usize;
    /// Returns the idx-th point of a series, if any.
    fn chart_point(&self, c: ObjRef, series: usize, idx: usize) -> Option<i32>;
}

impl UiChartExt for Ui {
    fn chart_add_series(&mut self, c: ObjRef, color: Color, capacity: usize) -> usize {
        self.update::<ChartState, _>(c, move |s| {
            s.series.push(Series::new(color, capacity));
            s.series.len() - 1
        })
        .unwrap_or(0)
    }

    fn chart_push(&mut self, c: ObjRef, series: usize, v: i32) {
        self.update::<ChartState, _>(c, |s| {
            let (min, max) = (s.min, s.max);
            if let Some(ser) = s.series.get_mut(series) {
                ser.push(v.clamp(min, max));
            }
        });
    }

    fn chart_set_points(&mut self, c: ObjRef, series: usize, points: &[i32]) {
        self.update::<ChartState, _>(c, |s| {
            let (min, max) = (s.min, s.max);
            if let Some(ser) = s.series.get_mut(series) {
                let start = points.len().saturating_sub(ser.capacity);
                ser.points.clear();
                ser.points.extend(points[start..].iter().map(|&v| v.clamp(min, max)));
            }
        });
    }

    fn chart_clear(&mut self, c: ObjRef, series: usize) {
        self.update::<ChartState, _>(c, |s| {
            if let Some(ser) = s.series.get_mut(series) {
                ser.points.clear();
            }
        });
    }

    fn chart_point_count(&self, c: ObjRef, series: usize) -> usize {
        self.widget::<ChartState>(c)
            .and_then(|s| s.series.get(series))
            .map(|ser| ser.points.len())
            .unwrap_or(0)
    }

    fn chart_point(&self, c: ObjRef, series: usize, idx: usize) -> Option<i32> {
        self.widget::<ChartState>(c)
            .and_then(|s| s.series.get(series))
            .and_then(|ser| ser.points.get(idx).copied())
    }
}
