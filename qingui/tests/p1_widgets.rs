use qingui::display::Flush;
use qingui::input::Key;
use qingui::{Color, EventKind, Rect, Ui};
use std::cell::RefCell;
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

fn setup() -> (Ui, Rc<RefCell<RecFlush>>) {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    ui.set_style(ui.screen(), bg);
    (ui, rec)
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

#[test]
fn led_brightness() {
    let (mut ui, rec) = setup();
    let led = ui.create_led(ui.screen(), Color::RED);
    ui.set_pos(led, 10, 10);
    ui.render();
    assert_eq!(px(&rec, 18, 18), Color::RED); // 全亮中心
    ui.set_value(led, 128);
    ui.render();
    let dim = px(&rec, 18, 18);
    assert!(dim.r < 255 && dim.r > 100, "半亮: {:?}", dim);
    assert_eq!(dim.g, 0);
}

#[test]
fn table_cells() {
    let (mut ui, rec) = setup();
    let t = ui.create_table(ui.screen(), 2, 2);
    ui.set_pos(t, 10, 10);
    ui.table_set_cell(t, 0, 0, "A1");
    ui.table_set_cell(t, 1, 1, "B2");
    ui.render();
    // 'A' 第一行 0x0C → 文本区有白色像素
    let glyph = qingui::font::glyph('A');
    assert_eq!(glyph[0], 0x0C);
    assert_eq!(px(&rec, 14 + 2, 14), Color::WHITE); // 'A' row0 bit2
    // 网格线
    assert_eq!(px(&rec, 10, 20), Color::rgb(70, 70, 90));
    // 底边网格线（半开区间修正后应存在）
    assert_eq!(px(&rec, 30, 41), Color::rgb(70, 70, 90));
    // 空单元格无文本
    assert_eq!(px(&rec, 74, 14), Color::BLACK);
}

#[test]
fn spinbox_digit_edit() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, _) = setup();
    let sb = ui.create_spinbox(ui.screen(), 0, 999, 3);
    let other = ui.create_button(ui.screen(), "X");
    ui.add_event_cb(sb, EventKind::ValueChanged, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(sb);
    ui.group_add(other);
    // 未进编辑态：方向键是焦点导航，值不变
    ui.keypad_input(Key::Up);
    assert_eq!(ui.value(sb), 0);
    assert_eq!(ui.focused(), Some(other));
    ui.keypad_input(Key::Prev);
    // Enter 进编辑态：Up → +1；Left 到十位 Up → +10；Left 到百位 Down → clamp min
    ui.keypad_input(Key::Enter);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.value(sb), 1);
    ui.keypad_input(Key::Left);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.value(sb), 11);
    ui.keypad_input(Key::Left);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.value(sb), 0);
    assert_eq!(log.borrow().len(), 3);
    // Esc 退出编辑态，方向键可以移出焦点
    ui.keypad_input(Key::Esc);
    ui.keypad_input(Key::Right);
    assert_eq!(ui.focused(), Some(other));
}

#[test]
fn spinbox_cursor_highlight() {
    let (mut ui, rec) = setup();
    let sb = ui.create_spinbox(ui.screen(), 0, 999, 3);
    ui.set_pos(sb, 10, 10);
    ui.set_value(sb, 5);
    ui.set_state(sb, qingui::node::State::EDITED, true); // 编辑态才显示光标高亮
    ui.render();
    // 个位（右端第 3 位）高亮：取高亮块内字形之外的像素 (40,15)
    assert_eq!(px(&rec, 40, 15), Color::rgb(80, 140, 255));
    // 百位无高亮：'0' 字形像素为文本白色（非高亮底色）
    assert_eq!(px(&rec, 19, 15), Color::WHITE);
}

#[test]
fn roller_rapid_select_continues_from_visual_pos() {
    let (mut ui, _) = setup();
    let r = ui.create_roller(ui.screen(), &["One", "Two", "Three", "Four"]);
    ui.group_add(r);
    ui.keypad_input(Key::Down); // 0 → 1（动画开始）
    ui.tick_inc(50); // 动画中途（约 1/3）
    ui.keypad_input(Key::Down); // 1 → 2（连按）
    // 新动画应从插值位置（0 < from < 1）续接，而非从 1 跳变
    match ui.debug_kind(r) {
        qingui::node::WidgetKind::Roller(s) => {
            let (from, _) = s.sel_from.expect("有滚动动画");
            assert!(from > 0.0 && from < 1.0, "from={}", from);
        }
        _ => panic!(),
    }
}

#[test]
fn roller_navigation_and_fx() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, _) = setup();
    let r = ui.create_roller(ui.screen(), &["One", "Two", "Three"]);
    ui.add_event_cb(r, EventKind::Clicked, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(r);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.roller_selected(r), 1);
    assert_eq!(ui.timer_handler(), 0); // 滚动动画活动
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down); // 到末尾停止
    assert_eq!(ui.roller_selected(r), 2);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.roller_selected(r), 1);
    ui.tick_inc(300);
    ui.timer_handler();
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}

#[test]
fn dropdown_open_select_close() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, _) = setup();
    let dd = ui.create_dropdown(ui.screen(), &["Red", "Green", "Blue"]);
    ui.add_event_cb(dd, EventKind::ValueChanged, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(dd);
    // Enter 打开浮层列表（模态）
    ui.keypad_input(Key::Enter);
    let overlay = ui.focused().expect("有焦点");
    assert_ne!(overlay, dd);
    // Down 到 Green，Enter 选中
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(dd), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    assert_eq!(ui.focused(), Some(dd)); // 焦点还原
    // 再次打开，Esc 关闭不改值
    ui.keypad_input(Key::Enter);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Esc);
    assert_eq!(ui.value(dd), 1);
    assert_eq!(ui.focused(), Some(dd));
}
