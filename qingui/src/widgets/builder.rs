//! Shared builder scaffolding: common config + the generic WidgetBuilder.
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::event::{EventCb, EventKind};
use crate::layout::Sizing;
use crate::style::{Layout, Style};
use crate::ui::Ui;

/// Common fields shared by every widget builder.
#[derive(Default)]
pub(crate) struct CommonBuilder {
    pub size: Option<(i32, i32)>,
    pub style: Option<Style>,
    pub style_pressed: Option<Style>,
    pub style_focused: Option<Style>,
    pub layout: Option<Layout>,
    pub sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    pub transition: Option<(u32, Easing)>,
    pub events: Vec<(EventKind, EventCb)>,
}

impl CommonBuilder {
    /// Applies the layout/sizing/transition/events tail to an inserted node.
    /// Style defaults are widget-specific and stay in each `WidgetCfg::build`.
    pub fn apply_tail(self, ui: &mut Ui, r: ObjRef) {
        if let Some(l) = self.layout {
            ui.set_layout(r, l);
        }
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
    }
}

/// Widget-specific build logic: default size/style and post-insert setup.
pub(crate) trait WidgetCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, common: CommonBuilder) -> ObjRef;
    fn default_style() -> Style {
        Style::default()
    }
}

/// A fluent builder for any widget. Common setters live here once.
pub struct WidgetBuilder<Cfg> {
    pub(crate) common: CommonBuilder,
    pub(crate) cfg: Cfg,
}

#[allow(private_bounds)]
impl<Cfg: WidgetCfg> WidgetBuilder<Cfg> {
    pub fn size(mut self, w: i32, h: i32) -> Self { self.common.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.common.style = Some(s); self }
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.common.style = Some(f(self.common.style.take().unwrap_or_else(Cfg::default_style)));
        self
    }
    pub fn style_pressed(mut self, s: Style) -> Self { self.common.style_pressed = Some(s); self }
    pub fn style_focused(mut self, s: Style) -> Self { self.common.style_focused = Some(s); self }
    pub fn layout(mut self, l: Layout) -> Self { self.common.layout = Some(l); self }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.common.sizing = Some((w, h));
        self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.common.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.common.events.push((kind, cb));
        self
    }
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        Cfg::build(self.cfg, ui, parent, self.common)
    }
}
