use qingui::display::Flush;
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
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let btn = ui.create_button(ui.screen(), "ok");
    ui.set_pos(btn, 10, 10);
    ui.set_draw_hook(btn, Some(Box::new(|d, abs, clip, _now| {
        d.fill_rect(Rect::new(abs.x, abs.y, 3, 3), Color::RED, 255, clip);
    })));
    ui.render();
    // 钩子叠加在按钮自带内容之上（左上角 3x3 被覆盖为红色）
    assert_eq!(px(&rec, 10, 10), Color::RED);
    assert_eq!(px(&rec, 11, 11), Color::RED);
}

#[test]
fn tick_hook_drives_wakeup_and_redraw() {
    let mut ui = Ui::new(64, 64, 16);
    let o = ui.create_obj(ui.screen());
    let hits = Rc::new(Cell::new(0u32));
    let h = hits.clone();
    ui.set_tick_hook(o, Some(Box::new(move |_ui, _obj, _now| {
        h.set(h.get() + 1);
        true
    })));
    ui.tick_inc(16);
    ui.timer_handler(); // 首帧（含建屏全屏脏）
    assert!(hits.get() >= 1);
    assert_eq!(ui.timer_handler(), 0); // 活动 hook 保持唤醒
    // 换成不活动的 hook → 睡眠
    ui.set_tick_hook(o, Some(Box::new(|_, _, _| false)));
    assert_eq!(ui.timer_handler(), u32::MAX);
}
