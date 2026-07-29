use alloc::string::String;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub const CELL_W: i32 = 60;
pub const CELL_H: i32 = 16;

pub(crate) fn draw(cols: u8, rows: u8, cells: &[String], ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let line_c = Color::rgb(70, 70, 90);
    let ap = ctx.ap(255);
    // 网格线（底/右边收进半开区间边界内 1px）
    for c in 0..=cols as i32 {
        let x = (abs.x + c * CELL_W).min(abs.right() - 1);
        d.draw_line(Point { x, y: abs.y }, Point { x, y: abs.bottom() }, 1, line_c, ap, lclip);
    }
    for r in 0..=rows as i32 {
        let y = (abs.y + r * CELL_H).min(abs.bottom() - 1);
        d.draw_line(Point { x: abs.x, y }, Point { x: abs.right(), y }, 1, line_c, ap, lclip);
    }
    // 单元格文本
    for r in 0..rows as usize {
        for c in 0..cols as usize {
            let idx = r * cols as usize + c;
            if let Some(text) = cells.get(idx) {
                if !text.is_empty() {
                    d.draw_text_opa(
                        Point { x: abs.x + c as i32 * CELL_W + 4, y: abs.y + r as i32 * CELL_H + 4 },
                        text,
                        ctx.resolved.text_color,
                        ap,
                        lclip,
                    );
                }
            }
        }
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, cols: u8, rows: u8) -> ObjRef {
    let n = cols as usize * rows as usize;
    let r = ui.insert_node(
        parent,
        Rect::new(0, 0, cols as i32 * CELL_W, rows as i32 * CELL_H),
        WidgetKind::Table { cols, rows, cells: alloc::vec![String::new(); n] },
    );
    let mut s = crate::style::Style::default();
    s.bg_opa = Some(0);
    s.text_color = Some(Color::WHITE);
    ui.set_style(r, s);
    r
}
