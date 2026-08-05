use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Sizing};
use crate::node::State;
use crate::style::{Layout, Style};
use crate::ui::Ui;
use super::{KeyCtx, KeyOutcome, WidgetKind};

/// Container-type list: items are ordinary child nodes (the user builds the content freely), the widget only handles selection/navigation/scrolling.
/// Structure: ItemList (viewport, CLIP_CHILDREN) > content (Flex column, translated to scroll) > items
pub struct ItemListState {
    pub selected: usize,
    pub(crate) content: ObjRef,
    pub(crate) sel_style: Style,
}

impl ItemListState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: KeyCtx) -> KeyOutcome {
        match key {
            // Navigation details need Ui (child nodes/scroll/events); executed via the Deferred exec fn after the kind is put back
            Key::Up => KeyOutcome::Deferred(nav_select_exec, -1),
            Key::Down => KeyOutcome::Deferred(nav_select_exec, 1),
            _ => KeyOutcome::Pass,
        }
    }
}

/// List navigation exec fn: Ui calls it after putting the kind back (obj's kind is restored, so it can safely access itself via ui).
/// Semantics match the old NavSelect branch of apply_key_outcome exactly: an empty list is consumed too.
pub(crate) fn nav_select_exec(ui: &mut Ui, il: ObjRef, d: i32) {
    let n = ui.itemlist_len(il);
    if n > 0 {
        let cur = ui.itemlist_selected(il);
        let next = (cur as i32 + d).rem_euclid(n as i32) as usize;
        ui.itemlist_select(il, next);
    }
}

/// Transparent container style (only for layout/scroll, draws no background)
fn transparent() -> Style {
    let mut s = Style::default();
    s.bg_opa = Some(0);
    s
}

/// Base style for item containers: transparent background (highlight overlaid by style_selected when SELECTED)
pub(crate) fn item_base_style() -> Style {
    transparent()
}

fn column_layout() -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Column,
        wrap: false,
        main: Align::Start,
        cross: Align::Start,
        track: Align::Start,
        gap: 0,
    })
}

/// ItemList builder: default 120x100, with inline default viewport style (dark background + border) and default selected style (blue background)
pub struct ItemListBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_selected: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl ItemListBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self { size: None, style: None, style_selected: None, style_focused: None, sizing: None, transition: None, events: Vec::new() }
    }
    /// Sets the widget size.
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    /// Sets the style.
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    /// The selected style for items (overlaid on State::SELECTED).
    /// Note: it must explicitly include bg_opa, otherwise the item base's bg_opa(0) makes the highlight invisible
    pub fn style_selected(mut self, s: Style) -> Self { self.style_selected = Some(s); self }
    /// The focused style for the viewport (overlaid on State::FOCUSED)
    pub fn style_focused(mut self, s: Style) -> Self { self.style_focused = Some(s); self }
    /// Sets the width/height sizing.
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self { self.sizing = Some((w, h)); self }
    /// Sets the transition duration and easing.
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self { self.transition = Some((dur, easing)); self }
    /// Registers an event callback.
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    /// Builds the widget into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((120, 100));
        // The viewport node is first created as an Obj placeholder (the content reference needs the handle after the self-reference)
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(super::obj::ObjState));
        ui.set_clip_children(r, true);
        // content: a Flex column container, width GROW, transparent background
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0), WidgetKind::Obj(super::obj::ObjState));
        ui.set_style(content, transparent());
        ui.set_sizing(content, Some(Sizing::GROW), None);
        ui.set_layout(content, column_layout());
        // Replace the placeholder kind with the real one
        let sel_style = self.style_selected.unwrap_or_else(default_sel_style);
        if let Some(n) = ui.arena.get_mut(r) {
            n.kind = WidgetKind::ItemList(ItemListState { selected: 0, content, sel_style });
        }
        // Viewport style (defaults to theme_list's dark background + border)
        let mut vs = self.style.unwrap_or_else(|| {
            let mut s = Style::default();
            s.bg_color = Some(Color::rgb(34, 34, 44));
            s.bg_opa = Some(255);
            s.border_color = Some(Color::rgb(70, 70, 90));
            s.border_width = Some(1);
            s
        });
        ui.set_style(r, {
            if vs.bg_opa.is_none() { vs.bg_opa = Some(255); }
            vs
        });
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        ui.set_style_focused(r, self.style_focused.unwrap_or_else(crate::style::theme_list_focused));
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}

/// Default selected style (matches the text List highlight color rgb(50,70,120); must explicitly set bg_opa(255))
fn default_sel_style() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(50, 70, 120));
    s.bg_opa = Some(255);
    s
}

impl super::WidgetBehavior for ItemListState {
    // ItemList is also a container: its content is drawn by child nodes
    fn draw(&self, _ctx: &super::WidgetCtx, _d: &mut crate::draw::DrawBuf, _clip: Rect) {}
    fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.selected as i32 }
}

/// ItemList data/navigation API (brought in via prelude or an explicit use)
pub trait UiItemListExt {
    /// Appends an item container to the ItemList and returns it; None if il is not an ItemList.
    fn itemlist_add_item(&mut self, il: ObjRef) -> Option<ObjRef>;
    /// Deletes the selected item; returns false if the list is empty.
    fn itemlist_remove_selected(&mut self, il: ObjRef) -> bool;
    /// Selects the idx-th item (clamped to a valid range).
    fn itemlist_select(&mut self, il: ObjRef, idx: usize);
    /// Returns the currently selected index.
    fn itemlist_selected(&self, il: ObjRef) -> usize;
    /// Returns the number of items.
    fn itemlist_len(&self, il: ObjRef) -> usize;
}

impl UiItemListExt for Ui {
    /// Appends an item container to the ItemList (an Obj, width GROW, transparent background, with the SELECTED style),
    /// and returns that container (the user builds content inside it); returns None if il is not an ItemList
    fn itemlist_add_item(&mut self, il: ObjRef) -> Option<ObjRef> {
        let (content, sel_style, was_empty) = {
            let s = self.kind(il)?.as_itemlist()?;
            (s.content, s.sel_style.clone(), self.children(s.content).is_empty())
        };
        let item = self.insert_node(content, Rect::default(), WidgetKind::Obj(super::obj::ObjState));
        let mut st = item_base_style();
        st.sizing_w = Some(Sizing::GROW);
        self.set_style(item, st);
        self.set_style_selected(item, sel_style);
        // The first item is automatically selected
        if was_empty {
            self.set_state(item, State::SELECTED, true);
        }
        Some(item)
    }

    /// Deletes the ItemList's selected item (returns false on an empty list), clamps selected and shifts the selection to an adjacent item
    fn itemlist_remove_selected(&mut self, il: ObjRef) -> bool {
        let Some((content, selected)) = self
            .kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| (s.content, s.selected))
        else {
            return false;
        };
        let kids = self.children(content);
        if kids.is_empty() || selected >= kids.len() {
            return false;
        }
        self.delete(kids[selected]);
        let new_len = kids.len() - 1;
        let new_sel = if new_len == 0 { 0 } else { selected.min(new_len - 1) };
        if let Some(s) = self.kind_mut(il).and_then(|k| k.as_itemlist_mut()) {
            s.selected = new_sel;
        }
        // Shift the selection to an adjacent item (deleting a middle item → the former next item; deleting the last item → the former previous item)
        if new_len > 0 {
            let target = if selected < new_len { kids[selected + 1] } else { kids[selected - 1] };
            self.set_state(target, State::SELECTED, true);
        }
        ensure_visible(self, il);
        true
    }

    /// Selects the idx-th item of the ItemList (clamped to a valid range); switches and sends ValueChanged only on change
    fn itemlist_select(&mut self, il: ObjRef, idx: usize) {
        let Some((content, cur)) = self
            .kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| (s.content, s.selected))
        else {
            return;
        };
        let kids = self.children(content);
        if kids.is_empty() {
            return;
        }
        // The user may bypass itemlist_remove_selected and delete an item directly: clamp the out-of-range selected and write it back to eliminate drift
        let cur = cur.min(kids.len() - 1);
        if let Some(s) = self.kind_mut(il).and_then(|k| k.as_itemlist_mut()) {
            if s.selected != cur {
                s.selected = cur;
            }
        }
        let nidx = idx.min(kids.len() - 1);
        if nidx == cur {
            return;
        }
        self.set_state(kids[cur], State::SELECTED, false);
        self.set_state(kids[nidx], State::SELECTED, true);
        if let Some(s) = self.kind_mut(il).and_then(|k| k.as_itemlist_mut()) {
            s.selected = nidx;
        }
        ensure_visible(self, il);
        self.send_event(il, crate::event::EventKind::ValueChanged);
    }

    fn itemlist_selected(&self, il: ObjRef) -> usize {
        self.kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| s.selected)
            .unwrap_or(0)
    }

    fn itemlist_len(&self, il: ObjRef) -> usize {
        self.kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| self.children(s.content).len())
            .unwrap_or(0)
    }
}

/// Scrolls content (translate.y) so the selected item is visible in the viewport (instant, no animation)
fn ensure_visible(ui: &mut Ui, il: ObjRef) {
    // Item positions are produced by Flex layout: flush pending layout first so the rects read below are current
    if ui.layout_dirty {
        ui.layout_pass();
        ui.layout_dirty = false;
    }
    let Some((content, selected)) = ui
        .kind(il)
        .and_then(|k| k.as_itemlist())
        .map(|s| (s.content, s.selected))
    else {
        return;
    };
    let Some(item) = ui.children(content).get(selected).copied() else {
        return;
    };
    let vp_h = ui.rect(il).h;
    // The viewport has no layout, so content's height is not stretched: use the child items' maximum bottom edge as the total content height
    let content_h = ui
        .children(content)
        .iter()
        .map(|&k| ui.rect(k).bottom())
        .max()
        .unwrap_or(0);
    let ir = ui.rect(item); // the item's local rect relative to content
    let off = ui.translate(content).y;
    let mut new_off = if content_h <= vp_h {
        0 // content fits on one screen: no scrolling
    } else if ir.h >= vp_h {
        -ir.y // item taller than the viewport: align to the top
    } else {
        let top = ir.y + off;
        let bottom = top + ir.h;
        if top < 0 {
            off - top // scroll up: align to the top
        } else if bottom > vp_h {
            off - (bottom - vp_h) // scroll down: align to the bottom
        } else {
            off
        }
    };
    new_off = new_off.min(0); // never reveal blank space above the content top
    if new_off != off {
        ui.set_translate(content, 0, new_off);
    }
}
