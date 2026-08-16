//! Framebuffer pixel formats: the bridge between qingui's internal RGB888 `Color`
//! and the device-native pixel type stored in the framebuffer.

use crate::geometry::Color;
use embedded_graphics::pixelcolor::{Bgr555, Bgr565, Bgr666, Bgr888, PixelColor, Rgb555, Rgb565, Rgb666, Rgb888};

/// A framebuffer pixel format: convertible to/from the internal RGB888 `Color`.
///
/// Implemented for `Color` (which IS `Rgb888`, the default) and for the other
/// embedded-graphics RGB/BGR color types, so the framebuffer can directly use
/// the display's native format (e.g. `Rgb565`). Conversions delegate to
/// embedded-graphics' own `From` impls (rounding quantization).
pub trait PixelFormat: PixelColor + Copy + PartialEq + Default + Into<Color> + From<Color> {
    /// Converts a framebuffer pixel to the internal RGB888 `Color`.
    fn to_color(self) -> Color {
        self.into()
    }
    /// Converts an internal RGB888 `Color` to a framebuffer pixel (quantizes).
    fn from_color(c: Color) -> Self {
        c.into()
    }
}

macro_rules! impl_pixel_format {
    ($($t:ty),* $(,)?) => {$( impl PixelFormat for $t {} )*};
}

impl_pixel_format!(Rgb888, Rgb565, Rgb555, Rgb666, Bgr888, Bgr565, Bgr555, Bgr666);

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::raw::{RawData, RawU16};
    use embedded_graphics::pixelcolor::{Bgr555, Bgr565, Bgr666, Bgr888, Rgb555, Rgb565, Rgb666, Rgb888, RgbColor};

    #[test]
    fn color_identity() {
        let c = Color::new(80, 140, 255);
        assert_eq!(PixelFormat::to_color(c), c);
        assert_eq!(<Color as PixelFormat>::from_color(c), c);
    }

    #[test]
    fn rgb888_lossless_roundtrip() {
        let c = Color::new(1, 128, 255);
        assert_eq!(Rgb888::from_color(c).to_color(), c);
    }

    #[test]
    fn rgb565_full_scale_values() {
        assert_eq!(Rgb565::from_color(Color::WHITE), Rgb565::WHITE);
        assert_eq!(Rgb565::from_color(Color::BLACK), Rgb565::BLACK);
        assert_eq!(Rgb565::from_color(Color::RED), Rgb565::RED);
    }

    #[test]
    fn rgb565_quantization_rounds_like_eg() {
        // e-g converts 8->5 bits with rounding (not truncation): 250*31/255 rounds to 30.
        let raw = RawU16::from(Rgb565::from_color(Color::new(250, 0, 0))).into_inner();
        assert_eq!(raw, 30 << 11);
    }

    #[test]
    fn rgb565_decode_expands_via_eg_rounding() {
        // 565 -> 888 expansion follows e-g's rounding (`convert_channel`), which
        // agrees with the classic bit-replication values on some inputs only.
        assert_eq!(Rgb565::from(RawU16::new(0xF800)).to_color(), Color::new(255, 0, 0));
        // r5 = 16 -> (16<<3)|(16>>2) = 132; rounding agrees here.
        assert_eq!(Rgb565::from(RawU16::new(16 << 11)).to_color(), Color::new(132, 0, 0));
        // r5 = 3 -> 25 via rounding; the old bit-replication code produced 24.
        assert_eq!(Rgb565::from(RawU16::new(3 << 11)).to_color(), Color::new(25, 0, 0));
    }

    #[test]
    fn all_formats_roundtrip_midrange() {
        // Regression: the old hand-written macro bodies passed 8-bit values through
        // native-depth new()/r() accessors, corrupting every format except Rgb888 and
        // the hand-written Rgb565. A mid-range color must survive a quantize
        // round-trip within one target-depth LSB (8-bit space) for every format.
        fn check<T: PixelFormat + core::fmt::Debug>(c: Color, tol: i16) {
            let back = T::from_color(c).to_color();
            assert!((back.r() as i16 - c.r() as i16).abs() <= tol, "r drift: {c:?} -> {back:?}");
            assert!((back.g() as i16 - c.g() as i16).abs() <= tol, "g drift: {c:?} -> {back:?}");
            assert!((back.b() as i16 - c.b() as i16).abs() <= tol, "b drift: {c:?} -> {back:?}");
        }
        let c = Color::new(80, 140, 255);
        check::<Rgb888>(c, 0);
        check::<Bgr888>(c, 0);
        check::<Rgb666>(c, 4); // 6-bit: 1 LSB = 4 in 8-bit space
        check::<Bgr666>(c, 4);
        check::<Rgb565>(c, 8); // 5/6-bit
        check::<Bgr565>(c, 8);
        check::<Rgb555>(c, 8);
        check::<Bgr555>(c, 8);
    }

    #[test]
    fn color_is_pixel_color() {
        // Compile-time proof that Color: PixelColor, usable as the default framebuffer format.
        fn assert_pc<T: PixelColor>() {}
        assert_pc::<Color>();
    }
}
