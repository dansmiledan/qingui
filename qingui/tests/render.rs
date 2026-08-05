use qingui::display::Flush;
use qingui::style::theme_screen;
use qingui::widgets::obj::ObjBuilder;
use qingui::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}

/// Rc is not a fundamental type, so the orphan rule requires wrapping it in a local newtype
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

#[test]
fn chunked_render_covers_dirty_area() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16); // 16-row buffer → 48-row full screen = 3 chunks
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    ui.set_style(scr, theme_screen());
    let o = ObjBuilder::new().build(&mut ui, scr);
    ui.set_pos(o, 8, 8);
    ui.set_size(o, 16, 16);
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(Color::RED);
    s.bg_opa = Some(255);
    ui.set_style(o, s);

    ui.render();

    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].0, Rect::new(0, 0, 64, 16));
    assert_eq!(chunks[1].0, Rect::new(0, 16, 64, 16));
    assert_eq!(chunks[2].0, Rect::new(0, 32, 64, 16));
    // The object is in chunk0: screen (8,8) → buffer (8,8)
    assert_eq!(chunks[0].1[8 * 64 + 8], Color::RED);
    // Outside the object is the screen background color
    assert_eq!(chunks[0].1[0], theme_screen().bg_color.unwrap());
}

#[test]
fn partial_last_chunk_height() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 50, 16); // 48 + 2 rows
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
    ui.render(); // no dirty rects
    assert_eq!(rec.borrow().chunks.len(), 3);
}

#[test]
fn small_dirty_flushes_only_that_area() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    ui.set_style(scr, theme_screen());
    ui.render();
    let o = ObjBuilder::new().build(&mut ui, scr);
    ui.set_pos(o, 40, 40);
    ui.set_size(o, 8, 8);
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(Color::GREEN);
    ui.set_style(o, s);
    ui.render();
    let chunks = &rec.borrow().chunks;
    // 3 cumulative (full screen on the first frame) + 1: the last chunk exactly covers the object's dirty area
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[3].0, Rect::new(40, 40, 8, 8));
}
