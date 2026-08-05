use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::style::Style;
use crate::ui::Ui;
use super::WidgetKind;

/// Builder for the generic container Obj (no built-in drawn content; hosts layout and child objects)
#[derive(Default)]
pub struct ObjBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<crate::layout::Sizing>, Option<crate::layout::Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    layout: Option<crate::style::Layout>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl ObjBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets the widget size.
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    /// Sets the style.
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    /// Modifies on top of the default style.
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.style = Some(f(self.style.unwrap_or_default())); self
    }
    /// Sets the width/height sizing.
    pub fn sizing(mut self, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    /// Sets the transition duration and easing.
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    /// Sets the layout.
    pub fn layout(mut self, layout: crate::style::Layout) -> Self { self.layout = Some(layout); self }
    /// Registers an event callback.
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    /// Builds the widget into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((0, 0));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(ObjState));
        if let Some(s) = self.style {
            ui.set_style(r, s);
        }
        if let Some(l) = self.layout {
            ui.set_layout(r, l);
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

/// Placeholder state: Obj carries no data, it only keeps the macro treating all variants uniformly
pub struct ObjState;

impl super::WidgetBehavior for ObjState {
    fn draw(&self, _ctx: &super::WidgetCtx, _d: &mut DrawBuf, _clip: Rect) {}
}
