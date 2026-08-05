use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

/// Switch widget state.
#[derive(Clone)]
pub struct SwitchState {
    pub on: bool,
}

impl SwitchState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: super::KeyCtx) -> super::KeyOutcome {
        if key == Key::Enter { self.on = !self.on; super::KeyOutcome::ValueChanged } else { super::KeyOutcome::Pass }
    }
}

pub(crate) fn draw(on: bool, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let tc = if on { Color::rgb(60, 180, 90) } else { Color::rgb(90, 90, 90) };
    d.fill_rounded(abs, abs.h / 2, tc, ctx.ap(255), clip);
    let k = abs.h - 4;
    let kx = if on { abs.right() - k - 2 } else { abs.x + 2 };
    d.fill_rounded(Rect::new(kx, abs.y + 2, k, k), k / 2, Color::WHITE, ctx.ap(255), clip);
}

/// Switch builder: default 40x20 + theme_switch/focused
pub struct SwitchBuilder {
    on: bool,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl SwitchBuilder {
    /// Creates a builder with the switch initially off.
    pub fn new() -> Self {
        Self {
            on: false,
            size: None, style: None, style_focused: None,
            sizing: None, transition: None, events: Vec::new(),
        }
    }
    /// Sets the initial on/off state.
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
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
        self.style = Some(f(self.style.unwrap_or_else(crate::style::theme_switch)));
        self
    }
    /// Sets the focused style.
    pub fn style_focused(mut self, s: Style) -> Self {
        self.style_focused = Some(s);
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
    pub fn on_event(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }

    /// Builds the widget into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((40, 20));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Switch(SwitchState { on: self.on }));
        ui.set_style(r, self.style.unwrap_or_else(crate::style::theme_switch));
        ui.set_style_focused(r, self.style_focused.unwrap_or_else(crate::style::theme_switch_focused));
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

/// Switch toggle API (brought in via prelude or an explicit use)
pub trait UiSwitchExt {
    /// Flips the switch's on/off state and sends a ValueChanged event.
    fn toggle_switch(&mut self, obj: ObjRef);
}

impl UiSwitchExt for Ui {
    fn toggle_switch(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
        if let Some(s) = self.kind_mut(obj).and_then(|k| k.as_switch_mut()) {
            s.on = !s.on;
        }
        self.invalidate_obj(obj);
        self.send_event(obj, EventKind::ValueChanged);
    }
}

impl super::WidgetBehavior for SwitchState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.on, ctx, d, clip) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.on as i32 }
}
