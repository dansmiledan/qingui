use crate::geometry::{Color, Rect};

pub trait Flush {
    /// area 为屏幕绝对坐标矩形；pixels 为 area.w*area.h 个像素（行优先，RGB888）
    fn flush(&mut self, area: Rect, pixels: &[Color]);
}
