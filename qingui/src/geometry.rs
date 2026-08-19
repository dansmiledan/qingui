use embedded_graphics::pixelcolor::{Rgb888, RgbColor};

/// A 2D point in screen coordinates (re-exported from embedded-graphics).
pub use embedded_graphics::geometry::Point;

/// An axis-aligned rectangle with integer coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    /// X coordinate of the left edge.
    pub x: i32,
    /// Y coordinate of the top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl Rect {
    /// Creates a rect from an origin and a size.
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    /// Returns `true` if the rect has zero or negative width or height.
    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }
    /// X coordinate of the right edge (exclusive).
    pub fn right(&self) -> i32 {
        self.x + self.w
    }
    /// Y coordinate of the bottom edge (exclusive).
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
    /// Returns `true` if the two rects overlap with positive area.
    pub fn intersects(&self, other: &Rect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
    /// Returns the overlapping region of the two rects, or `None` if they do not overlap.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        if r > x && b > y {
            Some(Rect::new(x, y, r - x, b - y))
        } else {
            None
        }
    }
    /// Returns the smallest rect containing both rects.
    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let r = self.right().max(other.right());
        let b = self.bottom().max(other.bottom());
        Rect::new(x, y, r - x, b - y)
    }
    /// Returns `true` if `p` lies inside the rect (edges inclusive at left/top, exclusive at right/bottom).
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }
    /// Returns a copy of the rect shifted by `(dx, dy)`.
    pub fn translate(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

/// Mixes `fg` onto `bg` by weight `t` (0..=255), producing an opaque color.
/// This is plain color mixing (used for LED brightness), not alpha compositing —
/// qingui has no translucency; the result fully replaces the pixel.
/// Mixing happens in 8-bit RGB888 space (via e-g's built-in conversions), so the
/// result is identical for every target format up to its quantization.
pub fn blend<C>(bg: C, fg: C, t: u8) -> C
where
    C: Into<Rgb888> + From<Rgb888>,
{
    let (bg, fg): (Rgb888, Rgb888) = (bg.into(), fg.into());
    let a = t as u32;
    let inv = 255 - a;
    let m = |s: u8, o: u8| ((s as u32 * inv + o as u32 * a + 127) / 255) as u8;
    Rgb888::new(m(bg.r(), fg.r()), m(bg.g(), fg.g()), m(bg.b(), fg.b())).into()
}

impl From<Rect> for embedded_graphics::primitives::Rectangle {
    fn from(r: Rect) -> Self {
        embedded_graphics::primitives::Rectangle::new(
            Point::new(r.x, r.y),
            embedded_graphics::geometry::Size::new(r.w.max(0) as u32, r.h.max(0) as u32),
        )
    }
}

impl From<embedded_graphics::primitives::Rectangle> for Rect {
    fn from(r: embedded_graphics::primitives::Rectangle) -> Self {
        Rect::new(r.top_left.x, r.top_left.y, r.size.width as i32, r.size.height as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::primitives::Rectangle as EgRect;

    #[test]
    fn point_is_eg_point() {
        // Compile-time proof: qingui::Point IS embedded-graphics' Point.
        let p: Point = embedded_graphics::geometry::Point::new(3, 4);
        assert_eq!((p.x, p.y), (3, 4));
        let q = Point::new(3, 4);
        assert_eq!(p, q);
    }

    #[test]
    fn rect_eg_roundtrip() {
        let r = Rect::new(2, 3, 10, 20);
        let eg: EgRect = r.into();
        assert_eq!(eg.top_left, Point::new(2, 3));
        assert_eq!((eg.size.width, eg.size.height), (10, 20));
        let back: Rect = eg.into();
        assert_eq!(back, r);
    }

    #[test]
    fn rect_to_eg_clamps_negative_size() {
        let eg: EgRect = Rect::new(5, 5, -3, 7).into();
        assert_eq!((eg.size.width, eg.size.height), (0, 7));
    }
}
