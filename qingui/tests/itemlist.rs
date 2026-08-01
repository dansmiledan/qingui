use qingui::display::Flush;
use qingui::input::Key;
use qingui::node::State;
use qingui::style::Style;
use qingui::widgets::itemlist::ItemListBuilder;
use qingui::widgets::label::LabelBuilder;
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

/// 建 60x40 视口 + 4 个 item（各 20 高：Label 8px + 上下 pad）
fn build4() -> (Ui, ObjRef, Vec<ObjRef>) {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    let mut items = Vec::new();
    for t in ["a", "b", "c", "d"] {
        let it = ui.itemlist_add_item(il).expect("add_item on ItemList");
        LabelBuilder::new(t).build(&mut ui, it);
        ui.set_size(it, 60, 20);
        items.push(it);
    }
    // 布局：ItemList 在 screen 上，默认无 flex → 手动布局
    ui.set_pos(il, 0, 0);
    (ui, il, items)
}

#[test]
fn add_items_and_initial_selection() {
    let (ui, il, items) = build4();
    assert_eq!(ui.itemlist_len(il), 4);
    assert_eq!(ui.itemlist_selected(il), 0);
    assert!(ui.state(items[0]).contains(State::SELECTED)); // 首项自动选中
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
    ui.itemlist_select(il, 2); // 无变化：不重发事件
    assert_eq!(hits.get(), 1);
    assert_eq!(ui.value(il), 2); // value() 接入
}

#[test]
fn keyboard_nav_wraps_and_consumes() {
    let (mut ui, il, _items) = build4();
    ui.group_add(il);
    ui.keypad_input(Key::Up); // 循环：0 → 3
    assert_eq!(ui.itemlist_selected(il), 3);
    ui.keypad_input(Key::Down); // 循环：3 → 0
    assert_eq!(ui.itemlist_selected(il), 0);
}

#[test]
fn ensure_visible_scrolls_content() {
    let (mut ui, il, items) = build4();
    ui.itemlist_select(il, 3); // item3 在视口（40 高）之外 → 滚动
    // 滚动后 item3 底对齐视口底：item3 abs y = 40 - 20 = 20
    assert_eq!(ui.abs_rect(items[3]).y, 20);
    assert_eq!(ui.abs_rect(items[0]).y, -40); // item0 被滚出视口上方
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
    ui.set_pos(il, 10, 30); // 视口避开屏幕边缘，滚出区域仍是屏内可断言区
    for _ in 0..4 {
        let it = ui.itemlist_add_item(il).unwrap();
        // 每项整块白色背景，便于像素断言
        ui.set_style(it, Style::new().bg(Color::WHITE));
        ui.set_size(it, 60, 20);
    }
    ui.itemlist_select(il, 3); // 滚动 40px：item2 → y 30..50，item3 → y 50..70
    ui.render();
    assert_eq!(px(&rec, 15, 35), Color::WHITE); // item2 可见
    assert_eq!(px(&rec, 15, 55), Color::rgb(50, 70, 120)); // item3 选中：叠加默认选中样式
    assert_eq!(px(&rec, 15, 25), Color::BLACK); // item1（abs y 10..30）滚出视口上方：被裁
    assert_eq!(px(&rec, 15, 5), Color::BLACK);  // item0（abs y -10..10）滚出视口上方：被裁
}

#[test]
fn remove_selected_clamps_and_reselects() {
    let (mut ui, il, items) = build4();
    ui.itemlist_select(il, 3);
    assert!(ui.itemlist_remove_selected(il)); // 删 item3
    assert_eq!(ui.itemlist_len(il), 3);
    assert_eq!(ui.itemlist_selected(il), 2); // 收敛到末项
    assert!(ui.state(items[2]).contains(State::SELECTED)); // 新选中项置位
}

#[test]
fn empty_list_key_does_not_panic_and_consumes() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    ui.group_add(il);
    ui.keypad_input(Key::Up); // 空列表：不 panic，按键被消费（不移焦）
    assert_eq!(ui.focused(), Some(il));
    assert!(!ui.itemlist_remove_selected(il));
}

/// 用户绕过 remove_selected 直接 delete item：selected 越界漂移后 select/键盘导航不 panic，
/// itemlist_select 会把越界的 selected clamp 回合法范围并写回
#[test]
fn direct_item_delete_does_not_panic_and_clamps_selection() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let il = ItemListBuilder::new().size(60, 40).build(&mut ui, scr);
    let mut items = Vec::new();
    for t in ["a", "b", "c"] {
        let it = ui.itemlist_add_item(il).expect("add_item on ItemList");
        LabelBuilder::new(t).build(&mut ui, it);
        ui.set_size(it, 60, 20);
        items.push(it);
    }
    ui.set_pos(il, 0, 0);
    ui.group_add(il);
    // 直接删除末项（非选中项）：select 与键盘导航不 panic，selected 保持合法
    ui.delete(items[2]);
    ui.itemlist_select(il, 0);
    ui.keypad_input(Key::Down); // 键盘导航走同一条 itemlist_select 路径
    assert!(ui.itemlist_selected(il) < ui.itemlist_len(il));
    assert_eq!(ui.itemlist_selected(il), 1);
    // 直接删除当前选中项：selected=1 越界（len=1），select 时 clamp 写回，不 panic
    ui.delete(items[1]);
    ui.itemlist_select(il, 0);
    assert_eq!(ui.itemlist_selected(il), 0); // 漂移被 clamp 消除
    assert!(ui.itemlist_selected(il) < ui.itemlist_len(il));
}
