use qingui::display::Flush;
use qingui::widgets::image::{Frame, ImageBuilder, ImageData};
use qingui::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

/// 2x2 全红图
static RED: ImageData = ImageData {
    frames: &[Frame { w: 2, h: 2, rgb565: &[0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8] }],
    delays_ms: &[0],
};
/// 两帧动画:红/蓝,各 100ms
static ANIM: ImageData = ImageData {
    frames: &[
        Frame { w: 2, h: 2, rgb565: &[0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8] },
        Frame { w: 2, h: 2, rgb565: &[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00] },
    ],
    delays_ms: &[100, 100],
};

#[test]
fn builder_default_size_is_first_frame() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let im = ImageBuilder::new(&RED).build(&mut ui, s);
    assert_eq!(ui.rect(im), Rect::new(0, 0, 2, 2));
}

#[test]
fn static_image_sleeps() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    ImageBuilder::new(&RED).build(&mut ui, s);
    ui.tick_inc(16);
    ui.timer_handler();
    assert_eq!(ui.timer_handler(), u32::MAX); // 单帧无逐帧行为
}

#[test]
fn gif_advances_and_wraps() {
    #[derive(Default)]
    struct Rec { n: usize }
    struct Shared(Rc<RefCell<Rec>>);
    impl Flush for Shared {
        fn flush(&mut self, _a: Rect, _p: &[Color]) { self.0.borrow_mut().n += 1; }
    }
    let rec = Rc::new(RefCell::new(Rec::default()));
    let mut ui = Ui::new(64, 64, 16);
    ui.set_flush(Box::new(Shared(rec.clone())));
    let s = ui.screen();
    let im = ImageBuilder::new(&ANIM).build(&mut ui, s);
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // 动画保持唤醒
    rec.borrow_mut().n = 0;
    ui.tick_inc(50); // 未到 100ms:不切帧不重绘
    ui.timer_handler();
    assert_eq!(rec.borrow().n, 0);
    ui.tick_inc(60); // 累计 110ms:切到帧 1 并重绘
    ui.timer_handler();
    assert!(rec.borrow().n > 0);
    ui.tick_inc(100); // 再 100ms:回卷到帧 0
    ui.timer_handler();
    assert!(rec.borrow().n > 0);
    let _ = im;
}
