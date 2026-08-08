use qingui::display::Flush;
use qingui::layout::Attach;
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

fn solid(ui: &mut Ui, parent: qingui::ObjRef, c: Color) -> qingui::ObjRef {
    let o = ObjCfg::new().build(ui, parent);
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(c);
    ui.set_style(o, s);
    o
}

#[test]
fn floating_center_on_target() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let target = solid(&mut ui, scr, Color::BLUE);
    ui.set_pos(target, 50, 50);
    ui.set_size(target, 100, 60);
    let tip = solid(&mut ui, scr, Color::RED);
    ui.set_size(tip, 20, 20);
    ui.set_floating(tip, target, Attach::Center);
    ui.timer_handler();
    // Centered: (50+(100-20)/2, 50+(60-20)/2) = (90, 70)
    assert_eq!(ui.abs_rect(tip), Rect::new(90, 70, 20, 20));
}

#[test]
fn floating_bottom_of_target() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let target = solid(&mut ui, scr, Color::BLUE);
    ui.set_pos(target, 50, 50);
    ui.set_size(target, 100, 60);
    let tip = solid(&mut ui, scr, Color::RED);
    ui.set_size(tip, 20, 20);
    ui.set_floating(tip, target, Attach::Bottom);
    ui.timer_handler();
    // Centered on the target's bottom edge: (90, 110)
    assert_eq!(ui.abs_rect(tip), Rect::new(90, 110, 20, 20));
}

#[test]
fn floating_follows_target_move() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let target = solid(&mut ui, scr, Color::BLUE);
    ui.set_pos(target, 50, 50);
    ui.set_size(target, 100, 60);
    let tip = solid(&mut ui, scr, Color::RED);
    ui.set_size(tip, 20, 20);
    ui.set_floating(tip, target, Attach::Center);
    ui.timer_handler();
    ui.set_pos(target, 100, 100); // target moves → the floating layer follows
    ui.timer_handler();
    assert_eq!(ui.abs_rect(tip), Rect::new(140, 120, 20, 20));
}

#[test]
fn move_to_back_changes_draw_order() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    ui.set_style(scr, bg);
    let a = solid(&mut ui, scr, Color::RED);
    ui.set_pos(a, 0, 0);
    ui.set_size(a, 20, 20);
    let b = solid(&mut ui, scr, Color::GREEN);
    ui.set_pos(b, 10, 10);
    ui.set_size(b, 20, 20);
    ui.render();
    // Later-created b is on top
    assert_eq!(px(&rec, 15, 15), Color::GREEN);
    // b moved to the back → a is on top
    ui.move_to_back(b);
    ui.render();
    assert_eq!(px(&rec, 15, 15), Color::RED);
}
