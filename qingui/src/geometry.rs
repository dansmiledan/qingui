/// A 2D point in screen coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Point {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

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

/// An RGB color.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Color {
    /// Red channel (0..=255).
    pub r: u8,
    /// Green channel (0..=255).
    pub g: u8,
    /// Blue channel (0..=255).
    pub b: u8,
}

impl Color {
    /// Pure black.
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// Pure white.
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    /// Pure red.
    pub const RED: Color = Color::rgb(255, 0, 0);
    /// Pure green.
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    /// Pure blue.
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    /// Medium gray.
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    /// Light gray.
    pub const LIGHT_GRAY: Color = Color::rgb(200, 200, 200);
    /// Dark gray.
    pub const DARK_GRAY: Color = Color::rgb(40, 40, 40);

    /// Builds a color from its RGB channels.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }
    /// Converts to RGB565.
    pub fn to_rgb565(&self) -> u16 {
        (((self.r as u16) & 0xF8) << 8) | (((self.g as u16) & 0xFC) << 3) | ((self.b as u16) >> 3)
    }
    /// Blends `over` on top of `self` (the background) at opacity `opa` (0..=255).
    pub fn blend(self, over: Color, opa: u8) -> Color {
        let a = opa as u32;
        let inv = 255 - a;
        let m = |s: u8, o: u8| ((s as u32 * inv + o as u32 * a + 127) / 255) as u8;
        Color::rgb(m(self.r, over.r), m(self.g, over.g), m(self.b, over.b))
    }
    /// RGB565 (5-6-5) → RGB888 (bit-copy expansion, lossless round-trip).
    pub fn from_rgb565(v: u16) -> Color {
        let r = ((v >> 11) & 0x1F) as u8;
        let g = ((v >> 5) & 0x3F) as u8;
        let b = (v & 0x1F) as u8;
        Color::rgb((r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2))
    }
}
