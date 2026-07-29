use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

/// 表盘起始角与扫掠范围（LVGL 风格：底部留缺口）
pub const START_DEG: i32 = 135;
pub const SWEEP_DEG: i32 = 270;
pub const TRACK_W: i32 = 4;

pub(crate) fn draw(min: i32, max: i32, value: i32, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 3;
    if r <= 0 {
        return;
    }
    let ap = |b: u8| ctx.ap(b);
    // 背景弧（全轨）
    d.draw_arc(c, r, TRACK_W, START_DEG, START_DEG + SWEEP_DEG, Color::rgb(70, 70, 80), ap(255), clip);
    // 指示弧（编辑态变黄）
    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
    let ind_end = START_DEG + (SWEEP_DEG as f32 * frac) as i32;
    if ind_end > START_DEG {
        let ic = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::rgb(80, 140, 255) };
        d.draw_arc(c, r, TRACK_W, START_DEG, ind_end, ic, ap(255), clip);
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, min: i32, max: i32) -> ObjRef {
    let r = ui.insert_node(parent, Rect::new(0, 0, 60, 60), WidgetKind::Arc { min, max, value: min });
    let mut s = crate::style::Style::default();
    s.bg_opa = Some(0);
    ui.set_style(r, s);
    r
}
