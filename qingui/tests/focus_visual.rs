use qingui::display::Flush;
use qingui::{Color, Rect, Ui};
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

fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Color {
    let chunks = &rec.borrow().chunks;
    // 反向查找：后渲染的 chunk 覆盖先渲染的
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
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

#[test]
fn slider_shows_focus_border() {
    let (mut ui, rec) = setup();
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.set_pos(s, 10, 10);
    ui.group_add(s); // 成为焦点
    ui.render();
    // 聚焦态：白色边框，轨道顶边中点
    assert_eq!(px(&rec, 60, 10), Color::WHITE);
}

#[test]
fn switch_shows_focus_border() {
    let (mut ui, rec) = setup();
    let sw = ui.create_switch(ui.screen());
    ui.set_pos(sw, 10, 10);
    ui.group_add(sw);
    ui.render();
    // 聚焦态：白色边框，轨道顶边中点
    assert_eq!(px(&rec, 30, 10), Color::WHITE);
}

#[test]
fn slider_knob_overflow_area_redrawn_on_move() {
    let (mut ui, rec) = setup();
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.set_pos(s, 10, 10);
    ui.render();
    // 初始 knob 在 x 6..14, y 8..24（轨道上方溢出 2px）
    assert_eq!(px(&rec, 10, 8), Color::WHITE);
    ui.set_value(s, 50);
    ui.render();
    // 旧 knob 溢出区域被重绘为背景（不留残影）
    assert_eq!(px(&rec, 10, 8), Color::BLACK);
    // 新 knob 位置（kx = 10+50 = 60，knob x 56..64）
    assert_eq!(px(&rec, 60, 8), Color::WHITE);
}
