use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Color, Point, Rect};
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

/// 表盘起始角与扫掠范围（LVGL 风格：底部留缺口）
pub const START_DEG: i32 = 135;
pub const SWEEP_DEG: i32 = 270;
pub const TRACK_W: i32 = 4;

#[derive(Clone)]
pub struct ArcState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
}

pub(crate) fn draw(min: i32, max: i32, value: i32, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 3;
    if r <= 0 {
        return;
    }
    let ap = |b: u8| ctx.ap(b);
    // 背景弧（全轨）
    d.draw_arc(c, r, TRACK_W, START_DEG, START_DEG + SWEEP_DEG, Color::rgb(70, 70, 80), ap(255), clip);
    // 指示弧（编辑态变黄）
    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
    let ind_end = START_DEG + (SWEEP_DEG as f32 * frac) as i32;
    if ind_end > START_DEG {
        let ic = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::rgb(80, 140, 255) };
        d.draw_arc(c, r, TRACK_W, START_DEG, ind_end, ic, ap(255), clip);
    }
}

/// Arc 构建器：默认 60x60 + bg 透明
pub struct ArcBuilder {
    min: i32,
    max: i32,
    value: Option<i32>,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl ArcBuilder {
    pub fn new(min: i32, max: i32) -> Self {
        Self {
            min, max,
            value: None, size: None, style: None,
            sizing: None, transition: None, events: Vec::new(),
        }
    }
    pub fn value(mut self, v: i32) -> Self {
        self.value = Some(v);
        self
    }
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
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
        let (w, h) = self.size.unwrap_or((60, 60));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Arc(ArcState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min) }),
        );
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

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, min: i32, max: i32) -> ObjRef {
    ArcBuilder::new(min, max).build(ui, parent)
}
