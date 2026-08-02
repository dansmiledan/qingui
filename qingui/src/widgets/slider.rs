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

#[derive(Clone)]
pub struct SliderState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
}

impl SliderState {
    pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        if ctx.edited {
            return match key {
                Key::Left | Key::Right => {
                    let d = if key == Key::Left { -1 } else { 1 };
                    let nv = (self.value + d).clamp(self.min, self.max);
                    if nv != self.value { self.value = nv; ValueChanged } else { Consumed }
                }
                Key::Enter | Key::Esc => ExitEdit,
                _ => Consumed,
            };
        }
        if key == Key::Enter { EnterEdit } else { Pass }
    }
}

pub(crate) fn draw(min: i32, max: i32, value: i32, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
    let iw = (abs.w as f32 * frac) as i32;
    if iw > 0 {
        // 按整条轨道形状绘制，水平裁剪出指示部分：左端半圆始终与轨道吻合
        let band = Rect::new(abs.x, abs.y, iw, abs.h);
        let ind_clip = band.intersect(&clip).unwrap_or(band);
        d.fill_rounded(abs, ctx.resolved.radius, Color::rgb(80, 140, 255), ctx.ap(255), ind_clip);
    }
    let kx = abs.x + iw;
    let knob = Rect::new(kx - 4, abs.y - 2, 8, abs.h + 4);
    let kc = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::WHITE };
    d.fill_rounded(knob, 3, kc, ctx.ap(255), clip);
}

/// Slider 构建器：默认 100x12 + theme_slider/focused，链式覆盖
pub struct SliderBuilder {
    min: i32,
    max: i32,
    value: Option<i32>,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl SliderBuilder {
    pub fn new(min: i32, max: i32) -> Self {
        Self {
            min, max,
            value: None, size: None, style: None, style_focused: None,
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
    /// 整体替换默认样式
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }
    /// 在默认样式上修改
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.style = Some(f(self.style.unwrap_or_else(crate::style::theme_slider)));
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
        let (w, h) = self.size.unwrap_or((100, 12));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Slider(SliderState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min) }),
        );
        ui.set_style(r, self.style.unwrap_or_else(crate::style::theme_slider));
        ui.set_style_focused(r, self.style_focused.unwrap_or_else(crate::style::theme_slider_focused));
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

impl super::WidgetBehavior for SliderState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.min, self.max, self.value, ctx, d, clip) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn set_range(&mut self, min: i32, max: i32) { self.min = min; self.max = max; self.value = self.value.clamp(min, max); }
    // Slider 旋钮 ±4px 横向 ±2px 纵向
    fn overflow(&self) -> i32 { 4 }
}
