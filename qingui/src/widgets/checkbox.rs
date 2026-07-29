
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

const BOX: i32 = 12;

pub(crate) fn draw(text: &str, checked: bool, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let ap = |b: u8| ctx.ap(b);
    let by = abs.y + (abs.h - BOX) / 2;
    let brect = Rect::new(abs.x, by, BOX, BOX);
    // 方框
    d.draw_border(brect, 1, 2, Color::rgb(150, 150, 160), ap(255), clip);
    if checked {
        // 勾：两条线
        let p1 = Point { x: abs.x + 2, y: by + 6 };
        let p2 = Point { x: abs.x + 5, y: by + 9 };
        let p3 = Point { x: abs.x + 10, y: by + 3 };
        d.draw_line(p1, p2, 2, Color::rgb(80, 140, 255), ap(255), clip);
        d.draw_line(p2, p3, 2, Color::rgb(80, 140, 255), ap(255), clip);
    }
    d.draw_text_opa(
        Point { x: abs.x + BOX + 6, y: abs.y + (abs.h - 8) / 2 },
        text,
        ctx.resolved.text_color,
        ap(255),
        clip,
    );
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, text: &str) -> ObjRef {
    let (tw, _) = crate::font::text_size(text);
    let r = ui.insert_node(
        parent,
        Rect::new(0, 0, BOX + 6 + tw, 16),
        WidgetKind::Checkbox { text: text.into(), checked: false },
    );
    let mut s = crate::style::Style::default();
    s.bg_opa = Some(0);
    s.text_color = Some(Color::WHITE);
    ui.set_style(r, s.clone());
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(1);
    ui.set_style_focused(r, s);
    r
}
