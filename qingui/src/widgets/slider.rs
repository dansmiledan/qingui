use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Slider widget state: value drawn as a filled track between `min` and `max` with a knob.
#[derive(Clone)]
pub struct SliderState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
    pub knob_w: i32,
}

/// Builder for the Slider widget.
pub type SliderBuilder = WidgetBuilder<SliderCfg>;

/// Slider configuration: value range and initial value.
pub struct SliderCfg {
    min: i32,
    max: i32,
    value: Option<i32>,
    knob_w: i32,
}

impl SliderCfg {
    /// Creates a builder for the given range.
    pub fn new(min: i32, max: i32) -> WidgetBuilder<SliderCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: SliderCfg { min, max, value: None, knob_w: 8 } }
    }
}

impl WidgetBuilder<SliderCfg> {
    /// Sets the initial value.
    pub fn value(mut self, v: i32) -> Self {
        self.cfg.value = Some(v);
        self
    }

    /// Sets the knob width in pixels (default 8).
    pub fn knob_w(mut self, w: i32) -> Self {
        self.cfg.knob_w = w;
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
            alloc::boxed::Box::new(SliderState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min), knob_w: self.knob_w }),
        );
        ui.set_style(r, common.style.take().unwrap_or_else(Self::default_style));
        ui.set_style_focused(r, common.style_focused.take().unwrap_or_else(crate::style::theme_slider_focused));
        common.apply_tail(ui, r);
        r
    }
}

impl SliderState {
    fn draw_track(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
        let abs = ctx.abs;
        let frac = if self.max > self.min { (self.value - self.min) as f32 / (self.max - self.min) as f32 } else { 0.0 };
        let iw = (abs.w as f32 * frac) as i32;
        if iw > 0 {
            // Draw the indicator clipped to the full track's shape so the left end stays a half-circle aligned with the track
            let band = Rect::new(abs.x, abs.y, iw, abs.h);
            let ind_clip = band.intersect(&clip).unwrap_or(band);
            d.fill_rounded(abs, ctx.resolved.radius, Color::rgb(80, 140, 255), ctx.ap(255), ind_clip);
        }
        let kx = abs.x + iw;
        let knob = Rect::new(kx - self.knob_w / 2, abs.y - 2, self.knob_w, abs.h + 4);
        let kc = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::WHITE };
        d.fill_rounded(knob, 3, kc, ctx.ap(255), clip);
    }
}

impl super::Widget for SliderState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { self.draw_track(ctx, c, clip) }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        let edited = ui.state(obj).contains(crate::node::State::EDITED);
        if edited {
            return match key {
                // Up/Down adjust the value just like Left/Right, so a single rotary
                // axis can drive the slider while it is being edited
                Key::Left | Key::Down => {
                    let nv = (self.value - 1).clamp(self.min, self.max);
                    if nv != self.value { self.value = nv; ValueChanged } else { Consumed }
                }
                Key::Right | Key::Up => {
                    let nv = (self.value + 1).clamp(self.min, self.max);
                    if nv != self.value { self.value = nv; ValueChanged } else { Consumed }
                }
                Key::Enter => Commit, // confirm the value and leave the edit mode
                Key::Esc => ExitEdit,
                _ => Consumed,
            };
        }
        if key == Key::Enter { EnterEdit } else { Pass }
    }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn set_range(&mut self, min: i32, max: i32) { self.min = min; self.max = max; self.value = self.value.clamp(min, max); }
    // Slider knob: ±knob_w/2 horizontal, ±2 vertical
    fn overflow(&self) -> i32 { self.knob_w / 2 }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
