use crate::arena::ObjRef;
use crate::layout::Grid;
use crate::ui::Ui;
use super::Widget;

/// Grid container layout widget: arranges children per `grid` each layout pass.
pub struct GridLayout {
    pub grid: Grid,
}

impl Widget for GridLayout {
    fn layout(&mut self, ui: &mut Ui, obj: ObjRef) {
        crate::layout::layout_grid(ui, obj, &self.grid);
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
