use alloc::string::String;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    d.draw_text_opa(
        Point { x: ctx.abs.x, y: ctx.abs.y },
        text,
        ctx.resolved.text_color,
        ctx.ap(255),
        clip,
    );
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, text: &str) -> ObjRef {
    let (w, h) = crate::font::text_size(text);
    let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Label { text: text.into() });
    ui.set_style(r, crate::style::theme_label());
    r
}

pub(crate) fn set_text(ui: &mut Ui, obj: ObjRef, text: &str) {
    ui.invalidate_obj(obj);
    let (w, h) = crate::font::text_size(text);
    if let Some(n) = ui.arena.get_mut(obj) {
        if let WidgetKind::Label { text: t } = &mut n.kind {
            *t = text.into();
            n.rect.w = w;
            n.rect.h = h;
        }
    }
    ui.invalidate_obj(obj);
    ui.layout_dirty = true;
}

pub(crate) fn text(ui: &Ui, obj: ObjRef) -> String {
    if let Some(n) = ui.arena.get(obj) {
        if let WidgetKind::Label { text } = &n.kind {
            return text.clone();
        }
    }
    String::new()
}
