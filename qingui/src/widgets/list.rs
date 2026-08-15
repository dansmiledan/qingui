use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::pixel::PixelFormat;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Row height in pixels.
pub const ROW_H: i32 = 16;
/// Duration of the item/highlight/scroll effects in ms.
pub const FX_DUR: u64 = 200;

/// List widget state: items, selection, scroll offset and animation effects.
#[derive(Clone)]
pub struct ListState {
    pub items: Vec<String>,
    pub selected: usize,
    pub scroll: i32,
    pub fx: ListFx,
    pub row_h: i32,
    pub fx_dur: u64,
}

/// Entry/shift effect for a single item (interpolated by time while drawing, cleaned up by prune once settled)
#[derive(Clone)]
pub struct ItemFx {
    pub index: usize,
    pub dy: i32, // starting offset (settles to 0)
    pub fade_in: bool,
    pub start: u64,
}

/// Fading-out item being deleted (data already removed, only visual residue)
#[derive(Clone)]
pub struct Ghost {
    pub text: String,
    pub index: usize,
    pub start: u64,
}

/// Per-list animation effects: entry/shift fx, the fading ghost, and the recorded highlight/scroll start positions.
#[derive(Clone, Default)]
pub struct ListFx {
    pub item_fx: Vec<ItemFx>,
    pub ghost: Option<Ghost>,
    /// Highlight slide: (old selected index, start time)
    pub sel_from: Option<(usize, u64)>,
    /// Smooth scroll: (old scroll, start time)
    pub scroll_from: Option<(i32, u64)>,
}

impl ListFx {
    /// Returns whether any effect is still active at the given time.
    pub fn active(&self, now: u64, dur: u64) -> bool {
        let fresh = |start: u64| now.saturating_sub(start) < dur;
        self.item_fx.iter().any(|f| fresh(f.start))
            || self.ghost.as_ref().is_some_and(|g| fresh(g.start))
            || self.sel_from.is_some_and(|(_, s)| fresh(s))
            || self.scroll_from.is_some_and(|(_, s)| fresh(s))
    }

    /// Removes effects that have settled; returns whether anything was cleaned up.
    pub fn prune(&mut self, now: u64, dur: u64) -> bool {
        let had = !self.item_fx.is_empty()
            || self.ghost.is_some()
            || self.sel_from.is_some()
            || self.scroll_from.is_some();
        let fresh = |start: u64| now.saturating_sub(start) < dur;
        self.item_fx.retain(|f| fresh(f.start));
        if self.ghost.as_ref().is_some_and(|g| !fresh(g.start)) {
            self.ghost = None;
        }
        if self.sel_from.is_some_and(|(_, s)| !fresh(s)) {
            self.sel_from = None;
        }
        if self.scroll_from.is_some_and(|(_, s)| !fresh(s)) {
            self.scroll_from = None;
        }
        let has = !self.item_fx.is_empty()
            || self.ghost.is_some()
            || self.sel_from.is_some()
            || self.scroll_from.is_some();
        had && !has // something was cleaned up
    }
}

fn lerp_t(start: u64, now: u64, dur: u64) -> f32 {
    (now.saturating_sub(start) as f32 / dur as f32).clamp(0.0, 1.0)
}

impl ListState {
    fn draw_rows<C: PixelFormat>(&self, ctx: &WidgetCtx, d: &mut Canvas<'_, C>, clip: Rect) {
        let abs = ctx.abs;
        let now = ctx.now;
        let lclip = abs.intersect(&clip).unwrap_or(clip);

        // Effective scroll (smooth-scroll interpolation)
        let eff_scroll = match self.fx.scroll_from {
            Some((from, start)) => from + ((self.scroll - from) as f32 * lerp_t(start, now, self.fx_dur)) as i32,
            None => self.scroll,
        };
        // Highlight row position (slide interpolation, in rows)
        let hl_row_f = match self.fx.sel_from {
            Some((from, start)) => {
                let t = lerp_t(start, now, self.fx_dur);
                from as f32 * (1.0 - t) + self.selected as f32 * t
            }
            None => self.selected as f32,
        };
        if !self.items.is_empty() {
            let hl = Rect::new(abs.x, abs.y + (hl_row_f * self.row_h as f32) as i32 - eff_scroll, abs.w, self.row_h);
            if hl.intersects(&lclip) {
                // Highlight with rounded corners so it doesn't cover the list's own rounded border
                d.fill_rounded(hl, ctx.resolved.radius.min(self.row_h / 2), Color::rgb(50, 70, 120), ctx.ap(255), lclip);
            }
        }
        // items (with entry/shift effects)
        for (i, item) in self.items.iter().enumerate() {
            let mut dy = 0;
            let mut opa = ctx.ap(255);
            for f in &self.fx.item_fx {
                if f.index == i {
                    let t = lerp_t(f.start, now, self.fx_dur);
                    dy = (f.dy as f32 * (1.0 - t)) as i32;
                    if f.fade_in {
                        opa = ctx.ap((255.0 * t) as u8);
                    }
                }
            }
            let ry = abs.y + i as i32 * self.row_h + dy - eff_scroll;
            let row = Rect::new(abs.x, ry, abs.w, self.row_h);
            if !row.intersects(&lclip) {
                continue;
            }
            d.draw_text_opa(Point { x: abs.x + 4, y: ry + 4 }, ctx.resolved.font, item, ctx.resolved.text_color, opa, lclip);
        }
        // Fading out of the ghost being deleted
        if let Some(g) = &self.fx.ghost {
            let t = lerp_t(g.start, now, self.fx_dur);
            let ry = abs.y + g.index as i32 * self.row_h - eff_scroll;
            let row = Rect::new(abs.x, ry, abs.w, self.row_h);
            if row.intersects(&lclip) {
                d.draw_text_opa(
                    Point { x: abs.x + 4, y: ry + 4 },
                    ctx.resolved.font,
                    &g.text,
                    ctx.resolved.text_color,
                    ctx.ap((255.0 * (1.0 - t)) as u8),
                    lclip,
                );
            }
        }
    }

    /// Selects the idx-th item (recording the highlight slide/smooth scroll effects) and adjusts scroll to keep it visible.
    /// scroll is always row-aligned (an integer multiple of row_h) to avoid half-row misalignment.
    pub(crate) fn select(&mut self, idx: usize, vis_h: i32, now: u64) {
        if self.items.is_empty() {
            return;
        }
        let nidx = idx.min(self.items.len() - 1);
        if nidx != self.selected {
            self.fx.sel_from = Some((self.selected, now));
            self.selected = nidx;
        }
        self.ensure_visible(vis_h, now);
    }

    /// Adjusts scroll: keeps selected visible and leaves no blank window at the tail (auto-scrolls up after deleting tail items).
    /// scroll is row-aligned; records a smooth scroll effect when it changes.
    pub(crate) fn ensure_visible(&mut self, vis_h: i32, now: u64) {
        let old = self.scroll;
        let item_count = self.items.len();
        if item_count == 0 {
            self.scroll = 0;
            if old != 0 {
                self.fx.scroll_from = Some((old, now));
            }
            return;
        }
        let vis_rows = (vis_h / self.row_h).max(1);
        let count = item_count as i32;
        let sel = self.selected as i32;
        let mut first = self.scroll / self.row_h; // the first currently visible row
        // Tail blank window: pull back up
        if first + vis_rows > count {
            first = (count - vis_rows).max(0);
        }
        if sel < first {
            first = sel;
        } else if sel >= first + vis_rows {
            first = sel - vis_rows + 1;
        }
        self.scroll = first * self.row_h;
        if self.scroll != old {
            self.fx.scroll_from = Some((old, now));
        }
    }

    /// Inserts an item at idx: items below slide down to make room, the new item fades in.
    /// (The capacity limit is a business decision left to the caller; the widget itself does not restrict it)
    pub(crate) fn insert(&mut self, idx: usize, text: &str, now: u64) {
        let idx = idx.min(self.items.len());
        self.items.insert(idx, text.into());
        // Shift indices of in-flight fx
        for f in self.fx.item_fx.iter_mut() {
            if f.index >= idx {
                f.index += 1;
            }
        }
        // Items below slide from their old position (the row above) into the new position
        for i in (idx + 1)..self.items.len() {
            self.fx.item_fx.push(ItemFx { index: i, dy: -self.row_h, fade_in: false, start: now });
        }
        self.fx.item_fx.push(ItemFx { index: idx, dy: 0, fade_in: true, start: now });
    }

    /// Deletes the selected item: ghost fades out, items below shift up to fill the gap
    pub(crate) fn remove(&mut self, now: u64) -> bool {
        if self.items.is_empty() || self.selected >= self.items.len() {
            return false;
        }
        let text = self.items.remove(self.selected);
        self.fx.ghost = Some(Ghost { text, index: self.selected, start: now });
        // In-flight fx: drop the deleted item, shift those below
        self.fx.item_fx.retain(|f| f.index != self.selected);
        for f in self.fx.item_fx.iter_mut() {
            if f.index > self.selected {
                f.index -= 1;
            }
        }
        // Items below slide from their old position (the row below) into the new position
        for i in self.selected..self.items.len() {
            self.fx.item_fx.push(ItemFx { index: i, dy: self.row_h, fade_in: false, start: now });
        }
        if self.selected >= self.items.len() && self.selected > 0 {
            self.selected -= 1;
        }
        true
    }
}

/// List builder: default 120 x (min(5,n)*16+2), theme_list/focused
pub type ListBuilder<C = crate::geometry::Color> = WidgetBuilder<ListCfg, C>;

/// List configuration: items, the initially selected index, and the geometry/fx props.
pub struct ListCfg {
    items: Vec<String>,
    selected: usize,
    row_h: i32,
    fx_dur: u64,
    visible_rows: usize,
}

impl ListCfg {
    /// Creates a builder with the given items.
    pub fn new<C: PixelFormat>(items: &[&str]) -> WidgetBuilder<ListCfg, C> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: ListCfg {
                items: items.iter().map(|s| (*s).into()).collect(),
                selected: 0,
                row_h: ROW_H,
                fx_dur: FX_DUR,
                visible_rows: 5,
            },
        }
    }
}

impl<C> WidgetBuilder<ListCfg, C> {
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
    /// Sets the item/highlight/scroll effect duration in ms (default `FX_DUR` = 200).
    pub fn fx_dur(mut self, ms: u64) -> Self {
        self.cfg.fx_dur = ms;
        self
    }
    /// Sets the number of visible rows used by the default height (default 5).
    pub fn visible_rows(mut self, n: usize) -> Self {
        self.cfg.visible_rows = n;
        self
    }
}

impl<C: PixelFormat> WidgetCfg<C> for ListCfg {
    fn default_style() -> Style {
        crate::style::theme_list()
    }

    fn build(self, ui: &mut Ui<C>, parent: ObjRef, mut common: CommonBuilder<C>) -> ObjRef {
        let rows = self.items.len().min(self.visible_rows).max(1) as i32;
        let (w, h) = common.size.unwrap_or((120, rows * self.row_h + 2));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(ListState { items: self.items, selected, scroll: 0, fx: ListFx::default(), row_h: self.row_h, fx_dur: self.fx_dur }),
        );
        ui.set_style(r, common.style.take().unwrap_or_else(<Self as WidgetCfg<C>>::default_style));
        let focused = common.style_focused.take().unwrap_or_else(crate::style::theme_list_focused);
        ui.set_style_focused(r, focused.clone());
        ui.set_style_edited(r, common.style_edited.take().unwrap_or_else(|| crate::style::theme_edited(&focused)));
        common.apply_tail(ui, r);
        r
    }
}

impl<C: PixelFormat> super::Widget<C> for ListState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas<'_, C>, clip: Rect) { self.draw_rows(ctx, c, clip) }
    fn tick(&mut self, _ui: &mut Ui<C>, _obj: ObjRef, now: u64) -> super::TickOut {
        let was_active = self.fx.active(now, self.fx_dur);
        let removed = self.fx.prune(now, self.fx_dur);
        // Redraw every frame while active; the frame that clears an effect also repaints once (to remove the ghost residue)
        super::TickOut { redraw: was_active || removed, active: self.fx.active(now, self.fx_dur) }
    }
    fn on_key(&mut self, ui: &mut Ui<C>, obj: ObjRef, key: Key) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        // Inner (EDITED) mode: direction keys move the selection, Enter confirms the
        // selected item (Commit = Click + exit), Esc exits without acting. Outside the
        // inner mode the list consumes nothing, so rotation moves the focus instead.
        if !ui.state(obj).contains(crate::node::State::EDITED) {
            return if key == Key::Enter { EnterEdit } else { Pass };
        }
        let n = self.items.len();
        match key {
            Key::Up | Key::Down => {
                if n > 0 {
                    let idx = if key == Key::Up { (self.selected + n - 1) % n } else { (self.selected + 1) % n };
                    let vis_h = ui.rect(obj).h;
                    let now = ui.time();
                    self.select(idx, vis_h, now);
                }
                Consumed
            }
            Key::Enter => Commit,
            Key::Esc => ExitEdit,
            _ => Consumed,
        }
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}

/// List-specific API (brought in via prelude or an explicit use)
pub trait UiListExt {
    /// Selects the idx-th item (clamped to the list range).
    fn list_select(&mut self, obj: ObjRef, idx: usize);
    /// Returns the currently selected index.
    fn list_selected(&self, obj: ObjRef) -> usize;
    /// Inserts an item at idx (items below slide down to make room, the new item fades in).
    /// The capacity limit is up to the caller (use list_len to check).
    fn list_insert(&mut self, obj: ObjRef, idx: usize, text: &str);
    /// Deletes the currently selected item (fades out + items below shift up), returns whether it succeeded
    fn list_remove(&mut self, obj: ObjRef) -> bool;
    /// Returns the number of items.
    fn list_len(&self, obj: ObjRef) -> usize;
}

impl<C: PixelFormat> UiListExt for Ui<C> {
    fn list_select(&mut self, obj: ObjRef, idx: usize) {
        let now = self.time();
        let vis_h = self.rect(obj).h;
        self.update::<ListState, _>(obj, |s| {
            s.select(idx, vis_h, now);
        });
    }

    fn list_selected(&self, obj: ObjRef) -> usize {
        self.widget::<ListState>(obj).map(|s| s.selected).unwrap_or(0)
    }

    fn list_insert(&mut self, obj: ObjRef, idx: usize, text: &str) {
        let now = self.time();
        self.update::<ListState, _>(obj, |s| {
            let idx = idx.min(s.items.len());
            // When the insertion point is above the selected item, shift the selected index down
            if !s.items.is_empty() && s.selected >= idx {
                s.selected += 1;
            }
            s.insert(idx, text, now);        });
    }

    fn list_remove(&mut self, obj: ObjRef) -> bool {
        let now = self.time();
        let vis_h = self.rect(obj).h;
        self.update::<ListState, _>(obj, |s| {
            let ok = s.remove(now);
            // Auto-scroll up to fill the window when the tail leaves a blank gap after deletion
            s.ensure_visible(vis_h, now);
            ok
        })
        .unwrap_or(false)
    }

    fn list_len(&self, obj: ObjRef) -> usize {
        self.widget::<ListState>(obj).map(|s| s.items.len()).unwrap_or(0)
    }
}
