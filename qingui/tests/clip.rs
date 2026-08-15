use qingui::display::Flush;
use qingui::widgets::obj::ObjCfg;
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
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

fn build(bg: Color) -> (Rc<RefCell<RecFlush>>, Ui, qingui::ObjRef, qingui::ObjRef) {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui: Ui = Ui::new(64, 64, 16);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(bg);
    let scr = ui.screen();
    ui.set_style(scr, s);
    // Viewport 20x20 @ (5,5)
    let vp = ObjCfg::new().size(20, 20).build(&mut ui, scr);
    ui.set_pos(vp, 5, 5);
    // White 20x20 child placed at (10,0) inside the vp: the right half extends past the viewport
    let child = ObjCfg::new()
        .size(20, 20)
        .style(qingui::style::Style::new().bg(Color::WHITE))
        .build(&mut ui, vp);
    ui.set_pos(child, 10, 0);
    (rec, ui, vp, child)
}

#[test]
fn clip_children_cuts_descendant_at_parent_edge() {
    let (rec, mut ui, vp, _child) = build(Color::BLACK);
    ui.set_clip_children(vp, true);
    ui.render();
    assert_eq!(px(&rec, 15, 10), Color::WHITE); // the part inside the viewport draws normally
    assert_eq!(px(&rec, 26, 10), Color::BLACK); // the part beyond the viewport is clipped
}

#[test]
fn no_clip_children_draws_beyond_parent() {
    let (rec, mut ui, _vp, _child) = build(Color::BLACK);
    ui.render();
    assert_eq!(px(&rec, 15, 10), Color::WHITE);
    assert_eq!(px(&rec, 26, 10), Color::WHITE); // no clipping by default: children may draw outside the parent's bounds
}
