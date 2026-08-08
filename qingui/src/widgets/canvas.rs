use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::node::DrawHook;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetKind;

/// Builder for the Canvas widget.
pub type CanvasBuilder = WidgetBuilder<CanvasCfg>;

/// Canvas configuration: the draw hook.
pub struct CanvasCfg {
    cb: DrawHook,
}

impl CanvasCfg {
    /// Creates a builder with the given draw hook.
    pub fn new(cb: DrawHook) -> WidgetBuilder<CanvasCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: CanvasCfg { cb } }
    }
}

impl WidgetCfg for CanvasCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_opa = Some(0); // default transparent background: the canvas only hosts custom drawing
        s
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((32, 32));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(WidgetKind::Obj(super::obj::ObjState)));
        let mut s = common.style.take().unwrap_or_else(Self::default_style);
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0); // default transparent background: the canvas only hosts custom drawing
        }
        ui.set_style(r, s);
        ui.set_draw_hook(r, Some(self.cb));
        common.apply_tail(ui, r);
        r
    }
}
