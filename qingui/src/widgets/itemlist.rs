use alloc::boxed::Box;

use crate::arena::ObjRef;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Sizing};
use crate::node::State;
use crate::pixel::PixelFormat;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::KeyOutcome;

/// Container-type list: items are ordinary child nodes (the user builds the content freely), the widget only handles selection/navigation/scrolling.
/// Structure: ItemList (viewport, CLIP_CHILDREN) > content (Flex column, translated to scroll) > items
pub struct ItemListState {
    pub selected: usize,
    pub(crate) content: ObjRef,
    pub(crate) sel_style: Style,
}

/// Transparent container style (only for layout/scroll, draws no background)
fn transparent() -> Style {
    Style::default()
}

/// Base style for item containers: transparent background (highlight overlaid by style_selected when SELECTED)
pub(crate) fn item_base_style() -> Style {
    transparent()
}

fn column_flex() -> Flex {
    Flex {
        dir: FlexDir::Column,
        wrap: false,
        main: Align::Start,
        cross: Align::Start,
        track: Align::Start,
        gap: 0,
    }
}

/// Builder for the ItemList widget.
pub type ItemListBuilder<C = crate::geometry::Color> = WidgetBuilder<ItemListCfg, C>;

/// ItemList configuration: optional custom selected-item style.
pub struct ItemListCfg {
    style_selected: Option<Style>,
}

impl ItemListCfg {
    /// Creates an empty builder.
    pub fn new<C: PixelFormat>() -> WidgetBuilder<ItemListCfg, C> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ItemListCfg { style_selected: None } }
    }
}

impl<C> WidgetBuilder<ItemListCfg, C> {
    /// The selected style for items (overlaid on State::SELECTED).
    /// Note: the highlight sets bg_color explicitly; the item base leaves it None,
    /// so a selected style without bg_color paints no highlight.
    pub fn style_selected(mut self, s: Style) -> Self {
        self.cfg.style_selected = Some(s);
        self
    }
}

impl<C: PixelFormat> WidgetCfg<C> for ItemListCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_color = Some(Color::rgb(34, 34, 44));
        s.border_color = Some(Color::rgb(70, 70, 90));
        s.border_width = Some(1);
        s
    }

    fn build(self, ui: &mut Ui<C>, parent: ObjRef, mut common: CommonBuilder<C>) -> ObjRef {
        let (w, h) = common.size.unwrap_or((120, 100));
        // The viewport node is first created as an Obj placeholder (the content reference needs the handle after the self-reference)
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(super::obj::Manual));
        ui.set_clip_children(r, true);
        // content: a Flex column container, width GROW, transparent background
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0),
            Box::new(super::flexbox::FlexLayout { flex: column_flex() }));
        ui.set_style(content, transparent());
        ui.set_sizing(content, Some(Sizing::GROW), None);
        // Replace the placeholder kind with the real one
        let sel_style = self.style_selected.unwrap_or_else(default_sel_style);
        if let Some(n) = ui.kind_mut(r) {
            *n = Box::new(ItemListState { selected: 0, content, sel_style });
        }
        // Viewport style (defaults to theme_list's dark background + border)
        let vs = common.style.take().unwrap_or_else(<Self as WidgetCfg<C>>::default_style);
        ui.set_style(r, vs);
        let focused = common.style_focused.take().unwrap_or_else(crate::style::theme_list_focused);
        ui.set_style_focused(r, focused.clone());
        ui.set_style_edited(r, common.style_edited.take().unwrap_or_else(|| crate::style::theme_edited(&focused)));
        common.apply_tail(ui, r);
        r
    }
}

/// Default selected style (matches the text List highlight color rgb(50,70,120); sets bg_color explicitly so the highlight paints)
fn default_sel_style() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(50, 70, 120));
    s
}

impl<C: PixelFormat> super::Widget<C> for ItemListState {
    // ItemList is also a container: its content is drawn by child nodes (the default draw paints nothing)
    fn on_key(&mut self, ui: &mut Ui<C>, obj: ObjRef, key: Key) -> KeyOutcome {
        // Inner (EDITED) mode: direction keys move the selection, Enter confirms the
        // selected item (Commit = Click + exit), Esc exits without acting. Outside the
        // inner mode nothing is consumed, so rotation moves the focus instead.
        if !ui.state(obj).contains(State::EDITED) {
            return if key == Key::Enter { KeyOutcome::EnterEdit } else { KeyOutcome::Pass };
        }
        match key {
            // Navigation needs child nodes/scroll/events; the kind is taken out during
            // on_key, so mutate `self` directly and operate on the children via ui.
            Key::Up | Key::Down => {
                let d = if key == Key::Up { -1 } else { 1 };
                let kids = ui.children(self.content);
                let n = kids.len();
                if n > 0 {
                    let next = (self.selected as i32 + d).rem_euclid(n as i32) as usize;
                    // The user may bypass itemlist_remove_selected and delete an item directly: clamp the out-of-range selected and write it back to eliminate drift
                    let cur = self.selected.min(n - 1);
                    self.selected = cur;
                    let nidx = next.min(n - 1);
                    if nidx != cur {
                        ui.set_state(kids[cur], State::SELECTED, false);
                        ui.set_state(kids[nidx], State::SELECTED, true);
                        self.selected = nidx;
                        ensure_visible(ui, obj, self.content, nidx);
                        ui.send_event(obj, crate::event::EventKind::ValueChanged);
                    }
                }
                // An empty list is consumed too (matches the old NavSelect semantics)
                KeyOutcome::Consumed
            }
            Key::Enter => KeyOutcome::Commit,
            Key::Esc => KeyOutcome::ExitEdit,
            _ => KeyOutcome::Consumed,
        }
    }
    fn value(&self) -> i32 { self.selected as i32 }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
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

impl<C: PixelFormat> UiItemListExt for Ui<C> {
    /// Appends an item container to the ItemList (an Obj, width GROW, transparent background, with the SELECTED style),
    /// and returns that container (the user builds content inside it); returns None if il is not an ItemList
    fn itemlist_add_item(&mut self, il: ObjRef) -> Option<ObjRef> {
        let (content, sel_style, was_empty) = {
            let s = self.widget::<ItemListState>(il)?;
            (s.content, s.sel_style.clone(), self.children(s.content).is_empty())
        };
        let item = self.insert_node(content, Rect::default(), alloc::boxed::Box::new(super::obj::Manual));
        let st = item_base_style();
        self.set_style(item, st);
        self.set_sizing(item, Some(Sizing::GROW), None);
        self.set_style_selected(item, sel_style);
        // The first item is automatically selected
        if was_empty {
            self.set_state(item, State::SELECTED, true);
        }
        Some(item)
    }

    /// Deletes the ItemList's selected item (returns false on an empty list), clamps selected and shifts the selection to an adjacent item
    fn itemlist_remove_selected(&mut self, il: ObjRef) -> bool {
        let Some((content, selected)) = self.widget::<ItemListState>(il).map(|s| (s.content, s.selected)) else {
            return false;
        };
        let kids = self.children(content);
        if kids.is_empty() || selected >= kids.len() {
            return false;
        }
        self.delete(kids[selected]);
        let new_len = kids.len() - 1;
        let new_sel = if new_len == 0 { 0 } else { selected.min(new_len - 1) };
        self.update::<ItemListState, _>(il, |s| s.selected = new_sel);
        // Shift the selection to an adjacent item (deleting a middle item → the former next item; deleting the last item → the former previous item)
        if new_len > 0 {
            let target = if selected < new_len { kids[selected + 1] } else { kids[selected - 1] };
            self.set_state(target, State::SELECTED, true);
        }
        ensure_visible(self, il, content, new_sel);
        true
    }

    /// Selects the idx-th item of the ItemList (clamped to a valid range); switches and sends ValueChanged only on change
    fn itemlist_select(&mut self, il: ObjRef, idx: usize) {
        let Some((content, selected)) = self.widget::<ItemListState>(il).map(|s| (s.content, s.selected)) else {
            return;
        };
        let kids = self.children(content);
        if kids.is_empty() {
            return;
        }
        // The user may bypass itemlist_remove_selected and delete an item directly: clamp the out-of-range selected and write it back to eliminate drift
        let cur = selected.min(kids.len() - 1);
        if cur != selected {
            self.update::<ItemListState, _>(il, |s| {
                if s.selected != cur {
                    s.selected = cur;
                }
            });
        }
        let nidx = idx.min(kids.len() - 1);
        if nidx == cur {
            return;
        }
        self.set_state(kids[cur], State::SELECTED, false);
        self.set_state(kids[nidx], State::SELECTED, true);
        self.update::<ItemListState, _>(il, |s| s.selected = nidx);
        ensure_visible(self, il, content, nidx);
        self.send_event(il, crate::event::EventKind::ValueChanged);
    }

    fn itemlist_selected(&self, il: ObjRef) -> usize {
        self.widget::<ItemListState>(il).map(|s| s.selected).unwrap_or(0)
    }

    fn itemlist_len(&self, il: ObjRef) -> usize {
        self.widget::<ItemListState>(il).map(|s| self.children(s.content).len()).unwrap_or(0)
    }
}

/// Scrolls content (translate.y) so the selected item is visible in the viewport (instant, no animation)
fn ensure_visible<C: PixelFormat>(ui: &mut Ui<C>, il: ObjRef, content: ObjRef, selected: usize) {
    // Item positions are produced by Flex layout: flush pending layout first so the rects read below are current
    if ui.layout_dirty {
        ui.layout_pass();
        ui.layout_dirty = false;
    }
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
