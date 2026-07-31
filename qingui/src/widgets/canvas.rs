use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::node::DrawHook;
use crate::style::Style;
use crate::ui::Ui;
use super::WidgetKind;

/// Canvas 构建器：空节点 + 叠加绘制钩子的糖；size 必填（无默认），默认透明背景
pub struct CanvasBuilder {
    cb: DrawHook,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<crate::layout::Sizing>, Option<crate::layout::Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl CanvasBuilder {
    pub fn new(cb: DrawHook) -> Self {
        Self { cb, size: None, style: None, sizing: None, transition: None, events: Vec::new() }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    pub fn sizing(mut self, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((32, 32));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj);
        let mut s = self.style.unwrap_or_default();
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0); // 默认透明背景：画布只承载自定义绘制
        }
        ui.set_style(r, s);
        ui.set_draw_hook(r, Some(self.cb));
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
