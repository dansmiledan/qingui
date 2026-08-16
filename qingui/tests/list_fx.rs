use qingui::prelude::*;
use qingui::widgets::list::ListCfg;
use qingui::Ui;

fn list_fx(ui: &Ui, l: qingui::ObjRef) -> qingui::widgets::list::ListFx {
    ui.as_list(l).unwrap().fx.clone()
}

#[test]
fn insert_adds_item_with_fade_and_shift_fx() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.list_insert(l, 1, "x");
    assert_eq!(ui.list_len(l), 4);
    let s = ui.as_list(l).unwrap();
    assert_eq!(s.items, ["a", "x", "b", "c"]);
    // The new item fades in
    assert!(s.fx.item_fx.iter().any(|f| f.index == 1 && f.fade_in));
    // Items below slide down to make room (start offset is negative)
    assert!(s.fx.item_fx.iter().any(|f| f.index == 2 && f.dy < 0));
    assert!(s.fx.item_fx.iter().any(|f| f.index == 3 && f.dy < 0));
}

#[test]
fn insert_not_capped_by_widget() {
    // The widget itself does not cap capacity (the cap is a business policy controlled by the caller)
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["x"; 20]).build(&mut ui, scr);
    ui.list_insert(l, 0, "y");
    assert_eq!(ui.list_len(l), 21);
}

#[test]
fn remove_selected_deletes_and_shifts_up() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.list_select(l, 1);
    assert!(ui.list_remove(l));
    assert_eq!(ui.list_len(l), 2);
    let s = ui.as_list(l).unwrap();
    // The deleted item disappears immediately (no fade-out ghost)
    assert_eq!(s.items, ["a", "c"]);
    assert!(!s.items.iter().any(|i| i == "b"));
    assert_eq!(s.selected, 1); // still points at the original slot (now "c")
    // Items below shift up to fill the gap (start offset is positive)
    assert!(s.fx.item_fx.iter().any(|f| f.index == 1 && f.dy > 0));
}

#[test]
fn remove_last_item_clamps_selected() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.list_select(l, 2);
    assert!(ui.list_remove(l));
    assert_eq!(ui.list_selected(l), 1);
}

#[test]
fn select_records_highlight_slide_fx() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.list_select(l, 2);
    let fx = list_fx(&ui, l);
    assert_eq!(fx.sel_from, Some((0, ui.time())));
}

#[test]
fn active_fx_keeps_timer_busy_then_expires() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.list_select(l, 1); // triggers the highlight-slide fx
    assert_eq!(ui.timer_handler(), 0); // fx active: keeps it awake
    ui.tick_inc(500); // beyond FX_DUR
    assert_eq!(ui.timer_handler(), u32::MAX); // fx expired: idle
}

#[test]
fn scroll_is_row_aligned() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["0", "1", "2", "3", "4", "5", "6", "7"]).build(&mut ui, scr);
    for i in 1..8 {
        ui.list_select(l, i);
        let s = ui.as_list(l).unwrap();
        assert_eq!(s.scroll % 16, 0, "scroll 必须行对齐");
    }
    // Select 6: the window is already at rows 3..7 (scroll=48), 6 is still visible, scroll unchanged
    ui.list_select(l, 6);
    let s = ui.as_list(l).unwrap();
    assert_eq!(s.scroll, 48);
}

#[test]
fn remove_pulls_window_up_when_tail_emptied() {
    let mut ui: Ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListCfg::new(&["0", "1", "2", "3", "4", "5", "6", "7"]).build(&mut ui, scr);
    ui.list_select(l, 7); // scroll=48 (rows 3..7)
    for _ in 0..5 {
        assert!(ui.list_remove(l)); // deleted down to 3 items
    }
    assert_eq!(ui.list_len(l), 3);
    // The window auto-scrolls up to the top (no empty tail window left)
    let s = ui.as_list(l).unwrap();
    assert_eq!(s.scroll, 0);
    assert!(s.fx.scroll_from.is_some()); // the scroll-up has a smooth animation
}
