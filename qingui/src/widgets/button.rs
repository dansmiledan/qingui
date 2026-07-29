use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let (tw, th) = crate::font::text_size(text);
    let p = Point {
        x: ctx.abs.x + (ctx.abs.w - tw) / 2,
        y: ctx.abs.y + (ctx.abs.h - th) / 2,
    };
    d.draw_text_opa(p, text, ctx.resolved.text_color, ctx.ap(255), clip);
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, text: &str) -> ObjRef {
    let (tw, th) = crate::font::text_size(text);
    let r = ui.insert_node(parent, Rect::new(0, 0, tw + 24, th + 12),
        WidgetKind::Button { text: text.into() });
    ui.set_style(r, crate::style::theme_button());
    ui.set_style_pressed(r, crate::style::theme_button_pressed());
    ui.set_style_focused(r, crate::style::theme_button_focused());
    if let Some(n) = ui.arena.get_mut(r) {
        n.flags |= crate::node::Flag::CLICKABLE;
    }
    r
}
