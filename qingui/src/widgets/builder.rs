//! Shared builder scaffolding: common config + the generic WidgetBuilder.
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::event::{EventCb, EventKind};
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use embedded_graphics::pixelcolor::PixelColor;

type DefaultColor = embedded_graphics::pixelcolor::Rgb888;

/// Container layout kind selected via `WidgetBuilder::layout`.
#[derive(Clone, PartialEq, Debug)]
pub enum Layout {
    /// Flex layout.
    Flex(crate::layout::Flex),
    /// Grid layout.
    Grid(crate::layout::Grid),
}

/// Common fields shared by every widget builder.
pub(crate) struct CommonBuilder<C = DefaultColor> {
    pub size: Option<(i32, i32)>,
    pub style: Option<Style<C>>,
    pub style_focused: Option<Style<C>>,
    pub style_edited: Option<Style<C>>,
    pub layout: Option<Layout>,
    pub pad: Option<(i32, i32, i32, i32)>,
    pub sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    pub aspect: Option<u32>,
    pub transition: Option<(u32, Easing)>,
    pub events: Vec<(EventKind, EventCb<C>)>,
}

impl<C> Default for CommonBuilder<C> {
    fn default() -> Self {
        Self {
            size: None, style: None, style_focused: None, style_edited: None,
            layout: None, pad: None, sizing: None, aspect: None,
            transition: None, events: Vec::new(),
        }
    }
}

impl<C: PixelColor> CommonBuilder<C> {
    /// Applies the sizing/transition/events tail to an inserted node.
    /// Style defaults are widget-specific and stay in each `WidgetCfg::build`;
    /// `layout` is consumed by `ObjCfg::build` (it decides the widget kind) and
    /// never reaches this tail.
    pub fn apply_tail(self, ui: &mut Ui<C>, r: ObjRef) {
        if let Some(p) = self.pad { ui.set_pad(r, p); }
        if let Some((sw, sh)) = self.sizing { ui.set_sizing(r, sw, sh); }
        if let Some(a) = self.aspect { ui.set_aspect(r, Some(a)); }
        if let Some(t) = self.transition { ui.set_transition(r, Some(t)); }
        for (k, cb) in self.events { ui.add_event_cb(r, k, cb); }
    }
}

/// Widget-specific build logic: default size/style and post-insert setup.
pub(crate) trait WidgetCfg<C = DefaultColor> {
    fn build(self, ui: &mut Ui<C>, parent: ObjRef, common: CommonBuilder<C>) -> ObjRef;
    fn default_style() -> Style<C> {
        Style::default()
    }
}

/// A fluent builder for any widget. Common setters live here once.
///
/// `C` is the target UI's pixel format; it is inferred at `build` from the
/// `Ui` being built into, so constructors do not name it.
pub struct WidgetBuilder<Cfg, C = DefaultColor> {
    pub(crate) common: CommonBuilder<C>,
    pub(crate) cfg: Cfg,
}

impl<Cfg, C> WidgetBuilder<Cfg, C> {
    /// Sets the widget size.
    pub fn size(mut self, w: i32, h: i32) -> Self { self.common.size = Some((w, h)); self }
    /// Sets the style.
    pub fn style(mut self, s: Style<C>) -> Self { self.common.style = Some(s); self }
    /// Modifies on top of the default style.
    #[allow(private_bounds)]
    pub fn style_with(mut self, f: impl FnOnce(Style<C>) -> Style<C>) -> Self
    where
        Cfg: WidgetCfg<C>,
    {
        self.common.style = Some(f(self.common.style.take().unwrap_or_else(Cfg::default_style)));
        self
    }
    /// Sets the focused style. Only honored by widgets that have a focused-state style (currently Button, Checkbox, Dropdown, ItemList, List, Roller, ScrollView, Slider, Spinbox, Switch).
    pub fn style_focused(mut self, s: Style<C>) -> Self { self.common.style_focused = Some(s); self }
    /// Sets the edited (inner-mode) style. Only honored by widgets with an inner mode
    /// (ItemList, List, Roller, ScrollView, Slider, Spinbox); when unset it falls back
    /// to `style::theme_edited` derived from the focused style.
    pub fn style_edited(mut self, s: Style<C>) -> Self { self.common.style_edited = Some(s); self }
    /// Sets the container layout. Layout is a widget kind, not a common property:
    /// only `ObjCfg` honors it (it becomes the node's FlexLayout/GridLayout kind);
    /// every other widget silently ignores it.
    pub fn layout(mut self, l: Layout) -> Self { self.common.layout = Some(l); self }
    /// Sets padding on all four sides.
    pub fn pads(mut self, v: i32) -> Self { self.common.pad = Some((v, v, v, v)); self }
    /// Sets padding per side: (left, right, top, bottom).
    pub fn pad(mut self, l: i32, r: i32, t: i32, b: i32) -> Self { self.common.pad = Some((l, r, t, b)); self }
    /// Sets the aspect ratio (per-mille).
    pub fn aspect(mut self, ratio: u32) -> Self { self.common.aspect = Some(ratio); self }
    /// Sets the width/height sizing.
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.common.sizing = Some((w, h));
        self
    }
    /// Sets the transition duration and easing.
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.common.transition = Some((dur, easing));
        self
    }
    /// Registers an event callback.
    pub fn on(mut self, kind: EventKind, cb: EventCb<C>) -> Self {
        self.common.events.push((kind, cb));
        self
    }
    /// Builds the widget into the parent node.
    #[allow(private_bounds)]
    pub fn build(self, ui: &mut Ui<C>, parent: ObjRef) -> ObjRef
    where
        Cfg: WidgetCfg<C>,
        C: PixelColor,
    {
        Cfg::build(self.cfg, ui, parent, self.common)
    }
}
