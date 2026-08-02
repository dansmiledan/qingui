//! 字体：embedded-graphics MonoFont 渲染与度量。
//! 经 DrawTarget 适配复用 e-g 的 Text 渲染器；Off 像素不写（背景透明）。

use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::text::renderer::TextRenderer;

/// 默认字体（最接近原 font8x8 的紧凑度）
pub const DEFAULT_FONT: &MonoFont = &embedded_graphics::mono_font::ascii::FONT_6X10;

/// 逐字水平步进（字宽 + 字距）
pub fn advance(font: &'static MonoFont) -> i32 {
    (font.character_size.width + font.character_spacing) as i32
}

/// 行高
pub fn line_height(font: &'static MonoFont) -> i32 {
    font.character_size.height as i32
}

/// 文本尺寸（支持 '\n'；逐行经 e-g measure_string 测宽，行高按 font 行高，
/// 语义与 Text 渲染器的换行严格一致）。空串为 (0, line_height)。
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
