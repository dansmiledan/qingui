use alloc::string::String;

use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use super::WidgetCtx;

pub const ROW_H: i32 = 16;

pub(crate) fn draw(items: &[String], selected: usize, scroll: i32, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    for (i, item) in items.iter().enumerate() {
        let ry = abs.y + i as i32 * ROW_H - scroll;
        let row = Rect::new(abs.x, ry, abs.w, ROW_H);
        if !row.intersects(&lclip) {
            continue;
        }
        if i == selected {
            d.fill_rect(row, Color::rgb(50, 70, 120), ctx.ap(255), lclip);
        }
        d.draw_text_opa(
            Point { x: abs.x + 4, y: ry + 4 },
            item,
            ctx.resolved.text_color,
            ctx.ap(255),
            lclip,
        );
    }
}
