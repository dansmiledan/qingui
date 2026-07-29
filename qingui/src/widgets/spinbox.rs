use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(min: i32, max: i32, value: i32, digits: u8, cursor: u8, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let _ = (min, max);
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let text = alloc::format!("{:0width$}", value, width = digits as usize);
    let ap = ctx.ap(255);
    let x0 = abs.x + (abs.w - digits as i32 * 8) / 2;
    let y = abs.y + (abs.h - 8) / 2;
    for (i, ch) in text.chars().enumerate() {
        let x = x0 + i as i32 * 8;
        if i as u8 == cursor && ctx.edited {
            // 光标位：反色高亮
            d.fill_rounded(Rect::new(x - 1, abs.y + 1, 10, abs.h - 2), 2, Color::rgb(80, 140, 255), ap, lclip);
            let g = crate::font::glyph(ch);
            for row in 0..8i32 {
                for col in 0..8i32 {
                    if g[row as usize] & (1 << col) != 0 {
                        d.fill_rect(Rect::new(x + col, y + row, 1, 1), Color::BLACK, ap, lclip);
                    }
                }
            }
        } else {
            let mut buf = [0u8; 4];
            d.draw_text_opa(Point { x, y }, ch.encode_utf8(&mut buf), ctx.resolved.text_color, ap, lclip);
        }
    }
}

/// 光标移动（±1，范围内循环）
pub(crate) fn move_cursor(digits: u8, cursor: &mut u8, dir: i32) {
    let n = digits.max(1) as i32;
    *cursor = (*cursor as i32 + dir).rem_euclid(n) as u8;
}

/// 当前光标位数字增减（按位权改变值，范围 clamp）
pub(crate) fn step_digit(min: i32, max: i32, value: &mut i32, digits: u8, cursor: u8, dir: i32) {
    let pos = (digits.max(1) - 1 - cursor.min(digits.max(1) - 1)) as u32;
    let step = 10i32.pow(pos);
    *value = (*value + dir * step).clamp(min, max);
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, min: i32, max: i32, digits: u8) -> ObjRef {
    let d = digits.max(1);
    let r = ui.insert_node(
        parent,
        Rect::new(0, 0, d as i32 * 8 + 12, 18),
        WidgetKind::Spinbox { min, max, value: min, digits: d, cursor: d - 1 },
    );
    let mut s = crate::style::Style::default();
    s.bg_color = Some(Color::rgb(40, 40, 52));
    s.radius = Some(4);
    s.text_color = Some(Color::WHITE);
    ui.set_style(r, s.clone());
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(1);
    ui.set_style_focused(r, s);
    r
}
