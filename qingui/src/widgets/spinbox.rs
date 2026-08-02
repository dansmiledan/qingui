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

#[derive(Clone)]
pub struct SpinboxState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
    pub digits: u8,
    pub cursor: u8,
}

impl SpinboxState {
    pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        if !ctx.edited {
            return if key == Key::Enter { EnterEdit } else { Pass };
        }
        match key {
            Key::Left => { move_cursor(self.digits, &mut self.cursor, -1); Consumed }
            Key::Right => { move_cursor(self.digits, &mut self.cursor, 1); Consumed }
            Key::Up | Key::Down => {
                let d = if key == Key::Up { 1 } else { -1 };
                let mut nv = self.value;
                step_digit(self.min, self.max, &mut nv, self.digits, self.cursor, d);
                if nv != self.value { self.value = nv; ValueChanged } else { Consumed }
            }
            Key::Enter | Key::Esc => ExitEdit,
            _ => Consumed,
        }
    }
}

pub(crate) fn draw(min: i32, max: i32, value: i32, digits: u8, cursor: u8, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let _ = (min, max);
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let text = alloc::format!("{:0width$}", value, width = digits as usize);
    let ap = ctx.ap(255);
    let x0 = abs.x + (abs.w - digits as i32 * 8) / 2;
    let y = abs.y + (abs.h - 8) / 2;
    for (i, ch) in text.chars().enumerate() {
        let x = x0 + i as i32 * 8;
        if i as u8 == cursor && ctx.edited {
            // 光标位：反色高亮
            d.fill_rounded(Rect::new(x - 1, abs.y + 1, 10, abs.h - 2), 2, Color::rgb(80, 140, 255), ap, lclip);
            let g = crate::font::glyph(ch);
            for row in 0..8i32 {
                for col in 0..8i32 {
                    if g[row as usize] & (1 << col) != 0 {
                        d.fill_rect(Rect::new(x + col, y + row, 1, 1), Color::BLACK, ap, lclip);
                    }
                }
            }
        } else {
            let mut buf = [0u8; 4];
            d.draw_text_opa(Point { x, y }, ch.encode_utf8(&mut buf), ctx.resolved.text_color, ap, lclip);
        }
    }
}

/// 光标移动（±1，范围内循环）
pub(crate) fn move_cursor(digits: u8, cursor: &mut u8, dir: i32) {
    let n = digits.max(1) as i32;
    *cursor = (*cursor as i32 + dir).rem_euclid(n) as u8;
}

/// 当前光标位数字增减（按位权改变值，范围 clamp）
pub(crate) fn step_digit(min: i32, max: i32, value: &mut i32, digits: u8, cursor: u8, dir: i32) {
    let pos = (digits.max(1) - 1 - cursor.min(digits.max(1) - 1)) as u32;
    let step = 10i32.pow(pos);
    *value = (*value + dir * step).clamp(min, max);
}

/// Spinbox 构建器：默认 digits*8+12 x 18，bg(40,40,52) r4 + focused 白边
pub struct SpinboxBuilder {
    min: i32,
    max: i32,
    digits: u8,
    value: Option<i32>,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl SpinboxBuilder {
    pub fn new(min: i32, max: i32, digits: u8) -> Self {
        Self {
            min, max,
            digits: digits.max(1),
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
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((self.digits as i32 * 8 + 12, 18));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Spinbox(SpinboxState {
                min: self.min,
                max: self.max,
                value: self.value.unwrap_or(self.min),
                digits: self.digits,
                cursor: self.digits - 1,
            }),
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

impl super::WidgetBehavior for SpinboxState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.min, self.max, self.value, self.digits, self.cursor, ctx, d, clip) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
}
