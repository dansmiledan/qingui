use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(min: i32, max: i32, value: i32, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
    let iw = (abs.w as f32 * frac) as i32;
    if iw > 0 {
        d.fill_rounded(Rect::new(abs.x, abs.y, iw, abs.h), ctx.resolved.radius, Color::rgb(80, 140, 255), ctx.ap(255), clip);
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, min: i32, max: i32) -> ObjRef {
    let r = ui.insert_node(parent, Rect::new(0, 0, 100, 8),
        WidgetKind::Bar { min, max, value: min });
    ui.set_style(r, crate::style::theme_bar());
    r
}
