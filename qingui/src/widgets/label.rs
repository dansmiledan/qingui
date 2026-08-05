use alloc::string::String;
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Point, Rect};
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

/// Label widget state.
#[derive(Clone)]
pub struct LabelState {
    pub text: String,
}

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    d.draw_text_opa(
        Point { x: ctx.abs.x, y: ctx.abs.y },
        ctx.resolved.font,
        text,
        ctx.resolved.text_color,
        ctx.ap(255),
        clip,
    );
}

/// Label builder: default text-measured size + theme_label
pub struct LabelBuilder {
    text: String,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl LabelBuilder {
    /// Creates a builder with the given text.
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            style: None, sizing: None, transition: None, events: Vec::new(),
        }
    }
    /// Sets the style.
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }
    /// Modifies on top of the default style.
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.style = Some(f(self.style.unwrap_or_else(crate::style::theme_label)));
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
        let font = crate::font::measure_font(self.style.as_ref(), ui);
        let (w, h) = crate::font::text_size(font, &self.text);
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Label(LabelState { text: self.text }));
        ui.set_style(r, self.style.unwrap_or_else(crate::style::theme_label));
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

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, text: &str) -> ObjRef {
    LabelBuilder::new(text).build(ui, parent)
}

pub(crate) fn set_text(ui: &mut Ui, obj: ObjRef, text: &str) {
    ui.invalidate_obj(obj);
    let font = crate::font::measure_font(ui.arena.get(obj).map(|n| &n.style), ui);
    let (w, h) = crate::font::text_size(font, text);
    if let Some(n) = ui.arena.get_mut(obj) {
        if let WidgetKind::Label(s) = &mut n.kind {
            s.text = text.into();
            n.rect.w = w;
            n.rect.h = h;
        }
    }
    ui.invalidate_obj(obj);
    ui.layout_dirty = true;
}

pub(crate) fn text(ui: &Ui, obj: ObjRef) -> String {
    if let Some(n) = ui.arena.get(obj) {
        if let WidgetKind::Label(s) = &n.kind {
            return s.text.clone();
        }
    }
    String::new()
}

impl super::WidgetBehavior for LabelState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(&self.text, ctx, d, clip) }
}

/// Text API (brought in via prelude or an explicit use)
pub trait UiTextExt {
    /// Sets the label's text (also resizes the node to fit).
    fn set_text(&mut self, obj: ObjRef, text: &str);
    /// Returns the label's current text.
    fn text(&self, obj: ObjRef) -> String;
}

impl UiTextExt for Ui {
    fn set_text(&mut self, obj: ObjRef, text: &str) {
        set_text(self, obj, text);
    }

    fn text(&self, obj: ObjRef) -> String {
        text(self, obj)
    }
}
