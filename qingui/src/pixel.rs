//! Framebuffer pixel formats: the bridge between qingui's internal RGB888 `Color`
//! and the device-native pixel type stored in the framebuffer.

use crate::geometry::Color;
use embedded_graphics::pixelcolor::raw::{RawData, RawU16};
use embedded_graphics::pixelcolor::{Bgr555, Bgr565, Bgr666, Bgr888, PixelColor, Rgb555, Rgb565, Rgb666, Rgb888};

/// A framebuffer pixel format: convertible to/from the internal RGB888 `Color`.
///
/// Implemented for qingui's own `Color` (identity, the default) and for the
/// embedded-graphics RGB/BGR color types, so the framebuffer can directly use
/// the display's native format (e.g. `Rgb565`).
pub trait PixelFormat: PixelColor + Copy + PartialEq + Default {
    /// Converts a framebuffer pixel to the internal RGB888 `Color`.
    fn to_color(self) -> Color;
    /// Converts an internal RGB888 `Color` to a framebuffer pixel (quantizes).
    fn from_color(c: Color) -> Self;
}

impl PixelFormat for Color {
    fn to_color(self) -> Color { self }
    fn from_color(c: Color) -> Self { c }
}

/// Implements `PixelFormat` for an e-g RGB/BGR color type via its `RgbColor`
/// constructor/accessors (8-bit channels in, quantized storage out).
macro_rules! impl_pixel_format_rgb {
    ($($t:ty),* $(,)?) => {$(
        impl PixelFormat for $t {
            fn to_color(self) -> Color {
                use embedded_graphics::pixelcolor::RgbColor;
                Color::rgb(self.r(), self.g(), self.b())
            }
            fn from_color(c: Color) -> Self {
                <$t>::new(c.r, c.g, c.b)
            }
        }
    )*};
}

impl_pixel_format_rgb!(Rgb888, Rgb555, Rgb666, Bgr888, Bgr565, Bgr555, Bgr666);

// Rgb565 is implemented via raw storage so it stays bit-consistent with
// `Color::to_rgb565`/`from_rgb565`, which `Canvas::blit565` relies on.
impl PixelFormat for Rgb565 {
    fn to_color(self) -> Color {
        Color::from_rgb565(RawU16::from(self).into_inner())
    }
    fn from_color(c: Color) -> Self {
        Rgb565::from(RawU16::new(c.to_rgb565()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::pixelcolor::{Rgb565, Rgb888, RgbColor};

    #[test]
    fn color_identity() {
        let c = Color::rgb(80, 140, 255);
        assert_eq!(PixelFormat::to_color(c), c);
        assert_eq!(<Color as PixelFormat>::from_color(c), c);
    }

    #[test]
    fn rgb888_lossless_roundtrip() {
        let c = Color::rgb(1, 128, 255);
        assert_eq!(Rgb888::from_color(c).to_color(), c);
    }

    #[test]
    fn rgb565_matches_color_helpers() {
        for &c in &[Color::BLACK, Color::WHITE, Color::rgb(80, 140, 255), Color::rgb(1, 2, 3), Color::rgb(255, 128, 0)] {
            let px = Rgb565::from_color(c);
            assert_eq!(RawU16::from(px).into_inner(), c.to_rgb565(), "from_color mismatch for {c:?}");
            assert_eq!(px.to_color(), Color::from_rgb565(c.to_rgb565()), "to_color mismatch for {c:?}");
        }
    }

    #[test]
    fn rgb565_quantizes() {
        assert_eq!(Rgb565::from_color(Color::RED), Rgb565::RED);
        assert_eq!(Rgb565::from_color(Color::WHITE), Rgb565::WHITE);
        assert_eq!(Rgb565::from_color(Color::BLACK), Rgb565::BLACK);
    }

    #[test]
    fn color_is_pixel_color() {
        // Compile-time proof that Color: PixelColor, usable as the default framebuffer format.
        fn assert_pc<T: PixelColor>() {}
        assert_pc::<Color>();
    }
}
