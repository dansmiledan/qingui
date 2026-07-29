use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 2;
    if r <= 0 {
        return;
    }
    // 旋转起点连续 + 三角波扫长（平滑伸缩，无跳变）
    let start = (ctx.now / 5) as i32 % 360;
    let phase = (ctx.now / 7) as i32 % 300;
    let tri = if phase < 150 { phase } else { 300 - phase };
    let sweep = 60 + tri;
    d.draw_arc(c, r, 3, start, start + sweep, Color::rgb(80, 140, 255), ctx.ap(255), clip);
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef) -> ObjRef {
    let r = ui.insert_node(parent, Rect::new(0, 0, 32, 32), WidgetKind::Spinner);
    let mut s = crate::style::Style::default();
    s.bg_opa = Some(0);
    ui.set_style(r, s);
    r
}
