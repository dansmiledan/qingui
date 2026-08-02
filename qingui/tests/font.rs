use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_10X20};
use qingui::draw::DrawBuf;
use qingui::font::{advance, line_height, text_size};
use qingui::{Color, Point, Rect, Ui};

#[test]
fn text_size_monospace_metrics() {
    let (w, h) = text_size(&FONT_6X10, "abc");
    assert_eq!(h, 10);
    assert_eq!(w, 3 * advance(&FONT_6X10) - FONT_6X10.character_spacing as i32); // 末字不计字距
    let (_, h2) = text_size(&FONT_6X10, "a\nb");
    assert_eq!(h2, 20);
    assert_eq!(line_height(&FONT_10X20), 20);
}

#[test]
fn draw_text_origin_is_top_left() {
    // 'A' 在 FONT_6X10 的字形盒左上角区域应有 On 像素，且不超过 character_size 盒
    let mut buf = [Color::BLACK; 64];
    {
        let mut d = DrawBuf { pixels: &mut buf, area: Rect::new(0, 0, 8, 8), stride: 8 };
        d.draw_text(Point { x: 0, y: 0 }, &FONT_6X10, "A", Color::WHITE, Rect::new(0, 0, 8, 8));
    }
    let on = |x: usize, y: usize| buf[y * 8 + x] == Color::WHITE;
    assert!(buf.iter().any(|&p| p == Color::WHITE)); // 画出了东西
    // 字形盒右侧 spacing 区无像素（缓冲区仅 8 行，行高以下区域未覆盖不断言）
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
    // 注：e-g 字体是 const 而非 static，每次取址可能是不同晋升实例，
    // ptr::eq/PartialEq（含 glyph_mapping 指针比较）不可靠，按公开 metrics 断言
    let metrics = |f: &embedded_graphics::mono_font::MonoFont| (f.character_size, f.character_spacing, f.baseline);
    assert_eq!(metrics(ui.resolved_style(l).font), metrics(&FONT_6X10));
    // 全局默认可换
    ui.set_default_font(&FONT_10X20);
    assert_eq!(metrics(ui.resolved_style(l).font), metrics(&FONT_10X20));
    // style.font 覆盖全局默认
    let mut st = qingui::style::Style::default();
    st.font = Some(&FONT_6X10);
    ui.set_style(l, st);
    assert_eq!(metrics(ui.resolved_style(l).font), metrics(&FONT_6X10));
}

#[test]
fn content_size_follows_default_and_style_font() {
    use qingui::widgets::label::UiTextExt;
    // 全局默认字体影响内容尺寸（build 前设置）
    let mut ui = Ui::new(64, 64, 16);
    ui.set_default_font(&FONT_10X20);
    let s = ui.screen();
    let l = qingui::widgets::label::LabelBuilder::new("hi").build(&mut ui, s);
    assert_eq!(ui.rect(l).h, 20); // FONT_10X20 行高 20，而非 FONT_6X10 的 10
    // set_text 重测同样跟随全局默认
    ui.set_text(l, "hi!");
    assert_eq!(ui.rect(l).h, 20);
    // style.font 覆盖优先于全局默认（build 时测量）
    let mut st = qingui::style::Style::default();
    st.font = Some(&FONT_6X10);
    let l2 = qingui::widgets::label::LabelBuilder::new("hi").style(st).build(&mut ui, s);
    assert_eq!(ui.rect(l2).h, 10);
    // set_text 时节点 base style.font 同样优先
    ui.set_text(l2, "hi!");
    assert_eq!(ui.rect(l2).h, 10);
    // button 默认尺寸同样跟随默认字体（文本高 20 + 12）
    let b = qingui::widgets::button::ButtonBuilder::new("OK").build(&mut ui, s);
    assert_eq!(ui.rect(b).h, 20 + 12);
}
