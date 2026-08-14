use crate::geometry::{Color, Rect};

/// Callback used to push rendered pixels to the display driver.
///
/// `C` is the framebuffer pixel format (default: RGB888 `Color`).
pub trait Flush<C = Color> {
    /// `area` is a rectangle in absolute screen coordinates; `pixels` holds `area.w * area.h`
    /// pixels (row-major) in the framebuffer pixel format `C`.
    fn flush(&mut self, area: Rect, pixels: &[C]);
}
