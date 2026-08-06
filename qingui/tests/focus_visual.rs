use qingui::display::Flush;
use qingui::prelude::*;
use qingui::widgets::list::ListCfg;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::widgets::switch::SwitchCfg;
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
    // Search backwards: later-rendered chunks cover earlier ones
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
    let scr = ui.screen();
    ui.set_style(scr, bg);
    (ui, rec)
}

#[test]
fn slider_shows_focus_border() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let s = SliderCfg::new(0, 100).build(&mut ui, scr);
    ui.set_pos(s, 10, 10);
    ui.group_add(s); // becomes focused
    ui.render();
    // Focused state: white border, midpoint of the track's top edge
    assert_eq!(px(&rec, 60, 10), Color::WHITE);
}

#[test]
fn moving_container_repaints_children_old_area() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let parent = ObjCfg::new().build(&mut ui, scr);
    ui.set_pos(parent, 10, 10);
    ui.set_size(parent, 20, 20);
    let child = ObjCfg::new().build(&mut ui, parent);
    ui.set_pos(child, -10, 0); // child extends beyond the parent's left edge
    ui.set_size(child, 10, 10);
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(Color::RED);
    ui.set_style(child, s);
    ui.render();
    assert_eq!(px(&rec, 5, 15), Color::RED); // child's old position
    ui.set_pos(parent, 40, 10); // move the parent container
    ui.render();
    assert_eq!(px(&rec, 5, 15), Color::BLACK); // the old area must be repainted (no ghosting)
    assert_eq!(px(&rec, 35, 15), Color::RED); // new position
}

#[test]
fn moving_slider_repaints_knob_overflow() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let s = SliderCfg::new(0, 100).build(&mut ui, scr);
    ui.set_pos(s, 10, 10);
    ui.set_value(s, 0); // knob at the far left, overflowing into x 6..14
    ui.render();
    assert_eq!(px(&rec, 7, 16), Color::WHITE); // the knob overflow area's old position
    ui.set_pos(s, 40, 10); // move the slider (same path as layout animations)
    ui.render();
    assert_eq!(px(&rec, 7, 16), Color::BLACK); // old overflow pixels must be cleared
}

#[test]
fn switch_shows_focus_border() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let sw = SwitchCfg::new().build(&mut ui, scr);
    ui.set_pos(sw, 10, 10);
    ui.group_add(sw);
    ui.render();
    // Focused state: white border, midpoint of the track's top edge
    assert_eq!(px(&rec, 30, 10), Color::WHITE);
}

#[test]
fn slider_knob_overflow_area_redrawn_on_move() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let s = SliderCfg::new(0, 100).build(&mut ui, scr);
    ui.set_pos(s, 10, 10);
    ui.render();
    // Initially the knob is at x 6..14, y 8..24 (overflows 2px above the track)
    assert_eq!(px(&rec, 10, 8), Color::WHITE);
    ui.set_value(s, 50);
    ui.render();
    // The old knob overflow area is repainted as background (no ghosting)
    assert_eq!(px(&rec, 10, 8), Color::BLACK);
    // New knob position (kx = 10+50 = 60, knob x 56..64)
    assert_eq!(px(&rec, 60, 8), Color::WHITE);
}

#[test]
fn list_highlight_respects_rounded_corner() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.set_pos(l, 10, 10);
    ui.render();
    // The first row highlight's top-left corner (inside the rounded-corner area) should not be the highlight color
    assert_ne!(px(&rec, 10, 12), Color::rgb(50, 70, 120));
    // Inside the first row (below the border) is the highlight color
    assert_eq!(px(&rec, 60, 12), Color::rgb(50, 70, 120));
}

#[test]
fn list_ghost_fully_cleared_after_fade() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let l = ListCfg::new(&["a", "b", "c"]).build(&mut ui, scr);
    ui.set_pos(l, 10, 10);
    ui.list_select(l, 2);
    ui.render();
    assert!(ui.list_remove(l)); // delete "c", ghost fades out
    ui.tick_inc(500); // beyond FX_DUR
    ui.timer_handler();
    // The ghost's row (row 2) area should be restored to the list background color, with no text residue
    for x in 14..40 {
        assert_eq!(px(&rec, x, 50), Color::rgb(34, 34, 44), "x={}", x);
    }
}
