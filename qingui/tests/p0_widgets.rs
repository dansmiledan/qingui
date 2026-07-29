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
fn arc_value_and_indicator() {
    let (mut ui, rec) = setup();
    let a = ui.create_arc(ui.screen(), 0, 100);
    ui.set_pos(a, 10, 10);
    ui.set_value(a, 50);
    ui.render();
    assert_eq!(ui.value(a), 50);
    // 中心 (40,40)，r=27。START=135°(左下)。50% → 指示到 135+135=270°(正上)
    // 轨道色采样 300° 方向（指示范围之外、非边界）：(40+13, 40-23) = (53, 17)
    assert_eq!(px(&rec, 53, 17), Color::rgb(70, 70, 80));
    // 50% 指示弧覆盖左下到正上：180°(正左) 方向环带中部 (40-25, 40) = (15, 40)
    assert_eq!(px(&rec, 15, 40), Color::rgb(80, 140, 255));
    // 指示弧覆盖 200° 方向 (40-23, 40+8) = (17, 48)
    assert_eq!(px(&rec, 17, 48), Color::rgb(80, 140, 255));
    // 90°(正下，扫掠缺口内) 无弧：(40, 40+25) = (40, 65) 是背景
    assert_eq!(px(&rec, 40, 65), Color::BLACK);
}

#[test]
fn arc_edited_turns_indicator_yellow() {
    let (mut ui, rec) = setup();
    let a = ui.create_arc(ui.screen(), 0, 100);
    ui.set_pos(a, 10, 10);
    ui.set_value(a, 50);
    ui.set_state(a, qingui::node::State::EDITED, true);
    ui.render();
    // 编辑态：指示弧变黄（180° 方向 (15,40)）
    assert_eq!(px(&rec, 15, 40), Color::rgb(255, 200, 60));
}

#[test]
fn checkbox_toggles_on_enter() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let (mut ui, rec) = setup();
    let cb = ui.create_checkbox(ui.screen(), "OK");
    ui.set_pos(cb, 10, 10);
    ui.add_event_cb(cb, EventKind::ValueChanged, Box::new(move |_ui, _t, k| l2.borrow_mut().push(k)));
    ui.group_add(cb);
    ui.render();
    // 未选中：方框顶边灰（避开控件聚焦边框），内部无勾
    assert_eq!(px(&rec, 16, 12), Color::rgb(150, 150, 160)); // 框顶边
    assert_ne!(px(&rec, 15, 16), Color::rgb(80, 140, 255)); // 无勾
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(cb), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    ui.render();
    // 勾选后勾线经过 (17,19)
    assert_eq!(px(&rec, 17, 19), Color::rgb(80, 140, 255));
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(cb), 0);
}

#[test]
fn spinner_keeps_timer_busy_and_draws_arc() {
    let (mut ui, rec) = setup();
    let s = ui.create_spinner(ui.screen());
    ui.set_pos(s, 10, 10);
    ui.render();
    assert_eq!(ui.timer_handler(), 0); // 自转：永远唤醒
    // 某处有弧像素
    let mut found = false;
    for y in 10..42 {
        for x in 10..42 {
            if px(&rec, x, y) == Color::rgb(80, 140, 255) {
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn msgbox_click_records_index_and_closes() {
    let (mut ui, _) = setup();
    let sel_log = Rc::new(RefCell::new(Vec::new()));
    let sl = sel_log.clone();
    let mb = ui.create_msgbox(ui.screen(), "Title", "Body text", &["Yes", "No"]);
    ui.add_event_cb(mb, EventKind::ValueChanged, Box::new(move |ui, t, _| {
        sl.borrow_mut().push(ui.msgbox_selected(t));
    }));
    assert!(ui.is_valid(mb));
    // 焦点锁定在 msgbox 内：Tab 应在两个按钮间循环
    let f0 = ui.focused();
    ui.keypad_input(Key::Next);
    let f1 = ui.focused();
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), f0);
    assert_ne!(f0, f1);
    // 点击当前按钮（第 1 个）→ selected=0，msgbox 被删除
    ui.keypad_input(Key::Enter);
    assert_eq!(*sel_log.borrow(), vec![0]);
    assert!(!ui.is_valid(mb));
}

#[test]
fn msgbox_esc_closes_with_minus_one() {
    let (mut ui, _) = setup();
    let sel_log = Rc::new(RefCell::new(Vec::new()));
    let sl = sel_log.clone();
    let mb = ui.create_msgbox(ui.screen(), "T", "B", &["OK"]);
    ui.add_event_cb(mb, EventKind::ValueChanged, Box::new(move |ui, t, _| {
        sl.borrow_mut().push(ui.msgbox_selected(t));
    }));
    ui.keypad_input(Key::Esc);
    assert_eq!(*sel_log.borrow(), vec![-1]);
    assert!(!ui.is_valid(mb));
}
