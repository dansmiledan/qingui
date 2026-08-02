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

#[derive(Clone)]
pub struct ButtonState {
    pub text: alloc::string::String,
}

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let (tw, th) = crate::font::text_size(text);
    let p = Point {
        x: ctx.abs.x + (ctx.abs.w - tw) / 2,
        y: ctx.abs.y + (ctx.abs.h - th) / 2,
    };
    d.draw_text_opa(p, text, ctx.resolved.text_color, ctx.ap(255), clip);
}

/// Button 构建器：默认文本尺寸 + padding，theme_button/pressed/focused
pub struct ButtonBuilder {
    text: alloc::string::String,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_pressed: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl ButtonBuilder {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            size: None, style: None, style_pressed: None, style_focused: None,
            sizing: None, transition: None, events: Vec::new(),
        }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.style = Some(f(self.style.unwrap_or_else(crate::style::theme_button)));
        self
    }
    pub fn style_pressed(mut self, s: Style) -> Self {
        self.style_pressed = Some(s);
        self
    }
    pub fn style_focused(mut self, s: Style) -> Self {
        self.style_focused = Some(s);
        self
    }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h));
        self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or_else(|| {
            let (tw, th) = crate::font::text_size(&self.text);
            (tw + 24, th + 12)
        });
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Button(ButtonState { text: self.text }));
        ui.set_style(r, self.style.unwrap_or_else(crate::style::theme_button));
        ui.set_style_pressed(r, self.style_pressed.unwrap_or_else(crate::style::theme_button_pressed));
        ui.set_style_focused(r, self.style_focused.unwrap_or_else(crate::style::theme_button_focused));
        if let Some(n) = ui.arena.get_mut(r) {
            n.flags |= crate::node::Flag::CLICKABLE;
        }
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
    ButtonBuilder::new(text).build(ui, parent)
}

impl super::WidgetBehavior for ButtonState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(&self.text, ctx, d, clip) }
}
