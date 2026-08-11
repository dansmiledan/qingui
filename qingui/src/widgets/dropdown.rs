use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::list::UiListExt;
use super::WidgetCtx;

/// Dropdown widget state.
#[derive(Clone)]
pub struct DropdownState {
    pub items: Vec<String>,
    pub selected: usize,
    pub popup_rows: usize,
    pub popup_row_h: i32,
    pub popup_min_w: i32,
}

impl DropdownState {
    /// Opens the popup list (anchored via Attach::Bottom, modal locked).
    /// Runs inside take-out: `self` is the dropdown state, the popup is a new
    /// screen child (operating on other nodes is unrestricted).
    fn open_popup(&mut self, ui: &mut Ui, obj: ObjRef) {
        if self.items.is_empty() {
            return;
        }
        let w = ui.rect(obj).w;
        let sel = self.selected;
        let prev = ui.focused();
        let screen = ui.screen();
        let refs: Vec<&str> = self.items.iter().map(|s| s.as_str()).collect();
        let lst = crate::widgets::list::ListCfg::new(&refs).row_h(self.popup_row_h).build(ui, screen);
        ui.move_to_front(lst); // popups draw on top (children order is the stacking order)
        let popup_h = self.items.len().min(self.popup_rows) as i32 * self.popup_row_h + 2;
        ui.set_size(lst, w.max(self.popup_min_w), popup_h);
        ui.list_select(lst, sel);
        ui.set_floating(lst, obj, crate::layout::Attach::Bottom);
        ui.group_add(lst);
        ui.set_modal(lst);
        // The popup list opens in its inner (EDITED) mode: rotation moves the options,
        // Enter commits (Commit -> Click on the list), Esc cancels (Key(Esc) above).
        ui.set_state(lst, crate::node::State::EDITED, true);
        // On select: write back to the dropdown and send ValueChanged, close the popup, restore focus.
        // The callback runs on event dispatch (outside take-out), so `ui.update` reaches the state.
        ui.add_event_cb(lst, crate::event::EventKind::Clicked, Box::new(move |ui, l, _| {
            let idx = ui.list_selected(l);
            ui.update::<DropdownState, _>(obj, |s| s.selected = idx);
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

    fn draw_label(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
        let abs = ctx.abs;
        let lclip = abs.intersect(&clip).unwrap_or(clip);
        let ap = ctx.ap(255);
        let text = self.items.get(self.selected).map(|s| s.as_str()).unwrap_or("");
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
}

/// Dropdown builder: default 100x20, bg(40,40,52) r4 + white focused border
pub type DropdownBuilder = WidgetBuilder<DropdownCfg>;

/// Dropdown configuration: items, the initially selected index, and the popup geometry props.
pub struct DropdownCfg {
    items: Vec<String>,
    selected: usize,
    popup_rows: usize,
    popup_row_h: i32,
    popup_min_w: i32,
}

impl DropdownCfg {
    /// Creates a builder with the given items.
    pub fn new(items: &[&str]) -> WidgetBuilder<DropdownCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: DropdownCfg {
                items: items.iter().map(|s| (*s).into()).collect(),
                selected: 0,
                popup_rows: 5,
                popup_row_h: super::list::ROW_H,
                popup_min_w: 80,
            },
        }
    }
}

impl WidgetBuilder<DropdownCfg> {
    /// Sets the initially selected index.
    pub fn selected(mut self, idx: usize) -> Self {
        self.cfg.selected = idx;
        self
    }
    /// Sets the popup's maximum visible rows (default 5).
    pub fn popup_rows(mut self, n: usize) -> Self {
        self.cfg.popup_rows = n;
        self
    }
    /// Sets the popup's row height in pixels (default `list::ROW_H` = 16).
    pub fn popup_row_h(mut self, h: i32) -> Self {
        self.cfg.popup_row_h = h;
        self
    }
    /// Sets the popup's minimum width in pixels (default 80).
    pub fn popup_min_w(mut self, w: i32) -> Self {
        self.cfg.popup_min_w = w;
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
            alloc::boxed::Box::new(DropdownState {
                items: self.items,
                selected,
                popup_rows: self.popup_rows,
                popup_row_h: self.popup_row_h,
                popup_min_w: self.popup_min_w,
            }),
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

impl super::Widget for DropdownState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { self.draw_label(ctx, c, clip) }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        if key == Key::Enter {
            self.open_popup(ui, obj);
            super::KeyOutcome::Consumed
        } else {
            super::KeyOutcome::Pass
        }
    }
    fn value(&self) -> i32 { self.selected as i32 }
    fn set_value(&mut self, v: i32) -> bool { super::select_clamp(self.items.len(), &mut self.selected, v) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
