use crate::geometry::Color;

/// 布局描述。Task 13 追加 Flex/Grid 变体与参数类型。
#[derive(Clone, PartialEq, Debug)]
pub enum Layout {
    None,
}

impl Default for Layout {
    fn default() -> Self {
        Layout::None
    }
}

/// 扁平样式：Option 字段，None 表示"不覆盖"。
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Style {
    pub bg_color: Option<Color>,
    pub bg_opa: Option<u8>,
    pub border_color: Option<Color>,
    pub border_width: Option<i32>,
    pub radius: Option<i32>,
    pub pad_left: Option<i32>,
    pub pad_right: Option<i32>,
    pub pad_top: Option<i32>,
    pub pad_bottom: Option<i32>,
    pub text_color: Option<Color>,
    pub layout: Option<Layout>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ResolvedStyle {
    pub bg_color: Color,
    pub bg_opa: u8,
    pub border_color: Color,
    pub border_width: i32,
    pub radius: i32,
    pub pad_left: i32,
    pub pad_right: i32,
    pub pad_top: i32,
    pub pad_bottom: i32,
    pub text_color: Color,
    pub layout: Layout,
}

impl Default for ResolvedStyle {
    fn default() -> Self {
        Self {
            bg_color: Color::BLACK,
            bg_opa: 255,
            border_color: Color::BLACK,
            border_width: 0,
            radius: 0,
            pad_left: 0,
            pad_right: 0,
            pad_top: 0,
            pad_bottom: 0,
            text_color: Color::WHITE,
            layout: Layout::None,
        }
    }
}

/// 逐字段回落：overlay -> base -> ResolvedStyle::default()
pub fn resolve(base: &Style, overlay: Option<&Style>) -> ResolvedStyle {
    let d = ResolvedStyle::default();
    let pick = |o: Option<&Style>, f: fn(&Style) -> Option<Color>| -> Option<Color> {
        o.and_then(f).or_else(|| f(base))
    };
    let pick_i = |o: Option<&Style>, f: fn(&Style) -> Option<i32>| -> Option<i32> {
        o.and_then(f).or_else(|| f(base))
    };
    let pick_u8 = |o: Option<&Style>, f: fn(&Style) -> Option<u8>| -> Option<u8> {
        o.and_then(f).or_else(|| f(base))
    };
    ResolvedStyle {
        bg_color: pick(overlay, |s| s.bg_color).unwrap_or(d.bg_color),
        bg_opa: pick_u8(overlay, |s| s.bg_opa).unwrap_or(d.bg_opa),
        border_color: pick(overlay, |s| s.border_color).unwrap_or(d.border_color),
        border_width: pick_i(overlay, |s| s.border_width).unwrap_or(d.border_width),
        radius: pick_i(overlay, |s| s.radius).unwrap_or(d.radius),
        pad_left: pick_i(overlay, |s| s.pad_left).unwrap_or(d.pad_left),
        pad_right: pick_i(overlay, |s| s.pad_right).unwrap_or(d.pad_right),
        pad_top: pick_i(overlay, |s| s.pad_top).unwrap_or(d.pad_top),
        pad_bottom: pick_i(overlay, |s| s.pad_bottom).unwrap_or(d.pad_bottom),
        text_color: pick(overlay, |s| s.text_color).unwrap_or(d.text_color),
        layout: overlay
            .and_then(|s| s.layout.clone())
            .or_else(|| base.layout.clone())
            .unwrap_or(Layout::None),
    }
}

pub fn theme_screen() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(24, 24, 32));
    s
}

pub fn theme_obj() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(40, 40, 52));
    s.radius = Some(4);
    s
}

pub fn theme_label() -> Style {
    let mut s = Style::default();
    s.text_color = Some(Color::WHITE);
    s.bg_opa = Some(0); // 透明背景
    s
}

pub fn theme_button() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(60, 90, 160));
    s.radius = Some(6);
    s.border_color = Some(Color::rgb(90, 120, 200));
    s.border_width = Some(1);
    s.text_color = Some(Color::WHITE);
    s
}

pub fn theme_button_pressed() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(40, 60, 110));
    s
}

pub fn theme_button_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(2);
    s
}
