use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
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
    pub fn active(&self, now: u64) -> bool {
        let fresh = |start: u64| now.saturating_sub(start) < FX_DUR;
        self.item_fx.iter().any(|f| fresh(f.start))
            || self.ghost.as_ref().is_some_and(|g| fresh(g.start))
            || self.sel_from.is_some_and(|(_, s)| fresh(s))
            || self.scroll_from.is_some_and(|(_, s)| fresh(s))
    }

    /// Removes effects that have settled; returns whether anything was cleaned up.
    pub fn prune(&mut self, now: u64) -> bool {
        let had = !self.item_fx.is_empty()
            || self.ghost.is_some()
            || self.sel_from.is_some()
            || self.scroll_from.is_some();
        let fresh = |start: u64| now.saturating_sub(start) < FX_DUR;
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

fn lerp_t(start: u64, now: u64) -> f32 {
    (now.saturating_sub(start) as f32 / FX_DUR as f32).clamp(0.0, 1.0)
}

pub(crate) fn draw(items: &[String], selected: usize, scroll: i32, fx: &ListFx, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let now = ctx.now;
    let lclip = abs.intersect(&clip).unwrap_or(clip);

    // Effective scroll (smooth-scroll interpolation)
    let eff_scroll = match fx.scroll_from {
        Some((from, start)) => from + ((scroll - from) as f32 * lerp_t(start, now)) as i32,
        None => scroll,
    };
    // Highlight row position (slide interpolation, in rows)
    let hl_row_f = match fx.sel_from {
        Some((from, start)) => {
            let t = lerp_t(start, now);
            from as f32 * (1.0 - t) + selected as f32 * t
        }
        None => selected as f32,
    };
    if !items.is_empty() {
        let hl = Rect::new(abs.x, abs.y + (hl_row_f * ROW_H as f32) as i32 - eff_scroll, abs.w, ROW_H);
        if hl.intersects(&lclip) {
            // Highlight with rounded corners so it doesn't cover the list's own rounded border
            d.fill_rounded(hl, ctx.resolved.radius.min(ROW_H / 2), Color::rgb(50, 70, 120), ctx.ap(255), lclip);
        }
    }
    // items (with entry/shift effects)
    for (i, item) in items.iter().enumerate() {
        let mut dy = 0;
        let mut opa = ctx.ap(255);
        for f in &fx.item_fx {
            if f.index == i {
                let t = lerp_t(f.start, now);
                dy = (f.dy as f32 * (1.0 - t)) as i32;
                if f.fade_in {
                    opa = ctx.ap((255.0 * t) as u8);
                }
            }
        }
        let ry = abs.y + i as i32 * ROW_H + dy - eff_scroll;
        let row = Rect::new(abs.x, ry, abs.w, ROW_H);
        if !row.intersects(&lclip) {
            continue;
        }
        d.draw_text_opa(Point { x: abs.x + 4, y: ry + 4 }, ctx.resolved.font, item, ctx.resolved.text_color, opa, lclip);
    }
    // Fading out of the ghost being deleted
    if let Some(g) = &fx.ghost {
        let t = lerp_t(g.start, now);
        let ry = abs.y + g.index as i32 * ROW_H - eff_scroll;
        let row = Rect::new(abs.x, ry, abs.w, ROW_H);
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
/// scroll is always row-aligned (an integer multiple of ROW_H) to avoid half-row misalignment.
pub(crate) fn select(items: &[String], selected: &mut usize, scroll: &mut i32, fx: &mut ListFx, idx: usize, vis_h: i32, now: u64) {
    if items.is_empty() {
        return;
    }
    let nidx = idx.min(items.len() - 1);
    if nidx != *selected {
        fx.sel_from = Some((*selected, now));
        *selected = nidx;
    }
    ensure_visible(*selected, items.len(), scroll, fx, vis_h, now);
}

/// Adjusts scroll: keeps selected visible and leaves no blank window at the tail (auto-scrolls up after deleting tail items).
/// scroll is row-aligned; records a smooth scroll effect when it changes.
pub(crate) fn ensure_visible(selected: usize, item_count: usize, scroll: &mut i32, fx: &mut ListFx, vis_h: i32, now: u64) {
    let old = *scroll;
    if item_count == 0 {
        *scroll = 0;
        if old != 0 {
            fx.scroll_from = Some((old, now));
        }
        return;
    }
    let vis_rows = (vis_h / ROW_H).max(1);
    let count = item_count as i32;
    let sel = selected as i32;
    let mut first = *scroll / ROW_H; // the first currently visible row
    // Tail blank window: pull back up
    if first + vis_rows > count {
        first = (count - vis_rows).max(0);
    }
    if sel < first {
        first = sel;
    } else if sel >= first + vis_rows {
        first = sel - vis_rows + 1;
    }
    *scroll = first * ROW_H;
    if *scroll != old {
        fx.scroll_from = Some((old, now));
    }
}

/// Inserts an item at idx: items below slide down to make room, the new item fades in.
/// (The capacity limit is a business decision left to the caller; the widget itself does not restrict it)
pub(crate) fn insert(items: &mut Vec<String>, fx: &mut ListFx, idx: usize, text: &str, now: u64) {
    let idx = idx.min(items.len());
    items.insert(idx, text.into());
    // Shift indices of in-flight fx
    for f in fx.item_fx.iter_mut() {
        if f.index >= idx {
            f.index += 1;
        }
    }
    // Items below slide from their old position (the row above) into the new position
    for i in (idx + 1)..items.len() {
        fx.item_fx.push(ItemFx { index: i, dy: -ROW_H, fade_in: false, start: now });
    }
    fx.item_fx.push(ItemFx { index: idx, dy: 0, fade_in: true, start: now });
}

/// Deletes the selected item: ghost fades out, items below shift up to fill the gap
pub(crate) fn remove(items: &mut Vec<String>, fx: &mut ListFx, selected: &mut usize, now: u64) -> bool {
    if items.is_empty() || *selected >= items.len() {
        return false;
    }
    let text = items.remove(*selected);
    fx.ghost = Some(Ghost { text, index: *selected, start: now });
    // In-flight fx: drop the deleted item, shift those below
    fx.item_fx.retain(|f| f.index != *selected);
    for f in fx.item_fx.iter_mut() {
        if f.index > *selected {
            f.index -= 1;
        }
    }
    // Items below slide from their old position (the row below) into the new position
    for i in *selected..items.len() {
        fx.item_fx.push(ItemFx { index: i, dy: ROW_H, fade_in: false, start: now });
    }
    if *selected >= items.len() && *selected > 0 {
        *selected -= 1;
    }
    true
}

/// List builder: default 120 x (min(5,n)*16+2), theme_list/focused
pub type ListBuilder = WidgetBuilder<ListCfg>;

/// List configuration: items and the initially selected index.
pub struct ListCfg {
    items: Vec<String>,
    selected: usize,
}

impl ListCfg {
    /// Creates a builder with the given items.
    pub fn new(items: &[&str]) -> WidgetBuilder<ListCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: ListCfg { items: items.iter().map(|s| (*s).into()).collect(), selected: 0 },
        }
    }
}

impl WidgetBuilder<ListCfg> {
    /// Sets the initially selected index.
    pub fn selected(mut self, idx: usize) -> Self {
        self.cfg.selected = idx;
        self
    }
}

impl WidgetCfg for ListCfg {
    fn default_style() -> Style {
        crate::style::theme_list()
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let rows = self.items.len().min(5).max(1) as i32;
        let (w, h) = common.size.unwrap_or((120, rows * ROW_H + 2));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(ListState { items: self.items, selected, scroll: 0, fx: ListFx::default() }),
        );
        ui.set_style(r, common.style.take().unwrap_or_else(Self::default_style));
        ui.set_style_focused(r, common.style_focused.take().unwrap_or_else(crate::style::theme_list_focused));
        common.apply_tail(ui, r);
        r
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, items: &[&str]) -> ObjRef {
    ListCfg::new(items).build(ui, parent)
}

impl super::Widget for ListState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(&self.items, self.selected, self.scroll, &self.fx, ctx, c, clip) }
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, now: u64) -> super::TickOut {
        let was_active = self.fx.active(now);
        let removed = self.fx.prune(now);
        // Redraw every frame while active; the frame that clears an effect also repaints once (to remove the ghost residue)
        super::TickOut { redraw: was_active || removed, active: self.fx.active(now) }
    }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        let n = self.items.len();
        match key {
            Key::Up | Key::Down => {
                if n > 0 {
                    let idx = if key == Key::Up { (self.selected + n - 1) % n } else { (self.selected + 1) % n };
                    let vis_h = ui.rect(obj).h;
                    let now = ui.time();
                    select(&self.items, &mut self.selected, &mut self.scroll, &mut self.fx, idx, vis_h, now);
                }
                super::KeyOutcome::Consumed
            }
            _ => super::KeyOutcome::Pass,
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

impl UiListExt for Ui {
    fn list_select(&mut self, obj: ObjRef, idx: usize) {
        let now = self.time();
        let vis_h = self.rect(obj).h;
        self.update::<ListState, _>(obj, |s| {
            select(&s.items, &mut s.selected, &mut s.scroll, &mut s.fx, idx, vis_h, now);
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
            insert(&mut s.items, &mut s.fx, idx, text, now);
        });
    }

    fn list_remove(&mut self, obj: ObjRef) -> bool {
        let now = self.time();
        let vis_h = self.rect(obj).h;
        self.update::<ListState, _>(obj, |s| {
            let ok = remove(&mut s.items, &mut s.fx, &mut s.selected, now);
            // Auto-scroll up to fill the window when the tail leaves a blank gap after deletion
            ensure_visible(s.selected, s.items.len(), &mut s.scroll, &mut s.fx, vis_h, now);
            ok
        })
        .unwrap_or(false)
    }

    fn list_len(&self, obj: ObjRef) -> usize {
        self.widget::<ListState>(obj).map(|s| s.items.len()).unwrap_or(0)
    }
}
