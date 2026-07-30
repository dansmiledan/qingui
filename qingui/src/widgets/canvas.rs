use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::style::Style;
use crate::ui::Ui;
use super::WidgetKind;

/// Canvas 绘制回调：参数为 (画板, 控件绝对矩形, 裁剪矩形, 当前时间 ms)。
/// 回调内用 DrawBuf 的绘制原语自由绘制（均带 clip 与 alpha 混合）。
pub type CanvasCb = Box<dyn FnMut(&mut DrawBuf, Rect, Rect, u64)>;

/// Canvas 构建器：size 必填（无默认），默认透明背景
pub struct CanvasBuilder {
    cb: CanvasCb,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<crate::layout::Sizing>, Option<crate::layout::Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl CanvasBuilder {
    pub fn new(cb: CanvasCb) -> Self {
        Self {
            cb,
            size: None, style: None, sizing: None, transition: None, events: Vec::new(),
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
    pub fn sizing(mut self, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) -> Self {
        self.sizing = Some((w, h));
        self
    }
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((32, 32));
        let idx = ui.register_canvas_cb(self.cb);
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Canvas { cb: idx });
        let mut s = self.style.unwrap_or_default();
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0); // 默认透明背景：画布只承载自定义绘制
        }
        ui.set_style(r, s);
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

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, w: i32, h: i32, cb: CanvasCb) -> ObjRef {
    CanvasBuilder::new(cb).size(w, h).build(ui, parent)
}
