use alloc::string::String;
use alloc::vec::Vec;

use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::style::ResolvedStyle;

pub mod bar;
pub mod button;
pub mod canvas;
pub mod label;
pub mod list;
pub mod slider;
pub mod switch;

#[derive(Clone)]
pub enum WidgetKind {
    Obj,
    Label { text: String },
    Button { text: String },
    Slider { min: i32, max: i32, value: i32 },
    Switch { on: bool },
    Bar { min: i32, max: i32, value: i32 },
    List { items: Vec<String>, selected: usize, scroll: i32, fx: list::ListFx },
    /// 自定义绘制控件：cb 为 Ui 回调注册表中的索引（回调本身不可 Clone，故存索引）
    Canvas { cb: usize },
}

/// 控件绘制上下文：通用部分（背景/边框）由 Ui::draw_node 处理，
/// 各控件 draw 只画自己的内容。
pub struct WidgetCtx<'a> {
    pub abs: Rect,
    pub resolved: &'a ResolvedStyle,
    pub edited: bool,
    pub opa: u8, // node opa 0..=255
    pub now: u64, // 当前时间（ms），供控件内部效果插值
}

impl WidgetCtx<'_> {
    /// 与节点 opa 合成后的透明度
    pub fn ap(&self, base: u8) -> u8 {
        (base as u32 * self.opa as u32 / 255) as u8
    }
}

pub(crate) fn draw(kind: &WidgetKind, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    match kind {
        WidgetKind::Obj => {}
        WidgetKind::Label { text } => label::draw(text, ctx, d, clip),
        WidgetKind::Button { text } => button::draw(text, ctx, d, clip),
        WidgetKind::Slider { min, max, value } => slider::draw(*min, *max, *value, ctx, d, clip),
        WidgetKind::Switch { on } => switch::draw(*on, ctx, d, clip),
        WidgetKind::Bar { min, max, value } => bar::draw(*min, *max, *value, ctx, d, clip),
        WidgetKind::List { items, selected, scroll, fx } => list::draw(items, *selected, *scroll, fx, ctx, d, clip),
        // Canvas 由 Ui::draw_node 单独处理（回调在 Ui 的注册表中）
        WidgetKind::Canvas { .. } => {}
    }
}

/// 控件的当前值（Switch：on=1/off=0；无值控件返回 0）
pub(crate) fn value_of(kind: &WidgetKind) -> i32 {
    match kind {
        WidgetKind::Slider { value, .. } | WidgetKind::Bar { value, .. } => *value,
        WidgetKind::Switch { on } => *on as i32,
        _ => 0,
    }
}

/// 设置控件值（clamp 到 range），返回是否有变化
pub(crate) fn set_value_of(kind: &mut WidgetKind, v: i32) -> bool {
    match kind {
        WidgetKind::Slider { min, max, value } | WidgetKind::Bar { min, max, value } => {
            let nv = v.clamp(*min, *max);
            let changed = nv != *value;
            *value = nv;
            changed
        }
        _ => false,
    }
}

/// 设置控件 range（值随之 clamp）
pub(crate) fn set_range_of(kind: &mut WidgetKind, min: i32, max: i32) {
    if let WidgetKind::Slider { min: mn, max: mx, value } | WidgetKind::Bar { min: mn, max: mx, value } = kind {
        *mn = min;
        *mx = max;
        *value = (*value).clamp(min, max);
    }
}
