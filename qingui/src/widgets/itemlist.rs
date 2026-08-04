use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Sizing};
use crate::node::State;
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
            // 导航细节需要 Ui（子节点/滚动/事件），经 Deferred 执行函数在 kind 放回后执行
            Key::Up => KeyOutcome::Deferred(nav_select_exec, -1),
            Key::Down => KeyOutcome::Deferred(nav_select_exec, 1),
            _ => KeyOutcome::Pass,
        }
    }
}

/// 列表导航执行函数：Ui 在 kind 放回后调用（obj 的 kind 已还原，可安全经 ui 访问自身）。
/// 语义与旧 apply_key_outcome 的 NavSelect 分支完全一致：空列表也消费。
pub(crate) fn nav_select_exec(ui: &mut Ui, il: ObjRef, d: i32) {
    let n = ui.itemlist_len(il);
    if n > 0 {
        let cur = ui.itemlist_selected(il);
        let next = (cur as i32 + d).rem_euclid(n as i32) as usize;
        ui.itemlist_select(il, next);
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
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(super::obj::ObjState));
        ui.set_clip_children(r, true);
        // content：Flex column 容器，宽 GROW，透明背景
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0), WidgetKind::Obj(super::obj::ObjState));
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

impl super::WidgetBehavior for ItemListState {
    // ItemList 同为容器：内容由子节点绘制
    fn draw(&self, _ctx: &super::WidgetCtx, _d: &mut crate::draw::DrawBuf, _clip: Rect) {}
    fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.selected as i32 }
}

/// itemlist 数据/导航 API(经 prelude 或显式 use 引入)
pub trait UiItemListExt {
    fn itemlist_add_item(&mut self, il: ObjRef) -> Option<ObjRef>;
    fn itemlist_remove_selected(&mut self, il: ObjRef) -> bool;
    fn itemlist_select(&mut self, il: ObjRef, idx: usize);
    fn itemlist_selected(&self, il: ObjRef) -> usize;
    fn itemlist_len(&self, il: ObjRef) -> usize;
}

impl UiItemListExt for Ui {
    /// 向 ItemList 追加一个 item 容器（Obj，宽 GROW，透明背景，带 SELECTED 样式），
    /// 返回该容器（用户往里搭建内容）；il 非 ItemList 时返回 None
    fn itemlist_add_item(&mut self, il: ObjRef) -> Option<ObjRef> {
        let (content, sel_style, was_empty) = {
            let s = self.kind(il)?.as_itemlist()?;
            (s.content, s.sel_style.clone(), self.children(s.content).is_empty())
        };
        let item = self.insert_node(content, Rect::default(), WidgetKind::Obj(super::obj::ObjState));
        let mut st = item_base_style();
        st.sizing_w = Some(Sizing::GROW);
        self.set_style(item, st);
        self.set_style_selected(item, sel_style);
        // 首项自动选中
        if was_empty {
            self.set_state(item, State::SELECTED, true);
        }
        Some(item)
    }

    /// 删除 ItemList 的选中 item（空列表返回 false），selected 收敛并把选中位移给相邻项
    fn itemlist_remove_selected(&mut self, il: ObjRef) -> bool {
        let Some((content, selected)) = self
            .kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| (s.content, s.selected))
        else {
            return false;
        };
        let kids = self.children(content);
        if kids.is_empty() || selected >= kids.len() {
            return false;
        }
        self.delete(kids[selected]);
        let new_len = kids.len() - 1;
        let new_sel = if new_len == 0 { 0 } else { selected.min(new_len - 1) };
        if let Some(s) = self.kind_mut(il).and_then(|k| k.as_itemlist_mut()) {
            s.selected = new_sel;
        }
        // 选中位移给相邻项（删除中间项 → 原下一项；删除末项 → 原上一项）
        if new_len > 0 {
            let target = if selected < new_len { kids[selected + 1] } else { kids[selected - 1] };
            self.set_state(target, State::SELECTED, true);
        }
        ensure_visible(self, il);
        true
    }

    /// 选中 ItemList 第 idx 项（clamp 到合法范围）；变化才切换并发 ValueChanged
    fn itemlist_select(&mut self, il: ObjRef, idx: usize) {
        let Some((content, cur)) = self
            .kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| (s.content, s.selected))
        else {
            return;
        };
        let kids = self.children(content);
        if kids.is_empty() {
            return;
        }
        // 用户可能绕过 itemlist_remove_selected 直接 delete item：clamp 掉越界的 selected 并写回，消除漂移
        let cur = cur.min(kids.len() - 1);
        if let Some(s) = self.kind_mut(il).and_then(|k| k.as_itemlist_mut()) {
            if s.selected != cur {
                s.selected = cur;
            }
        }
        let nidx = idx.min(kids.len() - 1);
        if nidx == cur {
            return;
        }
        self.set_state(kids[cur], State::SELECTED, false);
        self.set_state(kids[nidx], State::SELECTED, true);
        if let Some(s) = self.kind_mut(il).and_then(|k| k.as_itemlist_mut()) {
            s.selected = nidx;
        }
        ensure_visible(self, il);
        self.send_event(il, crate::event::EventKind::ValueChanged);
    }

    fn itemlist_selected(&self, il: ObjRef) -> usize {
        self.kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| s.selected)
            .unwrap_or(0)
    }

    fn itemlist_len(&self, il: ObjRef) -> usize {
        self.kind(il)
            .and_then(|k| k.as_itemlist())
            .map(|s| self.children(s.content).len())
            .unwrap_or(0)
    }
}

/// 滚动 content（translate.y）使选中 item 在视口内可见（瞬时，无动画）
fn ensure_visible(ui: &mut Ui, il: ObjRef) {
    // item 位置由 Flex 布局产出：先冲刷待处理布局，保证下面读到的是最新 rect
    if ui.layout_dirty {
        ui.layout_pass();
        ui.layout_dirty = false;
    }
    let Some((content, selected)) = ui
        .kind(il)
        .and_then(|k| k.as_itemlist())
        .map(|s| (s.content, s.selected))
    else {
        return;
    };
    let Some(item) = ui.children(content).get(selected).copied() else {
        return;
    };
    let vp_h = ui.rect(il).h;
    // 视口无布局，content 高度不会被撑开：取子项最大底边作为内容总高
    let content_h = ui
        .children(content)
        .iter()
        .map(|&k| ui.rect(k).bottom())
        .max()
        .unwrap_or(0);
    let ir = ui.rect(item); // 相对 content 的本地 rect
    let off = ui.translate(content).y;
    let mut new_off = if content_h <= vp_h {
        0 // 内容不足一屏：不滚动
    } else if ir.h >= vp_h {
        -ir.y // item 高于视口：顶对齐
    } else {
        let top = ir.y + off;
        let bottom = top + ir.h;
        if top < 0 {
            off - top // 上滚：顶对齐
        } else if bottom > vp_h {
            off - (bottom - vp_h) // 下滚：底对齐
        } else {
            off
        }
    };
    new_off = new_off.min(0); // 不允许向下露出内容顶部空白
    if new_off != off {
        ui.set_translate(content, 0, new_off);
    }
}
