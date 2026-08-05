use qingui::display::Flush;
use qingui::prelude::*;
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
    // FONT_6X10: glyph width 6 (spacing 0), line height 10
    let (w, h) = qingui::font::text_size(&embedded_graphics::mono_font::ascii::FONT_6X10, "AB\nABC");
    assert_eq!(w, 3 * 6);
    assert_eq!(h, 2 * 10);
}

#[test]
fn non_ascii_falls_back_to_question_mark() {
    // e-g GlyphMapping: characters not present fall back to the '?' glyph (consistent with the old font8x8 semantics, switched to pixel-level assertions)
    use embedded_graphics::mono_font::ascii::FONT_6X10;
    let render = |s: &str| -> [Color; 60] {
        let mut buf = [Color::BLACK; 60];
        let mut d = qingui::draw::DrawBuf { pixels: &mut buf, area: Rect::new(0, 0, 6, 10), stride: 6 };
        d.draw_text(qingui::Point { x: 0, y: 0 }, &FONT_6X10, s, Color::WHITE, Rect::new(0, 0, 6, 10));
        buf
    };
    assert_eq!(render("中"), render("?"));
}

#[test]
fn label_renders_glyph_pixels() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48); // single-row buffer: 1 chunk
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
    // FONT_6X10 'A' glyph: row 1 is 001000 → (2,1) lit; row 2 is 010100 → (1,2)/(3,2) lit
    assert_eq!(px[1 * 64 + 2], Color::WHITE);
    assert_eq!(px[2 * 64 + 1], Color::WHITE);
    assert_eq!(px[2 * 64 + 3], Color::WHITE);
    assert_eq!(px[0], Color::BLACK); // (0,0) inside the glyph box's top-left has no pixel (transparent background)
    assert_eq!(ui.text(l), "A");
    assert_eq!(ui.rect(l).w, 6);
    assert_eq!(ui.rect(l).h, 10);
}

#[test]
fn set_text_invalidates_and_resizes() {
    let mut ui = Ui::new(64, 48, 48);
    let scr = ui.screen();
    let l = LabelBuilder::new("A").build(&mut ui, scr);
    ui.set_pos(l, 10, 10);
    ui.take_dirty();
    ui.set_text(l, "ABCD");
    assert_eq!(ui.rect(l).w, 24);
    let dirty = ui.take_dirty();
    // Old area (10,10,6,10) and new area (10,10,24,10) overlap and merge
    assert_eq!(dirty.len(), 1);
    assert!(dirty[0].contains(qingui::Point { x: 33, y: 10 }));
}
