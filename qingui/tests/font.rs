use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_10X20};
use qingui::draw::DrawBuf;
use qingui::font::{advance, line_height, text_size};
use qingui::{Color, Point, Rect, Ui};

#[test]
fn text_size_monospace_metrics() {
    let (w, h) = text_size(&FONT_6X10, "abc");
    assert_eq!(h, 10);
    assert_eq!(w, 3 * advance(&FONT_6X10) - FONT_6X10.character_spacing as i32); // last char has no trailing spacing
    let (_, h2) = text_size(&FONT_6X10, "a\nb");
    assert_eq!(h2, 20);
    assert_eq!(line_height(&FONT_10X20), 20);
}

#[test]
fn draw_text_origin_is_top_left() {
    // 'A' in FONT_6X10 should have On pixels in the top-left region of its glyph box, and must not exceed the character_size box
    let mut buf = [Color::BLACK; 64];
    {
        let mut d = DrawBuf { pixels: &mut buf, area: Rect::new(0, 0, 8, 8), stride: 8 };
        d.draw_text(Point { x: 0, y: 0 }, &FONT_6X10, "A", Color::WHITE, Rect::new(0, 0, 8, 8));
    }
    let on = |x: usize, y: usize| buf[y * 8 + x] == Color::WHITE;
    assert!(buf.iter().any(|&p| p == Color::WHITE)); // something was drawn
    // The spacing area to the right of the glyph box has no pixels (the buffer is only 8 rows, so the area below the line height is not asserted)
    for x in 6..8 {
        for y in 0..8 {
            assert!(!on(x, y), "spacing 区不应有像素");
        }
    }
}

#[test]
fn default_font_and_override() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let l = qingui::widgets::label::LabelBuilder::new("hi").build(&mut ui, s);
    // Note: embedded-graphics fonts are const rather than static, so taking their address may yield different promoted instances,
    // so ptr::eq/PartialEq (including glyph_mapping pointer comparison) is unreliable; assert on the public metrics instead
    let metrics = |f: &embedded_graphics::mono_font::MonoFont| (f.character_size, f.character_spacing, f.baseline);
    assert_eq!(metrics(ui.resolved_style(l).font), metrics(&FONT_6X10));
    // Global default is changeable
    ui.set_default_font(&FONT_10X20);
    assert_eq!(metrics(ui.resolved_style(l).font), metrics(&FONT_10X20));
    // style.font overrides the global default
    let mut st = qingui::style::Style::default();
    st.font = Some(&FONT_6X10);
    ui.set_style(l, st);
    assert_eq!(metrics(ui.resolved_style(l).font), metrics(&FONT_6X10));
}

#[test]
fn content_size_follows_default_and_style_font() {
    use qingui::widgets::label::UiTextExt;
    // The global default font affects content size (set before build)
    let mut ui = Ui::new(64, 64, 16);
    ui.set_default_font(&FONT_10X20);
    let s = ui.screen();
    let l = qingui::widgets::label::LabelBuilder::new("hi").build(&mut ui, s);
    assert_eq!(ui.rect(l).h, 20); // FONT_10X20 line height 20, not FONT_6X10's 10
    // set_text re-measures and follows the global default too
    ui.set_text(l, "hi!");
    assert_eq!(ui.rect(l).h, 20);
    // style.font overrides the global default (measured at build time)
    let mut st = qingui::style::Style::default();
    st.font = Some(&FONT_6X10);
    let l2 = qingui::widgets::label::LabelBuilder::new("hi").style(st).build(&mut ui, s);
    assert_eq!(ui.rect(l2).h, 10);
    // At set_text time, the node's base style.font also takes precedence
    ui.set_text(l2, "hi!");
    assert_eq!(ui.rect(l2).h, 10);
    // Button default size also follows the default font (text height 20 + 12)
    let b = qingui::widgets::button::ButtonBuilder::new("OK").build(&mut ui, s);
    assert_eq!(ui.rect(b).h, 20 + 12);
}
