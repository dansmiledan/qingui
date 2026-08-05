use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Color, Point, Rect};
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

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
}

/// Draws only the data lines (background/border are handled by the common draw_node):
/// adjacent points are connected with lines, a single-point series draws a dot, empty series are skipped. No allocation.
pub(crate) fn draw(s: &ChartState, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
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
                Some(q) => d.draw_line(q, p, 2, ser.color, ctx.ap(255), clip),
                None if n == 1 => d.fill_circle(p, 1, ser.color, ctx.ap(255), clip),
                None => {}
            }
            prev = Some(p);
        }
    }
}

/// Chart builder: default 120x60 + range 0..100
pub struct ChartBuilder {
    min: i32,
    max: i32,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
    series: Vec<(Color, usize)>,
}

impl ChartBuilder {
    /// Creates a builder with the default range 0..100.
    pub fn new() -> Self {
        Self {
            min: 0, max: 100,
            size: None, style: None, sizing: None,
            transition: None, events: Vec::new(), series: Vec::new(),
        }
    }
    /// Sets the fixed Y-axis range.
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    /// Sets the widget size.
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }
    /// Sets the style.
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }
    /// Sets the width/height sizing.
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h));
        self
    }
    /// Sets the transition duration and easing.
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    /// Registers an event callback.
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }
    /// Pre-creates one series (may be called multiple times)
    pub fn series(mut self, color: Color, capacity: usize) -> Self {
        self.series.push((color, capacity));
        self
    }

    /// Builds the widget into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((120, 60));
        let state = ChartState {
            min: self.min,
            max: self.max,
            series: self.series.into_iter().map(|(c, cap)| Series::new(c, cap)).collect(),
        };
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Chart(state));
        ui.set_style(r, self.style.unwrap_or_default());
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}

impl super::WidgetBehavior for ChartState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self, ctx, d, clip) }
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
        self.kind(c)
            .and_then(|k| k.as_chart())
            .and_then(|s| s.series.get(series))
            .map(|ser| ser.points.len())
            .unwrap_or(0)
    }

    fn chart_point(&self, c: ObjRef, series: usize, idx: usize) -> Option<i32> {
        self.kind(c)
            .and_then(|k| k.as_chart())
            .and_then(|s| s.series.get(series))
            .and_then(|ser| ser.points.get(idx).copied())
    }
}
