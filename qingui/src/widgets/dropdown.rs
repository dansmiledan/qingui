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
use super::list::UiListExt;
use super::{WidgetCtx, WidgetKind};

/// Dropdown widget state.
#[derive(Clone)]
pub struct DropdownState {
    pub items: Vec<String>,
    pub selected: usize,
}

impl DropdownState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: super::KeyCtx) -> super::KeyOutcome {
        if key == Key::Enter { super::KeyOutcome::Deferred(open, 0) } else { super::KeyOutcome::Pass }
    }
}

/// Opens the dropdown's popup list (anchored via Attach::Bottom, modal locked).
/// The payload is unused; it only exists to match Deferred's fn(&mut Ui, ObjRef, i32) signature.
pub(crate) fn open(ui: &mut Ui, obj: ObjRef, _payload: i32) {
    let Some((items, sel, w)) = ui.arena.get(obj).map(|n| match &n.kind {
        WidgetKind::Dropdown(s) => (s.items.clone(), s.selected, n.rect.w),
        _ => (Vec::new(), 0, 0),
    }) else { return };
    if items.is_empty() {
        return;
    }
    let prev = ui.focused();
    let screen = ui.screen();
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let lst = crate::widgets::list::create(ui, screen, &refs);
    ui.move_to_front(lst); // popups draw on top (children order is the stacking order)
    ui.set_size(lst, w.max(80), (items.len().min(5) * 16 + 2) as i32);
    ui.list_select(lst, sel);
    ui.set_floating(lst, obj, crate::layout::Attach::Bottom);
    ui.group_add(lst);
    ui.set_modal(lst);
    // On select: write back to the dropdown and send ValueChanged, close the popup, restore focus
    ui.add_event_cb(lst, crate::event::EventKind::Clicked, Box::new(move |ui, l, _| {
        let idx = ui.list_selected(l);
        if let Some(n) = ui.arena.get_mut(obj) {
            if let Some(s) = n.kind.as_dropdown_mut() {
                s.selected = idx;
            }
        }
        ui.invalidate_obj(obj);
        ui.send_event(obj, crate::event::EventKind::ValueChanged);
        ui.clear_modal();
        ui.delete(l);
        if let Some(p) = prev {
            ui.group_focus(p);
        }
    }));
    // Esc: close without changing the value
    ui.add_event_cb(lst, crate::event::EventKind::Key(crate::input::Key::Esc), Box::new(move |ui, l, k| {
        if k == crate::event::EventKind::Key(crate::input::Key::Esc) {
            ui.clear_modal();
            ui.delete(l);
            if let Some(p) = prev {
                ui.group_focus(p);
            }
        }
    }));
}

pub(crate) fn draw(items: &[String], selected: usize, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let ap = ctx.ap(255);
    let text = items.get(selected).map(|s| s.as_str()).unwrap_or("");
    d.draw_text_opa(
        Point { x: abs.x + 6, y: abs.y + (abs.h - crate::font::line_height(ctx.resolved.font)) / 2 },
        ctx.resolved.font,
        text,
        ctx.resolved.text_color,
        ap,
        lclip,
    );
    // Dropdown arrow (small triangle)
    let ax = abs.right() - 10;
    let ay = abs.y + abs.h / 2;
    d.draw_line(Point { x: ax - 3, y: ay - 2 }, Point { x: ax, y: ay + 2 }, 1, ctx.resolved.text_color, ap, lclip);
    d.draw_line(Point { x: ax, y: ay + 2 }, Point { x: ax + 3, y: ay - 2 }, 1, ctx.resolved.text_color, ap, lclip);
}

/// Dropdown builder: default 100x20, bg(40,40,52) r4 + white focused border
pub type DropdownBuilder = WidgetBuilder<DropdownCfg>;

/// Dropdown configuration: items and the initially selected index.
pub struct DropdownCfg {
    items: Vec<String>,
    selected: usize,
}

impl DropdownCfg {
    /// Creates a builder with the given items.
    pub fn new(items: &[&str]) -> WidgetBuilder<DropdownCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: DropdownCfg { items: items.iter().map(|s| (*s).into()).collect(), selected: 0 },
        }
    }
}

impl WidgetBuilder<DropdownCfg> {
    /// Sets the initially selected index.
    pub fn selected(mut self, idx: usize) -> Self {
        self.cfg.selected = idx;
        self
    }
}

impl WidgetCfg for DropdownCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_color = Some(Color::rgb(40, 40, 52));
        s.radius = Some(4);
        s.text_color = Some(Color::WHITE);
        s
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((100, 20));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Dropdown(DropdownState { items: self.items, selected }),
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

impl super::WidgetBehavior for DropdownState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(&self.items, self.selected, ctx, d, clip) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.selected as i32 }
    fn set_value(&mut self, v: i32) -> bool { super::select_clamp(self.items.len(), &mut self.selected, v) }
}
