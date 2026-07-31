use core::any::Any;
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::custom::Widget;
use qingui::widgets::WidgetCtx;
use qingui::{Color, ObjRef, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

struct Gauge {
    v: i32,
    ticks: u32,
}
impl Widget for Gauge {
    fn draw(&self, ctx: &WidgetCtx, d: &mut qingui::draw::DrawBuf, clip: Rect) {
        d.fill_rect(ctx.abs, Color::RED, 255, clip);
    }
    fn tick(&mut self, _now: u64) -> qingui::widgets::TickOut {
        self.ticks += 1;
        qingui::widgets::TickOut::ACTIVE
    }
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, key: Key) -> bool {
        if key == Key::Up {
            self.v += 1;
            true
        } else {
            false
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

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
fn custom_widget_draws_and_handles_keys() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let g = ui.create_custom(ui.screen(), 20, 20, Box::new(Gauge { v: 0, ticks: 0 }));
    g.set_pos(&mut ui, 5, 5);
    ui.render();
    assert_eq!(px(&rec, 6, 6), Color::RED); // draw 被调用

    assert_eq!(g.custom::<Gauge>(&ui).unwrap().v, 0);
    g.group_add(&mut ui);
    ui.keypad_input(Key::Up); // 焦点对象收到键 → on_key 消费
    assert_eq!(g.custom::<Gauge>(&ui).unwrap().v, 1);

    g.custom_mut::<Gauge, _>(&mut ui, |g| g.v = 42);
    assert_eq!(g.custom::<Gauge>(&ui).unwrap().v, 42);
    assert!(g.custom::<String>(&ui).is_none()); // 类型不匹配 → None
}

#[test]
fn custom_widget_tick_dispatch() {
    let mut ui = Ui::new(160, 120, 120);
    let g = ui.create_custom(ui.screen(), 20, 20, Box::new(Gauge { v: 0, ticks: 0 }));
    // Gauge::tick 返回 ACTIVE → timer_handler 保持唤醒（返回 0）
    assert_eq!(ui.timer_handler(), 0);
    // WidgetKind::Custom(w) => w.tick(now) 分派被真正调用：每帧一次，计数器递增
    assert_eq!(g.custom::<Gauge>(&ui).unwrap().ticks, 1);
    assert_eq!(ui.timer_handler(), 0);
    assert_eq!(g.custom::<Gauge>(&ui).unwrap().ticks, 2);
}
