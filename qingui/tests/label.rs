use qingui::display::Flush;
use qingui::widgets::label::LabelBuilder;
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

#[test]
fn text_size_multiline() {
    let (w, h) = qingui::font::text_size("AB\nABC");
    assert_eq!(w, 3 * 8);
    assert_eq!(h, 2 * 8);
}

#[test]
fn non_ascii_falls_back_to_question_mark() {
    assert_eq!(qingui::font::glyph('中'), qingui::font::glyph('?'));
}

#[test]
fn label_renders_glyph_pixels() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48); // 单行缓冲：1 个 chunk
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    ui.set_style(scr, bg);
    let l = LabelBuilder::new("A").build(&mut ui, scr);
    ui.set_pos(l, 0, 0);
    ui.render();
    let chunks = &rec.borrow().chunks;
    let px = &chunks[chunks.len() - 1].1;
    // 'A' 的 8x8 字模：第一行 0x0C → 第 2、3 个像素点亮（bit 从低位起）
    let glyph = qingui::font::glyph('A');
    assert_eq!(glyph[0], 0x0C);
    assert_eq!(px[2], Color::WHITE); // (x=2, y=0)
    assert_eq!(px[3], Color::WHITE);
    assert_eq!(px[0], Color::BLACK);
    assert_eq!(ui.text(l), "A");
    assert_eq!(ui.rect(l).w, 8);
    assert_eq!(ui.rect(l).h, 8);
}

#[test]
fn set_text_invalidates_and_resizes() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let l = LabelBuilder::new("A").build(&mut ui, scr);
    ui.set_pos(l, 10, 10);
    ui.take_dirty();
    ui.set_text(l, "ABCD");
    assert_eq!(ui.rect(l).w, 32);
    let dirty = ui.take_dirty();
    // 旧区域 (10,10,8,8) 与新区域 (10,10,32,8) 共边合并
    assert_eq!(dirty.len(), 1);
    assert!(dirty[0].contains(qingui::Point { x: 41, y: 10 }));
}
