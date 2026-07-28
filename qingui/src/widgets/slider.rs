use crate::draw::DrawBuf;
use crate::geometry::{Color, Rect};
use super::WidgetCtx;

pub(crate) fn draw(min: i32, max: i32, value: i32, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
    let iw = (abs.w as f32 * frac) as i32;
    if iw > 0 {
        d.fill_rounded(Rect::new(abs.x, abs.y, iw, abs.h), ctx.resolved.radius, Color::rgb(80, 140, 255), ctx.ap(255), clip);
    }
    let kx = abs.x + iw;
    let knob = Rect::new(kx - 4, abs.y - 2, 8, abs.h + 4);
    let kc = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::WHITE };
    d.fill_rounded(knob, 3, kc, ctx.ap(255), clip);
}

/// 旋钮超出轨道的区域（±4px 横向，±2px 纵向）：值变化时的标脏外扩
pub(crate) fn overflow_rect(abs: Rect) -> Rect {
    Rect::new(abs.x - 4, abs.y - 2, abs.w + 8, abs.h + 4)
}
