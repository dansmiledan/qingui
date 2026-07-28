use qingui::display::Flush;
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

#[test]
fn canvas_callback_draws_custom_content() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = qingui::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    ui.set_style(ui.screen(), bg);

    let cv = ui.create_canvas(ui.screen(), 30, 30, Box::new(|d, abs, clip, _now| {
        d.fill_rect(Rect::new(abs.x + 2, abs.y + 2, 5, 5), Color::RED, 255, clip);
        d.draw_arc(
            qingui::Point { x: abs.x + 20, y: abs.y + 20 },
            6,
            3,
            0,
            270,
            Color::GREEN,
            255,
            clip,
        );
    }));
    ui.set_pos(cv, 10, 10);
    ui.render();

    // 自定义矩形
    assert_eq!(px(&rec, 12, 12), Color::RED);
    // 自定义圆弧（右下 45° 方向）
    assert_eq!(px(&rec, 33, 33), Color::GREEN);
    // 画布默认透明背景
    assert_eq!(px(&rec, 10, 10), Color::BLACK);
}

#[test]
fn canvas_clipped_by_chunk() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16); // 小缓冲 → 多 chunk
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let cv = ui.create_canvas(ui.screen(), 64, 48, Box::new(|d, abs, clip, _now| {
        d.fill_rect(Rect::new(abs.x, abs.y, 64, 48), Color::WHITE, 255, clip);
    }));
    let _ = cv;
    ui.render();
    // 全屏 3 个 chunk，每个 chunk 内全白（clip 生效，不越界）
    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 3);
    for (area, buf) in chunks {
        assert!(buf.iter().all(|&c| c == Color::WHITE), "chunk {:?} 应全白", area);
    }
}
