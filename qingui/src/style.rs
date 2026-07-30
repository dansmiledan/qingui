use crate::geometry::Color;

/// 布局描述。
#[derive(Clone, PartialEq, Debug)]
pub enum Layout {
    None,
    Flex(crate::layout::Flex),
    Grid(crate::layout::Grid),
}

impl Default for Layout {
    fn default() -> Self {
        Layout::None
    }
}

/// 扁平样式：Option 字段，None 表示"不覆盖"。
/// 可用结构体字面量，也可用 builder 链式构造：`Style::new().bg(RED).radius(4).pads(8)`
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
    /// 宽/高尺寸策略（None = 内容尺寸）
    pub sizing_w: Option<crate::layout::Sizing>,
    pub sizing_h: Option<crate::layout::Sizing>,
    /// 宽高比（千分比：1000 = 1:1，1778 ≈ 16:9）
    pub aspect_ratio: Option<u32>,
    /// 布局过渡：(时长 ms, 缓动)。布局改变位置/尺寸时自动动画过渡
    pub transition: Option<(u32, crate::anim::Easing)>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn bg(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }
    pub fn bg_opa(mut self, opa: u8) -> Self {
        self.bg_opa = Some(opa);
        self
    }
    pub fn border(mut self, color: Color, width: i32) -> Self {
        self.border_color = Some(color);
        self.border_width = Some(width);
        self
    }
    pub fn radius(mut self, radius: i32) -> Self {
        self.radius = Some(radius);
        self
    }
    /// 四边统一 padding
    pub fn pads(mut self, v: i32) -> Self {
        self.pad_left = Some(v);
        self.pad_right = Some(v);
        self.pad_top = Some(v);
        self.pad_bottom = Some(v);
        self
    }
    /// 分别设置 padding：(左, 右, 上, 下)
    pub fn pad(mut self, left: i32, right: i32, top: i32, bottom: i32) -> Self {
        self.pad_left = Some(left);
        self.pad_right = Some(right);
        self.pad_top = Some(top);
        self.pad_bottom = Some(bottom);
        self
    }
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }
    pub fn sizing(mut self, w: crate::layout::Sizing, h: crate::layout::Sizing) -> Self {
        self.sizing_w = Some(w);
        self.sizing_h = Some(h);
        self
    }
    pub fn aspect(mut self, ratio: u32) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }
    pub fn transition(mut self, duration_ms: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((duration_ms, easing));
        self
    }

    /// other 的 Some 字段覆盖 self 的同名字段（样式组合）
    pub fn merge(mut self, other: Style) -> Style {
        if other.bg_color.is_some() { self.bg_color = other.bg_color; }
        if other.bg_opa.is_some() { self.bg_opa = other.bg_opa; }
        if other.border_color.is_some() { self.border_color = other.border_color; }
        if other.border_width.is_some() { self.border_width = other.border_width; }
        if other.radius.is_some() { self.radius = other.radius; }
        if other.pad_left.is_some() { self.pad_left = other.pad_left; }
        if other.pad_right.is_some() { self.pad_right = other.pad_right; }
        if other.pad_top.is_some() { self.pad_top = other.pad_top; }
        if other.pad_bottom.is_some() { self.pad_bottom = other.pad_bottom; }
        if other.text_color.is_some() { self.text_color = other.text_color; }
        if other.layout.is_some() { self.layout = other.layout; }
        if other.sizing_w.is_some() { self.sizing_w = other.sizing_w; }
        if other.sizing_h.is_some() { self.sizing_h = other.sizing_h; }
        if other.aspect_ratio.is_some() { self.aspect_ratio = other.aspect_ratio; }
        if other.transition.is_some() { self.transition = other.transition; }
        self
    }
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
    pub sizing_w: Option<crate::layout::Sizing>,
    pub sizing_h: Option<crate::layout::Sizing>,
    pub aspect_ratio: Option<u32>,
    pub transition: Option<(u32, crate::anim::Easing)>,
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
            sizing_w: None,
            sizing_h: None,
            aspect_ratio: None,
            transition: None,
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
        sizing_w: overlay.and_then(|s| s.sizing_w).or(base.sizing_w),
        sizing_h: overlay.and_then(|s| s.sizing_h).or(base.sizing_h),
        aspect_ratio: overlay.and_then(|s| s.aspect_ratio).or(base.aspect_ratio),
        transition: overlay.and_then(|s| s.transition).or(base.transition),
    }
}

/// 通用基础样式：所有控件默认样式的基础。
/// 注意：只用于组合"基础样式"；状态覆盖样式（pressed/focused）保持稀疏，不要用它组合。
pub fn theme_base() -> Style {
    Style::new().text_color(Color::WHITE).bg_opa(255).radius(4)
}

pub fn theme_screen() -> Style {
    theme_base().bg(Color::rgb(24, 24, 32))
}

pub fn theme_obj() -> Style {
    theme_base().bg(Color::rgb(40, 40, 52))
}

pub fn theme_label() -> Style {
    theme_base().bg_opa(0) // 透明背景
}

pub fn theme_button() -> Style {
    theme_base()
        .bg(Color::rgb(60, 90, 160))
        .radius(6)
        .border(Color::rgb(90, 120, 200), 1)
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

pub fn theme_slider() -> Style {
    theme_base().bg(Color::rgb(70, 70, 80)).radius(6)
}

pub fn theme_slider_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(2);
    s
}

pub fn theme_switch() -> Style {
    theme_base().bg(Color::rgb(90, 90, 90)).radius(10) // 高度 20 的全圆角
}

pub fn theme_switch_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(2);
    s
}

pub fn theme_bar() -> Style {
    theme_base().bg(Color::rgb(70, 70, 80))
}

pub fn theme_list() -> Style {
    theme_base()
        .bg(Color::rgb(34, 34, 44))
        .border(Color::rgb(70, 70, 90), 1)
}

pub fn theme_list_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s
}
