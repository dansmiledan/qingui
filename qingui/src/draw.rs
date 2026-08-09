use crate::geometry::Rect;

/// 4x4 supersampling coverage: number of subsamples of pixel (dx, dy) (pixel offset from the
/// circle center) that fall inside a circle of radius r, in 0..=16.
/// Subsample points are taken at 1/4-spaced centers inside the pixel (1/16-pixel fixed-point coordinates).
pub(crate) fn circle_cov16(dx: i32, dy: i32, r: i32) -> i32 {
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
pub(crate) const COV_MARGIN: i64 = 9;

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
pub(crate) fn arc_cov16(dx: i32, dy: i32, outer: i32, inner: i32, s: (i32, i32), e: (i32, i32), and_mode: bool) -> i32 {
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
pub(crate) fn line_sdf_cov16(px: i32, py: i32, x0: i32, y0: i32, ux: i64, uy: i64, len2_64: i64, inv_len: i64, r16: i64) -> i32 {
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
pub(crate) fn isqrt(n: u64) -> u64 {
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
pub(crate) fn div_floor_pos(a: i64, b: i64) -> i64 {
    debug_assert!(b > 0);
    let q = a / b;
    if a < 0 && a % b != 0 { q - 1 } else { q }
}

pub(crate) fn div_ceil_pos(a: i64, b: i64) -> i64 {
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
pub(crate) fn line_row_span(y: i32, x0: i32, y0: i32, x1: i32, y1: i32, dx: i32, dy: i32, len2: i32, rm: i64) -> Option<(i32, i32)> {
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
pub(crate) fn eg_rect(r: Rect) -> embedded_graphics::primitives::Rectangle {
    embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(r.x, r.y),
        embedded_graphics::geometry::Size::new(r.w.max(0) as u32, r.h.max(0) as u32),
    )
}
