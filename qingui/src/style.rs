use embedded_graphics::pixelcolor::{Rgb888, RgbColor};

/// Flat style: `Option` fields, where `None` means "do not override".
/// Usable as a struct literal or built with a chained builder: `Style::new().bg(RED).radius(4)`
#[derive(Clone, PartialEq, Debug)]
pub struct Style<C = Rgb888> {
    /// Background color (`None` = no background painted).
    pub bg_color: Option<C>,
    /// Border color.
    pub border_color: Option<C>,
    /// Border width in pixels.
    pub border_width: Option<i32>,
    /// Corner radius in pixels.
    pub radius: Option<i32>,
    /// Text color.
    pub text_color: Option<C>,
    /// Text font (None = use the Ui default font).
    pub font: Option<&'static embedded_graphics::mono_font::MonoFont<'static>>,
}

// Hand-written (not derived): the derive would demand `C: Default`, but every
// field is `Option`/`&'static`, so `Style<C>` is default for ANY `C`.
impl<C> Default for Style<C> {
    fn default() -> Self {
        Self {
            bg_color: None,
            border_color: None,
            border_width: None,
            radius: None,
            text_color: None,
            font: None,
        }
    }
}

impl<C> Style<C> {
    /// Creates an empty style with all fields unset.
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets the background color.
    pub fn bg(mut self, color: C) -> Self {
        self.bg_color = Some(color);
        self
    }
    /// Sets the border color and width.
    pub fn border(mut self, color: C, width: i32) -> Self {
        self.border_color = Some(color);
        self.border_width = Some(width);
        self
    }
    /// Sets the corner radius.
    pub fn radius(mut self, radius: i32) -> Self {
        self.radius = Some(radius);
        self
    }
    /// Sets the text color.
    pub fn text_color(mut self, color: C) -> Self {
        self.text_color = Some(color);
        self
    }

    /// `other`'s `Some` fields override `self`'s same-named fields (style composition).
    pub fn merge(mut self, other: Style<C>) -> Style<C> {
        if other.bg_color.is_some() { self.bg_color = other.bg_color; }
        if other.border_color.is_some() { self.border_color = other.border_color; }
        if other.border_width.is_some() { self.border_width = other.border_width; }
        if other.radius.is_some() { self.radius = other.radius; }
        if other.text_color.is_some() { self.text_color = other.text_color; }
        if other.font.is_some() { self.font = other.font; }
        self
    }
}

/// A fully resolved style: every field concrete, with defaults applied for anything unset.
/// `bg_color: None` means "no background painted".
#[derive(Clone, PartialEq, Debug)]
pub struct ResolvedStyle<C = Rgb888> {
    /// Background color (`None` = no background painted).
    pub bg_color: Option<C>,
    /// Border color.
    pub border_color: C,
    /// Border width in pixels.
    pub border_width: i32,
    /// Corner radius in pixels.
    pub radius: i32,
    /// Text color.
    pub text_color: C,
    /// Text font.
    pub font: &'static embedded_graphics::mono_font::MonoFont<'static>,
}

impl<C: RgbColor> Default for ResolvedStyle<C> {
    fn default() -> Self {
        Self {
            bg_color: None,
            border_color: C::BLACK,
            border_width: 0,
            radius: 0,
            text_color: C::WHITE,
            font: crate::font::DEFAULT_FONT,
        }
    }
}

/// Per-field fallback: overlay -> base -> default (fields not hit use the rest of
/// `ResolvedStyle::default()`).
pub fn resolve<C: RgbColor>(base: &Style<C>, overlay: Option<&Style<C>>, default: &'static embedded_graphics::mono_font::MonoFont<'static>) -> ResolvedStyle<C> {
    let d = ResolvedStyle::default();
    let pick = |o: Option<&Style<C>>, f: fn(&Style<C>) -> Option<C>| -> Option<C> {
        o.and_then(f).or_else(|| f(base))
    };
    let pick_i = |o: Option<&Style<C>>, f: fn(&Style<C>) -> Option<i32>| -> Option<i32> {
        o.and_then(f).or_else(|| f(base))
    };
    ResolvedStyle {
        bg_color: pick(overlay, |s| s.bg_color),
        border_color: pick(overlay, |s| s.border_color).unwrap_or(d.border_color),
        border_width: pick_i(overlay, |s| s.border_width).unwrap_or(d.border_width),
        radius: pick_i(overlay, |s| s.radius).unwrap_or(d.radius),
        text_color: pick(overlay, |s| s.text_color).unwrap_or(d.text_color),
        font: overlay.and_then(|s| s.font).or(base.font).unwrap_or(default),
    }
}

/// The common base style: foundation for every widget's default style.
/// Note: only for composing the "base style"; state-overlay styles (edited/focused/selected) stay
/// sparse — do not build them from this.
pub fn theme_base<C: From<Rgb888>>() -> Style<C> {
    Style::new().text_color(Rgb888::WHITE.into()).radius(4)
}

/// Default style for the screen background.
pub fn theme_screen<C: From<Rgb888>>() -> Style<C> {
    theme_base().bg(Rgb888::new(24, 24, 32).into())
}

/// Default style for a plain object.
pub fn theme_obj<C: From<Rgb888>>() -> Style<C> {
    theme_base().bg(Rgb888::new(40, 40, 52).into())
}

/// Default style for a label (no bg_color = transparent background).
pub fn theme_label<C: From<Rgb888>>() -> Style<C> {
    theme_base()
}

/// Default style for a button.
pub fn theme_button<C: From<Rgb888>>() -> Style<C> {
    theme_base()
        .bg(Rgb888::new(60, 90, 160).into())
        .radius(6)
        .border(Rgb888::new(90, 120, 200).into(), 1)
}

/// Focused-state overlay style for a button.
pub fn theme_button_focused<C: From<Rgb888>>() -> Style<C> {
    let mut s = Style::default();
    s.border_color = Some(Rgb888::WHITE.into());
    s.border_width = Some(2);
    s
}

/// Default style for a slider.
pub fn theme_slider<C: From<Rgb888>>() -> Style<C> {
    theme_base().bg(Rgb888::new(70, 70, 80).into()).radius(6)
}

/// Focused-state overlay style for a slider.
pub fn theme_slider_focused<C: From<Rgb888>>() -> Style<C> {
    let mut s = Style::default();
    s.border_color = Some(Rgb888::WHITE.into());
    s.border_width = Some(2);
    s
}

/// Default style for a switch (fully rounded on a height of 20).
pub fn theme_switch<C: From<Rgb888>>() -> Style<C> {
    theme_base().bg(Rgb888::new(90, 90, 90).into()).radius(10) // full rounding for a height of 20
}

/// Focused-state overlay style for a switch.
pub fn theme_switch_focused<C: From<Rgb888>>() -> Style<C> {
    let mut s = Style::default();
    s.border_color = Some(Rgb888::WHITE.into());
    s.border_width = Some(2);
    s
}

/// Default style for a progress bar.
pub fn theme_bar<C: From<Rgb888>>() -> Style<C> {
    theme_base().bg(Rgb888::new(70, 70, 80).into())
}

/// Default style for a list.
pub fn theme_list<C: From<Rgb888>>() -> Style<C> {
    theme_base()
        .bg(Rgb888::new(34, 34, 44).into())
        .border(Rgb888::new(70, 70, 90).into(), 1)
}

/// Focused-state overlay style for a list.
pub fn theme_list_focused<C: From<Rgb888>>() -> Style<C> {
    let mut s = Style::default();
    s.border_color = Some(Rgb888::WHITE.into());
    s
}

/// Edit accent color: the amber used while a widget is in its inner (EDITED) mode.
/// The edited border (`theme_edited`) and the per-widget edit accents (slider knob,
/// arc indicator) share this so they stay visually consistent.
pub const EDIT_ACCENT: Rgb888 = Rgb888::new(255, 200, 60);

/// Edited (inner-mode) overlay derived from a focus overlay: same fields, border
/// recolored to the edit accent (see `EDIT_ACCENT`), so focus (white) and edit are
/// visually distinct.
pub fn theme_edited<C: Clone + From<Rgb888>>(focused: &Style<C>) -> Style<C> {
    let mut s = focused.clone();
    s.border_color = Some(EDIT_ACCENT.into());
    s
}
