use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use qingui::{Point, Rect};

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
    assert!(r.contains(Point {x: 9, y: 9 }));
    assert!(!r.contains(Point {x: 10, y: 0 }));
    assert_eq!(r.translate(3, -2), Rect::new(3, -2, 10, 10));
}

#[test]
fn color_rgb565() {
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::pixelcolor::raw::{RawData, RawU16};
    // e-g's From<Rgb888> for Rgb565 applies rounding quantization.
    let to565 = |c: Rgb888| RawU16::from(Rgb565::from(c)).into_inner();
    assert_eq!(to565(Rgb888::new(255, 255, 255)), 0xFFFF);
    assert_eq!(to565(Rgb888::new(0, 0, 0)), 0x0000);
    assert_eq!(to565(Rgb888::new(255, 0, 0)), 0xF800);
}

#[test]
fn color_blend() {
    use qingui::geometry::blend;
    let bg = Rgb888::BLACK;
    assert_eq!(blend(bg, Rgb888::WHITE, 255), Rgb888::WHITE);
    assert_eq!(blend(bg, Rgb888::WHITE, 0), Rgb888::BLACK);
    let half = blend(bg, Rgb888::new(200, 100, 50), 128);
    assert_eq!(half, Rgb888::new(100, 50, 25));
}
