use embedded_graphics::pixelcolor::RgbColor;
use qingui::display::Flush;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::obj::ObjCfg;
use qingui::{Color, Rect, Ui};
use std::cell::{Cell, RefCell};
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

#[test]
fn draw_hook_overlays_builtin_widget() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui: Ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let scr = ui.screen();
    let btn = ButtonCfg::new("ok").build(&mut ui, scr);
    ui.set_pos(btn, 10, 10);
    ui.set_draw_hook(btn, Some(Box::new(|d, abs, clip, _now| {
        d.fill_rect(Rect::new(abs.x, abs.y, 3, 3), Color::RED, clip);
    })));
    ui.render();
    // The hook overlays the button's own content (the 3x3 top-left corner is covered in red)
    assert_eq!(px(&rec, 10, 10), Color::RED);
    assert_eq!(px(&rec, 11, 11), Color::RED);
}

#[test]
fn tick_hook_drives_wakeup_and_redraw() {
    let mut ui: Ui = Ui::new(64, 64, 16);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    let hits = Rc::new(Cell::new(0u32));
    let h = hits.clone();
    ui.set_tick_hook(o, Some(Box::new(move |_ui, _obj, _now| {
        h.set(h.get() + 1);
        true
    })));
    ui.tick_inc(16);
    ui.timer_handler(); // first frame (including the full-screen dirty from screen creation)
    assert!(hits.get() >= 1);
    assert_eq!(ui.timer_handler(), 0); // an active hook keeps it awake
    // Switching to an inactive hook → sleeps
    ui.set_tick_hook(o, Some(Box::new(|_, _, _| false)));
    assert_eq!(ui.timer_handler(), u32::MAX);
}
