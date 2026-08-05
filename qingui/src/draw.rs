use crate::geometry::{Color, Rect};

/// 4x4 supersampling coverage: number of subsamples of pixel (dx, dy) (pixel offset from the
/// circle center) that fall inside a circle of radius r, in 0..=16.
/// Subsample points are taken at 1/4-spaced centers inside the pixel (1/16-pixel fixed-point coordinates).
fn circle_cov16(dx: i32, dy: i32, r: i32) -> i32 {
    let r16 = 16 * r;
    let rr = r16 * r16;
    let mut n = 0;
    for a in 0..4 {
        for b in 0..4 {
            let sx = 16 * dx - 6 + 4 * a;
            let sy = 16 * dy - 6 + 4 * b;
            if sx * sx + sy * sy <= rr {
                n += 1;
            }
        }
    }
    n
}

/// sin(0°..=90°), 256 fixed-point (sin(d°) * 256 rounded)
#[rustfmt::skip]
const SIN90: [i32; 91] = [
      0,   4,   9,  13,  18,  22,  27,  31,  36,  40,
     44,  49,  53,  57,  62,  66,  71,  75,  79,  83,
     88,  92,  96, 100, 104, 108, 112, 116, 120, 124,
    128, 132, 136, 139, 143, 147, 150, 154, 158, 161,
    165, 168, 172, 175, 179, 181, 184, 187, 190, 193,
    196, 199, 202, 205, 207, 210, 212, 215, 217, 220,
    222, 224, 226, 228, 230, 232, 234, 236, 238, 239,
    241, 242, 244, 245, 247, 248, 249, 250, 251, 252,
    253, 254, 254, 255, 255, 255, 256, 256, 256, 256,
    256,
];

/// Angle in degrees (screen coordinates: 0° = +x rightward, increasing clockwise) → unit vector (256 fixed-point)
pub(crate) fn dir_vec(deg: i32) -> (i32, i32) {
    let d = deg.rem_euclid(360);
    let (q, a) = (d / 90, d % 90);
    let (sin_a, cos_a) = (SIN90[a as usize], SIN90[(90 - a) as usize]);
    match q {
        0 => (cos_a, sin_a),
        1 => (-sin_a, cos_a),
        2 => (-cos_a, -sin_a),
        _ => (sin_a, -cos_a),
    }
}

/// 4x4 supersampling coverage of an arc sector: subsamples must satisfy both the ring band
/// (inner < d ≤ outer) and the angular wedge
fn arc_cov16(dx: i32, dy: i32, outer: i32, inner: i32, s: (i32, i32), e: (i32, i32), and_mode: bool) -> i32 {
    let out2 = (16 * outer) * (16 * outer);
    let in2 = (16 * inner) * (16 * inner);
    let mut n = 0;
    for a in 0..4 {
        for b in 0..4 {
            let sx = 16 * dx - 6 + 4 * a;
            let sy = 16 * dy - 6 + 4 * b;
            let d2 = sx * sx + sy * sy;
            if d2 > out2 || (inner > 0 && d2 <= in2) {
                continue;
            }
            // Wedge containment test (cross-product sign)
            let z1 = s.0 * sy - s.1 * sx;
            let z2 = e.0 * sy - e.1 * sx;
            let inside = if and_mode { z1 >= 0 && z2 <= 0 } else { z1 >= 0 || z2 <= 0 };
            if inside {
                n += 1;
            }
        }
    }
    n
}

/// Rect → e-g Rectangle (x/y origin + size; negative sizes are defended to 0)
fn eg_rect(r: Rect) -> embedded_graphics::primitives::Rectangle {
    embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(r.x, r.y),
        embedded_graphics::geometry::Size::new(r.w.max(0) as u32, r.h.max(0) as u32),
    )
}

/// Pixel buffer covering a screen region. All coordinates are absolute screen coordinates;
/// writes are offset by the `area` origin.
pub struct DrawBuf<'a> {
    /// The backing pixel storage.
    pub pixels: &'a mut [Color],
    /// The absolute screen region this buffer covers.
    pub area: Rect,
    /// Row length in pixels (usually `area.w`).
    pub stride: i32,
}

impl DrawBuf<'_> {
    /// Fills the whole buffer with `c`.
    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(c);
    }

    pub(crate) fn put(&mut self, x: i32, y: i32, c: Color, opa: u8) {
        if !self.area.contains(crate::geometry::Point { x, y }) {
            return;
        }
        let lx = x - self.area.x;
        let ly = y - self.area.y;
        let idx = (ly * self.stride + lx) as usize;
        if opa >= 255 {
            self.pixels[idx] = c;
        } else if opa > 0 {
            self.pixels[idx] = self.pixels[idx].blend(c, opa);
        }
    }

    fn put_clipped(&mut self, x: i32, y: i32, c: Color, opa: u8, clip: Rect) {
        if clip.contains(crate::geometry::Point { x, y }) {
            self.put(x, y, c, opa);
        }
    }

    /// Fills `r` with `c` at opacity `opa` (0..=255), clipped to `clip` and the buffer area.
    pub fn fill_rect(&mut self, r: Rect, c: Color, opa: u8, clip: Rect) {
        let Some(r) = r.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.put(x, y, c, opa);
            }
        }
    }

    /// 1:1 blit of an RGB565 (little-endian) bitmap; silently draws nothing when `data` is
    /// shorter than `w * h * 2`. No allocation.
    pub fn blit565(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u8], opa: u8, clip: Rect) {
        if w <= 0 || h <= 0 || data.len() < (w as usize) * (h as usize) * 2 {
            return;
        }
        let dst = Rect::new(x, y, w, h);
        let Some(r) = dst.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        for py in r.y..r.bottom() {
            for px in r.x..r.right() {
                let sx = (px - x) as usize;
                let sy = (py - y) as usize;
                let i = (sy * w as usize + sx) * 2;
                let v = data[i] as u16 | ((data[i + 1] as u16) << 8);
                self.put(px, py, crate::geometry::Color::from_rgb565(v), opa);
            }
        }
    }

    /// Filled rounded rectangle: corners use 4x4 supersampling anti-aliasing.
    pub fn fill_rounded(&mut self, r: Rect, radius: i32, c: Color, opa: u8, clip: Rect) {
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        if radius == 0 {
            self.fill_rect(r, c, opa, clip);
            return;
        }
        // Center band (the vertical strip covering the full height)
        self.fill_rect(Rect::new(r.x + radius, r.y, r.w - 2 * radius, r.h), c, opa, clip);
        // Left and right side bands (the straight segments between the corner areas)
        self.fill_rect(Rect::new(r.x, r.y + radius, radius, r.h - 2 * radius), c, opa, clip);
        self.fill_rect(Rect::new(r.right() - radius, r.y + radius, radius, r.h - 2 * radius), c, opa, clip);
        // Four corners: supersampled coverage blend (the 1px band outside the rounded edge
        // is partially covered too, forming a smooth transition)
        let corners = [
            (r.x + radius, r.y + radius, -1i32, -1i32),
            (r.right() - radius - 1, r.y + radius, 1, -1),
            (r.x + radius, r.bottom() - radius - 1, -1, 1),
            (r.right() - radius - 1, r.bottom() - radius - 1, 1, 1),
        ];
        for (cx, cy, sx, sy) in corners {
            for dy in 0..=radius + 1 {
                for dx in 0..=radius + 1 {
                    let cov = circle_cov16(dx, dy, radius);
                    if cov > 0 {
                        let o = (opa as u32 * cov as u32 / 16) as u8;
                        self.put_clipped(cx + sx * dx, cy + sy * dy, c, o, clip);
                    }
                }
            }
        }
    }

    /// Border: draws a `width`-thick ring along the inside of the bounding rect `r`, with
    /// corner radius `radius`.
    /// Implemented as `width` 1px rounded-rect strokes, each inset one more pixel.
    pub fn draw_border(&mut self, r: Rect, width: i32, radius: i32, c: Color, opa: u8, clip: Rect) {
        for i in 0..width {
            let inner = Rect::new(r.x + i, r.y + i, r.w - 2 * i, r.h - 2 * i);
            if inner.is_empty() {
                break;
            }
            let rad = (radius - i).max(0).min(inner.w / 2).min(inner.h / 2);
            // Four straight edges
            self.fill_rect(Rect::new(inner.x + rad, inner.y, inner.w - 2 * rad, 1), c, opa, clip);
            self.fill_rect(Rect::new(inner.x + rad, inner.bottom() - 1, inner.w - 2 * rad, 1), c, opa, clip);
            self.fill_rect(Rect::new(inner.x, inner.y + rad, 1, inner.h - 2 * rad), c, opa, clip);
            self.fill_rect(Rect::new(inner.right() - 1, inner.y + rad, 1, inner.h - 2 * rad), c, opa, clip);
            // Four corners as 1px arc bands (supersampled anti-aliasing)
            if rad > 0 {
                let corners = [
                    (inner.x + rad, inner.y + rad, -1i32, -1i32),
                    (inner.right() - rad - 1, inner.y + rad, 1, -1),
                    (inner.x + rad, inner.bottom() - rad - 1, -1, 1),
                    (inner.right() - rad - 1, inner.bottom() - rad - 1, 1, 1),
                ];
                for (cx, cy, sx, sy) in corners {
                    for dy in 0..=rad + 1 {
                        for dx in 0..=rad + 1 {
                            let cov = circle_cov16(dx, dy, rad) - circle_cov16(dx, dy, rad - 1);
                            if cov > 0 {
                                let o = (opa as u32 * cov as u32 / 16) as u8;
                                self.put_clipped(cx + sx * dx, cy + sy * dy, c, o, clip);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draws text via the e-g Text renderer; On pixels go through `put(fg/opa)`, Off pixels
    /// are not written (transparent background).
    pub fn draw_text(&mut self, pos: crate::geometry::Point, font: &'static embedded_graphics::mono_font::MonoFont, s: &str, c: Color, clip: Rect) {
        self.draw_text_opa(pos, font, s, c, 255, clip);
    }

    /// Filled disc (solid circle), 4x4 supersampling anti-aliased.
    pub fn fill_circle(&mut self, center: crate::geometry::Point, radius: i32, c: Color, opa: u8, clip: Rect) {
        if radius <= 0 {
            return;
        }
        for dy in -radius - 1..=radius + 1 {
            for dx in -radius - 1..=radius + 1 {
                let cov = circle_cov16(dx, dy, radius);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_clipped(center.x + dx, center.y + dy, c, o, clip);
                }
            }
        }
    }

    /// Ring (a `width`-thick circle edge, inset inward), 4x4 supersampling anti-aliased.
    pub fn draw_circle(&mut self, center: crate::geometry::Point, radius: i32, width: i32, c: Color, opa: u8, clip: Rect) {
        if radius <= 0 || width <= 0 {
            return;
        }
        let inner = radius - width;
        for dy in -radius - 1..=radius + 1 {
            for dx in -radius - 1..=radius + 1 {
                let cov = circle_cov16(dx, dy, radius) - circle_cov16(dx, dy, inner);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_clipped(center.x + dx, center.y + dy, c, o, clip);
                }
            }
        }
    }

    /// Arc/pie sector: sweeps clockwise from `start_deg` to `end_deg` (screen coordinates, 0° = +x rightward).
    /// `width` = ring thickness (a pie chart when equal to `radius`). Full-edge 4x4 supersampling anti-aliasing.
    pub fn draw_arc(
        &mut self,
        center: crate::geometry::Point,
        radius: i32,
        width: i32,
        start_deg: i32,
        end_deg: i32,
        c: Color,
        opa: u8,
        clip: Rect,
    ) {
        if radius <= 0 || width <= 0 {
            return;
        }
        let mut end = end_deg;
        while end <= start_deg {
            end += 360;
        }
        let sweep = end - start_deg;
        if sweep >= 360 {
            self.draw_circle(center, radius, width, c, opa, clip);
            return;
        }
        let s = dir_vec(start_deg);
        let e = dir_vec(end);
        let and_mode = sweep <= 180;
        let inner = radius - width;
        for dy in -radius - 1..=radius + 1 {
            for dx in -radius - 1..=radius + 1 {
                let cov = arc_cov16(dx, dy, radius, inner, s, e, and_mode);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_clipped(center.x + dx, center.y + dy, c, o, clip);
                }
            }
        }
    }

    /// Version of `draw_text` with an explicit opacity.
    pub fn draw_text_opa(&mut self, pos: crate::geometry::Point, font: &'static embedded_graphics::mono_font::MonoFont, s: &str, c: Color, opa: u8, clip: Rect) {
        use embedded_graphics::draw_target::DrawTarget;
        use embedded_graphics::Drawable;
        struct EgTarget<'a, 'b> {
            d: &'a mut DrawBuf<'b>,
            c: Color,
            opa: u8,
        }
        impl DrawTarget for EgTarget<'_, '_> {
            type Color = embedded_graphics::pixelcolor::BinaryColor;
            type Error = core::convert::Infallible;
            fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
            where
                I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
            {
                for embedded_graphics::Pixel(p, col) in pixels {
                    if col == embedded_graphics::pixelcolor::BinaryColor::On {
                        self.d.put(p.x, p.y, self.c, self.opa);
                    }
                }
                Ok(())
            }
        }
        // `Dimensions` must be implemented manually rather than `OriginDimensions`: the latter
        // always reports origin (0,0), while `clipped()` intersects the clip with the bounding
        // box, so a non-zero `area` origin would incorrectly crop the text
        impl embedded_graphics::geometry::Dimensions for EgTarget<'_, '_> {
            fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
                eg_rect(self.d.area)
            }
        }
        let style = embedded_graphics::mono_font::MonoTextStyle::new(font, embedded_graphics::pixelcolor::BinaryColor::On);
        let mut t = EgTarget { d: self, c, opa };
        let mut t = embedded_graphics::draw_target::DrawTargetExt::clipped(&mut t, &eg_rect(clip));
        let _ = embedded_graphics::text::Text::with_baseline(
            s,
            embedded_graphics::geometry::Point::new(pos.x, pos.y),
            style,
            embedded_graphics::text::Baseline::Top,
        )
        .draw(&mut t);
    }

    /// Line (Bresenham + a radius stamp for line width).
    pub fn draw_line(&mut self, p1: crate::geometry::Point, p2: crate::geometry::Point, width: i32, c: Color, opa: u8, clip: Rect) {
        let (mut x0, mut y0) = (p1.x, p1.y);
        let (x1, y1) = (p2.x, p2.y);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let r = width / 2;
        loop {
            if r > 0 {
                self.fill_circle(crate::geometry::Point { x: x0, y: y0 }, r, c, opa, clip);
            } else {
                self.put_clipped(x0, y0, c, opa, clip);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}
