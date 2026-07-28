use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(on: bool, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let tc = if on { Color::rgb(60, 180, 90) } else { Color::rgb(90, 90, 90) };
    d.fill_rounded(abs, abs.h / 2, tc, ctx.ap(255), clip);
    let k = abs.h - 4;
    let kx = if on { abs.right() - k - 2 } else { abs.x + 2 };
    d.fill_rounded(Rect::new(kx, abs.y + 2, k, k), k / 2, Color::WHITE, ctx.ap(255), clip);
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef) -> ObjRef {
    let r = ui.insert_node(parent, Rect::new(0, 0, 40, 20), WidgetKind::Switch { on: false });
    ui.set_style(r, crate::style::theme_switch());
    ui.set_style_focused(r, crate::style::theme_switch_focused());
    r
}
