use crate::geometry::{Color, Point, Rect};
use crate::pixel::PixelFormat;
use embedded_graphics::draw_target::DrawTargetExt;
use embedded_graphics::geometry::{Angle, Size};
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::primitives::{Circle, CornerRadii, Line, Primitive, PrimitiveStyle, RoundedRectangle};
use embedded_graphics::Drawable;

/// Pixel buffer covering a screen region. All coordinates are absolute screen coordinates;
/// writes are offset by the `area` origin.
/// The pixel type `C` defaults to RGB888 `Color`; set it to the display's native
/// format (e.g. `Rgb565`) to render directly in device format.
pub struct Canvas<'a, C = Color> {
    /// The backing pixel storage.
    pub pixels: &'a mut [C],
    /// The absolute screen region this buffer covers.
    pub area: Rect,
    /// Row length in pixels (usually `area.w`).
    pub stride: i32,
}

impl<C: PixelFormat> Canvas<'_, C> {
    /// Fills the whole buffer with `c`.
    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(C::from_color(c));
    }

    /// Writes a single pixel (opaque, bounds-checked against the buffer area).
    pub(crate) fn put(&mut self, x: i32, y: i32, c: Color) {
        if !self.area.contains(Point { x, y }) {
            return;
        }
        let idx = ((y - self.area.y) * self.stride + (x - self.area.x)) as usize;
        self.pixels[idx] = C::from_color(c);
    }

    /// Batch-fills the pre-clipped rect rows (terminal write path, no delegation).
    fn fill_rows(&mut self, r: Rect, c: C) {
        let area_x = self.area.x;
        let area_y = self.area.y;
        let stride = self.stride;
        let w = r.w as usize;
        for y in r.y..r.bottom() {
            let row = ((y - area_y) * stride + (r.x - area_x)) as usize;
            self.pixels[row..row + w].fill(c);
        }
    }

    /// Fills `r` with `c`, clipped to `clip` and the buffer area.
    pub fn fill_rect(&mut self, r: Rect, c: Color, clip: Rect) {
        let Some(r) = r.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        self.fill_rows(r, C::from_color(c));
    }

    /// 1:1 blit of an RGB565 (little-endian) bitmap; silently draws nothing when `data` is
    /// shorter than `w * h * 2`. No allocation.
    pub fn blit565(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u8], clip: Rect) {
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
                self.put(px, py, Color::from_rgb565(v));
            }
        }
    }

    /// Filled rounded rectangle (aliased edges — no AA).
    pub fn fill_rounded(&mut self, r: Rect, radius: i32, c: Color, clip: Rect) {
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        if radius == 0 {
            self.fill_rect(r, c, clip);
            return;
        }
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = RoundedRectangle::new(r.into(), CornerRadii::new(Size::new(radius as u32, radius as u32)))
            .into_styled(PrimitiveStyle::with_fill(C::from_color(c)))
            .draw(&mut t);
    }

    /// Border inside `r` (aliased). `width <= 0` draws nothing.
    pub fn draw_border(&mut self, r: Rect, width: i32, radius: i32, c: Color, clip: Rect) {
        if width <= 0 {
            return;
        }
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        // e-g centers the stroke on the primitive's edge; inset the path by half the
        // stroke width so the band lands inside `r` (the old rasterizer's semantics).
        let inset = width / 2;
        let path = Rect::new(r.x + inset, r.y + inset, r.w - 2 * inset, r.h - 2 * inset);
        if path.is_empty() {
            return;
        }
        let path_radius = (radius - inset).max(0);
        let style = PrimitiveStyle::with_stroke(C::from_color(c), width as u32);
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        if path_radius == 0 {
            let _ = embedded_graphics::primitives::Rectangle::from(path).into_styled(style).draw(&mut t);
        } else {
            let _ = RoundedRectangle::new(path.into(), CornerRadii::new(Size::new(path_radius as u32, path_radius as u32)))
                .into_styled(style)
                .draw(&mut t);
        }
    }

    /// Filled circle (aliased).
    pub fn fill_circle(&mut self, center: Point, radius: i32, c: Color, clip: Rect) {
        if radius <= 0 {
            return;
        }
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = Circle::new(center - Point::new(radius, radius), (radius * 2) as u32)
            .into_styled(PrimitiveStyle::with_fill(C::from_color(c)))
            .draw(&mut t);
    }

    /// Circle outline with stroke `width` (aliased). The ring sits inside the nominal
    /// `radius` (see `draw_arc` for the stroke-centering adjustment).
    pub fn draw_circle(&mut self, center: Point, radius: i32, width: i32, c: Color, clip: Rect) {
        if radius <= 0 || width <= 0 {
            return;
        }
        let r = (radius - width / 2).max(1);
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = Circle::new(center - Point::new(r, r), (r * 2) as u32)
            .into_styled(PrimitiveStyle::with_stroke(C::from_color(c), width as u32))
            .draw(&mut t);
    }

    /// Arc (LVGL angle convention: 0 deg at 3 o'clock, positive clockwise), stroke `width`,
    /// square ends (e-g arcs have no round caps).
    /// e-g centers the stroke on the circle's edge while the old rasterizer kept the ring
    /// inside the nominal radius (band `(radius - width, radius]`), so the circle is shrunk
    /// by half the stroke width to land the band in the same place.
    pub fn draw_arc(&mut self, center: Point, radius: i32, width: i32, start_deg: i32, end_deg: i32, c: Color, clip: Rect) {
        if radius <= 0 || width <= 0 || end_deg <= start_deg {
            return;
        }
        let r = (radius - width / 2).max(1);
        let arc = embedded_graphics::primitives::Arc::new(
            center - Point::new(r, r),
            (r * 2) as u32,
            Angle::from_degrees(start_deg as f32),
            Angle::from_degrees((end_deg - start_deg) as f32),
        );
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = arc
            .into_styled(PrimitiveStyle::with_stroke(C::from_color(c), width as u32))
            .draw(&mut t);
    }

    /// Thick line with round caps (width >= 2 adds a circle cap at each end); 1px is a plain e-g line.
    pub fn draw_line(&mut self, p1: Point, p2: Point, width: i32, c: Color, clip: Rect) {
        if width <= 0 {
            return;
        }
        let c = C::from_color(c);
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = Line::new(p1, p2)
            .into_styled(PrimitiveStyle::with_stroke(c, width as u32))
            .draw(&mut t);
        if width >= 2 {
            let off = Point::new(-width / 2, -width / 2);
            let cap = PrimitiveStyle::with_fill(c);
            let _ = Circle::new(p1 + off, width as u32).into_styled(cap).draw(&mut t);
            let _ = Circle::new(p2 + off, width as u32).into_styled(cap).draw(&mut t);
        }
    }

    /// Mono text (top-baseline), clipped. No background: only glyph pixels are drawn.
    pub fn draw_text(&mut self, pos: Point, font: &'static MonoFont<'static>, s: &str, c: Color, clip: Rect) {
        let style = embedded_graphics::mono_font::MonoTextStyle::new(font, C::from_color(c));
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = embedded_graphics::text::Text::with_baseline(
            s,
            pos,
            style,
            embedded_graphics::text::Baseline::Top,
        )
        .draw(&mut t);
    }
}

impl<C: PixelFormat> embedded_graphics::draw_target::DrawTarget for Canvas<'_, C> {
    type Color = C;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        // Per-pixel path: ecosystem compatibility, no performance promise.
        for embedded_graphics::Pixel(p, color) in pixels {
            self.put(p.x, p.y, color.to_color());
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &embedded_graphics::primitives::Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let r: Rect = (*area).into();
        if let Some(r) = r.intersect(&self.area) {
            self.fill_rows(r, color);
        }
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &embedded_graphics::primitives::Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        // Row-wise writes without per-pixel bounds checks (e-g's default falls back to draw_iter).
        // `colors` maps to the UNclipped `area` row-major (typically exactly w*h items); it may
        // be shorter (drawing just stops) or infinite (fill_solid's default `repeat`), so it is
        // never drained eagerly.
        let src: Rect = (*area).into();
        let Some(r) = src.intersect(&self.area) else {
            return Ok(());
        };
        let mut colors = colors.into_iter();
        let stride = self.stride;
        let area_x = self.area.x;
        let area_y = self.area.y;
        let row_total = src.w as usize;
        let full_w = r.w as usize;
        // Skip the rows clipped off the top.
        let mut colors = colors.by_ref().skip((r.y - src.y) as usize * row_total);
        for y in r.y..r.bottom() {
            let row = ((y - area_y) * stride + (r.x - area_x)) as usize;
            // Skip the clipped-off x-prefix of this source row.
            let skip = (r.x - src.x) as usize;
            for (i, px) in colors.by_ref().skip(skip).take(full_w).enumerate() {
                self.pixels[row + i] = px;
            }
            // Discard the clipped-off suffix of this source row.
            for _ in colors.by_ref().take(row_total.saturating_sub(skip + full_w)) {}
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        Canvas::clear(self, color.to_color());
        Ok(())
    }
}

// `Dimensions` is implemented manually rather than `OriginDimensions`: the latter always
// reports origin (0,0), which would make eg clip/coordinate reasoning wrong for a canvas
// whose `area` has a non-zero screen origin.
impl<C> embedded_graphics::geometry::Dimensions for Canvas<'_, C> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        self.area.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{Circle as EgCircle, PrimitiveStyle as EgStyle};

    fn canvas565(buf: &mut [Rgb565]) -> Canvas<'_, Rgb565> {
        Canvas { pixels: buf, area: Rect::new(0, 0, 10, 10), stride: 10 }
    }

    #[test]
    fn rgb565_opaque_fill_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(255, 0, 0), clip);
        assert!(d.pixels.iter().all(|&p| p == Rgb565::RED));
    }

    #[test]
    fn fill_rect_respects_clip() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::WHITE, Rect::new(2, 2, 4, 4));
        assert_eq!(d.pixels[2 * 10 + 2], Color::WHITE);
        assert_eq!(d.pixels[0], Color::BLACK);
        assert_eq!(d.pixels[6 * 10 + 6], Color::BLACK); // just outside the clip
    }

    #[test]
    fn fill_rounded_fills_center_and_spares_corners() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rounded(Rect::new(1, 1, 8, 8), 3, Color::WHITE, clip);
        assert_eq!(d.pixels[5 * 10 + 5], Color::WHITE); // center
        assert_eq!(d.pixels[1 * 10 + 1], Color::BLACK); // rounded-off corner
        assert_eq!(d.pixels[0], Color::BLACK);          // outside the rect
    }

    #[test]
    fn fill_circle_covers_center_not_corner() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_circle(Point::new(5, 5), 3, Color::WHITE, clip);
        assert_eq!(d.pixels[5 * 10 + 5], Color::WHITE);
        assert_eq!(d.pixels[0], Color::BLACK);
    }

    #[test]
    fn draw_line_hits_endpoints() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.draw_line(Point::new(1, 1), Point::new(8, 8), 1, Color::WHITE, clip);
        assert_eq!(d.pixels[1 * 10 + 1], Color::WHITE);
        assert_eq!(d.pixels[8 * 10 + 8], Color::WHITE);
        assert_eq!(d.pixels[1 * 10 + 8], Color::BLACK); // off the diagonal
    }

    #[test]
    fn draw_border_paints_edges_not_center() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.draw_border(Rect::new(2, 2, 6, 6), 1, 0, Color::WHITE, clip);
        assert_eq!(d.pixels[2 * 10 + 2], Color::WHITE); // top-left edge
        assert_eq!(d.pixels[5 * 10 + 5], Color::BLACK); // center untouched
    }

    #[test]
    fn draw_arc_paints_ring_pixels() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.draw_arc(Point::new(5, 5), 3, 1, 0, 360, Color::WHITE, clip);
        // e-g centers an even-diameter circle between pixels: the bounding box is (2,2)..(7,7),
        // so the 3 o'clock ring pixel is at x=7 (not x=8 as with the old integer-centered grid).
        assert_eq!(d.pixels[5 * 10 + 7], Color::WHITE); // 3 o'clock point on the ring
        assert_eq!(d.pixels[5 * 10 + 5], Color::BLACK); // center hollow
    }

    #[test]
    fn draw_text_draws_glyph_pixels_only() {
        let mut buf = [Color::BLACK; 200];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 20, 10), stride: 20 };
        let clip = Rect::new(0, 0, 20, 10);
        d.draw_text(Point::new(0, 0), crate::font::DEFAULT_FONT, "I", Color::WHITE, clip);
        let white = buf.iter().filter(|&&p| p == Color::WHITE).count();
        assert!(white > 0 && white < 50, "glyph pixels drawn, background untouched ({white})");
    }

    #[test]
    fn draw_target_accepts_native_rgb565() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        EgCircle::new(embedded_graphics::geometry::Point::new(0, 0), 5)
            .into_styled(EgStyle::with_fill(Rgb565::GREEN))
            .draw(&mut d)
            .unwrap();
        assert_eq!(d.pixels[1 * 10 + 1], Rgb565::GREEN);
    }

    #[test]
    fn rgb565_put_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        d.put(2, 2, Color::WHITE);
        assert_eq!(RawU16::from(d.pixels[2 * 10 + 2]).into_inner(), Color::WHITE.to_rgb565());
    }

    #[test]
    fn default_canvas_still_rgb888() {
        let mut buf = [Color::BLACK; 100];
        let mut d: Canvas<'_> = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(80, 140, 255), clip);
        assert!(d.pixels.iter().all(|&p| p == Color::rgb(80, 140, 255)));
    }
}
