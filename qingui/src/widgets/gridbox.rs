use crate::arena::ObjRef;
use embedded_graphics::pixelcolor::PixelColor;
use crate::geometry::Rect;
use crate::layout::Grid;
use crate::ui::Ui;
use super::Widget;

/// Grid container layout widget: arranges children per `grid` each layout pass.
pub struct GridLayout {
    pub grid: Grid,
}

impl<C: PixelColor> Widget<C> for GridLayout {
    fn layout(&mut self, ui: &mut Ui<C>, obj: ObjRef, content: Rect) {
        crate::layout::layout_grid(ui, obj, &self.grid, content);
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
