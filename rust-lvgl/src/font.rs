use font8x8::{UnicodeFonts, BASIC_FONTS};

pub const GLYPH_W: i32 = 8;
pub const GLYPH_H: i32 = 8;
pub const LINE_H: i32 = GLYPH_H;

/// 取 8x8 字模（每行 1 字节，bit0 = 最左像素）。非 ASCII 回落 '?'。
pub fn glyph(ch: char) -> [u8; 8] {
    BASIC_FONTS
        .get(ch)
        .or_else(|| BASIC_FONTS.get('?'))
        .unwrap_or([0; 8])
}

/// 返回 (宽, 高)。支持 '\n' 换行；空串为 (0, LINE_H)。
pub fn text_size(s: &str) -> (i32, i32) {
    let mut max_w = 0i32;
    let mut lines = 0i32;
    for line in s.split('\n') {
        max_w = max_w.max(line.chars().count() as i32 * GLYPH_W);
        lines += 1;
    }
    (max_w, lines * LINE_H)
}
