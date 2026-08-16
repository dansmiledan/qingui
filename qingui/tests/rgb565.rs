//! End-to-end: qingui renders directly into an Rgb565 framebuffer, and e-g
//! ecosystem code draws into a qingui canvas using the device-native color type.

use std::cell::RefCell;
use std::rc::Rc;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle};

use embedded_graphics::pixelcolor::RgbColor;
use qingui::anim::{Anim, AnimProp};
use qingui::canvas::Canvas;
use qingui::display::Flush;
use qingui::layout::{Align, Flex, FlexDir};
use qingui::pixel::PixelFormat;
use qingui::style::Style;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::switch::{SwitchCfg, UiSwitchExt};
use qingui::widgets::Layout;
use qingui::{Color, Rect, Ui};

struct Rec(Rc<RefCell<Vec<(Rect, Vec<Rgb565>)>>>);

impl Flush<Rgb565> for Rec {
    fn flush(&mut self, area: Rect, pixels: &[Rgb565]) {
        self.0.borrow_mut().push((area, pixels.to_vec()));
    }
}

fn render_solid(bg: Color) -> Vec<(Rect, Vec<Rgb565>)> {
    let mut ui = Ui::<Rgb565>::new(40, 20, 20);
    let mut s = Style::default();
    s.bg_color = Some(bg);
    let screen = ui.screen();
    ui.set_style(screen, s);
    let rec = Rc::new(RefCell::new(Vec::new()));
    ui.set_flush(Box::new(Rec(rec.clone())));
    ui.render();
    // Drop the Ui (and its Box<dyn Flush>) so `rec` is the sole Rc owner.
    drop(ui);
    Rc::try_unwrap(rec).unwrap().into_inner()
}

#[test]
fn ui_rgb565_flushes_device_native_pixels() {
    let chunks = render_solid(Color::RED);
    let total: usize = chunks.iter().map(|(_, px)| px.len()).sum();
    assert_eq!(total, 40 * 20);
    assert!(chunks.iter().all(|(_, px)| px.iter().all(|&p| p == Rgb565::RED)));
}

#[test]
fn ui_rgb565_quantization_is_self_consistent() {
    let bg = Color::new(80, 140, 255);
    let chunks = render_solid(bg);
    let expected = Rgb565::from_color(bg);
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|(_, px)| px.iter().all(|&p| p == expected)));
}

#[test]
fn ui_rgb565_hosts_builtin_widgets() {
    let mut ui = Ui::<Rgb565>::new(80, 40, 40);
    let screen = ui.screen();
    ButtonCfg::new("OK").size(40, 20).build(&mut ui, screen);
    // (a) A container with a real flex layout hosting a child.
    let panel = ObjCfg::new()
        .size(80, 20)
        .layout(Layout::Flex(Flex {
            dir: FlexDir::Row, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
        }))
        .build(&mut ui, screen);
    ObjCfg::new().size(10, 10).build(&mut ui, panel);
    // (b) An interactive widget driven through its ext trait.
    let sw = SwitchCfg::new().build(&mut ui, panel);
    ui.toggle_switch(sw);
    assert_eq!(ui.value(sw), 1);
    // (c) An animation with an on_done callback.
    let done = Rc::new(RefCell::new(false));
    let d2 = done.clone();
    ui.anim_start(Anim::new(sw, AnimProp::X, 0, 20, 50).on_done(move |_ui| *d2.borrow_mut() = true));
    ui.tick_inc(50);
    ui.timer_handler();
    assert!(*done.borrow());
    // Rendering completes and flushes exactly the full screen area.
    let rec = Rc::new(RefCell::new(Vec::new()));
    ui.set_flush(Box::new(Rec(rec.clone())));
    ui.invalidate_area(Rect::new(0, 0, 80, 40));
    ui.render();
    let chunks = rec.borrow();
    assert!(!chunks.is_empty());
    let total: usize = chunks.iter().map(|(_, px)| px.len()).sum();
    assert_eq!(total, 80 * 40);
}

#[test]
fn eg_primitives_draw_into_rgb565_canvas() {
    let mut buf = [Rgb565::BLACK; 100];
    let mut d = Canvas { pixels: &mut buf[..], area: Rect::new(0, 0, 10, 10), stride: 10 };
    Circle::new(Point::new(0, 0), 5)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
        .draw(&mut d)
        .unwrap();
    assert_eq!(d.pixels[1 * 10 + 1], Rgb565::GREEN); // pixel (1, 1)
}
