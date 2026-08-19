use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use qingui::display::Flush;
use qingui::node::State;
use qingui::style::Style;
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

fn build() -> (Rc<RefCell<RecFlush>>, Ui, qingui::ObjRef) {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui: Ui = Ui::new(64, 64, 16);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    let o = ObjCfg::new()
        .size(10, 10)
        .style(Style::new().bg(Rgb888::RED))
        .build(&mut ui, scr);
    (rec, ui, o)
}

#[test]
fn selected_state_applies_style_selected() {
    let (rec, mut ui, o) = build();
    ui.set_style_selected(o, Style::new().bg(Rgb888::BLUE));
    ui.render();
    assert_eq!(px(&rec, 1, 1), Rgb888::RED); // not selected: base style
    ui.set_state(o, State::SELECTED, true);
    ui.render();
    assert_eq!(px(&rec, 1, 1), Rgb888::BLUE); // selected: the selected overlay
    ui.set_state(o, State::SELECTED, false);
    ui.render();
    assert_eq!(px(&rec, 1, 1), Rgb888::RED); // deselected: restored
}

#[test]
fn overlay_priority_focused_over_selected() {
    let (rec, mut ui, o) = build();
    ui.set_style_selected(o, Style::new().bg(Rgb888::BLUE));
    ui.set_style_focused(o, Style::new().bg(Rgb888::GREEN));
    // selected < focused
    ui.set_state(o, State::SELECTED, true);
    ui.set_state(o, State::FOCUSED, true);
    ui.render();
    assert_eq!(px(&rec, 1, 1), Rgb888::GREEN);
    // only selected
    ui.set_state(o, State::FOCUSED, false);
    ui.render();
    assert_eq!(px(&rec, 1, 1), Rgb888::BLUE);
}
