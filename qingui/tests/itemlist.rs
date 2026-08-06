use qingui::display::Flush;
use qingui::input::Key;
use qingui::node::State;
use qingui::prelude::*;
use qingui::style::Style;
use qingui::widgets::itemlist::ItemListBuilder;
use qingui::widgets::label::LabelCfg;
use qingui::{Color, EventKind, ObjRef, Rect, Ui};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}
fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Color {
    let chunks = &rec.borrow().chunks;
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

/// Builds a 60x40 viewport + 4 items (each 20 high: Label 8px + top/bottom padding)
fn build4() -> (Ui, ObjRef, Vec<ObjRef>) {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    let mut items = Vec::new();
    for t in ["a", "b", "c", "d"] {
        let it = ui.itemlist_add_item(il).expect("add_item on ItemList");
        LabelCfg::new(t).build(&mut ui, it);
        ui.set_size(it, 60, 20);
        items.push(it);
    }
    // Layout: ItemList sits on the screen, no flex by default → manual layout
    ui.set_pos(il, 0, 0);
    (ui, il, items)
}

#[test]
fn add_items_and_initial_selection() {
    let (ui, il, items) = build4();
    assert_eq!(ui.itemlist_len(il), 4);
    assert_eq!(ui.itemlist_selected(il), 0);
    assert!(ui.state(items[0]).contains(State::SELECTED)); // first item auto-selected
    assert!(!ui.state(items[1]).contains(State::SELECTED));
}

#[test]
fn select_moves_selected_state_and_fires_value_changed() {
    let (mut ui, il, items) = build4();
    let hits = Rc::new(Cell::new(0));
    let h = hits.clone();
    ui.add_event_cb(il, EventKind::ValueChanged, Box::new(move |_, _, _| h.set(h.get() + 1)));
    ui.itemlist_select(il, 2);
    assert_eq!(ui.itemlist_selected(il), 2);
    assert!(!ui.state(items[0]).contains(State::SELECTED));
    assert!(ui.state(items[2]).contains(State::SELECTED));
    assert_eq!(hits.get(), 1);
    ui.itemlist_select(il, 2); // no change: event not re-fired
    assert_eq!(hits.get(), 1);
    assert_eq!(ui.value(il), 2); // value() integration
}

#[test]
fn keyboard_nav_wraps_and_consumes() {
    let (mut ui, il, _items) = build4();
    ui.group_add(il);
    ui.keypad_input(Key::Up); // wraps: 0 → 3
    assert_eq!(ui.itemlist_selected(il), 3);
    ui.keypad_input(Key::Down); // wraps: 3 → 0
    assert_eq!(ui.itemlist_selected(il), 0);
}

#[test]
fn ensure_visible_scrolls_content() {
    let (mut ui, il, items) = build4();
    ui.itemlist_select(il, 3); // item3 is outside the viewport (40 high) → scroll
    // After scrolling, item3 aligns to the bottom of the viewport: item3 abs y = 40 - 20 = 20
    assert_eq!(ui.abs_rect(items[3]).y, 20);
    assert_eq!(ui.abs_rect(items[0]).y, -40); // item0 scrolled out above the viewport
}

#[test]
fn viewport_clips_scrolled_items() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut ss = Style::default();
    ss.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    ui.set_style(scr, ss);
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    ui.set_pos(il, 10, 30); // viewport kept away from the screen edges so scrolled-out areas are still assertable on screen
    for _ in 0..4 {
        let it = ui.itemlist_add_item(il).unwrap();
        // Solid white background per item for easy pixel assertions
        ui.set_style(it, Style::new().bg(Color::WHITE));
        ui.set_size(it, 60, 20);
    }
    ui.itemlist_select(il, 3); // scroll 40px: item2 → y 30..50, item3 → y 50..70
    ui.render();
    assert_eq!(px(&rec, 15, 35), Color::WHITE); // item2 visible
    assert_eq!(px(&rec, 15, 55), Color::rgb(50, 70, 120)); // item3 selected: default selected style overlaid
    assert_eq!(px(&rec, 15, 25), Color::BLACK); // item1 (abs y 10..30) scrolled out above the viewport: clipped
    assert_eq!(px(&rec, 15, 5), Color::BLACK);  // item0 (abs y -10..10) scrolled out above the viewport: clipped
}

#[test]
fn remove_selected_clamps_and_reselects() {
    let (mut ui, il, items) = build4();
    ui.itemlist_select(il, 3);
    assert!(ui.itemlist_remove_selected(il)); // delete item3
    assert_eq!(ui.itemlist_len(il), 3);
    assert_eq!(ui.itemlist_selected(il), 2); // converges to the last item
    assert!(ui.state(items[2]).contains(State::SELECTED)); // the new selection is set
}

#[test]
fn empty_list_key_does_not_panic_and_consumes() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    ui.group_add(il);
    ui.keypad_input(Key::Up); // empty list: no panic, key consumed (focus does not move)
    assert_eq!(ui.focused(), Some(il));
    assert!(!ui.itemlist_remove_selected(il));
}

/// If the user deletes items directly, bypassing remove_selected, the selected index drifts out of range but
/// select/keyboard navigation must not panic; itemlist_select clamps the out-of-range selected back into a legal range and writes it back
#[test]
fn direct_item_delete_does_not_panic_and_clamps_selection() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    let mut items = Vec::new();
    for t in ["a", "b", "c"] {
        let it = ui.itemlist_add_item(il).expect("add_item on ItemList");
        LabelCfg::new(t).build(&mut ui, it);
        ui.set_size(it, 60, 20);
        items.push(it);
    }
    ui.set_pos(il, 0, 0);
    ui.group_add(il);
    // Delete the last item directly (not selected): select and keyboard navigation do not panic, selected stays legal
    ui.delete(items[2]);
    ui.itemlist_select(il, 0);
    ui.keypad_input(Key::Down); // keyboard navigation goes through the same itemlist_select path
    assert!(ui.itemlist_selected(il) < ui.itemlist_len(il));
    assert_eq!(ui.itemlist_selected(il), 1);
    // Delete the currently selected item directly: selected=1 out of range (len=1), select clamps it back, no panic
    ui.delete(items[1]);
    ui.itemlist_select(il, 0);
    assert_eq!(ui.itemlist_selected(il), 0); // the drift is removed by clamping
    assert!(ui.itemlist_selected(il) < ui.itemlist_len(il));
}

/// Uneven-height items: navigation does not scroll for already-visible items; items not visible are aligned to the top/bottom
/// Viewport 60x40, item heights 10/30/20 (gap 0 → rect y is 0..10 / 10..40 / 40..60 respectively, total height 60 > 40)
#[test]
fn uneven_items_scroll_minimally() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    let mut items = Vec::new();
    for h in [10, 30, 20] {
        let it = ui.itemlist_add_item(il).expect("add_item on ItemList");
        ui.set_size(it, 60, h);
        items.push(it);
    }
    ui.set_pos(il, 0, 0);
    // Select item1 (height 30, y 10..40): already inside the viewport (top 10 ≥ 0, bottom 40 ≤ 40), no scrolling
    ui.itemlist_select(il, 1);
    assert_eq!(ui.abs_rect(items[1]).y, 10);
    // Select item2 (y 40..60, fully outside the viewport) → bottom-align: item2's bottom touches the viewport bottom 40, i.e. y = 20
    ui.itemlist_select(il, 2);
    assert_eq!(ui.abs_rect(items[2]).y, 20);
    assert_eq!(ui.abs_rect(items[1]).y, -10); // item1 scrolls out above the viewport with it
    // Select item0 again (y 0..10, already scrolled out above) → top-align, scrolls back to 0
    ui.itemlist_select(il, 0);
    assert_eq!(ui.abs_rect(items[0]).y, 0);
}

/// Enter fires Clicked on a focused ItemList; navigation keys (Down) are consumed by the widget and do not fire it
#[test]
fn enter_fires_clicked_but_nav_key_does_not() {
    let (mut ui, il, _items) = build4();
    ui.group_add(il);
    let hits = Rc::new(Cell::new(0));
    let h = hits.clone();
    ui.add_event_cb(il, EventKind::Clicked, Box::new(move |_, _, _| h.set(h.get() + 1)));
    ui.keypad_input(Key::Enter);
    assert_eq!(hits.get(), 1);
    ui.keypad_input(Key::Down); // navigation key consumed by the ItemList → does not fire Clicked
    assert_eq!(hits.get(), 1);
}
