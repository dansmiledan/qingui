use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use qingui::canvas::Canvas;
use qingui::display::Flush;
use qingui::widgets::obj::ObjCfg;
use qingui::{Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Rgb888>)>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Rgb888]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Rgb888 {
    let chunks = &rec.borrow().chunks;
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

/// Transparent background style (what the deleted canvas widget defaulted to).
fn transparent_style() -> qingui::style::Style {
    qingui::style::Style::default()
}

#[test]
fn draw_hook_paints_custom_content() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui: Ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Rgb888::BLACK);
    let scr = ui.screen();
    ui.set_style(scr, bg);

    let cv = ObjCfg::new()
        .size(30, 30)
        .style(transparent_style())
        .build(&mut ui, scr);
    ui.set_pos(cv, 10, 10);
    ui.set_draw_hook(cv, Some(Box::new(|d, abs, clip, _now| {
        d.fill_rect(Rect::new(abs.x + 2, abs.y + 2, 5, 5), Rgb888::RED, clip);
        d.draw_arc(
            qingui::Point {x: abs.x + 20, y: abs.y + 20 },
            6,
            3,
            0,
            270,
            Rgb888::GREEN,
            clip,
        );
    })));
    ui.render();

    // Custom rect
    assert_eq!(px(&rec, 12, 12), Rgb888::RED);
    // Custom arc (bottom-right 45° direction)
    assert_eq!(px(&rec, 33, 33), Rgb888::GREEN);
    // Transparent background: the screen's black shows through
    assert_eq!(px(&rec, 10, 10), Rgb888::BLACK);
}

#[test]
fn draw_hook_clipped_by_chunk() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui: Ui = Ui::new(64, 48, 16); // small buffer → multiple chunks
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    let cv = ObjCfg::new().size(64, 48).build(&mut ui, scr);
    ui.set_draw_hook(cv, Some(Box::new(|d, abs, clip, _now| {
        d.fill_rect(Rect::new(abs.x, abs.y, 64, 48), Rgb888::WHITE, clip);
    })));
    ui.render();
    // Full screen is 3 chunks, each chunk fully white (clip in effect, no out-of-bounds)
    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 3);
    for (area, buf) in chunks {
        assert!(buf.iter().all(|&c| c == Rgb888::WHITE), "chunk {:?} 应全白", area);
    }
}

#[test]
fn eg_draw_target_fill_rect_via_primitives() {
    // Draw a filled Rectangle through eg's primitive pipeline onto a Canvas, assert pixels.
    let mut buf = [Rgb888::BLACK; 100];
    let area = Rect::new(0, 0, 10, 10);
    {
        let mut c = Canvas {pixels: &mut buf, area, stride: 10 };
        use embedded_graphics::prelude::*;
        let r = embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::new(2, 2),
            embedded_graphics::geometry::Size::new(4, 4),
        );
        r.into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            Rgb888::new(255, 0, 0),
        ))
        .draw(&mut c)
        .unwrap();
    }
    assert_eq!(buf[2 * 10 + 2], Rgb888::new(255, 0, 0));
    assert_eq!(buf[0], Rgb888::BLACK);
}
