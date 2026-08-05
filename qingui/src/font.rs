//! Fonts: embedded-graphics MonoFont rendering and measurement.
//! Reuses e-g's Text renderer through a DrawTarget adapter; Off pixels are not written
//! (transparent background).

use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::text::renderer::TextRenderer;

/// Default font (closest to the original font8x8's compactness)
pub const DEFAULT_FONT: &MonoFont = &embedded_graphics::mono_font::ascii::FONT_6X10;

/// Horizontal advance per character (glyph width + spacing)
pub fn advance(font: &'static MonoFont) -> i32 {
    (font.character_size.width + font.character_spacing) as i32
}

/// Line height
pub fn line_height(font: &'static MonoFont) -> i32 {
    font.character_size.height as i32
}

/// Font used for content-size measurement: base `style.font` → Ui default.
/// Consistent with the three-level `resolved_style` resolution (the overlay is unknown at
/// build/set time, so only `base style.font` is considered),
/// ensuring widget content size matches the font actually used for drawing.
pub(crate) fn measure_font(style: Option<&crate::style::Style>, ui: &crate::ui::Ui) -> &'static MonoFont<'static> {
    style.and_then(|s| s.font).unwrap_or_else(|| ui.default_font())
}

/// Text size (supports '\n'; each line's width is measured via e-g `measure_string`, line
/// height follows the font, semantics strictly matching the Text renderer's line breaking).
/// An empty string is `(0, line_height)`.
pub fn text_size(font: &'static MonoFont, s: &str) -> (i32, i32) {
    let style = embedded_graphics::mono_font::MonoTextStyle::new(font, embedded_graphics::pixelcolor::BinaryColor::On);
    let mut max_w = 0i32;
    let mut lines = 0i32;
    for line in s.split('\n') {
        let m = style.measure_string(line, embedded_graphics::geometry::Point::zero(), embedded_graphics::text::Baseline::Top);
        max_w = max_w.max(m.bounding_box.size.width as i32);
        lines += 1;
    }
    (max_w, lines * line_height(font))
}
