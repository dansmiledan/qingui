use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Bar widget state: value drawn as a filled track between `min` and `max`.
/// Bar widget state: value drawn as a filled track between `min` and `max`.
#[derive(Clone)]
pub struct BarState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
}

pub(crate) fn draw(min: i32, max: i32, value: i32, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
    let iw = (abs.w as f32 * frac) as i32;
    if iw > 0 {
        // Draw the indicator clipped to the full track's shape so the left end stays a half-circle aligned with the track
        let band = Rect::new(abs.x, abs.y, iw, abs.h);
        let ind_clip = band.intersect(&clip).unwrap_or(band);
        d.fill_rounded(abs, ctx.resolved.radius, Color::rgb(80, 140, 255), ctx.ap(255), ind_clip);
    }
}

/// Builder for the Bar widget.
pub type BarBuilder = WidgetBuilder<BarCfg>;

/// Bar configuration: value range and initial value.
pub struct BarCfg {
    min: i32,
    max: i32,
    value: Option<i32>,
}

impl BarCfg {
    /// Creates a builder for the given range.
    pub fn new(min: i32, max: i32) -> WidgetBuilder<BarCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: BarCfg { min, max, value: None } }
    }
}

impl WidgetBuilder<BarCfg> {
    /// Sets the initial value.
    pub fn value(mut self, v: i32) -> Self {
        self.cfg.value = Some(v);
        self
    }
}

impl WidgetCfg for BarCfg {
    fn default_style() -> Style {
        crate::style::theme_bar()
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((100, 8));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(BarState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min) }),
        );
        ui.set_style(r, common.style.take().unwrap_or_else(Self::default_style));
        common.apply_tail(ui, r);
        r
    }
}

impl super::Widget for BarState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(self.min, self.max, self.value, ctx, c, clip) }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn set_range(&mut self, min: i32, max: i32) { self.min = min; self.max = max; self.value = self.value.clamp(min, max); }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
