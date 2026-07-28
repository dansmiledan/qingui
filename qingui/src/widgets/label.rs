use crate::draw::DrawBuf;
use crate::geometry::{Point, Rect};
use super::WidgetCtx;

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    d.draw_text_opa(
        Point { x: ctx.abs.x, y: ctx.abs.y },
        text,
        ctx.resolved.text_color,
        ctx.ap(255),
        clip,
    );
}
