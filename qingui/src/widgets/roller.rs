use alloc::string::String;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub const ROW_H: i32 = 16;
pub const ROLL_DUR: u64 = 150;

/// 滚动位置：从 from 平滑过渡到 selected
fn sel_f(selected: usize, sel_from: Option<(f32, u64)>, now: u64) -> f32 {
    match sel_from {
        Some((from, start)) => {
            let t = (now.saturating_sub(start) as f32 / ROLL_DUR as f32).clamp(0.0, 1.0);
            from * (1.0 - t) + selected as f32 * t
        }
        None => selected as f32,
    }
}

pub(crate) fn fx_active(sel_from: Option<(f32, u64)>, now: u64) -> bool {
    sel_from.is_some_and(|(_, s)| now.saturating_sub(s) < ROLL_DUR)
}

pub(crate) fn draw(items: &[String], selected: usize, sel_from: Option<(f32, u64)>, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let ap = ctx.ap(255);
    let cy = abs.y + abs.h / 2;
    // 中心选中行高亮（滚轮在行下滑动）
    d.fill_rounded(Rect::new(abs.x, cy - ROW_H / 2, abs.w, ROW_H), 3, Color::rgb(50, 70, 120), ap, lclip);
    let sf = sel_f(selected, sel_from, ctx.now);
    for (i, item) in items.iter().enumerate() {
        let ry = cy + ((i as f32 - sf) * ROW_H as f32) as i32 - 4;
        if ry + 8 < lclip.y || ry - 4 > lclip.bottom() {
            continue;
        }
        let (tw, _) = crate::font::text_size(item);
        d.draw_text_opa(
            Point { x: abs.x + (abs.w - tw) / 2, y: ry },
            item,
            ctx.resolved.text_color,
            ap,
            lclip,
        );
    }
}

/// 选中第 idx 项（首尾停止，不循环），带滚动动画。
/// 动画中途连按时从当前视觉位置续接（不跳变）。
pub(crate) fn select(items: &[String], selected: &mut usize, sel_from: &mut Option<(f32, u64)>, idx: usize, now: u64) {
    if items.is_empty() {
        return;
    }
    let nidx = idx.min(items.len() - 1);
    if nidx != *selected {
        let cur = sel_f(*selected, *sel_from, now);
        *sel_from = Some((cur, now));
        *selected = nidx;
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, items: &[&str]) -> ObjRef {
    let rows = items.len().min(3).max(1) as i32;
    let r = ui.insert_node(
        parent,
        Rect::new(0, 0, 80, rows * ROW_H + 8),
        WidgetKind::Roller { items: items.iter().map(|s| (*s).into()).collect(), selected: 0, sel_from: None },
    );
    let mut s = crate::style::Style::default();
    s.bg_color = Some(Color::rgb(34, 34, 44));
    s.radius = Some(4);
    s.text_color = Some(Color::WHITE);
    ui.set_style(r, s.clone());
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(1);
    ui.set_style_focused(r, s);
    r
}
