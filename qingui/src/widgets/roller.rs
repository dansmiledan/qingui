use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{WidgetCtx, WidgetKind};

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
}

impl RollerState {
    pub(crate) fn tick(&mut self, now: u64) -> super::TickOut {
        let had_fx = self.sel_from.is_some();
        let active = fx_active(self.sel_from, now);
        if !active {
            self.sel_from = None;
        }
        // Redraw if there was fx (including a frame whose fx just expired): the completing frame must render the final settle
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

/// Scroll position: smoothly transitions from `from` to `selected`
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
    // Highlight of the center selected row (the wheel slides beneath the row)
    d.fill_rounded(Rect::new(abs.x, cy - ROW_H / 2, abs.w, ROW_H), 3, Color::rgb(50, 70, 120), ap, lclip);
    let sf = sel_f(selected, sel_from, ctx.now);
    let lh = crate::font::line_height(ctx.resolved.font);
    for (i, item) in items.iter().enumerate() {
        // Vertically center the text within the row height ROW_H
        let ry = cy + ((i as f32 - sf) * ROW_H as f32) as i32 - lh / 2;
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

/// Roller builder: default 80 x (min(3,n)*16+8), bg(34,34,44) r4 + white focused border
pub type RollerBuilder = WidgetBuilder<RollerCfg>;

/// Roller configuration: items and the initially selected index.
pub struct RollerCfg {
    items: Vec<String>,
    selected: usize,
}

impl RollerCfg {
    /// Creates a builder with the given items.
    pub fn new(items: &[&str]) -> WidgetBuilder<RollerCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: RollerCfg { items: items.iter().map(|s| (*s).into()).collect(), selected: 0 },
        }
    }
}

impl WidgetBuilder<RollerCfg> {
    /// Sets the initially selected index.
    pub fn selected(mut self, idx: usize) -> Self {
        self.cfg.selected = idx;
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
        let rows = self.items.len().min(3).max(1) as i32;
        let (w, h) = common.size.unwrap_or((80, rows * ROW_H + 8));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Roller(Box::new(RollerState { items: self.items, selected, sel_from: None })),
        );
        let base = common.style.take().unwrap_or_else(Self::default_style);
        ui.set_style(r, base.clone());
        let focused = common.style_focused.take().unwrap_or_else(|| {
            let mut s = base;
            s.border_color = Some(Color::WHITE);
            s.border_width = Some(1);
            s
        });
        ui.set_style_focused(r, focused);
        common.apply_tail(ui, r);
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

/// Roller-specific API (brought in via prelude or an explicit use)
pub trait UiRollerExt {
    /// Returns the currently selected index.
    fn roller_selected(&self, obj: ObjRef) -> usize;
}

impl UiRollerExt for Ui {
    fn roller_selected(&self, obj: ObjRef) -> usize {
        self.kind(obj).and_then(|k| k.as_roller()).map(|s| s.selected).unwrap_or(0)
    }
}
