use qingui::display::Flush;
use qingui::layout::Attach;
use qingui::widgets::obj::ObjBuilder;
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
    let o = ObjBuilder::new().build(ui, parent);
    let mut s = qingui::style::Style::default();
    s.bg_color = Some(c);
    o.set_style(ui, s);
    o
}

#[test]
fn floating_center_on_target() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let target = solid(&mut ui, scr, Color::BLUE);
    target.set_pos(&mut ui, 50, 50);
    target.set_size(&mut ui, 100, 60);
    let tip = solid(&mut ui, scr, Color::RED);
    tip.set_size(&mut ui, 20, 20);
    tip.set_floating(&mut ui, target, Attach::Center);
    ui.timer_handler();
    // 居中：(50+(100-20)/2, 50+(60-20)/2) = (90, 70)
    assert_eq!(tip.abs_rect(&ui), Rect::new(90, 70, 20, 20));
}

#[test]
fn floating_bottom_of_target() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let target = solid(&mut ui, scr, Color::BLUE);
    target.set_pos(&mut ui, 50, 50);
    target.set_size(&mut ui, 100, 60);
    let tip = solid(&mut ui, scr, Color::RED);
    tip.set_size(&mut ui, 20, 20);
    tip.set_floating(&mut ui, target, Attach::Bottom);
    ui.timer_handler();
    // 目标下边缘居中：(90, 110)
    assert_eq!(tip.abs_rect(&ui), Rect::new(90, 110, 20, 20));
}

#[test]
fn floating_follows_target_move() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let target = solid(&mut ui, scr, Color::BLUE);
    target.set_pos(&mut ui, 50, 50);
    target.set_size(&mut ui, 100, 60);
    let tip = solid(&mut ui, scr, Color::RED);
    tip.set_size(&mut ui, 20, 20);
    tip.set_floating(&mut ui, target, Attach::Center);
    ui.timer_handler();
    target.set_pos(&mut ui, 100, 100); // 目标移动 → 浮层跟随
    ui.timer_handler();
    assert_eq!(tip.abs_rect(&ui), Rect::new(140, 120, 20, 20));
}

#[test]
fn z_index_changes_draw_order() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    let scr = ui.screen();
    scr.set_style(&mut ui, bg);
    let a = solid(&mut ui, scr, Color::RED);
    a.set_pos(&mut ui, 0, 0);
    a.set_size(&mut ui, 20, 20);
    let b = solid(&mut ui, scr, Color::GREEN);
    b.set_pos(&mut ui, 10, 10);
    b.set_size(&mut ui, 20, 20);
    ui.render();
    // 后创建的 b 在上
    assert_eq!(px(&rec, 15, 15), Color::GREEN);
    // b 降到 -1 → a 在上
    b.set_z_index(&mut ui, -1);
    ui.render();
    assert_eq!(px(&rec, 15, 15), Color::RED);
}
