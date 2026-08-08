use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::Widget;

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
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(Manual));
        if let Some(s) = common.style.take() {
            ui.set_style(r, s);
        }
        common.apply_tail(ui, r);
        r
    }
}

/// Manual-positioning container: hosts children and (via Task 9's bridge) a layout
/// config; draws nothing itself. Replaces the old unit `ObjState`.
pub struct Manual;

impl Widget for Manual {
    // Bridge (Task 9 deletes this with the Node.layout field): read the container
    // layout config from the node and dispatch, same as the `WidgetKind` shim.
    fn layout(&mut self, ui: &mut Ui, obj: ObjRef) {
        let layout = ui.arena.get(obj).and_then(|n| match &n.layout {
            Some(crate::layout::Layout::Flex(f)) => Some(crate::layout::Layout::Flex(*f)),
            other => other.clone(),
        });
        match layout {
            Some(crate::layout::Layout::Flex(f)) => crate::layout::layout_flex(ui, obj, &f),
            Some(crate::layout::Layout::Grid(g)) => crate::layout::layout_grid(ui, obj, &g),
            _ => {}
        }
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
