use rust_lvgl::display::Flush;
use rust_lvgl::style::theme_screen;
use rust_lvgl::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}

/// Rc 不是 fundamental type，orphan rule 要求包一层本地 newtype
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

#[test]
fn chunked_render_covers_dirty_area() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16); // 缓冲 16 行 → 全屏 48 行 = 3 chunks
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.set_style(ui.screen(), theme_screen());
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 8, 8);
    ui.set_size(o, 16, 16);
    let mut s = rust_lvgl::style::Style::default();
    s.bg_color = Some(Color::RED);
    s.bg_opa = Some(255);
    ui.set_style(o, s);

    ui.render();

    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].0, Rect::new(0, 0, 64, 16));
    assert_eq!(chunks[1].0, Rect::new(0, 16, 64, 16));
    assert_eq!(chunks[2].0, Rect::new(0, 32, 64, 16));
    // 对象在 chunk0 中：屏幕 (8,8) → 缓冲 (8,8)
    assert_eq!(chunks[0].1[8 * 64 + 8], Color::RED);
    // 对象之外是 screen 背景色
    assert_eq!(chunks[0].1[0], theme_screen().bg_color.unwrap());
}

#[test]
fn partial_last_chunk_height() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 50, 16); // 48 + 2 行
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.render();
    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[3].0, Rect::new(0, 48, 64, 2));
    assert_eq!(chunks[3].1.len(), (64 * 2) as usize);
}

#[test]
fn no_dirty_no_flush() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.render();
    assert_eq!(rec.borrow().chunks.len(), 3);
    ui.render(); // 无脏矩形
    assert_eq!(rec.borrow().chunks.len(), 3);
}

#[test]
fn small_dirty_flushes_only_that_area() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.set_style(ui.screen(), theme_screen());
    ui.render();
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 40, 40);
    ui.set_size(o, 8, 8);
    let mut s = rust_lvgl::style::Style::default();
    s.bg_color = Some(Color::GREEN);
    ui.set_style(o, s);
    ui.render();
    let chunks = &rec.borrow().chunks;
    // 累计 3（首帧全屏）+ 1：最后一个 chunk 恰好覆盖对象脏区
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[3].0, Rect::new(40, 40, 8, 8));
}
