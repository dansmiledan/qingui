use qingui::display::Flush;
use qingui::prelude::*;
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
    ui.set_style(scr, bg);
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
    ui.set_pos(s, 10, 10);
    ui.set_value(s, 50);
    ui.render();
    assert_eq!(ui.value(s), 50);
    // Track y center = 10+6, the indicator reaches 50% ≈ x=10+50
    assert_eq!(px(&rec, 20, 16), Color::rgb(80, 140, 255));
    // Past the end of the indicator is the track color (not the indicator color)
    assert_ne!(px(&rec, 100, 16), Color::rgb(80, 140, 255));
    // The knob is white around ~x=10+50-4..
    assert_eq!(px(&rec, 58, 16), Color::WHITE);
}

#[test]
fn slider_value_clamped_to_range() {
    let (mut ui, _) = setup();
    let scr = ui.screen();
    let s = SliderBuilder::new(10, 20).build(&mut ui, scr);
    ui.set_value(s, 999);
    assert_eq!(ui.value(s), 20);
    ui.set_value(s, -5);
    assert_eq!(ui.value(s), 10);
}

#[test]
fn switch_toggle_visual() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let sw = SwitchBuilder::new().build(&mut ui, scr);
    ui.set_pos(sw, 10, 10);
    ui.render();
    // off: track gray, knob on the left (sampling interior points of the circle to avoid anti-aliased edges)
    assert_eq!(px(&rec, 16, 20), Color::WHITE); // knob left
    assert_eq!(px(&rec, 44, 20), Color::rgb(90, 90, 90)); // right-end track
}

#[test]
fn bar_renders_progress() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let b = BarBuilder::new(0, 100).build(&mut ui, scr);
    ui.set_pos(b, 10, 10);
    ui.set_value(b, 25);
    ui.render();
    assert_eq!(px(&rec, 20, 14), Color::rgb(80, 140, 255));
    assert_ne!(px(&rec, 100, 14), Color::rgb(80, 140, 255));
}

#[test]
fn bar_small_value_keeps_left_semicircle() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let b = BarBuilder::new(0, 100).build(&mut ui, scr);
    ui.set_pos(b, 10, 10); // default size 100x8, radius=4
    ui.set_value(b, 5); // indicator width iw=5
    ui.render();
    let ind = Color::rgb(80, 140, 255);
    // The left end is clipped to the track shape (radius=4): (11,11) is outside the semicircle → not the indicator color
    assert_ne!(px(&rec, 11, 11), ind);
    // (11,14) is inside the semicircle → indicator color
    assert_eq!(px(&rec, 11, 14), ind);
    // Beyond the indicator's right boundary → not the indicator color
    assert_ne!(px(&rec, 20, 14), ind);
}

#[test]
fn list_selected_row_highlighted() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let l = ListBuilder::new(&["alpha", "beta", "gamma"]).build(&mut ui, scr);
    ui.set_pos(l, 10, 10);
    ui.list_select(l, 1);
    assert_eq!(ui.list_selected(l), 1);
    ui.tick_inc(300); // let the highlight-slide animation finish
    ui.timer_handler();
    // Row 2 (beta) background = highlight color. Row height 16, row 1 center y = 10+16+8=34, text left at x=12
    assert_eq!(px(&rec, 12, 34), Color::rgb(50, 70, 120));
}

#[test]
fn button_renders_text_centered() {
    let (mut ui, rec) = setup();
    let scr = ui.screen();
    let b = ButtonBuilder::new("OK").build(&mut ui, scr);
    ui.set_pos(b, 10, 10);
    ui.render();
    let r = ui.rect(b);
    // The text "OK" is 12px wide (FONT_6X10 glyph width 6), centered: start x = 10 + (w-12)/2
    assert!(r.w > 12);
    assert_eq!(qingui::font::text_size(&embedded_graphics::mono_font::ascii::FONT_6X10, "OK"), (12, 10));
    let text_x = 10 + (r.w - 12) / 2;
    // The text color (white) should appear somewhere in the text area
    let mut found_white = false;
    for y in 10..10 + r.h {
        for x in text_x..text_x + 12 {
            if px(&rec, x, y) == Color::WHITE {
                found_white = true;
            }
        }
    }
    assert!(found_white);
}
