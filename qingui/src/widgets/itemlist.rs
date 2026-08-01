use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Sizing};
use crate::style::{Layout, Style};
use crate::ui::Ui;
use super::{KeyCtx, KeyOutcome, WidgetKind};

/// 容器型列表：item 为普通子节点（用户自由搭建内容），控件只管选中/导航/滚动。
/// 结构：ItemList（视口，CLIP_CHILDREN）> content（Flex column，translate 滚动）> items
pub struct ItemListState {
    pub selected: usize,
    pub(crate) content: ObjRef,
    pub(crate) sel_style: Style,
}

impl ItemListState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: KeyCtx) -> KeyOutcome {
        match key {
            // 导航细节需要 Ui（子节点/滚动/事件），由 apply_key_outcome 执行
            Key::Up => KeyOutcome::NavSelect(-1),
            Key::Down => KeyOutcome::NavSelect(1),
            _ => KeyOutcome::Pass,
        }
    }
}

/// 透明容器样式（只做布局/滚动，不画背景）
fn transparent() -> Style {
    let mut s = Style::default();
    s.bg_opa = Some(0);
    s
}

/// item 容器的基础样式：透明背景（SELECTED 时由 style_selected 叠加高亮）
pub(crate) fn item_base_style() -> Style {
    transparent()
}

fn column_layout() -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Column,
        wrap: false,
        main: Align::Start,
        cross: Align::Start,
        track: Align::Start,
        gap: 0,
    })
}

/// ItemList 构建器：默认 120x100，内联默认视口样式（深色底+边框）与默认选中样式（蓝底）
pub struct ItemListBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_selected: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl ItemListBuilder {
    pub fn new() -> Self {
        Self { size: None, style: None, style_selected: None, style_focused: None, sizing: None, transition: None, events: Vec::new() }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    /// item 的选中样式（叠加于 State::SELECTED）。
    /// 注意：必须显式含 bg_opa，否则 item 基底的 bg_opa(0) 会让高亮不可见
    pub fn style_selected(mut self, s: Style) -> Self { self.style_selected = Some(s); self }
    /// 视口的聚焦样式（叠加于 State::FOCUSED）
    pub fn style_focused(mut self, s: Style) -> Self { self.style_focused = Some(s); self }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self { self.sizing = Some((w, h)); self }
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self { self.transition = Some((dur, easing)); self }
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((120, 100));
        // 视口节点先以 Obj 占位（content 引用需要自指后的句柄）
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj);
        ui.set_clip_children(r, true);
        // content：Flex column 容器，宽 GROW，透明背景
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0), WidgetKind::Obj);
        ui.set_style(content, transparent());
        ui.set_sizing(content, Some(Sizing::GROW), None);
        ui.set_layout(content, column_layout());
        // 占位 kind 换真身
        let sel_style = self.style_selected.unwrap_or_else(default_sel_style);
        if let Some(n) = ui.arena.get_mut(r) {
            n.kind = WidgetKind::ItemList(ItemListState { selected: 0, content, sel_style });
        }
        // 视口样式（默认对齐 theme_list 的深色底 + 边框）
        let mut vs = self.style.unwrap_or_else(|| {
            let mut s = Style::default();
            s.bg_color = Some(Color::rgb(34, 34, 44));
            s.bg_opa = Some(255);
            s.border_color = Some(Color::rgb(70, 70, 90));
            s.border_width = Some(1);
            s
        });
        ui.set_style(r, {
            if vs.bg_opa.is_none() { vs.bg_opa = Some(255); }
            vs
        });
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        ui.set_style_focused(r, self.style_focused.unwrap_or_else(crate::style::theme_list_focused));
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}

/// 默认选中样式（对齐文本 List 高亮色 rgb(50,70,120)；必须显式 bg_opa(255)）
fn default_sel_style() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(50, 70, 120));
    s.bg_opa = Some(255);
    s
}
