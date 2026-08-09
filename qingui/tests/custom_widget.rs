use core::any::Any;
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::{Canvas, KeyOutcome, TickOut, Widget, WidgetCtx};
use qingui::{Color, ObjRef, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

struct Gauge {
    v: i32,
    ticks: u32,
}
impl Widget for Gauge {
    fn draw(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
        d.fill_rect(ctx.abs, Color::RED, 255, clip);
    }
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, _now: u64) -> TickOut {
        self.ticks += 1;
        TickOut::ACTIVE
    }
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, key: Key) -> KeyOutcome {
        if key == Key::Up {
            self.v += 1;
            KeyOutcome::Consumed
        } else {
            KeyOutcome::Pass
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
    let g = ui.create_widget(ui.screen(), 20, 20, Box::new(Gauge { v: 0, ticks: 0 }));
    ui.set_pos(g, 5, 5);
    ui.render();
    assert_eq!(px(&rec, 6, 6), Color::RED); // draw was called

    assert_eq!(ui.widget::<Gauge>(g).unwrap().v, 0);
    ui.group_add(g);
    ui.keypad_input(Key::Up); // the focused object receives the key → on_key consumes it
    assert_eq!(ui.widget::<Gauge>(g).unwrap().v, 1);

    ui.update::<Gauge, _>(g, |g| g.v = 42);
    assert_eq!(ui.widget::<Gauge>(g).unwrap().v, 42);
    assert!(ui.widget::<String>(g).is_none()); // type mismatch → None
}

// --- Task 3: take-out dispatch through the unified `widgets::Widget` trait ---

/// A widget whose on_key mutates its own state on `self` and creates a sibling
/// label through `&mut Ui` (only safe because its kind is taken out of the arena).
struct KeyProbe {
    hit: bool,
}
impl qingui::widgets::Widget for KeyProbe {
    fn on_key(&mut self, ui: &mut Ui, _obj: ObjRef, key: Key) -> qingui::widgets::KeyOutcome {
        if key == Key::Enter {
            self.hit = true;
            let scr = ui.screen();
            qingui::widgets::label::LabelCfg::new("x").build(ui, scr);
            qingui::widgets::KeyOutcome::Consumed
        } else {
            qingui::widgets::KeyOutcome::Pass
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[test]
fn on_key_receives_mut_ui_via_takeout() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let before = ui.children(scr).len();
    let w = ui.create_widget(scr, 10, 10, Box::new(KeyProbe { hit: false }));
    ui.group_add(w);
    ui.keypad_input(Key::Enter);
    assert!(ui.widget::<KeyProbe>(w).unwrap().hit, "on_key should have run on the taken-out state");
    assert_eq!(ui.children(scr).len(), before + 2, "the probe plus the sibling label created inside on_key");
}

#[test]
fn custom_widget_tick_dispatch() {
    let mut ui = Ui::new(160, 120, 120);
    let g = ui.create_widget(ui.screen(), 20, 20, Box::new(Gauge { v: 0, ticks: 0 }));
    // Gauge::tick returns ACTIVE → timer_handler stays awake (returns 0)
    assert_eq!(ui.timer_handler(), 0);
    // The take-out tick dispatch really runs: once per frame, counter increments
    assert_eq!(ui.widget::<Gauge>(g).unwrap().ticks, 1);
    assert_eq!(ui.timer_handler(), 0);
    assert_eq!(ui.widget::<Gauge>(g).unwrap().ticks, 2);
}
