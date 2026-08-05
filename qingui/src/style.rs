use crate::geometry::Color;

/// Layout description for a container.
#[derive(Clone, PartialEq, Debug)]
pub enum Layout {
    /// No automatic layout.
    None,
    /// Flex layout.
    Flex(crate::layout::Flex),
    /// Grid layout.
    Grid(crate::layout::Grid),
}

impl Default for Layout {
    fn default() -> Self {
        Layout::None
    }
}

/// Flat style: `Option` fields, where `None` means "do not override".
/// Usable as a struct literal or built with a chained builder: `Style::new().bg(RED).radius(4).pads(8)`
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Style {
    /// Background color.
    pub bg_color: Option<Color>,
    /// Background opacity (0..=255).
    pub bg_opa: Option<u8>,
    /// Border color.
    pub border_color: Option<Color>,
    /// Border width in pixels.
    pub border_width: Option<i32>,
    /// Corner radius in pixels.
    pub radius: Option<i32>,
    /// Left padding.
    pub pad_left: Option<i32>,
    /// Right padding.
    pub pad_right: Option<i32>,
    /// Top padding.
    pub pad_top: Option<i32>,
    /// Bottom padding.
    pub pad_bottom: Option<i32>,
    /// Text color.
    pub text_color: Option<Color>,
    /// Container layout.
    pub layout: Option<Layout>,
    /// Width sizing strategy (None = content size).
    pub sizing_w: Option<crate::layout::Sizing>,
    /// Height sizing strategy (None = content size).
    pub sizing_h: Option<crate::layout::Sizing>,
    /// Aspect ratio (per-mille: 1000 = 1:1, 1778 ≈ 16:9).
    pub aspect_ratio: Option<u32>,
    /// Layout transition: (duration ms, easing). Position/size changes from layout are
    /// animated automatically when set.
    pub transition: Option<(u32, crate::anim::Easing)>,
    /// Text font (None = use the Ui default font).
    pub font: Option<&'static embedded_graphics::mono_font::MonoFont<'static>>,
}

impl Style {
    /// Creates an empty style with all fields unset.
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets the background color.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }
    /// Sets the background opacity (0..=255).
    pub fn bg_opa(mut self, opa: u8) -> Self {
        self.bg_opa = Some(opa);
        self
    }
    /// Sets the border color and width.
    pub fn border(mut self, color: Color, width: i32) -> Self {
        self.border_color = Some(color);
        self.border_width = Some(width);
        self
    }
    /// Sets the corner radius.
    pub fn radius(mut self, radius: i32) -> Self {
        self.radius = Some(radius);
        self
    }
    /// Uniform padding on all four sides.
    pub fn pads(mut self, v: i32) -> Self {
        self.pad_left = Some(v);
        self.pad_right = Some(v);
        self.pad_top = Some(v);
        self.pad_bottom = Some(v);
        self
    }
    /// Sets padding per side: (left, right, top, bottom).
    pub fn pad(mut self, left: i32, right: i32, top: i32, bottom: i32) -> Self {
        self.pad_left = Some(left);
        self.pad_right = Some(right);
        self.pad_top = Some(top);
        self.pad_bottom = Some(bottom);
        self
    }
    /// Sets the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
    /// Sets the container layout.
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }
    /// Sets the width and height sizing strategies.
    pub fn sizing(mut self, w: crate::layout::Sizing, h: crate::layout::Sizing) -> Self {
        self.sizing_w = Some(w);
        self.sizing_h = Some(h);
        self
    }
    /// Sets the aspect ratio (per-mille).
    pub fn aspect(mut self, ratio: u32) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }
    /// Sets the layout transition (duration ms, easing).
    pub fn transition(mut self, duration_ms: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((duration_ms, easing));
        self
    }

    /// `other`'s `Some` fields override `self`'s same-named fields (style composition).
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
        if other.font.is_some() { self.font = other.font; }
        self
    }
}

/// A fully resolved style: every field concrete, with defaults applied for anything unset.
#[derive(Clone, PartialEq, Debug)]
pub struct ResolvedStyle {
    /// Background color.
    pub bg_color: Color,
    /// Background opacity (0..=255).
    pub bg_opa: u8,
    /// Border color.
    pub border_color: Color,
    /// Border width in pixels.
    pub border_width: i32,
    /// Corner radius in pixels.
    pub radius: i32,
    /// Left padding.
    pub pad_left: i32,
    /// Right padding.
    pub pad_right: i32,
    /// Top padding.
    pub pad_top: i32,
    /// Bottom padding.
    pub pad_bottom: i32,
    /// Text color.
    pub text_color: Color,
    /// Container layout.
    pub layout: Layout,
    /// Width sizing strategy (None = content size).
    pub sizing_w: Option<crate::layout::Sizing>,
    /// Height sizing strategy (None = content size).
    pub sizing_h: Option<crate::layout::Sizing>,
    /// Aspect ratio (per-mille).
    pub aspect_ratio: Option<u32>,
    /// Layout transition: (duration ms, easing).
    pub transition: Option<(u32, crate::anim::Easing)>,
    /// Text font.
    pub font: &'static embedded_graphics::mono_font::MonoFont<'static>,
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
            font: crate::font::DEFAULT_FONT,
        }
    }
}

/// Per-field fallback: overlay -> base -> default (fields not hit use the rest of
/// `ResolvedStyle::default()`).
pub fn resolve(base: &Style, overlay: Option<&Style>, default: &'static embedded_graphics::mono_font::MonoFont<'static>) -> ResolvedStyle {
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
        font: overlay.and_then(|s| s.font).or(base.font).unwrap_or(default),
    }
}

/// The common base style: foundation for every widget's default style.
/// Note: only for composing the "base style"; state-overlay styles (pressed/focused) stay
/// sparse — do not build them from this.
pub fn theme_base() -> Style {
    Style::new().text_color(Color::WHITE).bg_opa(255).radius(4)
}

/// Default style for the screen background.
pub fn theme_screen() -> Style {
    theme_base().bg(Color::rgb(24, 24, 32))
}

/// Default style for a plain object.
pub fn theme_obj() -> Style {
    theme_base().bg(Color::rgb(40, 40, 52))
}

/// Default style for a label (transparent background).
pub fn theme_label() -> Style {
    theme_base().bg_opa(0) // transparent background
}

/// Default style for a button.
pub fn theme_button() -> Style {
    theme_base()
        .bg(Color::rgb(60, 90, 160))
        .radius(6)
        .border(Color::rgb(90, 120, 200), 1)
}

/// Pressed-state overlay style for a button.
pub fn theme_button_pressed() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(40, 60, 110));
    s
}

/// Focused-state overlay style for a button.
pub fn theme_button_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(2);
    s
}

/// Default style for a slider.
pub fn theme_slider() -> Style {
    theme_base().bg(Color::rgb(70, 70, 80)).radius(6)
}

/// Focused-state overlay style for a slider.
pub fn theme_slider_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(2);
    s
}

/// Default style for a switch (fully rounded on a height of 20).
pub fn theme_switch() -> Style {
    theme_base().bg(Color::rgb(90, 90, 90)).radius(10) // full rounding for a height of 20
}

/// Focused-state overlay style for a switch.
pub fn theme_switch_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(2);
    s
}

/// Default style for a progress bar.
pub fn theme_bar() -> Style {
    theme_base().bg(Color::rgb(70, 70, 80))
}

/// Default style for a list.
pub fn theme_list() -> Style {
    theme_base()
        .bg(Color::rgb(34, 34, 44))
        .border(Color::rgb(70, 70, 90), 1)
}

/// Focused-state overlay style for a list.
pub fn theme_list_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s
}
