use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

#[derive(Clone)]
pub struct DropdownState {
    pub items: Vec<String>,
    pub selected: usize,
}

pub(crate) fn draw(items: &[String], selected: usize, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let ap = ctx.ap(255);
    let text = items.get(selected).map(|s| s.as_str()).unwrap_or("");
    d.draw_text_opa(
        Point { x: abs.x + 6, y: abs.y + (abs.h - 8) / 2 },
        text,
        ctx.resolved.text_color,
        ap,
        lclip,
    );
    // 下拉箭头（小三角）
    let ax = abs.right() - 10;
    let ay = abs.y + abs.h / 2;
    d.draw_line(Point { x: ax - 3, y: ay - 2 }, Point { x: ax, y: ay + 2 }, 1, ctx.resolved.text_color, ap, lclip);
    d.draw_line(Point { x: ax, y: ay + 2 }, Point { x: ax + 3, y: ay - 2 }, 1, ctx.resolved.text_color, ap, lclip);
}

/// Dropdown 构建器：默认 100x20，bg(40,40,52) r4 + focused 白边
pub struct DropdownBuilder {
    items: Vec<String>,
    selected: usize,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl DropdownBuilder {
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
            s.bg_color = Some(Color::rgb(40, 40, 52));
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
        let (w, h) = self.size.unwrap_or((100, 20));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Dropdown(DropdownState { items: self.items, selected }),
        );
        let base = self.style.unwrap_or_else(|| {
            let mut s = Style::default();
            s.bg_color = Some(Color::rgb(40, 40, 52));
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

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, items: &[&str]) -> ObjRef {
    DropdownBuilder::new(items).build(ui, parent)
}
