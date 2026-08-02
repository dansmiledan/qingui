use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub const ROW_H: i32 = 16;
pub const ROLL_DUR: u64 = 150;

#[derive(Clone)]
pub struct RollerState {
    pub items: Vec<String>,
    pub selected: usize,
    pub sel_from: Option<(f32, u64)>,
}

impl RollerState {
    pub(crate) fn tick(&mut self, now: u64) -> super::TickOut {
        let had_fx = self.sel_from.is_some();
        let active = fx_active(self.sel_from, now);
        if !active {
            self.sel_from = None;
        }
        // 有 fx（含本帧过期）就重绘：完成帧必须补最后一定格
        super::TickOut { redraw: had_fx, active }
    }

    pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
        match key {
            Key::Up | Key::Down => {
                let dir = if key == Key::Up { -1 } else { 1 };
                let next = (self.selected as i32 + dir).clamp(0, self.items.len().saturating_sub(1) as i32);
                select(&self.items, &mut self.selected, &mut self.sel_from, next as usize, ctx.now);
                super::KeyOutcome::Consumed
            }
            _ => super::KeyOutcome::Pass,
        }
    }
}

/// 滚动位置：从 from 平滑过渡到 selected
fn sel_f(selected: usize, sel_from: Option<(f32, u64)>, now: u64) -> f32 {
    match sel_from {
        Some((from, start)) => {
            let t = (now.saturating_sub(start) as f32 / ROLL_DUR as f32).clamp(0.0, 1.0);
            from * (1.0 - t) + selected as f32 * t
        }
        None => selected as f32,
    }
}

pub(crate) fn fx_active(sel_from: Option<(f32, u64)>, now: u64) -> bool {
    sel_from.is_some_and(|(_, s)| now.saturating_sub(s) < ROLL_DUR)
}

pub(crate) fn draw(items: &[String], selected: usize, sel_from: Option<(f32, u64)>, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let ap = ctx.ap(255);
    let cy = abs.y + abs.h / 2;
    // 中心选中行高亮（滚轮在行下滑动）
    d.fill_rounded(Rect::new(abs.x, cy - ROW_H / 2, abs.w, ROW_H), 3, Color::rgb(50, 70, 120), ap, lclip);
    let sf = sel_f(selected, sel_from, ctx.now);
    for (i, item) in items.iter().enumerate() {
        let ry = cy + ((i as f32 - sf) * ROW_H as f32) as i32 - 4;
        if ry + 8 < lclip.y || ry - 4 > lclip.bottom() {
            continue;
        }
        let (tw, _) = crate::font::text_size(item);
        d.draw_text_opa(
            Point { x: abs.x + (abs.w - tw) / 2, y: ry },
            item,
            ctx.resolved.text_color,
            ap,
            lclip,
        );
    }
}

/// 选中第 idx 项（首尾停止，不循环），带滚动动画。
/// 动画中途连按时从当前视觉位置续接（不跳变）。
pub(crate) fn select(items: &[String], selected: &mut usize, sel_from: &mut Option<(f32, u64)>, idx: usize, now: u64) {
    if items.is_empty() {
        return;
    }
    let nidx = idx.min(items.len() - 1);
    if nidx != *selected {
        let cur = sel_f(*selected, *sel_from, now);
        *sel_from = Some((cur, now));
        *selected = nidx;
    }
}

/// Roller 构建器：默认 80 x (min(3,n)*16+8)，bg(34,34,44) r4 + focused 白边
pub struct RollerBuilder {
    items: Vec<String>,
    selected: usize,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl RollerBuilder {
    pub fn new(items: &[&str]) -> Self {
        Self {
            items: items.iter().map(|s| (*s).into()).collect(),
            selected: 0,
            size: None, style: None, style_focused: None,
            sizing: None, transition: None, events: Vec::new(),
        }
    }
    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = idx;
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
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        let base = self.style.take().unwrap_or_else(|| {
            let mut s = Style::default();
            s.bg_color = Some(Color::rgb(34, 34, 44));
            s.radius = Some(4);
            s.text_color = Some(Color::WHITE);
            s
        });
        self.style = Some(f(base));
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
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let rows = self.items.len().min(3).max(1) as i32;
        let (w, h) = self.size.unwrap_or((80, rows * ROW_H + 8));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Roller(RollerState { items: self.items, selected, sel_from: None }),
        );
        let base = self.style.unwrap_or_else(|| {
            let mut s = Style::default();
            s.bg_color = Some(Color::rgb(34, 34, 44));
            s.radius = Some(4);
            s.text_color = Some(Color::WHITE);
            s
        });
        ui.set_style(r, base.clone());
        let focused = self.style_focused.unwrap_or_else(|| {
            let mut s = base;
            s.border_color = Some(Color::WHITE);
            s.border_width = Some(1);
            s
        });
        ui.set_style_focused(r, focused);
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

impl super::WidgetBehavior for RollerState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(&self.items, self.selected, self.sel_from, ctx, d, clip) }
    fn tick(&mut self, now: u64) -> super::TickOut { self.tick(now) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.selected as i32 }
    fn set_value(&mut self, v: i32) -> bool { super::select_clamp(self.items.len(), &mut self.selected, v) }
}

/// roller 专属 API(经 prelude 或显式 use 引入)
pub trait UiRollerExt {
    fn roller_selected(&self, obj: ObjRef) -> usize;
}

impl UiRollerExt for Ui {
    fn roller_selected(&self, obj: ObjRef) -> usize {
        self.kind(obj).and_then(|k| k.as_roller()).map(|s| s.selected).unwrap_or(0)
    }
}
