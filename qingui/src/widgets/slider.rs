use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{WidgetCtx, WidgetKind};

/// Slider widget state: value drawn as a filled track between `min` and `max` with a knob.
#[derive(Clone)]
pub struct SliderState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
}

impl SliderState {
    pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        if ctx.edited {
            return match key {
                Key::Left | Key::Right => {
                    let d = if key == Key::Left { -1 } else { 1 };
                    let nv = (self.value + d).clamp(self.min, self.max);
                    if nv != self.value { self.value = nv; ValueChanged } else { Consumed }
                }
                Key::Enter | Key::Esc => ExitEdit,
                _ => Consumed,
            };
        }
        if key == Key::Enter { EnterEdit } else { Pass }
    }
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
    let kx = abs.x + iw;
    let knob = Rect::new(kx - 4, abs.y - 2, 8, abs.h + 4);
    let kc = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::WHITE };
    d.fill_rounded(knob, 3, kc, ctx.ap(255), clip);
}

/// Builder for the Slider widget.
pub type SliderBuilder = WidgetBuilder<SliderCfg>;

/// Slider configuration: value range and initial value.
pub struct SliderCfg {
    min: i32,
    max: i32,
    value: Option<i32>,
}

impl SliderCfg {
    /// Creates a builder for the given range.
    pub fn new(min: i32, max: i32) -> WidgetBuilder<SliderCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: SliderCfg { min, max, value: None } }
    }
}

impl WidgetBuilder<SliderCfg> {
    /// Sets the initial value.
    pub fn value(mut self, v: i32) -> Self {
        self.cfg.value = Some(v);
        self
    }
}

impl WidgetCfg for SliderCfg {
    fn default_style() -> Style {
        crate::style::theme_slider()
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((100, 12));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(WidgetKind::Slider(SliderState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min) })),
        );
        ui.set_style(r, common.style.take().unwrap_or_else(Self::default_style));
        ui.set_style_focused(r, common.style_focused.take().unwrap_or_else(crate::style::theme_slider_focused));
        common.apply_tail(ui, r);
        r
    }
}

impl super::WidgetBehavior for SliderState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.min, self.max, self.value, ctx, d, clip) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn set_range(&mut self, min: i32, max: i32) { self.min = min; self.max = max; self.value = self.value.clamp(min, max); }
    // Slider knob: ±4px horizontal, ±2px vertical
    fn overflow(&self) -> i32 { 4 }
}
