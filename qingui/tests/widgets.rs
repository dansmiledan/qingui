use qingui::display::Flush;
use qingui::widgets::bar::BarBuilder;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::slider::SliderBuilder;
use qingui::widgets::switch::SwitchBuilder;
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
    let scr = ui.screen();
    scr.set_style(&mut ui, bg);
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
    let scr = ui.screen();
    let s = SliderBuilder::new(0, 100).build(&mut ui, scr);
    s.set_pos(&mut ui, 10, 10);
    s.set_value(&mut ui, 50);
    ui.render();
    assert_eq!(s.value(&ui), 50);
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
    let scr = ui.screen();
    let s = SliderBuilder::new(10, 20).build(&mut ui, scr);
    s.set_value(&mut ui, 999);
    assert_eq!(s.value(&ui), 20);
    s.set_value(&mut ui, -5);
    assert_eq!(s.value(&ui), 10);
}

#[test]
fn switch_toggle_visual() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let sw = SwitchBuilder::new().build(&mut ui, scr);
    sw.set_pos(&mut ui, 10, 10);
    ui.render();
    // off：轨道灰，旋钮在左（采样圆内部点，避开抗锯齿边缘）
    assert_eq!(px(&rec, 16, 20), Color::WHITE); // 旋钮左
    assert_eq!(px(&rec, 44, 20), Color::rgb(90, 90, 90)); // 右端轨道
}

#[test]
fn bar_renders_progress() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let b = BarBuilder::new(0, 100).build(&mut ui, scr);
    b.set_pos(&mut ui, 10, 10);
    b.set_value(&mut ui, 25);
    ui.render();
    assert_eq!(px(&rec, 20, 14), Color::rgb(80, 140, 255));
    assert_ne!(px(&rec, 100, 14), Color::rgb(80, 140, 255));
}

#[test]
fn bar_small_value_keeps_left_semicircle() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let b = BarBuilder::new(0, 100).build(&mut ui, scr);
    b.set_pos(&mut ui, 10, 10); // 默认尺寸 100x8，radius=4
    b.set_value(&mut ui, 5); // 指示宽 iw=5
    ui.render();
    let ind = Color::rgb(80, 140, 255);
    // 左端按轨道形状(radius=4)裁剪：(11,11) 在半圆外 → 非指示色
    assert_ne!(px(&rec, 11, 11), ind);
    // (11,14) 在半圆内 → 指示色
    assert_eq!(px(&rec, 11, 14), ind);
    // 指示右边界之外 → 非指示色
    assert_ne!(px(&rec, 20, 14), ind);
}

#[test]
fn list_selected_row_highlighted() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let l = ListBuilder::new(&["alpha", "beta", "gamma"]).build(&mut ui, scr);
    l.set_pos(&mut ui, 10, 10);
    l.list_select(&mut ui, 1);
    assert_eq!(l.list_selected(&ui), 1);
    ui.tick_inc(300); // 让高亮滑动动画播完
    ui.timer_handler();
    // 第 2 行（beta）底色 = 高亮色。行高 16，行 1 中心 y = 10+16+8=34，文本左侧 x=12
    assert_eq!(px(&rec, 12, 34), Color::rgb(50, 70, 120));
}

#[test]
fn button_renders_text_centered() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let b = ButtonBuilder::new("OK").build(&mut ui, scr);
    b.set_pos(&mut ui, 10, 10);
    ui.render();
    let r = b.rect(&ui);
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
