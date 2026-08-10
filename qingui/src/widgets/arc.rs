use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Dial start angle and sweep range (LVGL style: gap left at the bottom)
pub const START_DEG: i32 = 135;
/// Dial track sweep range in degrees
pub const SWEEP_DEG: i32 = 270;
/// Track arc line width in pixels
pub const TRACK_W: i32 = 4;

/// Arc widget state: value drawn as a dial arc between `min` and `max`.
#[derive(Clone)]
pub struct ArcState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
    pub track_w: i32,
    pub start_deg: i32,
    pub sweep_deg: i32,
}

/// Builder for the Arc widget.
pub type ArcBuilder = WidgetBuilder<ArcCfg>;

/// Arc configuration: value range and initial value.
pub struct ArcCfg {
    min: i32,
    max: i32,
    value: Option<i32>,
    track_w: i32,
    start_deg: i32,
    sweep_deg: i32,
}

impl ArcCfg {
    /// Creates a builder for the given range.
    pub fn new(min: i32, max: i32) -> WidgetBuilder<ArcCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ArcCfg { min, max, value: None, track_w: TRACK_W, start_deg: START_DEG, sweep_deg: SWEEP_DEG } }
    }
}

impl WidgetBuilder<ArcCfg> {
    /// Sets the initial value.
    pub fn value(mut self, v: i32) -> Self {
        self.cfg.value = Some(v);
        self
    }
    /// Sets the arc line width in pixels (default `TRACK_W` = 4).
    pub fn track_w(mut self, w: i32) -> Self {
        self.cfg.track_w = w;
        self
    }
    /// Sets the dial start angle in degrees (default `START_DEG` = 135).
    pub fn start_deg(mut self, deg: i32) -> Self {
        self.cfg.start_deg = deg;
        self
    }
    /// Sets the dial sweep range in degrees (default `SWEEP_DEG` = 270).
    pub fn sweep_deg(mut self, deg: i32) -> Self {
        self.cfg.sweep_deg = deg;
        self
    }
}

impl WidgetCfg for ArcCfg {
    fn default_style() -> Style {
        Style { bg_opa: Some(0), ..Style::default() }
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((60, 60));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(ArcState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min), track_w: self.track_w, start_deg: self.start_deg, sweep_deg: self.sweep_deg }),
        );
        let mut s = common.style.take().unwrap_or_else(Self::default_style);
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0);
        }
        ui.set_style(r, s);
        common.apply_tail(ui, r);
        r
    }
}

impl ArcState {
    fn draw_dial(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
        let abs = ctx.abs;
        let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
        let r = abs.w.min(abs.h) / 2 - 3;
        if r <= 0 {
            return;
        }
        let ap = |b: u8| ctx.ap(b);
        // Background arc (full track)
        d.draw_arc(c, r, self.track_w, self.start_deg, self.start_deg + self.sweep_deg, Color::rgb(70, 70, 80), ap(255), clip);
        // Indicator arc (turns yellow in edit mode)
        let frac = if self.max > self.min { (self.value - self.min) as f32 / (self.max - self.min) as f32 } else { 0.0 };
        let ind_end = self.start_deg + (self.sweep_deg as f32 * frac) as i32;
        if ind_end > self.start_deg {
            let ic = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::rgb(80, 140, 255) };
            d.draw_arc(c, r, self.track_w, self.start_deg, ind_end, ic, ap(255), clip);
        }
    }
}

impl super::Widget for ArcState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { self.draw_dial(ctx, c, clip) }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    // Arc knob extends ~3px past the edge
    fn overflow(&self) -> i32 { 4 }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
