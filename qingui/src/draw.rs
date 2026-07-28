use crate::geometry::{Color, Rect};

/// 4x4 超采样覆盖率：像素 (dx, dy)（相对圆心的像素偏移）落在半径 r 圆内的子采样数，0..=16。
/// 子采样点取像素内 1/4 间隔的中心位置（1/16 像素定点坐标）。
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

/// 一块屏幕区域的像素缓冲。坐标一律为屏幕绝对坐标，写入时减去 area 原点。
pub struct DrawBuf<'a> {
    pub pixels: &'a mut [Color],
    pub area: Rect,
    pub stride: i32,
}

impl DrawBuf<'_> {
    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(c);
    }

    fn put(&mut self, x: i32, y: i32, c: Color, opa: u8) {
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

    /// 圆角实心矩形：角用 4x4 超采样抗锯齿
    pub fn fill_rounded(&mut self, r: Rect, radius: i32, c: Color, opa: u8, clip: Rect) {
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        if radius == 0 {
            self.fill_rect(r, c, opa, clip);
            return;
        }
        // 中间带（覆盖全高的中央竖带）
        self.fill_rect(Rect::new(r.x + radius, r.y, r.w - 2 * radius, r.h), c, opa, clip);
        // 左右侧带（角区之间的直边段）
        self.fill_rect(Rect::new(r.x, r.y + radius, radius, r.h - 2 * radius), c, opa, clip);
        self.fill_rect(Rect::new(r.right() - radius, r.y + radius, radius, r.h - 2 * radius), c, opa, clip);
        // 四个角：超采样覆盖率混合（圆边外 1px 也有部分覆盖，形成平滑过渡）
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

    /// 边框：外接矩形 r 的内侧画 width 宽的一圈，角半径 radius。
    /// 实现为 width 个 1px 圆角矩形描边（逐圈内缩）。
    pub fn draw_border(&mut self, r: Rect, width: i32, radius: i32, c: Color, opa: u8, clip: Rect) {
        for i in 0..width {
            let inner = Rect::new(r.x + i, r.y + i, r.w - 2 * i, r.h - 2 * i);
            if inner.is_empty() {
                break;
            }
            let rad = (radius - i).max(0).min(inner.w / 2).min(inner.h / 2);
            // 四条直边
            self.fill_rect(Rect::new(inner.x + rad, inner.y, inner.w - 2 * rad, 1), c, opa, clip);
            self.fill_rect(Rect::new(inner.x + rad, inner.bottom() - 1, inner.w - 2 * rad, 1), c, opa, clip);
            self.fill_rect(Rect::new(inner.x, inner.y + rad, 1, inner.h - 2 * rad), c, opa, clip);
            self.fill_rect(Rect::new(inner.right() - 1, inner.y + rad, 1, inner.h - 2 * rad), c, opa, clip);
            // 四个角的 1px 圆弧带（超采样抗锯齿）
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

    /// 逐行绘制文本，支持 '\n'。glyph bit0 = 最左像素。
    pub fn draw_text(&mut self, pos: crate::geometry::Point, s: &str, c: Color, clip: Rect) {
        self.draw_text_opa(pos, s, c, 255, clip);
    }

    /// draw_text 的带透明度版本
    pub fn draw_text_opa(&mut self, pos: crate::geometry::Point, s: &str, c: Color, opa: u8, clip: Rect) {
        let mut y = pos.y;
        for line in s.split('\n') {
            let mut x = pos.x;
            for ch in line.chars() {
                let g = crate::font::glyph(ch);
                for row in 0..8i32 {
                    let bits = g[row as usize];
                    for col in 0..8i32 {
                        if bits & (1 << col) != 0 {
                            self.put_clipped(x + col, y + row, c, opa, clip);
                        }
                    }
                }
                x += crate::font::GLYPH_W;
            }
            y += crate::font::LINE_H;
        }
    }
}
