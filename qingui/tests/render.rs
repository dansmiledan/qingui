use qingui::display::Flush;
use qingui::style::theme_screen;
use qingui::widgets::obj::ObjCfg;
use qingui::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}

/// Rc is not a fundamental type, so the orphan rule requires wrapping it in a local wrapper struct
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

#[test]
fn move_to_front_raises_stacking() {
    // Two overlapping siblings: the later-created B covers A; after move_to_front(A), A covers B
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(20, 20, 20);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    let a = ObjCfg::new().size(10, 10).build(&mut ui, scr);
    let b = ObjCfg::new().size(10, 10).build(&mut ui, scr);
    ui.set_style(a, { let mut s = qingui::style::Style::default(); s.bg_color = Some(Color::rgb(255, 0, 0)); s.bg_opa = Some(255); s });
    ui.set_style(b, { let mut s = qingui::style::Style::default(); s.bg_color = Some(Color::rgb(0, 0, 255)); s.bg_opa = Some(255); s });
    ui.render();
    // Initially B is on top → (5,5) is blue
    assert_eq!(px(&rec, 5, 5), Color::rgb(0, 0, 255));
    ui.move_to_front(a);
    ui.render();
    // Now A is on top → (5,5) is red
    assert_eq!(px(&rec, 5, 5), Color::rgb(255, 0, 0));
}

#[test]
fn chunked_render_covers_dirty_area() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16); // 16-row buffer → 48-row full screen = 3 chunks
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    ui.set_style(scr, theme_screen());
    let o = ObjCfg::new().build(&mut ui, scr);
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
    let o = ObjCfg::new().build(&mut ui, scr);
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
