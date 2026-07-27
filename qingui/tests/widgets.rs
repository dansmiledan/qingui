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
    for (area, buf) in chunks {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

#[test]
fn slider_value_and_indicator() {
    let (mut ui, rec) = setup();
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.set_pos(s, 10, 10);
    ui.set_value(s, 50);
    ui.render();
    assert_eq!(ui.value(s), 50);
    // 轨道 y 中心 = 10+6，指示条到 50% ≈ x=10+50
    assert_eq!(px(&rec, 20, 16), Color::rgb(80, 140, 255));
    // 指示条末端之后是轨道色（非指示色）
    assert_ne!(px(&rec, 100, 16), Color::rgb(80, 140, 255));
    // 旋钮在 ~x=10+50-4.. 处是白色
    assert_eq!(px(&rec, 58, 16), Color::WHITE);
}

#[test]
fn slider_value_clamped_to_range() {
    let (mut ui, _) = setup();
    let s = ui.create_slider(ui.screen(), 10, 20);
    ui.set_value(s, 999);
    assert_eq!(ui.value(s), 20);
    ui.set_value(s, -5);
    assert_eq!(ui.value(s), 10);
}

#[test]
fn switch_toggle_visual() {
    let (mut ui, rec) = setup();
    let sw = ui.create_switch(ui.screen());
    ui.set_pos(sw, 10, 10);
    ui.render();
    // off：轨道灰，旋钮在左
    assert_eq!(px(&rec, 12, 20), Color::WHITE); // 旋钮左
    assert_eq!(px(&rec, 44, 20), Color::rgb(90, 90, 90)); // 右端轨道
}

#[test]
fn bar_renders_progress() {
    let (mut ui, rec) = setup();
    let b = ui.create_bar(ui.screen(), 0, 100);
    ui.set_pos(b, 10, 10);
    ui.set_value(b, 25);
    ui.render();
    assert_eq!(px(&rec, 20, 14), Color::rgb(80, 140, 255));
    assert_ne!(px(&rec, 100, 14), Color::rgb(80, 140, 255));
}

#[test]
fn list_selected_row_highlighted() {
    let (mut ui, rec) = setup();
    let l = ui.create_list(ui.screen(), &["alpha", "beta", "gamma"]);
    ui.set_pos(l, 10, 10);
    ui.list_select(l, 1);
    assert_eq!(ui.list_selected(l), 1);
    ui.render();
    // 第 2 行（beta）底色 = 高亮色。行高 16，行 1 中心 y = 10+16+8=34，文本左侧 x=12
    assert_eq!(px(&rec, 12, 34), Color::rgb(50, 70, 120));
}

#[test]
fn button_renders_text_centered() {
    let (mut ui, rec) = setup();
    let b = ui.create_button(ui.screen(), "OK");
    ui.set_pos(b, 10, 10);
    ui.render();
    let r = ui.rect(b);
    // 文字 "OK" 宽 16px，居中：起始 x = 10 + (w-16)/2；'O' 第一行有像素点亮
    assert!(r.w > 16);
    let text_x = 10 + (r.w - 16) / 2;
    let g = qingui::font::glyph('O');
    assert!(g.iter().any(|&row| row != 0));
    // 文字颜色（白）应出现在文本区域内某处
    let mut found_white = false;
    for y in 10..10 + r.h {
        for x in text_x..text_x + 16 {
            if px(&rec, x, y) == Color::WHITE {
                found_white = true;
            }
        }
    }
    assert!(found_white);
}
