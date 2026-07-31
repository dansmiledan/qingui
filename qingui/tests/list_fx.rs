use qingui::widgets::list::ListBuilder;
use qingui::Ui;

fn list_fx(ui: &Ui, l: qingui::ObjRef) -> qingui::widgets::list::ListFx {
    l.as_list(ui).unwrap().fx.clone()
}

#[test]
fn insert_adds_item_with_fade_and_shift_fx() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    l.list_insert(&mut ui, 1, "x");
    assert_eq!(l.list_len(&ui), 4);
    let s = l.as_list(&ui).unwrap();
    assert_eq!(s.items, ["a", "x", "b", "c"]);
    // 新项淡入
    assert!(s.fx.item_fx.iter().any(|f| f.index == 1 && f.fade_in));
    // 下方 item 下滑让位（起始位移为负）
    assert!(s.fx.item_fx.iter().any(|f| f.index == 2 && f.dy < 0));
    assert!(s.fx.item_fx.iter().any(|f| f.index == 3 && f.dy < 0));
}

#[test]
fn insert_not_capped_by_widget() {
    // 控件本身不限制容量（上限是业务策略，由调用方控制）
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["x"; 20]).build(&mut ui, scr);
    l.list_insert(&mut ui, 0, "y");
    assert_eq!(l.list_len(&ui), 21);
}

#[test]
fn remove_selected_fades_ghost_and_shifts_up() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    l.list_select(&mut ui, 1);
    assert!(l.list_remove(&mut ui));
    assert_eq!(l.list_len(&ui), 2);
    let s = l.as_list(&ui).unwrap();
    assert_eq!(s.items, ["a", "c"]);
    assert_eq!(s.selected, 1); // 仍指向原位置（现在是 "c"）
    // ghost 渐隐
    assert!(s.fx.ghost.as_ref().is_some_and(|g| g.text == "b" && g.index == 1));
    // 下方 item 上移补位（起始位移为正）
    assert!(s.fx.item_fx.iter().any(|f| f.index == 1 && f.dy > 0));
}

#[test]
fn remove_last_item_clamps_selected() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    l.list_select(&mut ui, 2);
    assert!(l.list_remove(&mut ui));
    assert_eq!(l.list_selected(&ui), 1);
}

#[test]
fn select_records_highlight_slide_fx() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    l.list_select(&mut ui, 2);
    let fx = list_fx(&ui, l);
    assert_eq!(fx.sel_from, Some((0, ui.time())));
}

#[test]
fn active_fx_keeps_timer_busy_then_expires() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["a", "b", "c"]).build(&mut ui, scr);
    l.list_select(&mut ui, 1); // 触发高亮滑动 fx
    assert_eq!(ui.timer_handler(), 0); // fx 活动：持续唤醒
    ui.tick_inc(500); // 超过 FX_DUR
    assert_eq!(ui.timer_handler(), u32::MAX); // fx 过期：空闲
}

#[test]
fn scroll_is_row_aligned() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["0", "1", "2", "3", "4", "5", "6", "7"]).build(&mut ui, scr);
    for i in 1..8 {
        l.list_select(&mut ui, i);
        let s = l.as_list(&ui).unwrap();
        assert_eq!(s.scroll % 16, 0, "scroll 必须行对齐");
    }
    // 选中 6：窗口已是行 3..7（scroll=48），6 仍可见，scroll 不变
    l.list_select(&mut ui, 6);
    let s = l.as_list(&ui).unwrap();
    assert_eq!(s.scroll, 48);
}

#[test]
fn remove_pulls_window_up_when_tail_emptied() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let l = ListBuilder::new(&["0", "1", "2", "3", "4", "5", "6", "7"]).build(&mut ui, scr);
    l.list_select(&mut ui, 7); // scroll=48（行 3..7）
    for _ in 0..5 {
        assert!(l.list_remove(&mut ui)); // 删到剩 3 项
    }
    assert_eq!(l.list_len(&ui), 3);
    // 窗口自动上滚到顶（不留下尾部空窗）
    let s = l.as_list(&ui).unwrap();
    assert_eq!(s.scroll, 0);
    assert!(s.fx.scroll_from.is_some()); // 上滚有平滑动画
}
