use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Spinbox widget state.
#[derive(Clone)]
pub struct SpinboxState {
    pub min: i32,
    pub max: i32,
    pub value: i32,
    pub digits: u8,
    pub cursor: u8,
}

pub(crate) fn draw(min: i32, max: i32, value: i32, digits: u8, cursor: u8, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
    let _ = (min, max);
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let text = alloc::format!("{:0width$}", value, width = digits as usize);
    let ap = ctx.ap(255);
    let font = ctx.resolved.font;
    let adv = crate::font::advance(font);
    let lh = crate::font::line_height(font);
    let x0 = abs.x + (abs.w - digits as i32 * adv) / 2;
    let y = abs.y + (abs.h - lh) / 2;
    for (i, ch) in text.chars().enumerate() {
        let x = x0 + i as i32 * adv;
        if i as u8 == cursor && ctx.edited {
            // Cursor position: inverted highlight
            d.fill_rounded(Rect::new(x - 1, abs.y + 1, adv + 2, abs.h - 2), 2, Color::rgb(80, 140, 255), ap, lclip);
            let mut buf = [0u8; 4];
            d.draw_text_opa(Point { x, y }, font, ch.encode_utf8(&mut buf), Color::BLACK, ap, lclip);
        } else {
            let mut buf = [0u8; 4];
            d.draw_text_opa(Point { x, y }, font, ch.encode_utf8(&mut buf), ctx.resolved.text_color, ap, lclip);
        }
    }
}

/// Moves the cursor (±1, wrapping within range)
pub(crate) fn move_cursor(digits: u8, cursor: &mut u8, dir: i32) {
    let n = digits.max(1) as i32;
    *cursor = (*cursor as i32 + dir).rem_euclid(n) as u8;
}

/// Increases/decreases the digit under the cursor (changes the value by the digit's place weight, clamped to range)
pub(crate) fn step_digit(min: i32, max: i32, value: &mut i32, digits: u8, cursor: u8, dir: i32) {
    let pos = (digits.max(1) - 1 - cursor.min(digits.max(1) - 1)) as u32;
    let step = 10i32.pow(pos);
    *value = (*value + dir * step).clamp(min, max);
}

/// Builder for the Spinbox widget.
pub type SpinboxBuilder = WidgetBuilder<SpinboxCfg>;

/// Spinbox configuration: value range, digit count, and initial value.
pub struct SpinboxCfg {
    min: i32,
    max: i32,
    digits: u8,
    value: Option<i32>,
}

impl SpinboxCfg {
    /// Creates a builder for the given range and digit count.
    pub fn new(min: i32, max: i32, digits: u8) -> WidgetBuilder<SpinboxCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: SpinboxCfg { min, max, digits: digits.max(1), value: None },
        }
    }

    /// Base style: dark rounded background with white text.
    fn base_style() -> Style {
        let mut s = Style::default();
        s.bg_color = Some(Color::rgb(40, 40, 52));
        s.radius = Some(4);
        s.text_color = Some(Color::WHITE);
        s
    }
}

impl WidgetBuilder<SpinboxCfg> {
    /// Sets the initial value.
    pub fn value(mut self, v: i32) -> Self {
        self.cfg.value = Some(v);
        self
    }
}

impl WidgetCfg for SpinboxCfg {
    fn default_style() -> Style {
        Self::base_style()
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let font = crate::font::measure_font(common.style.as_ref(), ui);
        let (w, h) = common.size.unwrap_or((self.digits as i32 * crate::font::advance(font) + 12, crate::font::line_height(font) + 8));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(SpinboxState {
                min: self.min,
                max: self.max,
                value: self.value.unwrap_or(self.min),
                digits: self.digits,
                cursor: self.digits - 1,
            }),
        );
        let base = common.style.take().unwrap_or_else(Self::base_style);
        ui.set_style(r, base.clone());
        let focused = common.style_focused.take().unwrap_or_else(|| {
            let mut s = base;
            s.border_color = Some(Color::WHITE);
            s.border_width = Some(1);
            s
        });
        ui.set_style_focused(r, focused.clone());
        ui.set_style_edited(r, common.style_edited.take().unwrap_or_else(|| crate::style::theme_edited(&focused)));
        common.apply_tail(ui, r);
        r
    }
}

impl super::Widget for SpinboxState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(self.min, self.max, self.value, self.digits, self.cursor, ctx, c, clip) }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        if !ui.state(obj).contains(crate::node::State::EDITED) {
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
            Key::Enter => Commit, // confirm the value and leave the edit mode
            Key::Esc => ExitEdit,
            _ => Consumed,
        }
    }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
