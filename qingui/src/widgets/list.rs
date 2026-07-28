use alloc::string::String;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub const ROW_H: i32 = 16;

/// 选中第 idx 项并调整 scroll 保证可见
pub(crate) fn select(items: &[String], selected: &mut usize, scroll: &mut i32, idx: usize, vis_h: i32) {
    if items.is_empty() {
        return;
    }
    *selected = idx.min(items.len() - 1);
    let top = *selected as i32 * ROW_H;
    if top < *scroll {
        *scroll = top;
    } else if top + ROW_H > *scroll + vis_h {
        *scroll = top + ROW_H - vis_h;
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, items: &[&str]) -> ObjRef {
    let rows = items.len().min(5).max(1) as i32;
    let r = ui.insert_node(parent, Rect::new(0, 0, 120, rows * ROW_H + 8),
        WidgetKind::List { items: items.iter().map(|s| (*s).into()).collect(), selected: 0, scroll: 0 });
    ui.set_style(r, crate::style::theme_list());
    ui.set_style_focused(r, crate::style::theme_list_focused());
    r
}

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
