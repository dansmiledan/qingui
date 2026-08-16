use embedded_graphics::pixelcolor::RgbColor;
use qingui::{Color, Point, Rect};

#[test]
fn rect_intersect_overlap() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(5, 5, 10, 10);
    assert_eq!(a.intersect(&b), Some(Rect::new(5, 5, 5, 5)));
}

#[test]
fn rect_intersect_disjoint() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(20, 0, 5, 5);
    assert_eq!(a.intersect(&b), None);
}

#[test]
fn rect_intersect_touching_edges_is_none() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(10, 0, 5, 5);
    assert_eq!(a.intersect(&b), None);
}

#[test]
fn rect_union() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(5, 5, 10, 10);
    assert_eq!(a.union(&b), Rect::new(0, 0, 15, 15));
}

#[test]
fn rect_contains_point_and_translate() {
    let r = Rect::new(0, 0, 10, 10);
    assert!(r.contains(Point { x: 9, y: 9 }));
    assert!(!r.contains(Point { x: 10, y: 0 }));
    assert_eq!(r.translate(3, -2), Rect::new(3, -2, 10, 10));
}

#[test]
fn color_rgb565() {
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::pixelcolor::raw::{RawData, RawU16};
    use qingui::PixelFormat;
    // The public Rgb565 PixelFormat impl delegates to e-g's From conversions (rounding quantization).
    let to565 = |c: Color| RawU16::from(Rgb565::from_color(c)).into_inner();
    assert_eq!(to565(Color::new(255, 255, 255)), 0xFFFF);
    assert_eq!(to565(Color::new(0, 0, 0)), 0x0000);
    assert_eq!(to565(Color::new(255, 0, 0)), 0xF800);
}

#[test]
fn color_blend() {
    use qingui::geometry::blend;
    let bg = Color::BLACK;
    assert_eq!(blend(bg, Color::WHITE, 255), Color::WHITE);
    assert_eq!(blend(bg, Color::WHITE, 0), Color::BLACK);
    let half = blend(bg, Color::new(200, 100, 50), 128);
    assert_eq!(half, Color::new(100, 50, 25));
}
