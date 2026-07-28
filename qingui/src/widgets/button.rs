use crate::draw::DrawBuf;
use crate::geometry::{Point, Rect};
use super::WidgetCtx;

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let (tw, th) = crate::font::text_size(text);
    let p = Point {
        x: ctx.abs.x + (ctx.abs.w - tw) / 2,
        y: ctx.abs.y + (ctx.abs.h - th) / 2,
    };
    d.draw_text_opa(p, text, ctx.resolved.text_color, ctx.ap(255), clip);
}
