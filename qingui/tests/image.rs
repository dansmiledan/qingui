use embedded_graphics::pixelcolor::Rgb888;
use qingui::display::Flush;
use qingui::widgets::image::{Frame, ImageCfg, ImageData};
use qingui::{Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

/// 2x2 all-red image
static RED: ImageData = ImageData {
    frames: &[Frame {w: 2, h: 2, rgb565: &[0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8] }],
    delays_ms: &[0],
};
/// Two-frame animation: red/blue, 100ms each
static ANIM: ImageData = ImageData {
    frames: &[
        Frame {w: 2, h: 2, rgb565: &[0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8] },
        Frame {w: 2, h: 2, rgb565: &[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00] },
    ],
    delays_ms: &[100, 100],
};

#[test]
fn builder_default_size_is_first_frame() {
    let mut ui: Ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let im = ImageCfg::new(&RED).build(&mut ui, s);
    assert_eq!(ui.rect(im), Rect::new(0, 0, 2, 2));
}

#[test]
fn static_image_sleeps() {
    let mut ui: Ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    ImageCfg::new(&RED).build(&mut ui, s);
    ui.tick_inc(16);
    ui.timer_handler();
    assert_eq!(ui.timer_handler(), u32::MAX); // single frame has no per-frame behavior
}

#[test]
fn gif_advances_and_wraps() {
    #[derive(Default)]
    struct Rec {n: usize }
    struct Shared(Rc<RefCell<Rec>>);
    impl Flush for Shared {
        fn flush(&mut self, _a: Rect, _p: &[Rgb888]) {self.0.borrow_mut().n += 1; }
    }
    let rec = Rc::new(RefCell::new(Rec::default()));
    let mut ui: Ui = Ui::new(64, 64, 16);
    ui.set_flush(Box::new(Shared(rec.clone())));
    let s = ui.screen();
    let im = ImageCfg::new(&ANIM).build(&mut ui, s);
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // the animation keeps it awake
    rec.borrow_mut().n = 0;
    ui.tick_inc(50); // under 100ms: no frame switch, no redraw
    ui.timer_handler();
    assert_eq!(rec.borrow().n, 0);
    ui.tick_inc(60); // 110ms accumulated: switch to frame 1 and redraw
    ui.timer_handler();
    assert!(rec.borrow().n > 0);
    ui.tick_inc(100); // another 100ms: wrap back to frame 0
    ui.timer_handler();
    assert!(rec.borrow().n > 0);
    let _ = im;
}
