use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

pub const ROW_H: i32 = 16;
pub const FX_DUR: u64 = 200;

#[derive(Clone)]
pub struct ListState {
    pub items: Vec<String>,
    pub selected: usize,
    pub scroll: i32,
    pub fx: ListFx,
}

impl ListState {
    pub(crate) fn tick(&mut self, now: u64) -> super::TickOut {
        let was_active = self.fx.active(now);
        let removed = self.fx.prune(now);
        // 活动中逐帧重绘；清理掉效果的这一帧也补一次重绘（清掉 ghost 残影）
        super::TickOut { redraw: was_active || removed, active: self.fx.active(now) }
    }

    pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
        let n = self.items.len();
        match key {
            Key::Up | Key::Down => {
                if n > 0 {
                    let idx = if key == Key::Up { (self.selected + n - 1) % n } else { (self.selected + 1) % n };
                    select(&self.items, &mut self.selected, &mut self.scroll, &mut self.fx, idx, ctx.vis_h, ctx.now);
                }
                super::KeyOutcome::Consumed
            }
            _ => super::KeyOutcome::Pass,
        }
    }
}

/// 单个 item 的入场/位移效果（绘制时按时间插值，收敛后由 prune 清理）
#[derive(Clone)]
pub struct ItemFx {
    pub index: usize,
    pub dy: i32, // 起始位移（收敛到 0）
    pub fade_in: bool,
    pub start: u64,
}

/// 删除中的渐隐项（数据已移除，仅视觉残留）
#[derive(Clone)]
pub struct Ghost {
    pub text: String,
    pub index: usize,
    pub start: u64,
}

#[derive(Clone, Default)]
pub struct ListFx {
    pub item_fx: Vec<ItemFx>,
    pub ghost: Option<Ghost>,
    /// 高亮滑动：(旧选中索引, 开始时间)
    pub sel_from: Option<(usize, u64)>,
    /// 平滑滚动：(旧 scroll, 开始时间)
    pub scroll_from: Option<(i32, u64)>,
}

impl ListFx {
    pub fn active(&self, now: u64) -> bool {
        let fresh = |start: u64| now.saturating_sub(start) < FX_DUR;
        self.item_fx.iter().any(|f| fresh(f.start))
            || self.ghost.as_ref().is_some_and(|g| fresh(g.start))
            || self.sel_from.is_some_and(|(_, s)| fresh(s))
            || self.scroll_from.is_some_and(|(_, s)| fresh(s))
    }

    pub fn prune(&mut self, now: u64) -> bool {
        let had = !self.item_fx.is_empty()
            || self.ghost.is_some()
            || self.sel_from.is_some()
            || self.scroll_from.is_some();
        let fresh = |start: u64| now.saturating_sub(start) < FX_DUR;
        self.item_fx.retain(|f| fresh(f.start));
        if self.ghost.as_ref().is_some_and(|g| !fresh(g.start)) {
            self.ghost = None;
        }
        if self.sel_from.is_some_and(|(_, s)| !fresh(s)) {
            self.sel_from = None;
        }
        if self.scroll_from.is_some_and(|(_, s)| !fresh(s)) {
            self.scroll_from = None;
        }
        let has = !self.item_fx.is_empty()
            || self.ghost.is_some()
            || self.sel_from.is_some()
            || self.scroll_from.is_some();
        had && !has // 有内容被清理
    }
}

fn lerp_t(start: u64, now: u64) -> f32 {
    (now.saturating_sub(start) as f32 / FX_DUR as f32).clamp(0.0, 1.0)
}

pub(crate) fn draw(items: &[String], selected: usize, scroll: i32, fx: &ListFx, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let now = ctx.now;
    let lclip = abs.intersect(&clip).unwrap_or(clip);

    // 有效 scroll（平滑滚动插值）
    let eff_scroll = match fx.scroll_from {
        Some((from, start)) => from + ((scroll - from) as f32 * lerp_t(start, now)) as i32,
        None => scroll,
    };
    // 高亮行位置（滑动插值，单位：行）
    let hl_row_f = match fx.sel_from {
        Some((from, start)) => {
            let t = lerp_t(start, now);
            from as f32 * (1.0 - t) + selected as f32 * t
        }
        None => selected as f32,
    };
    if !items.is_empty() {
        let hl = Rect::new(abs.x, abs.y + (hl_row_f * ROW_H as f32) as i32 - eff_scroll, abs.w, ROW_H);
        if hl.intersects(&lclip) {
            // 高亮带圆角，避免覆盖列表自身的圆角边框
            d.fill_rounded(hl, ctx.resolved.radius.min(ROW_H / 2), Color::rgb(50, 70, 120), ctx.ap(255), lclip);
        }
    }
    // items（带入场/位移效果）
    for (i, item) in items.iter().enumerate() {
        let mut dy = 0;
        let mut opa = ctx.ap(255);
        for f in &fx.item_fx {
            if f.index == i {
                let t = lerp_t(f.start, now);
                dy = (f.dy as f32 * (1.0 - t)) as i32;
                if f.fade_in {
                    opa = ctx.ap((255.0 * t) as u8);
                }
            }
        }
        let ry = abs.y + i as i32 * ROW_H + dy - eff_scroll;
        let row = Rect::new(abs.x, ry, abs.w, ROW_H);
        if !row.intersects(&lclip) {
            continue;
        }
        d.draw_text_opa(Point { x: abs.x + 4, y: ry + 4 }, item, ctx.resolved.text_color, opa, lclip);
    }
    // 删除中的 ghost 渐隐
    if let Some(g) = &fx.ghost {
        let t = lerp_t(g.start, now);
        let ry = abs.y + g.index as i32 * ROW_H - eff_scroll;
        let row = Rect::new(abs.x, ry, abs.w, ROW_H);
        if row.intersects(&lclip) {
            d.draw_text_opa(
                Point { x: abs.x + 4, y: ry + 4 },
                &g.text,
                ctx.resolved.text_color,
                ctx.ap((255.0 * (1.0 - t)) as u8),
                lclip,
            );
        }
    }
}

/// 选中第 idx 项（记录高亮滑动/平滑滚动效果）并调整 scroll 保证可见。
/// scroll 始终按行对齐（ROW_H 整数倍），避免半行错位。
pub(crate) fn select(items: &[String], selected: &mut usize, scroll: &mut i32, fx: &mut ListFx, idx: usize, vis_h: i32, now: u64) {
    if items.is_empty() {
        return;
    }
    let nidx = idx.min(items.len() - 1);
    if nidx != *selected {
        fx.sel_from = Some((*selected, now));
        *selected = nidx;
    }
    ensure_visible(*selected, items.len(), scroll, fx, vis_h, now);
}

/// 调整 scroll：保证 selected 可见，且尾部不留空窗（删除尾部项后自动上滚）。
/// scroll 按行对齐；变化时记录平滑滚动效果。
pub(crate) fn ensure_visible(selected: usize, item_count: usize, scroll: &mut i32, fx: &mut ListFx, vis_h: i32, now: u64) {
    let old = *scroll;
    if item_count == 0 {
        *scroll = 0;
        if old != 0 {
            fx.scroll_from = Some((old, now));
        }
        return;
    }
    let vis_rows = (vis_h / ROW_H).max(1);
    let count = item_count as i32;
    let sel = selected as i32;
    let mut first = *scroll / ROW_H; // 当前首个可见行
    // 尾部空窗：向上收
    if first + vis_rows > count {
        first = (count - vis_rows).max(0);
    }
    if sel < first {
        first = sel;
    } else if sel >= first + vis_rows {
        first = sel - vis_rows + 1;
    }
    *scroll = first * ROW_H;
    if *scroll != old {
        fx.scroll_from = Some((old, now));
    }
}

/// 在 idx 处插入一项：下方 item 下滑让位，新项淡入。
/// （容量上限属于业务策略，由调用方控制，控件本身不限制）
pub(crate) fn insert(items: &mut Vec<String>, fx: &mut ListFx, idx: usize, text: &str, now: u64) {
    let idx = idx.min(items.len());
    items.insert(idx, text.into());
    // 进行中的 fx 索引顺延
    for f in fx.item_fx.iter_mut() {
        if f.index >= idx {
            f.index += 1;
        }
    }
    // 下方 item 从旧位置（上一行）滑入新位置
    for i in (idx + 1)..items.len() {
        fx.item_fx.push(ItemFx { index: i, dy: -ROW_H, fade_in: false, start: now });
    }
    fx.item_fx.push(ItemFx { index: idx, dy: 0, fade_in: true, start: now });
}

/// 删除选中项：ghost 渐隐，下方 item 上移补位
pub(crate) fn remove(items: &mut Vec<String>, fx: &mut ListFx, selected: &mut usize, now: u64) -> bool {
    if items.is_empty() || *selected >= items.len() {
        return false;
    }
    let text = items.remove(*selected);
    fx.ghost = Some(Ghost { text, index: *selected, start: now });
    // 进行中的 fx：被删项丢弃，下方顺延
    fx.item_fx.retain(|f| f.index != *selected);
    for f in fx.item_fx.iter_mut() {
        if f.index > *selected {
            f.index -= 1;
        }
    }
    // 下方 item 从旧位置（下一行）滑入新位置
    for i in *selected..items.len() {
        fx.item_fx.push(ItemFx { index: i, dy: ROW_H, fade_in: false, start: now });
    }
    if *selected >= items.len() && *selected > 0 {
        *selected -= 1;
    }
    true
}

/// List 构建器：默认 120 x (min(5,n)*16+2)，theme_list/focused
pub struct ListBuilder {
    items: Vec<String>,
    selected: usize,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_focused: Option<Style>,
    sizing: Option<(Option<crate::layout::Sizing>, Option<crate::layout::Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl ListBuilder {
    pub fn new(items: &[&str]) -> Self {
        Self {
            items: items.iter().map(|s| (*s).into()).collect(),
            selected: 0,
            size: None, style: None, style_focused: None,
            sizing: None, transition: None, events: Vec::new(),
        }
    }
    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = idx;
        self
    }
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.style = Some(f(self.style.unwrap_or_else(crate::style::theme_list)));
        self
    }
    pub fn style_focused(mut self, s: Style) -> Self {
        self.style_focused = Some(s);
        self
    }
    pub fn sizing(mut self, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) -> Self {
        self.sizing = Some((w, h));
        self
    }
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let rows = self.items.len().min(5).max(1) as i32;
        let (w, h) = self.size.unwrap_or((120, rows * ROW_H + 2));
        let selected = if self.items.is_empty() { 0 } else { self.selected.min(self.items.len() - 1) };
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::List(ListState { items: self.items, selected, scroll: 0, fx: ListFx::default() }),
        );
        ui.set_style(r, self.style.unwrap_or_else(crate::style::theme_list));
        ui.set_style_focused(r, self.style_focused.unwrap_or_else(crate::style::theme_list_focused));
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

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, items: &[&str]) -> ObjRef {
    ListBuilder::new(items).build(ui, parent)
}
