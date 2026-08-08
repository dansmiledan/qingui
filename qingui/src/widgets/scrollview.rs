use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Layout, Sizing};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
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

/// Builder for the ScrollView widget.
pub type ScrollViewBuilder = WidgetBuilder<ScrollViewCfg>;

/// ScrollView configuration.
pub struct ScrollViewCfg;

impl ScrollViewCfg {
    /// Creates an empty builder.
    pub fn new() -> WidgetBuilder<ScrollViewCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ScrollViewCfg }
    }
}

impl WidgetCfg for ScrollViewCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((120, 100));
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
        let mut vs = common.style.take().unwrap_or_default();
        if vs.bg_opa.is_none() { vs.bg_opa = Some(0); }
        ui.set_style(r, vs);
        // The viewport is a column flex by default: lets content's width GROW follow the viewport width (otherwise GROW is dead code)
        ui.set_layout(r, Layout::Flex(Flex {
            dir: FlexDir::Column, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
        }));
        ui.set_style_focused(r, common.style_focused.take().unwrap_or_else(crate::style::theme_list_focused));
        common.apply_tail(ui, r);
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
