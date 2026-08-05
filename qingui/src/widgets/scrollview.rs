use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::Rect;
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Sizing};
use crate::style::{Layout, Style};
use crate::ui::Ui;
use super::{KeyCtx, KeyOutcome, WidgetBehavior, WidgetCtx, WidgetKind};

/// Scroll step per key press (px)
pub const STEP: i32 = 20;

/// Scrolling container state: viewport CLIP_CHILDREN, content moved via translate
pub struct ScrollViewState {
    pub(crate) content: ObjRef,
    pub scroll: i32, // ≤0
}

impl ScrollViewState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: KeyCtx) -> KeyOutcome {
        match key {
            Key::Up => KeyOutcome::Deferred(scroll_by_exec, -STEP),
            Key::Down => KeyOutcome::Deferred(scroll_by_exec, STEP),
            _ => KeyOutcome::Pass,
        }
    }
}

/// Scroll exec fn: Ui calls it after putting the kind back.
pub(crate) fn scroll_by_exec(ui: &mut Ui, sv: ObjRef, delta: i32) {
    ui.scrollview_scroll_by(sv, delta);
}

impl WidgetBehavior for ScrollViewState {
    // Container: content is drawn by child nodes (viewport CLIP is handled by the common pipeline)
    fn draw(&self, _ctx: &WidgetCtx, _d: &mut DrawBuf, _clip: Rect) {}
    fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
        self.on_key(key, ctx)
    }
}

/// ScrollView builder: default 120x100, transparent viewport + content column flex
pub struct ScrollViewBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl ScrollViewBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self { size: None, style: None, sizing: None, transition: None, events: Vec::new() }
    }
    /// Sets the widget size.
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    /// Sets the style.
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    /// Sets the width/height sizing.
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    /// Sets the transition duration and easing.
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    /// Registers an event callback.
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    /// Builds the widget into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((120, 100));
        // The viewport is first created as an Obj placeholder (the content reference needs the handle after the self-reference)
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(super::obj::ObjState));
        ui.set_clip_children(r, true);
        // content: column flex, width GROW, transparent
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0), WidgetKind::Obj(super::obj::ObjState));
        let mut cs = Style::default();
        cs.bg_opa = Some(0);
        ui.set_style(content, cs);
        ui.set_sizing(content, Some(Sizing::GROW), None);
        ui.set_layout(content, Layout::Flex(Flex {
            dir: FlexDir::Column, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
        }));
        // Replace the placeholder kind with the real one
        if let Some(n) = ui.kind_mut(r) {
            *n = WidgetKind::ScrollView(ScrollViewState { content, scroll: 0 });
        }
        // Viewport style: transparent by default; focused style gives a default border highlight
        let mut vs = self.style.unwrap_or_default();
        if vs.bg_opa.is_none() { vs.bg_opa = Some(0); }
        ui.set_style(r, vs);
        // The viewport is a column flex by default: lets content's width GROW follow the viewport width (otherwise GROW is dead code)
        ui.set_layout(r, Layout::Flex(Flex {
            dir: FlexDir::Column, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
        }));
        ui.set_style_focused(r, crate::style::theme_list_focused());
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}

/// ScrollView API (brought in via prelude)
pub trait UiScrollViewExt {
    /// Returns the scrollable content node, if any.
    fn scrollview_content(&self, sv: ObjRef) -> Option<ObjRef>;
    /// Scrolls so the content's translate.y equals `y` (clamped to the scrollable range).
    fn scrollview_scroll_to(&mut self, sv: ObjRef, y: i32);
    /// Scrolls by `delta` pixels relative to the current position.
    fn scrollview_scroll_by(&mut self, sv: ObjRef, delta: i32);
}

impl UiScrollViewExt for Ui {
    fn scrollview_content(&self, sv: ObjRef) -> Option<ObjRef> {
        self.kind(sv).and_then(|k| k.as_scrollview()).map(|s| s.content)
    }

    fn scrollview_scroll_to(&mut self, sv: ObjRef, y: i32) {
        let Some(content) = self.scrollview_content(sv) else { return };
        // Child rects are produced by layout: flush pending layout first so the rects read below are current (same as itemlist ensure_visible)
        if self.layout_dirty {
            self.layout_pass();
            self.layout_dirty = false;
        }
        // content_h = the child nodes' maximum bottom edge; viewport height = sv height
        let content_h = self.children(content).iter()
            .map(|&c| self.rect(c).y + self.rect(c).h)
            .max()
            .unwrap_or(0);
        let view_h = self.rect(sv).h;
        let min = -(content_h - view_h).max(0);
        let ny = y.clamp(min, 0);
        // Early return if the clamped value equals the current scroll: no state write, no set_translate, avoids a needless repaint (same as itemlist ensure_visible)
        let cur = self.kind(sv).and_then(|k| k.as_scrollview()).map(|s| s.scroll);
        if cur == Some(ny) { return; }
        if let Some(s) = self.kind_mut(sv).and_then(|k| k.as_scrollview_mut()) {
            s.scroll = ny;
        }
        self.set_translate(content, 0, ny);
    }

    fn scrollview_scroll_by(&mut self, sv: ObjRef, delta: i32) {
        let cur = self.kind(sv).and_then(|k| k.as_scrollview()).map(|s| s.scroll);
        if let Some(cur) = cur {
            // scroll equals translate.y (≤0): a positive delta scrolls down = content moves up = translate decreases
            self.scrollview_scroll_to(sv, cur - delta);
        }
    }
}
