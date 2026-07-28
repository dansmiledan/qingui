use alloc::boxed::Box;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::ui::Ui;
use super::WidgetKind;

/// Canvas 绘制回调：参数为 (画板, 控件绝对矩形, 裁剪矩形, 当前时间 ms)。
/// 回调内用 DrawBuf 的绘制原语自由绘制（均带 clip 与 alpha 混合）。
pub type CanvasCb = Box<dyn FnMut(&mut DrawBuf, Rect, Rect, u64)>;

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, w: i32, h: i32, cb: CanvasCb) -> ObjRef {
    let idx = ui.register_canvas_cb(cb);
    let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Canvas { cb: idx });
    // 默认透明背景：画布只承载自定义绘制
    let mut s = crate::style::Style::default();
    s.bg_opa = Some(0);
    ui.set_style(r, s);
    r
}
