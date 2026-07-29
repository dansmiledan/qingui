use alloc::string::String;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(items: &[String], selected: usize, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let ap = ctx.ap(255);
    let text = items.get(selected).map(|s| s.as_str()).unwrap_or("");
    d.draw_text_opa(
        Point { x: abs.x + 6, y: abs.y + (abs.h - 8) / 2 },
        text,
        ctx.resolved.text_color,
        ap,
        lclip,
    );
    // 下拉箭头（小三角）
    let ax = abs.right() - 10;
    let ay = abs.y + abs.h / 2;
    d.draw_line(Point { x: ax - 3, y: ay - 2 }, Point { x: ax, y: ay + 2 }, 1, ctx.resolved.text_color, ap, lclip);
    d.draw_line(Point { x: ax, y: ay + 2 }, Point { x: ax + 3, y: ay - 2 }, 1, ctx.resolved.text_color, ap, lclip);
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, items: &[&str]) -> ObjRef {
    let r = ui.insert_node(
        parent,
        Rect::new(0, 0, 100, 20),
        WidgetKind::Dropdown { items: items.iter().map(|s| (*s).into()).collect(), selected: 0 },
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
