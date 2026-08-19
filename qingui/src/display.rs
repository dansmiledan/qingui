use crate::geometry::Rect;

/// Callback used to push rendered pixels to the display driver.
///
/// `C` is the framebuffer pixel format (default: RGB888).
pub trait Flush<C = embedded_graphics::pixelcolor::Rgb888> {
    /// `area` is a rectangle in absolute screen coordinates; `pixels` holds `area.w * area.h`
    /// pixels (row-major) in the framebuffer pixel format `C`.
    fn flush(&mut self, area: Rect, pixels: &[C]);
}
