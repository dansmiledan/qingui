use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Sizing};
use crate::pixel::PixelFormat;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};

/// Scroll step per key press (px)
pub const STEP: i32 = 20;

/// The content node's fixed arrangement (column flex).
pub(crate) const CONTENT_FLEX: Flex = Flex {
    dir: FlexDir::Column, wrap: false,
    main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
};

/// The viewport's own arrangement: a single column holding the content node.
/// Running this flex on the viewport makes the content's cross-axis
/// `Sizing::GROW` track the viewport width on every layout pass, including
/// runtime resizes of the viewport.
const SCROLL_FLEX: Flex = Flex {
    dir: FlexDir::Column, wrap: false,
    main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
};

/// Scrolling container state: viewport CLIP_CHILDREN, content moved via translate
pub struct ScrollViewState {
    pub(crate) content: ObjRef,
    pub scroll: i32, // ≤0
    /// Scroll step per key press (px)
    pub step: i32,
}

impl<C: PixelFormat> super::Widget<C> for ScrollViewState {
    // Container: content is drawn by child nodes (CLIP_CHILDREN handled by the pipeline).
    fn on_key(&mut self, ui: &mut Ui<C>, obj: ObjRef, key: Key) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        // Reentrancy: the kind is taken out during on_key, so `Ui::widget`/`Ui::update`
        // cannot reach this node — mutate `self` directly via `apply_scroll`.
        // Inner (EDITED) mode: direction keys scroll the content, Enter confirms
        // (Commit = Click + exit), Esc exits without acting. Outside the inner mode
        // nothing is consumed, so rotation moves the focus instead.
        if !ui.state(obj).contains(crate::node::State::EDITED) {
            return if key == Key::Enter { EnterEdit } else { Pass };
        }
        match key {
            Key::Up => {
                let y = self.scroll + self.step;
                apply_scroll(ui, obj, self, y);
                Consumed
            }
            Key::Down => {
                let y = self.scroll - self.step;
                apply_scroll(ui, obj, self, y);
                Consumed
            }
            Key::Enter => Commit,
            Key::Esc => ExitEdit,
            _ => Consumed,
        }
    }
    // The viewport arranges its single child (the content node): the column flex
    // consumes the content's cross-axis `Sizing::GROW`, keeping the content width
    // equal to the viewport width.
    fn layout(&mut self, ui: &mut Ui<C>, obj: ObjRef, content: Rect) {
        crate::layout::layout_flex(ui, obj, &SCROLL_FLEX, content);
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}

/// Core of scroll_to: clamps `y`, writes `state.scroll`, applies the translate.
/// Callable both from the ext trait (kind in arena) and from `on_key` (kind taken out).
pub(crate) fn apply_scroll<C: PixelFormat>(ui: &mut Ui<C>, sv: ObjRef, state: &mut ScrollViewState, y: i32) {
    // Child rects are produced by layout: flush pending layout first so the rects read below are current (same as itemlist ensure_visible)
    if ui.layout_dirty {
        ui.layout_pass();
        ui.layout_dirty = false;
    }
    // content_h = the child nodes' maximum bottom edge; viewport height = sv height
    let content_h = ui.children(state.content).iter()
        .map(|&c| ui.rect(c).y + ui.rect(c).h)
        .max()
        .unwrap_or(0);
    let view_h = ui.rect(sv).h;
    let ny = y.clamp(-(content_h - view_h).max(0), 0);
    // Early return if the clamped value equals the current scroll: no state write, no set_translate, avoids a needless repaint (same as itemlist ensure_visible)
    if state.scroll == ny { return; }
    state.scroll = ny;
    let content = state.content;
    ui.set_translate(content, 0, ny);
}

/// Builder for the ScrollView widget.
pub type ScrollViewBuilder<C = crate::geometry::Color> = WidgetBuilder<ScrollViewCfg, C>;

/// ScrollView configuration: scroll step per key press.
pub struct ScrollViewCfg {
    step: i32,
}

impl ScrollViewCfg {
    /// Creates an empty builder.
    pub fn new<C: PixelFormat>() -> WidgetBuilder<ScrollViewCfg, C> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ScrollViewCfg { step: STEP } }
    }
}

impl<C> WidgetBuilder<ScrollViewCfg, C> {
    /// Sets the scroll step per key press in pixels (default `STEP` = 20).
    pub fn step(mut self, v: i32) -> Self {
        self.cfg.step = v;
        self
    }
}

impl<C: PixelFormat> WidgetCfg<C> for ScrollViewCfg {
    fn build(self, ui: &mut Ui<C>, parent: ObjRef, mut common: CommonBuilder<C>) -> ObjRef {
        let (w, h) = common.size.unwrap_or((120, 100));
        // The viewport is first created as a Manual placeholder: the content node
        // needs the viewport as its parent, and the state needs the content handle.
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(super::obj::Manual));
        ui.set_clip_children(r, true);
        // content: column flex, width grows with the viewport, transparent
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0),
            alloc::boxed::Box::new(super::flexbox::FlexLayout { flex: CONTENT_FLEX }));
        let mut cs = Style::default();
        cs.bg_opa = Some(0);
        ui.set_style(content, cs);
        ui.set_sizing(content, Some(Sizing::GROW), None);
        // Replace the placeholder kind with the real one
        if let Some(n) = ui.kind_mut(r) {
            *n = alloc::boxed::Box::new(ScrollViewState { content, scroll: 0, step: self.step });
        }
        // Viewport style: transparent by default; focused style gives a default border highlight
        let mut vs = common.style.take().unwrap_or_default();
        if vs.bg_opa.is_none() { vs.bg_opa = Some(0); }
        ui.set_style(r, vs);
        let focused = common.style_focused.take().unwrap_or_else(crate::style::theme_list_focused);
        ui.set_style_focused(r, focused.clone());
        ui.set_style_edited(r, common.style_edited.take().unwrap_or_else(|| crate::style::theme_edited(&focused)));
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

impl<C: PixelFormat> UiScrollViewExt for Ui<C> {
    fn scrollview_content(&self, sv: ObjRef) -> Option<ObjRef> {
        self.widget::<ScrollViewState>(sv).map(|s| s.content)
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
        let ny = y.clamp(-(content_h - view_h).max(0), 0);
        let changed = self.update::<ScrollViewState, _>(sv, |s| {
            let changed = s.scroll != ny;
            s.scroll = ny;
            changed
        }).unwrap_or(false);
        // Early return if the clamped value equals the current scroll: no set_translate, avoids a needless repaint (same as itemlist ensure_visible)
        if changed {
            self.set_translate(content, 0, ny);
        }
    }

    fn scrollview_scroll_by(&mut self, sv: ObjRef, delta: i32) {
        if let Some(cur) = self.widget::<ScrollViewState>(sv).map(|s| s.scroll) {
            // scroll equals translate.y (≤0): a positive delta scrolls down = content moves up = translate decreases
            self.scrollview_scroll_to(sv, cur - delta);
        }
    }
}
