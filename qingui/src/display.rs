use crate::geometry::{Color, Rect};

/// Callback used to push rendered pixels to the display driver.
pub trait Flush {
    /// `area` is a rectangle in absolute screen coordinates; `pixels` holds `area.w * area.h`
    /// pixels (row-major, RGB888).
    fn flush(&mut self, area: Rect, pixels: &[Color]);
}
