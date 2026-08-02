use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

const BOX: i32 = 12;

#[derive(Clone)]
pub struct CheckboxState {
    pub text: alloc::string::String,
    pub checked: bool,
}

impl CheckboxState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: super::KeyCtx) -> super::KeyOutcome {
        if key == Key::Enter { self.checked = !self.checked; super::KeyOutcome::ValueChanged } else { super::KeyOutcome::Pass }
    }
}

pub(crate) fn draw(text: &str, checked: bool, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let ap = |b: u8| ctx.ap(b);
    let by = abs.y + (abs.h - BOX) / 2;
    let brect = Rect::new(abs.x, by, BOX, BOX);
    // 方框
    d.draw_border(brect, 1, 2, Color::rgb(150, 150, 160), ap(255), clip);
    if checked {
        // 勾：两条线
        let p1 = Point { x: abs.x + 2, y: by + 6 };
        let p2 = Point { x: abs.x + 5, y: by + 9 };
        let p3 = Point { x: abs.x + 10, y: by + 3 };
        d.draw_line(p1, p2, 2, Color::rgb(80, 140, 255), ap(255), clip);
        d.draw_line(p2, p3, 2, Color::rgb(80, 140, 255), ap(255), clip);
    }
    d.draw_text_opa(
        Point { x: abs.x + BOX + 6, y: abs.y + (abs.h - 8) / 2 },
        text,
        ctx.resolved.text_color,
        ap(255),
        clip,
    );
}

/// Checkbox 构建器：默认 BOX+6+文本宽 x 16，bg 透明 + focused 白边
pub struct CheckboxBuilder {
    text: alloc::string::String,
    checked: bool,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl CheckboxBuilder {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            checked: false,
            size: None, style: None, style_focused: None,
            sizing: None, transition: None, events: Vec::new(),
        }
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
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
            s.bg_opa = Some(0);
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
            let (tw, _) = crate::font::text_size(&self.text);
            (BOX + 6 + tw, 16)
        });
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Checkbox(CheckboxState { text: self.text, checked: self.checked }),
        );
        let base = self.style.unwrap_or_else(|| {
            let mut s = Style::default();
            s.bg_opa = Some(0);
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

/// checkbox 切换 API(经 prelude 或显式 use 引入)
pub trait UiCheckboxExt {
    fn toggle_checkbox(&mut self, obj: ObjRef);
}

impl UiCheckboxExt for Ui {
    fn toggle_checkbox(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
        if let Some(s) = self.kind_mut(obj).and_then(|k| k.as_checkbox_mut()) {
            s.checked = !s.checked;
        }
        self.invalidate_obj(obj);
        self.send_event(obj, EventKind::ValueChanged);
    }
}

impl super::WidgetBehavior for CheckboxState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(&self.text, self.checked, ctx, d, clip) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.checked as i32 }
    fn set_value(&mut self, v: i32) -> bool {
        let nv = v != 0;
        let c = nv != self.checked;
        self.checked = nv;
        c
    }
}
