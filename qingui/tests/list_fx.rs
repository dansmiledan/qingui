use qingui::node::WidgetKind;
use qingui::Ui;

fn list_fx(ui: &Ui, l: qingui::ObjRef) -> qingui::widgets::list::ListFx {
    match ui.debug_kind(l) {
        WidgetKind::List { fx, .. } => fx.clone(),
        _ => panic!("not a list"),
    }
}

#[test]
fn insert_adds_item_with_fade_and_shift_fx() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["a", "b", "c"]);
    assert!(ui.list_insert(l, 1, "x"));
    assert_eq!(ui.list_len(l), 4);
    match ui.debug_kind(l) {
        WidgetKind::List { items, fx, .. } => {
            assert_eq!(items, &["a", "x", "b", "c"]);
            // 新项淡入
            assert!(fx.item_fx.iter().any(|f| f.index == 1 && f.fade_in));
            // 下方 item 下滑让位（起始位移为负）
            assert!(fx.item_fx.iter().any(|f| f.index == 2 && f.dy < 0));
            assert!(fx.item_fx.iter().any(|f| f.index == 3 && f.dy < 0));
        }
        _ => panic!(),
    }
}

#[test]
fn insert_caps_at_max_items() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["x"; 20]);
    assert!(!ui.list_insert(l, 0, "y"));
    assert_eq!(ui.list_len(l), 20);
}

#[test]
fn remove_selected_fades_ghost_and_shifts_up() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["a", "b", "c"]);
    ui.list_select(l, 1);
    assert!(ui.list_remove(l));
    assert_eq!(ui.list_len(l), 2);
    match ui.debug_kind(l) {
        WidgetKind::List { items, selected, fx, .. } => {
            assert_eq!(items, &["a", "c"]);
            assert_eq!(*selected, 1); // 仍指向原位置（现在是 "c"）
            // ghost 渐隐
            assert!(fx.ghost.as_ref().is_some_and(|g| g.text == "b" && g.index == 1));
            // 下方 item 上移补位（起始位移为正）
            assert!(fx.item_fx.iter().any(|f| f.index == 1 && f.dy > 0));
        }
        _ => panic!(),
    }
}

#[test]
fn remove_last_item_clamps_selected() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["a", "b", "c"]);
    ui.list_select(l, 2);
    assert!(ui.list_remove(l));
    assert_eq!(ui.list_selected(l), 1);
}

#[test]
fn select_records_highlight_slide_fx() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["a", "b", "c"]);
    ui.list_select(l, 2);
    let fx = list_fx(&ui, l);
    assert_eq!(fx.sel_from, Some((0, ui.time())));
}

#[test]
fn active_fx_keeps_timer_busy_then_expires() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["a", "b", "c"]);
    ui.list_select(l, 1); // 触发高亮滑动 fx
    assert_eq!(ui.timer_handler(), 0); // fx 活动：持续唤醒
    ui.tick_inc(500); // 超过 FX_DUR
    assert_eq!(ui.timer_handler(), u32::MAX); // fx 过期：空闲
}

#[test]
fn scroll_is_row_aligned() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["0", "1", "2", "3", "4", "5", "6", "7"]);
    for i in 1..8 {
        ui.list_select(l, i);
        match ui.debug_kind(l) {
            WidgetKind::List { scroll, .. } => assert_eq!(scroll % 16, 0, "scroll 必须行对齐"),
            _ => panic!(),
        }
    }
    // 选中 6：窗口已是行 3..7（scroll=48），6 仍可见，scroll 不变
    ui.list_select(l, 6);
    match ui.debug_kind(l) {
        WidgetKind::List { scroll, .. } => assert_eq!(*scroll, 48),
        _ => panic!(),
    }
}

#[test]
fn remove_pulls_window_up_when_tail_emptied() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["0", "1", "2", "3", "4", "5", "6", "7"]);
    ui.list_select(l, 7); // scroll=48（行 3..7）
    for _ in 0..5 {
        assert!(ui.list_remove(l)); // 删到剩 3 项
    }
    assert_eq!(ui.list_len(l), 3);
    // 窗口自动上滚到顶（不留下尾部空窗）
    match ui.debug_kind(l) {
        WidgetKind::List { scroll, fx, .. } => {
            assert_eq!(*scroll, 0);
            assert!(fx.scroll_from.is_some()); // 上滚有平滑动画
        }
        _ => panic!(),
    }
}
