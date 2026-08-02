use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::Rect;
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{TickOut, WidgetBehavior, WidgetCtx, WidgetKind};

/// 一帧 RGB565(小端)位图
pub struct Frame {
    pub w: i32,
    pub h: i32,
    pub rgb565: &'static [u8],
}

/// 图片数据:静态图单帧;gif 多帧 + 逐帧延时。由 qingui-codegen 生成
pub struct ImageData {
    pub frames: &'static [Frame],
    pub delays_ms: &'static [u16],
}

pub struct ImageState {
    pub data: &'static ImageData,
    pub cur: usize,
    pub last_switch: u64,
}

impl WidgetBehavior for ImageState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
        let Some(f) = self.data.frames.get(self.cur) else { return };
        d.blit565(ctx.abs.x, ctx.abs.y, f.w, f.h, f.rgb565, ctx.ap(255), clip);
    }
    fn tick(&mut self, now: u64) -> TickOut {
        if self.data.frames.len() <= 1 {
            return TickOut::IDLE;
        }
        let delay = self.data.delays_ms.get(self.cur).copied().unwrap_or(100) as u64;
        if now.saturating_sub(self.last_switch) >= delay {
            self.cur = (self.cur + 1) % self.data.frames.len();
            self.last_switch = now;
            TickOut { redraw: true, active: true }
        } else {
            TickOut { redraw: false, active: true }
        }
    }
}

/// Image 构建器:默认尺寸 = 首帧尺寸 + bg 透明
pub struct ImageBuilder {
    data: &'static ImageData,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl ImageBuilder {
    pub fn new(data: &'static ImageData) -> Self {
        Self { data, size: None, style: None, sizing: None, transition: None, events: Vec::new() }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (fw, fh) = self.data.frames.first().map(|f| (f.w, f.h)).unwrap_or((0, 0));
        let (w, h) = self.size.unwrap_or((fw, fh));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Image(
            ImageState { data: self.data, cur: 0, last_switch: ui.time() },
        ));
        let mut s = self.style.unwrap_or_default();
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0);
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
