use crate::draw::{circle_cov16, dir_vec, div_ceil_pos, div_floor_pos, eg_rect, isqrt, ArcGeom, ThickLine, COV_MARGIN};
use crate::geometry::{Color, Rect};
use crate::pixel::PixelFormat;

/// e-g Rectangle → Rect (origin + size; inverse of `crate::draw::eg_rect`)
fn from_eg_rect(r: embedded_graphics::primitives::Rectangle) -> Rect {
    Rect::new(r.top_left.x, r.top_left.y, r.size.width as i32, r.size.height as i32)
}

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

    pub(crate) fn put(&mut self, x: i32, y: i32, c: Color, opa: u8) {
        if !self.area.contains(crate::geometry::Point { x, y }) {
            return;
        }
        let lx = x - self.area.x;
        let ly = y - self.area.y;
        let idx = (ly * self.stride + lx) as usize;
        if opa >= 255 {
            self.pixels[idx] = C::from_color(c);
        } else if opa > 0 {
            self.pixels[idx] = C::from_color(self.pixels[idx].to_color().blend(c, opa));
        }
    }

    fn put_clipped(&mut self, x: i32, y: i32, c: Color, opa: u8, clip: Rect) {
        if clip.contains(crate::geometry::Point { x, y }) {
            self.put(x, y, c, opa);
        }
    }

    /// Writes a pixel without bounds checking. The caller must ensure `(x, y)`
    /// lies inside the buffer area — used by internal paths that already
    /// clipped the region (e.g. `fill_rect` after intersecting with the area).
    fn put_fast(&mut self, x: i32, y: i32, c: Color, opa: u8) {
        let lx = x - self.area.x;
        let ly = y - self.area.y;
        let idx = (ly * self.stride + lx) as usize;
        if opa >= 255 {
            self.pixels[idx] = C::from_color(c);
        } else if opa > 0 {
            self.pixels[idx] = C::from_color(self.pixels[idx].to_color().blend(c, opa));
        }
    }

    /// Fills `r` with `c` at opacity `opa` (0..=255), clipped to `clip` and the buffer area.
    pub fn fill_rect(&mut self, r: Rect, c: Color, opa: u8, clip: Rect) {
        let Some(r) = r.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        if opa >= 255 {
            // Opaque fast path: batch-fill whole rows (no per-pixel bounds check,
            // no per-pixel blending).
            let c = C::from_color(c);
            let area_x = self.area.x;
            let area_y = self.area.y;
            let stride = self.stride;
            let w = r.w as usize;
            for y in r.y..r.bottom() {
                let row = ((y - area_y) * stride + (r.x - area_x)) as usize;
                self.pixels[row..row + w].fill(c);
            }
        } else {
            // Translucent: per-pixel blend on the already-clipped region.
            for y in r.y..r.bottom() {
                for x in r.x..r.right() {
                    self.put_fast(x, y, c, opa);
                }
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
    /// Filled circle: per-scanline chord span (one `isqrt` per row). Only the
    /// boundary fringe gets the exact `circle_cov16` evaluation; the solid
    /// interior is batch-written. Output is pixel-identical to evaluating
    /// every bounding-box pixel (see `COV_MARGIN`).
    pub fn fill_circle(&mut self, center: crate::geometry::Point, radius: i32, c: Color, opa: u8, clip: Rect) {
        if radius <= 0 {
            return;
        }
        let Some(vis) = clip.intersect(&self.area) else {
            return;
        };
        let r16 = 16 * radius as i64;
        let out_zero2 = (r16 + COV_MARGIN) * (r16 + COV_MARGIN);
        let out_full2 = (r16 - COV_MARGIN) * (r16 - COV_MARGIN);
        for dy in -radius - 1..=radius + 1 {
            let y = center.y + dy;
            if y < vis.y || y >= vis.bottom() {
                continue;
            }
            let dy16 = 16 * dy as i64;
            let dy2 = dy16 * dy16;
            let zero2 = out_zero2 - dy2;
            if zero2 < 0 {
                continue;
            }
            // Pixels past this half-chord are definitely uncovered.
            let hz = isqrt(zero2 as u64) as i64;
            let dx_lo = div_ceil_pos(-hz, 16) as i32;
            let dx_hi = div_floor_pos(hz, 16) as i32;
            // Pixels inside this half-chord are definitely fully covered.
            let full2 = out_full2 - dy2;
            let (f_lo, f_hi) = if full2 >= 0 {
                let hf = isqrt(full2 as u64) as i64;
                (div_ceil_pos(-hf, 16) as i32, div_floor_pos(hf, 16) as i32)
            } else {
                (1, 0) // empty run
            };
            // Clamp everything to the visible x-range (put_fast has no checks).
            let vx_lo = vis.x - center.x;
            let vx_hi = vis.right() - 1 - center.x;
            let dx_lo = dx_lo.max(vx_lo);
            let dx_hi = dx_hi.min(vx_hi);
            let fl = f_lo.max(dx_lo).min(dx_hi + 1);
            let fh = f_hi.min(dx_hi).max(dx_lo - 1);
            for dx in dx_lo..fl {
                let cov = circle_cov16(dx, dy, radius);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_fast(center.x + dx, y, c, o);
                }
            }
            if fl <= fh {
                if opa >= 255 {
                    let row = ((y - self.area.y) * self.stride + (center.x + fl - self.area.x)) as usize;
                    self.pixels[row..row + (fh - fl + 1) as usize].fill(C::from_color(c));
                } else {
                    for dx in fl..=fh {
                        self.put_fast(center.x + dx, y, c, opa);
                    }
                }
            }
            for dx in fh + 1..=dx_hi {
                let cov = circle_cov16(dx, dy, radius);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_fast(center.x + dx, y, c, o);
                }
            }
        }
    }

    /// Ring (a `width`-thick circle edge, inset inward), 4x4 supersampling anti-aliased.
    /// Ring (circle outline): per-scanline chord spans; only pixels near the
    /// outer/inner boundary get the exact `circle_cov16` difference, the
    /// solid band and the hollow interior are decided by squared-distance
    /// comparisons. Pixel-identical output (see `COV_MARGIN`).
    pub fn draw_circle(&mut self, center: crate::geometry::Point, radius: i32, width: i32, c: Color, opa: u8, clip: Rect) {
        if radius <= 0 || width <= 0 {
            return;
        }
        let Some(vis) = clip.intersect(&self.area) else {
            return;
        };
        let inner = radius - width;
        let r16 = 16 * radius as i64;
        let in16 = 16 * inner as i64;
        let out_zero2 = (r16 + COV_MARGIN) * (r16 + COV_MARGIN);
        // Per-pixel classification runs in whole-px units so it stays i32
        // (hot on 32-bit cores). Dividing the (1/16 px)^2 thresholds by 256
        // is exact against integer dx^2+dy^2: floor for the "full"/"hole"
        // thresholds, ceil for the "full condition" lower bound.
        let out_full2 = ((r16 - COV_MARGIN) * (r16 - COV_MARGIN) >> 8) as i32;
        let in_zero2 = (((in16 + COV_MARGIN) * (in16 + COV_MARGIN) + 255) >> 8) as i32;
        let in_full2 = ((in16 - COV_MARGIN) * (in16 - COV_MARGIN) >> 8) as i32;
        for dy in -radius - 1..=radius + 1 {
            let y = center.y + dy;
            if y < vis.y || y >= vis.bottom() {
                continue;
            }
            let dy16 = 16 * dy as i64;
            let zero2 = out_zero2 - dy16 * dy16;
            if zero2 < 0 {
                continue;
            }
            let hz = isqrt(zero2 as u64) as i64;
            let dx_lo = (div_ceil_pos(-hz, 16) as i32).max(vis.x - center.x);
            let dx_hi = (div_floor_pos(hz, 16) as i32).min(vis.right() - 1 - center.x);
            let dy2 = dy * dy;
            // Pixels with d2 <= in_full2 are deep inside the hole (both
            // coverages 16, difference 0): skip that run without per-pixel
            // work — it is the bulk of a thin ring's bounding rows.
            let skip2 = if inner > 0 { in_full2 - dy2 } else { -1 };
            let hs = if skip2 >= 0 { isqrt(skip2 as u64) as i32 } else { -1 };
            let mut process = |dx: i32| {
                let d2 = dx * dx + dy2;
                if d2 <= out_full2 && (inner <= 0 || d2 >= in_zero2) {
                    // Deep inside the band: cov16(outer)=16, cov16(inner)=0.
                    self.put_fast(center.x + dx, y, c, opa);
                } else if inner > 0 && d2 <= in_full2 {
                    // Deep inside the hole: both coverages are 16, difference 0.
                } else {
                    let cov = circle_cov16(dx, dy, radius) - circle_cov16(dx, dy, inner);
                    if cov > 0 {
                        let o = (opa as u32 * cov as u32 / 16) as u8;
                        self.put_fast(center.x + dx, y, c, o);
                    }
                }
            };
            if hs < 0 {
                for dx in dx_lo..=dx_hi {
                    process(dx);
                }
            } else {
                for dx in dx_lo..=(-hs - 1).min(dx_hi) {
                    process(dx);
                }
                for dx in (hs + 1).max(dx_lo)..=dx_hi {
                    process(dx);
                }
            }
        }
    }

    /// Arc/pie sector: sweeps clockwise from `start_deg` to `end_deg` (screen coordinates, 0° = +x rightward).
    /// `width` = ring thickness (a pie chart when equal to `radius`). Full-edge 4x4 supersampling anti-aliasing.
    /// Arc (ring sector): per-scanline chord spans; pixels are classified by
    /// squared distance to the ring boundaries and by the wedge half-plane
    /// signs at the pixel center, with a conservative margin — only pixels
    /// near a boundary (ring or wedge ray) get the exact `ArcGeom::cov16`
    /// evaluation. Pixel-identical output (see `COV_MARGIN`).
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
        let Some(vis) = clip.intersect(&self.area) else {
            return;
        };
        let s = dir_vec(start_deg);
        let e = dir_vec(end);
        let and_mode = sweep <= 180;
        let inner = radius - width;
        let r16 = 16 * radius as i64;
        let in16 = 16 * inner as i64;
        let out_zero2 = (r16 + COV_MARGIN) * (r16 + COV_MARGIN);
        // Per-pixel classification runs in whole-px units so it stays i32
        // (hot on 32-bit cores); see draw_circle for the exact /256 folding.
        let out_full2 = ((r16 - COV_MARGIN) * (r16 - COV_MARGIN) >> 8) as i32;
        let in_zero2 = (((in16 + COV_MARGIN) * (in16 + COV_MARGIN) + 255) >> 8) as i32;
        let in_full2 = ((in16 - COV_MARGIN) * (in16 - COV_MARGIN) >> 8) as i32;
        // Wedge margin in z-units (z = dir x p, |dir| = 256 per 1/16 px).
        const MW: i64 = COV_MARGIN * 256;
        let (sx, sy) = (s.0 as i64, s.1 as i64);
        let (ex, ey) = (e.0 as i64, e.1 as i64);
        let geom = ArcGeom { outer: radius, inner, s, e, and_mode };
        for dy in -radius - 1..=radius + 1 {
            let y = center.y + dy;
            if y < vis.y || y >= vis.bottom() {
                continue;
            }
            let dy16 = 16 * dy as i64;
            let zero2 = out_zero2 - dy16 * dy16;
            if zero2 < 0 {
                continue;
            }
            let hz = isqrt(zero2 as u64) as i64;
            let dx_lo = (div_ceil_pos(-hz, 16) as i32).max(vis.x - center.x);
            let dx_hi = (div_floor_pos(hz, 16) as i32).min(vis.right() - 1 - center.x);
            let dy2 = dy * dy;
            // Wedge half-plane signs at the pixel center, stepped
            // incrementally along the row: z(dx+1) = z(dx) - 16*dir.y.
            let mut z1 = sx * dy16 - sy * (16 * dx_lo as i64);
            let mut z2 = ex * dy16 - ey * (16 * dx_lo as i64);
            let dz1 = -16 * sy;
            let dz2 = -16 * ey;
            for dx in dx_lo..=dx_hi {
                let d2 = dx * dx + dy2;
                let ring_full = d2 <= out_full2 && (inner <= 0 || d2 >= in_zero2);
                let ring_zero = inner > 0 && d2 <= in_full2;
                let (w_full, w_zero) = if and_mode {
                    (z1 >= MW && z2 <= -MW, z1 <= -MW || z2 >= MW)
                } else {
                    (z1 >= MW || z2 <= -MW, z1 <= -MW && z2 >= MW)
                };
                z1 += dz1;
                z2 += dz2;
                if ring_full && w_full {
                    self.put_fast(center.x + dx, y, c, opa);
                } else if ring_zero || w_zero {
                    continue;
                } else {
                    let cov = geom.cov16(dx, dy);
                    if cov > 0 {
                        let o = (opa as u32 * cov as u32 / 16) as u8;
                        self.put_fast(center.x + dx, y, c, o);
                    }
                }
            }
        }
    }

    /// Version of `draw_text` with an explicit opacity.
    pub fn draw_text_opa(&mut self, pos: crate::geometry::Point, font: &'static embedded_graphics::mono_font::MonoFont, s: &str, c: Color, opa: u8, clip: Rect) {
        use embedded_graphics::draw_target::DrawTarget;
        use embedded_graphics::Drawable;
        struct EgTarget<'a, 'b, C> {
            d: &'a mut Canvas<'b, C>,
            c: Color,
            opa: u8,
        }
        impl<C: PixelFormat> DrawTarget for EgTarget<'_, '_, C> {
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
        impl<C> embedded_graphics::geometry::Dimensions for EgTarget<'_, '_, C> {
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

    /// Line as a thick segment (`width >= 2`): for each scanline, the span
    /// covered by the capsule (segment + round caps of radius `width/2`) is
    /// computed analytically (`ThickLine::row_span`), and only span pixels get a
    /// signed-distance coverage evaluation (`ThickLine::cov16`, 1px linear AA
    /// ramp). Replaces the old Bresenham + per-step `fill_circle` stamp
    /// (which repainted overlapping pixels on thick lines) — and avoids
    /// scanning the full bounding box, which made long diagonals quadratic.
    ///
    /// `width == 1` keeps the old plain Bresenham walk: it has no AA in the
    /// baseline either, and a coverage-evaluated 1px line is ~15x slower for
    /// no visual requirement (see the plan's review amendment).
    ///
    /// The width branches live in separate methods so this small dispatcher
    /// stays inlinable — with a constant `width` at the call site (the common
    /// case) the compiler specializes straight to the relevant path.
    pub fn draw_line(&mut self, p1: crate::geometry::Point, p2: crate::geometry::Point, width: i32, c: Color, opa: u8, clip: Rect) {
        if width <= 0 {
            return;
        }
        if width == 1 {
            self.draw_line_width1(p1, p2, c, opa, clip);
        } else {
            self.draw_line_thick(p1, p2, width, c, opa, clip);
        }
    }

    /// 1px fast path: Bresenham walk, one put per step, no AA (same output
    /// as the pre-optimization baseline).
    fn draw_line_width1(&mut self, p1: crate::geometry::Point, p2: crate::geometry::Point, c: Color, opa: u8, clip: Rect) {
        let (mut x, mut y) = (p1.x, p1.y);
        let (x1, y1) = (p2.x, p2.y);
        let dx = (x1 - x).abs();
        let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let sy = if y < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.put_clipped(x, y, c, opa, clip);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Thick line (`width >= 2`): per-scanline capsule span + signed-distance
    /// coverage at pixel centers; see `draw_line`.
    fn draw_line_thick(&mut self, p1: crate::geometry::Point, p2: crate::geometry::Point, width: i32, c: Color, opa: u8, clip: Rect) {
        let (x0, y0) = (p1.x, p1.y);
        let (x1, y1) = (p2.x, p2.y);
        if (x1 - x0) * (x1 - x0) + (y1 - y0) * (y1 - y0) == 0 {
            // Degenerate point: single stamped pixel.
            self.put_clipped(x0, y0, c, opa, clip);
            return;
        }
        let line = ThickLine::new(p1, p2, width);
        let r = width / 2;
        // Visible region: clip once, then the span loop needs no per-pixel
        // bounds checks (put_fast).
        let Some(vis) = clip.intersect(&self.area) else {
            return;
        };
        // Scanline range of the capsule, clamped to the visible rows.
        let miny = (y0.min(y1) - r - 1).max(vis.y);
        let maxy = (y0.max(y1) + r + 1).min(vis.bottom() - 1);
        for y in miny..=maxy {
            let Some((lo, hi)) = line.row_span(y) else {
                continue;
            };
            let lo = lo.max(vis.x);
            let hi = hi.min(vis.right() - 1);
            for x in lo..=hi {
                let cov = line.cov16(x, y);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_fast(x, y, c, o);
                }
            }
        }
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
            self.put(p.x, p.y, color.to_color(), 255);
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &embedded_graphics::primitives::Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // Fast path: route through the batch row fill (eg's default would fall back to draw_iter).
        let clip = self.area;
        self.fill_rect(from_eg_rect(*area), color.to_color(), 255, clip);
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
        eg_rect(self.area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{Circle, PrimitiveStyle};

    fn canvas565(buf: &mut [Rgb565]) -> Canvas<'_, Rgb565> {
        Canvas { pixels: buf, area: Rect::new(0, 0, 10, 10), stride: 10 }
    }

    #[test]
    fn rgb565_opaque_fill_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(255, 0, 0), 255, clip);
        assert!(d.pixels.iter().all(|&p| p == Rgb565::RED));
    }

    #[test]
    fn rgb565_fill_circle_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_circle(Point { x: 5, y: 5 }, 3, Color::WHITE, 255, clip);
        assert_eq!(d.pixels[5 * 10 + 5], Rgb565::WHITE); // center pixel
    }

    #[test]
    fn rgb565_blend_roundtrips_through_rgb888() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        // Blend pure white at 50% over black: internal math yields ~(128,128,128),
        // stored quantized to 565.
        d.put(2, 2, Color::WHITE, 128);
        let expected = Color::BLACK.blend(Color::WHITE, 128);
        assert_eq!(RawU16::from(d.pixels[2 * 10 + 2]).into_inner(), expected.to_rgb565());
    }

    #[test]
    fn draw_target_accepts_native_rgb565() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        Circle::new(embedded_graphics::geometry::Point::new(0, 0), 5)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
            .draw(&mut d)
            .unwrap();
        assert_eq!(d.pixels[1 * 10 + 1], Rgb565::GREEN); // pixel (1, 1)
    }

    #[test]
    fn default_canvas_still_rgb888() {
        let mut buf = [Color::BLACK; 100];
        let mut d: Canvas<'_> = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(80, 140, 255), 255, clip);
        assert!(d.pixels.iter().all(|&p| p == Color::rgb(80, 140, 255)));
    }
}
