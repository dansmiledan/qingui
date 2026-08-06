use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetKind;

/// Builder for the generic container Obj (hosts layout and child objects).
pub type ObjBuilder = WidgetBuilder<ObjCfg>;

pub struct ObjCfg;

impl ObjCfg {
    pub fn new() -> WidgetBuilder<ObjCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ObjCfg }
    }
}

impl WidgetCfg for ObjCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((0, 0));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(ObjState));
        if let Some(s) = common.style.take() {
            ui.set_style(r, s);
        }
        common.apply_tail(ui, r);
        r
    }
}

/// Placeholder state: Obj carries no data.
pub struct ObjState;

impl super::WidgetBehavior for ObjState {
    fn draw(&self, _ctx: &super::WidgetCtx, _d: &mut DrawBuf, _clip: Rect) {}
}
