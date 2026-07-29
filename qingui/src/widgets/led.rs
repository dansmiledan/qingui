use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(color: Color, bright: u8, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 1;
    if r <= 0 {
        return;
    }
    // 亮度：从黑渐变到纯色
    let on = Color::BLACK.blend(color, bright);
    d.fill_circle(c, r, on, ctx.ap(255), clip);
    d.draw_circle(c, r, 1, Color::rgb(90, 90, 100), ctx.ap(255), clip);
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, color: Color) -> ObjRef {
    let r = ui.insert_node(parent, Rect::new(0, 0, 16, 16), WidgetKind::Led { color, bright: 255 });
    let mut s = crate::style::Style::default();
    s.bg_opa = Some(0);
    ui.set_style(r, s);
    r
}
