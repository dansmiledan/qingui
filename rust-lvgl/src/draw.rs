use crate::geometry::{Color, Rect};

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

    /// 圆角实心矩形：角用整数圆判定（无抗锯齿）
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
        // 四个角：圆心 (cx, cy)，距圆心 (dx, dy) 且 dx²+dy² ≤ r² 的像素在圆盘内；
        // 靠近圆心的像素被填充，外角被切掉。与中间带重叠处重复绘制无害。
        let r2 = radius * radius;
        let corners = [
            (r.x + radius, r.y + radius, -1i32, -1i32),
            (r.right() - radius - 1, r.y + radius, 1, -1),
            (r.x + radius, r.bottom() - radius - 1, -1, 1),
            (r.right() - radius - 1, r.bottom() - radius - 1, 1, 1),
        ];
        for (cx, cy, sx, sy) in corners {
            for dy in 0..=radius {
                for dx in 0..=radius {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    if dx * dx + dy * dy <= r2 {
                        self.put_clipped(cx + sx * dx, cy + sy * dy, c, opa, clip);
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
            // 四个角的 1px 圆弧带
            if rad > 0 {
                let r2 = rad * rad;
                let inner2 = (rad - 1) * (rad - 1);
                let corners = [
                    (inner.x + rad, inner.y + rad, -1i32, -1i32),
                    (inner.right() - rad - 1, inner.y + rad, 1, -1),
                    (inner.x + rad, inner.bottom() - rad - 1, -1, 1),
                    (inner.right() - rad - 1, inner.bottom() - rad - 1, 1, 1),
                ];
                for (cx, cy, sx, sy) in corners {
                    for dy in 0..=rad {
                        for dx in 0..=rad {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let d2 = dx * dx + dy * dy;
                            if d2 <= r2 && d2 >= inner2 {
                                self.put_clipped(cx + sx * dx, cy + sy * dy, c, opa, clip);
                            }
                        }
                    }
                }
            }
        }
    }
}
