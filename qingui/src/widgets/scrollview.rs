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

/// 单次按键滚动步进(px)
pub const STEP: i32 = 20;

/// 滚动容器状态:视口 CLIP_CHILDREN,content 经 translate 移动
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

/// ScrollBy 的执行函数：Ui 在 kind 放回后调用。
pub(crate) fn scroll_by_exec(ui: &mut Ui, sv: ObjRef, delta: i32) {
    ui.scrollview_scroll_by(sv, delta);
}

impl WidgetBehavior for ScrollViewState {
    // 容器:内容由子节点绘制(视口 CLIP 已由通用管线处理)
    fn draw(&self, _ctx: &WidgetCtx, _d: &mut DrawBuf, _clip: Rect) {}
    fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
        self.on_key(key, ctx)
    }
}

/// ScrollView 构建器:默认 120x100,视口透明 + content column flex
pub struct ScrollViewBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl ScrollViewBuilder {
    pub fn new() -> Self {
        Self { size: None, style: None, sizing: None, transition: None, events: Vec::new() }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((120, 100));
        // 视口先以 Obj 占位(content 引用需要自指后的句柄)
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(super::obj::ObjState));
        ui.set_clip_children(r, true);
        // content:column flex,宽 GROW,透明
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0), WidgetKind::Obj(super::obj::ObjState));
        let mut cs = Style::default();
        cs.bg_opa = Some(0);
        ui.set_style(content, cs);
        ui.set_sizing(content, Some(Sizing::GROW), None);
        ui.set_layout(content, Layout::Flex(Flex {
            dir: FlexDir::Column, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
        }));
        // 占位 kind 换真身
        if let Some(n) = ui.kind_mut(r) {
            *n = WidgetKind::ScrollView(ScrollViewState { content, scroll: 0 });
        }
        // 视口样式:默认透明;聚焦样式给默认边框高亮
        let mut vs = self.style.unwrap_or_default();
        if vs.bg_opa.is_none() { vs.bg_opa = Some(0); }
        ui.set_style(r, vs);
        // 视口默认 column flex:让 content 的宽 GROW 跟随视口宽(否则 GROW 是死代码)
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

/// ScrollView API(经 prelude 引入)
pub trait UiScrollViewExt {
    fn scrollview_content(&self, sv: ObjRef) -> Option<ObjRef>;
    fn scrollview_scroll_to(&mut self, sv: ObjRef, y: i32);
    fn scrollview_scroll_by(&mut self, sv: ObjRef, delta: i32);
}

impl UiScrollViewExt for Ui {
    fn scrollview_content(&self, sv: ObjRef) -> Option<ObjRef> {
        self.kind(sv).and_then(|k| k.as_scrollview()).map(|s| s.content)
    }

    fn scrollview_scroll_to(&mut self, sv: ObjRef, y: i32) {
        let Some(content) = self.scrollview_content(sv) else { return };
        // 子节点 rect 由布局产出:先冲刷待处理布局,保证下面读到最新 rect(同 itemlist ensure_visible)
        if self.layout_dirty {
            self.layout_pass();
            self.layout_dirty = false;
        }
        // content_h = 子节点最大底边;视口高 = sv 高度
        let content_h = self.children(content).iter()
            .map(|&c| self.rect(c).y + self.rect(c).h)
            .max()
            .unwrap_or(0);
        let view_h = self.rect(sv).h;
        let min = -(content_h - view_h).max(0);
        let ny = y.clamp(min, 0);
        // clamp 后与当前 scroll 相同则早退:不写 state、不 set_translate,避免白重绘(同 itemlist ensure_visible)
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
            // scroll 即 translate.y(≤0):正 delta 向下滚 = 内容向上移 = translate 减小
            self.scrollview_scroll_to(sv, cur - delta);
        }
    }
}
