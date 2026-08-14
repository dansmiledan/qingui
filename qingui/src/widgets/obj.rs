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
        // The layout config decides the widget kind at insert time (layout is a kind).
        let kind: alloc::boxed::Box<dyn super::Widget> = match common.layout.take() {
            Some(super::builder::Layout::Flex(f)) => alloc::boxed::Box::new(super::flexbox::FlexLayout { flex: f }),
            Some(super::builder::Layout::Grid(g)) => alloc::boxed::Box::new(super::gridbox::GridLayout { grid: g }),
            _ => alloc::boxed::Box::new(Manual),
        };
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), kind);
        if let Some(s) = common.style.take() {
            ui.set_style(r, s);
        }
        common.apply_tail(ui, r);
        r
    }
}

/// Manual-positioning container: hosts children; draws nothing itself.
/// Replaces the old unit `ObjState`.
pub struct Manual;

impl<C> Widget<C> for Manual {
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
