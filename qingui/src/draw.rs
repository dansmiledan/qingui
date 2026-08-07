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

/// Conservative subsample-reach margin in 1/16 px: `circle_cov16`/`arc_cov16`
/// subsamples sit at most sqrt(6^2+6^2) ~= 8.49 sixteenths from the pixel
/// center. A pixel whose center is more than this margin inside (outside)
/// every boundary is therefore definitely fully covered (uncovered), so the
/// scanline loops below only evaluate the exact coverage functions on the
/// boundary fringe — output stays pixel-identical.
const COV_MARGIN: i64 = 9;

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

/// Signed-distance-field coverage of a thick line segment, evaluated at the
/// pixel CENTER: perpendicular distance to the segment while inside its
/// extent, Euclidean distance to the nearer endpoint at the round caps.
/// Coverage is a 1px-wide linear ramp: full (16/16) at distance <= r16 - 8,
/// falling to 0 at r16 + 8 — cheap analytic AA in the same 0..=16 scale as
/// `circle_cov16`, without a 16x subsample loop per pixel.
///
/// All coordinates are in 1/16-pixel fixed-point units (the pixel center is
/// `16*px`, matching `circle_cov16`'s subsample average). `ux`/`uy` are the
/// segment vector scaled by 16; `inv_len` is `(1<<32) / length_in_1/16px`,
/// precomputed once per line so the hot path has no division.
#[allow(clippy::too_many_arguments)]
fn line_sdf_cov16(px: i32, py: i32, x0: i32, y0: i32, ux: i64, uy: i64, len2_64: i64, inv_len: i64, r16: i64) -> i32 {
    let cx = (16 * px - 16 * x0) as i64;
    let cy = (16 * py - 16 * y0) as i64;
    let t = cx * ux + cy * uy;
    let d16 = if t >= 0 && t <= len2_64 {
        // Perpendicular distance to the infinite line: |cross(u, v)| / |u|.
        let cross = (cx * uy - cy * ux).abs();
        (cross * inv_len) >> 32
    } else {
        // Round cap: distance to the nearer endpoint (few pixels, isqrt ok).
        let (ex, ey) = if t < 0 { (cx, cy) } else { (cx - ux, cy - uy) };
        isqrt((ex * ex + ey * ey) as u64) as i64
    };
    (r16 + 8 - d16).clamp(0, 16) as i32
}

/// Integer square root (floor), bit-by-bit method; no_std-friendly.
fn isqrt(n: u64) -> u64 {
    let mut x = n;
    let mut c = 0u64;
    let mut d = 1u64 << 62; // largest power of four <= u64::MAX
    while d > n {
        d >>= 2;
    }
    while d != 0 {
        if x >= c + d {
            x -= c + d;
            c = (c >> 1) + d;
        } else {
            c >>= 1;
        }
        d >>= 2;
    }
    c
}

/// Floor/ceil division with a positive divisor (`div_floor`/`div_ceil` are
/// unstable on this toolchain). Rust's `/` truncates toward zero, so only
/// the non-exact cases with the "wrong" sign need a one-step adjustment.
fn div_floor_pos(a: i64, b: i64) -> i64 {
    debug_assert!(b > 0);
    let q = a / b;
    if a < 0 && a % b != 0 { q - 1 } else { q }
}

fn div_ceil_pos(a: i64, b: i64) -> i64 {
    debug_assert!(b > 0);
    let q = a / b;
    if a > 0 && a % b != 0 { q + 1 } else { q }
}

/// Inclusive x-span covered by the thick segment (capsule: segment swept with
/// a disk of radius `rm` = half-width + 1px AA margin) on scanline `y`, or
/// None when the row misses it. The capsule is convex, so each row's
/// intersection is a single interval: strip (|cross| <= rm*len) intersected
/// with the slab between the endpoint planes, unioned with the two round-cap
/// disks. All integer math; divisions are per-row, not per-pixel.
///
/// The +1px margin guarantees every pixel with nonzero coverage lies inside
/// the returned span (the AA ramp reaches at most half-width + 0.5px from the
/// segment), so scanning only the span misses no covered pixel.
#[allow(clippy::too_many_arguments)]
fn line_row_span(y: i32, x0: i32, y0: i32, x1: i32, y1: i32, dx: i32, dy: i32, len2: i32, rm: i64) -> Option<(i32, i32)> {
    let len = isqrt(len2 as u64) as i64;
    let strip = rm * len;
    let yy = (y - y0) as i64;
    let mut span: Option<(i64, i64)> = None;

    // Row-level checks for the degenerate axis-aligned forms of strip/slab.
    let in_strip = dy != 0 || (yy * dx as i64).abs() <= strip;
    let t_row = yy * dy as i64;
    let in_slab = dx != 0 || (0..=len2 as i64).contains(&t_row);
    if in_strip && in_slab {
        let mut lo = i64::MIN / 2;
        let mut hi = i64::MAX / 2;
        // Strip: c - strip <= (x-x0)*dy <= c + strip, with c = yy*dx.
        if dy != 0 {
            let c = yy * dx as i64;
            // Normalize to a positive divisor so ceil/floor division apply.
            let (a, b, d) = if dy > 0 {
                (c - strip, c + strip, dy as i64)
            } else {
                (-(c + strip), -(c - strip), -(dy as i64))
            };
            lo = x0 as i64 + div_ceil_pos(a, d);
            hi = x0 as i64 + div_floor_pos(b, d);
        }
        // Slab: 0 <= (x-x0)*dx + yy*dy <= len2.
        if dx != 0 {
            let (a, b, d) = if dx > 0 {
                (-t_row, len2 as i64 - t_row, dx as i64)
            } else {
                (t_row - len2 as i64, t_row, -(dx as i64))
            };
            lo = lo.max(x0 as i64 + div_ceil_pos(a, d));
            hi = hi.min(x0 as i64 + div_floor_pos(b, d));
        }
        if lo <= hi {
            span = Some((lo, hi));
        }
    }
    // Round caps: disk of radius rm around each endpoint.
    for (cx, cy) in [(x0, y0), (x1, y1)] {
        let dyc = (y - cy) as i64;
        let rem = rm * rm - dyc * dyc;
        if rem >= 0 {
            let half = isqrt(rem as u64) as i64;
            span = Some(match span {
                Some((lo, hi)) => (lo.min(cx as i64 - half), hi.max(cx as i64 + half)),
                None => (cx as i64 - half, cx as i64 + half),
            });
        }
    }
    let (lo, hi) = span?;
    Some((lo as i32, hi as i32))
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

    /// Writes a pixel without bounds checking. The caller must ensure `(x, y)`
    /// lies inside the buffer area — used by internal paths that already
    /// clipped the region (e.g. `fill_rect` after intersecting with the area).
    fn put_fast(&mut self, x: i32, y: i32, c: Color, opa: u8) {
        let lx = x - self.area.x;
        let ly = y - self.area.y;
        let idx = (ly * self.stride + lx) as usize;
        if opa >= 255 {
            self.pixels[idx] = c;
        } else if opa > 0 {
            self.pixels[idx] = self.pixels[idx].blend(c, opa);
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
                    self.pixels[row..row + (fh - fl + 1) as usize].fill(c);
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
    /// near a boundary (ring or wedge ray) get the exact `arc_cov16`
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
                    let cov = arc_cov16(dx, dy, radius, inner, s, e, and_mode);
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

    /// Line as a thick segment (`width >= 2`): for each scanline, the span
    /// covered by the capsule (segment + round caps of radius `width/2`) is
    /// computed analytically (`line_row_span`), and only span pixels get a
    /// signed-distance coverage evaluation (`line_sdf_cov16`, 1px linear AA
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
        // Segment vector (dx, dy) and its half-width normal radius.
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = dx * dx + dy * dy;
        if len2 == 0 {
            // Degenerate point: single stamped pixel.
            self.put_clipped(x0, y0, c, opa, clip);
            return;
        }
        let r = width / 2;
        let rm = (r + 1) as i64; // half-width + 1px AA margin
        // Per-line invariants for the SDF coverage (1/16 px fixed point).
        let r16 = (width * 16 / 2) as i64;
        let (ux, uy) = (16 * dx as i64, 16 * dy as i64);
        let len2_64 = len2 as i64 * 256;
        let inv_len = (1i64 << 32) / isqrt(len2_64 as u64) as i64;
        // Visible region: clip once, then the span loop needs no per-pixel
        // bounds checks (put_fast).
        let Some(vis) = clip.intersect(&self.area) else {
            return;
        };
        // Scanline range of the capsule, clamped to the visible rows.
        let miny = (y0.min(y1) - r - 1).max(vis.y);
        let maxy = (y0.max(y1) + r + 1).min(vis.bottom() - 1);
        for y in miny..=maxy {
            let Some((lo, hi)) = line_row_span(y, x0, y0, x1, y1, dx, dy, len2, rm) else {
                continue;
            };
            let lo = lo.max(vis.x);
            let hi = hi.min(vis.right() - 1);
            for x in lo..=hi {
                let cov = line_sdf_cov16(x, y, x0, y0, ux, uy, len2_64, inv_len, r16);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_fast(x, y, c, o);
                }
            }
        }
    }
}
