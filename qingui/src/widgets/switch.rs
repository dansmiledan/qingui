use crate::draw::DrawBuf;
use crate::geometry::{Color, Rect};
use super::WidgetCtx;

pub(crate) fn draw(on: bool, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let tc = if on { Color::rgb(60, 180, 90) } else { Color::rgb(90, 90, 90) };
    d.fill_rounded(abs, abs.h / 2, tc, ctx.ap(255), clip);
    let k = abs.h - 4;
    let kx = if on { abs.right() - k - 2 } else { abs.x + 2 };
    d.fill_rounded(Rect::new(kx, abs.y + 2, k, k), k / 2, Color::WHITE, ctx.ap(255), clip);
}
