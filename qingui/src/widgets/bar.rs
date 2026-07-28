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
}
