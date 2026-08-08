use crate::arena::ObjRef;
use crate::layout::Flex;
use crate::ui::Ui;
use super::Widget;

/// Flex container layout widget: arranges children per `flex` each layout pass.
pub struct FlexLayout {
    pub flex: Flex,
}

impl Widget for FlexLayout {
    fn layout(&mut self, ui: &mut Ui, obj: ObjRef) {
        crate::layout::layout_flex(ui, obj, &self.flex);
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
