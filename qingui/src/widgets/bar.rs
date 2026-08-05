use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Color, Rect};
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

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

/// Bar builder: default 100x8 + theme_bar
pub struct BarBuilder {
    min: i32,
    max: i32,
    value: Option<i32>,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl BarBuilder {
    /// Creates a builder for the given range.
    pub fn new(min: i32, max: i32) -> Self {
        Self {
            min, max,
            value: None, size: None, style: None,
            sizing: None, transition: None, events: Vec::new(),
        }
    }
    /// Sets the initial value.
    pub fn value(mut self, v: i32) -> Self {
        self.value = Some(v);
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
    /// Modifies on top of the default style.
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.style = Some(f(self.style.unwrap_or_else(crate::style::theme_bar)));
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

    /// Builds the widget into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((100, 8));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Bar(BarState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min) }),
        );
        ui.set_style(r, self.style.unwrap_or_else(crate::style::theme_bar));
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

impl super::WidgetBehavior for BarState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.min, self.max, self.value, ctx, d, clip) }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn set_range(&mut self, min: i32, max: i32) { self.min = min; self.max = max; self.value = self.value.clamp(min, max); }
}
