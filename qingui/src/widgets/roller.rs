use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Row height in pixels.
pub const ROW_H: i32 = 16;
/// Duration of the roll animation in ms.
pub const ROLL_DUR: u64 = 150;

/// Roller widget state.
#[derive(Clone)]
pub struct RollerState {
    pub items: Vec<String>,
    pub selected: usize,
    pub sel_from: Option<(f32, u64)>,
    pub row_h: i32,
    pub roll_dur: u64,
}

impl RollerState {
    /// Scroll position: smoothly transitions from `from` to `selected`
    fn sel_f(&self, now: u64) -> f32 {
        match self.sel_from {
            Some((from, start)) => {
                let t = (now.saturating_sub(start) as f32 / self.roll_dur as f32).clamp(0.0, 1.0);
                from * (1.0 - t) + self.selected as f32 * t
            }
            None => self.selected as f32,
        }
    }

    pub(crate) fn fx_active(&self, now: u64) -> bool {
        self.sel_from.is_some_and(|(_, s)| now.saturating_sub(s) < self.roll_dur)
    }

    fn draw_rows(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
        let abs = ctx.abs;
        let lclip = abs.intersect(&clip).unwrap_or(clip);
        let ap = ctx.ap(255);
        let cy = abs.y + abs.h / 2;
        // Highlight of the center selected row (the wheel slides beneath the row)
        d.fill_rounded(Rect::new(abs.x, cy - self.row_h / 2, abs.w, self.row_h), 3, Color::rgb(50, 70, 120), ap, lclip);
        let sf = self.sel_f(ctx.now);
        let lh = crate::font::line_height(ctx.resolved.font);
        for (i, item) in self.items.iter().enumerate() {
            // Vertically center the text within the row height row_h
            let ry = cy + ((i as f32 - sf) * self.row_h as f32) as i32 - lh / 2;
            if ry + lh < lclip.y || ry > lclip.bottom() {
                continue;
            }
            let (tw, _) = crate::font::text_size(ctx.resolved.font, item);
            d.draw_text_opa(
                Point { x: abs.x + (abs.w - tw) / 2, y: ry },
                ctx.resolved.font,
                item,
                ctx.resolved.text_color,
                ap,
                lclip,
            );
        }
    }

    /// Selects the idx-th item (stops at the ends, no wrap-around), with a scroll animation.
    /// Repeats during the animation continue from the current visual position (no jump).
    pub(crate) fn select(&mut self, idx: usize, now: u64) {
        if self.items.is_empty() {
            return;
        }
        let nidx = idx.min(self.items.len() - 1);
        if nidx != self.selected {
            let cur = self.sel_f(now);
            self.sel_from = Some((cur, now));
            self.selected = nidx;
        }
    }
}

/// Roller builder: default 80 x (min(3,n)*16+8), bg(34,34,44) r4 + white focused border
pub type RollerBuilder = WidgetBuilder<RollerCfg>;

/// Roller configuration: items, initial selection, and geometry/timing props.
pub struct RollerCfg {
    items: Vec<String>,
    selected: usize,
    row_h: i32,
    roll_dur: u64,
    visible_rows: usize,
}

impl RollerCfg {
    /// Creates a builder with the given items.
    pub fn new(items: &[&str]) -> WidgetBuilder<RollerCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: RollerCfg {
                items: items.iter().map(|s| (*s).into()).collect(),
                selected: 0,
                row_h: ROW_H,
                roll_dur: ROLL_DUR,
                visible_rows: 3,
            },
        }
    }
}

impl WidgetBuilder<RollerCfg> {
    /// Sets the initially selected index.
    pub fn selected(mut self, idx: usize) -> Self {
        self.cfg.selected = idx;
        self
    }
    /// Sets the row height in pixels (default `ROW_H` = 16).
    pub fn row_h(mut self, h: i32) -> Self {
        self.cfg.row_h = h;
        self
    }
    /// Sets the roll animation duration in ms (default `ROLL_DUR` = 150).
    pub fn roll_dur(mut self, ms: u64) -> Self {
        self.cfg.roll_dur = ms;
        self
    }
    /// Sets the number of visible rows used by the default height (default 3).
    pub fn visible_rows(mut self, n: usize) -> Self {
        self.cfg.visible_rows = n;
        self
    }
}

impl WidgetCfg for RollerCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_color = Some(Color::rgb(34, 34, 44));
        s.radius = Some(4);
        s.text_color = Some(Color::WHITE);
        s
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let rows = self.items.len().min(self.visible_rows).max(1) as i32;
        let (w, h) = common.size.unwrap_or((80, rows * self.row_h + 8));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(RollerState { items: self.items, selected, sel_from: None, row_h: self.row_h, roll_dur: self.roll_dur }),
        );
        let base = common.style.take().unwrap_or_else(Self::default_style);
        ui.set_style(r, base.clone());
        let focused = common.style_focused.take().unwrap_or_else(|| {
            let mut s = base;
            s.border_color = Some(Color::WHITE);
            s.border_width = Some(1);
            s
        });
        ui.set_style_focused(r, focused.clone());
        ui.set_style_edited(r, crate::style::theme_edited(&focused));
        common.apply_tail(ui, r);
        r
    }
}

impl super::Widget for RollerState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { self.draw_rows(ctx, c, clip) }
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, now: u64) -> super::TickOut {
        let had_fx = self.sel_from.is_some();
        let active = self.fx_active(now);
        if !active {
            self.sel_from = None;
        }
        // Redraw if there was fx (including a frame whose fx just expired): the completing frame must render the final settle
        super::TickOut { redraw: had_fx, active }
    }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        // Inner (EDITED) mode: direction keys roll the selection, Enter confirms the
        // selected value (Commit = Click + exit), Esc exits without acting. Outside the
        // inner mode nothing is consumed, so rotation moves the focus instead.
        if !ui.state(obj).contains(crate::node::State::EDITED) {
            return if key == Key::Enter { EnterEdit } else { Pass };
        }
        match key {
            Key::Up | Key::Down => {
                let dir = if key == Key::Up { -1 } else { 1 };
                let next = (self.selected as i32 + dir).clamp(0, self.items.len().saturating_sub(1) as i32);
                let now = ui.time();
                self.select(next as usize, now);
                Consumed
            }
            Key::Enter => Commit,
            Key::Esc => ExitEdit,
            _ => Consumed,
        }
    }
    fn value(&self) -> i32 { self.selected as i32 }
    fn set_value(&mut self, v: i32) -> bool { super::select_clamp(self.items.len(), &mut self.selected, v) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}

/// Roller-specific API (brought in via prelude or an explicit use)
pub trait UiRollerExt {
    /// Returns the currently selected index.
    fn roller_selected(&self, obj: ObjRef) -> usize;
}

impl UiRollerExt for Ui {
    fn roller_selected(&self, obj: ObjRef) -> usize {
        self.widget::<RollerState>(obj).map(|s| s.selected).unwrap_or(0)
    }
}
