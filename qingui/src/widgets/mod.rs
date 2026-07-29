use alloc::string::String;
use alloc::vec::Vec;

use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::style::ResolvedStyle;

pub mod arc;
pub mod bar;
pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod dropdown;
pub mod label;
pub mod led;
pub mod list;
pub mod msgbox;
pub mod roller;
pub mod slider;
pub mod spinbox;
pub mod spinner;
pub mod switch;
pub mod table;

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
    Arc { min: i32, max: i32, value: i32 },
    Checkbox { text: String, checked: bool },
    Spinner,
    Msgbox { selected: i32 },
    Led { color: crate::geometry::Color, bright: u8 },
    Table { cols: u8, rows: u8, cells: Vec<String> },
    Spinbox { min: i32, max: i32, value: i32, digits: u8, cursor: u8 },
    Roller { items: Vec<String>, selected: usize, sel_from: Option<(f32, u64)> },
    Dropdown { items: Vec<String>, selected: usize },
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
        WidgetKind::Arc { min, max, value } => arc::draw(*min, *max, *value, ctx, d, clip),
        WidgetKind::Checkbox { text, checked } => checkbox::draw(text, *checked, ctx, d, clip),
        WidgetKind::Spinner => spinner::draw(ctx, d, clip),
        // Msgbox 是普通容器（子对象正常绘制）
        WidgetKind::Msgbox { .. } => {}
        WidgetKind::Led { color, bright } => led::draw(*color, *bright, ctx, d, clip),
        WidgetKind::Table { cols, rows, cells } => table::draw(*cols, *rows, cells, ctx, d, clip),
        WidgetKind::Spinbox { min, max, value, digits, cursor } => spinbox::draw(*min, *max, *value, *digits, *cursor, ctx, d, clip),
        WidgetKind::Roller { items, selected, sel_from } => roller::draw(items, *selected, *sel_from, ctx, d, clip),
        WidgetKind::Dropdown { items, selected } => dropdown::draw(items, *selected, ctx, d, clip),
    }
}

/// 控件绘制超出自身矩形的最大距离（用于标脏外扩，对齐 LVGL ext_draw_size）
pub(crate) fn overflow_of(kind: &WidgetKind) -> i32 {
    match kind {
        // Slider 旋钮 ±4px 横向 ±2px 纵向；Arc 旋钮超出边缘 ~3px
        WidgetKind::Slider { .. } | WidgetKind::Arc { .. } => 4,
        _ => 0,
    }
}

/// 控件的当前值（Switch/Checkbox：on=1/off=0；Roller/Dropdown：选中索引；无值控件返回 0）
pub(crate) fn value_of(kind: &WidgetKind) -> i32 {
    match kind {
        WidgetKind::Slider { value, .. } | WidgetKind::Bar { value, .. } | WidgetKind::Arc { value, .. } => *value,
        WidgetKind::Switch { on } => *on as i32,
        WidgetKind::Checkbox { checked, .. } => *checked as i32,
        WidgetKind::Spinbox { value, .. } => *value,
        WidgetKind::Led { bright, .. } => *bright as i32,
        WidgetKind::Roller { selected, .. } | WidgetKind::Dropdown { selected, .. } => *selected as i32,
        _ => 0,
    }
}

/// 设置控件值（clamp 到 range），返回是否有变化
pub(crate) fn set_value_of(kind: &mut WidgetKind, v: i32) -> bool {
    match kind {
        WidgetKind::Slider { min, max, value } | WidgetKind::Bar { min, max, value } | WidgetKind::Arc { min, max, value } => {
            let nv = v.clamp(*min, *max);
            let changed = nv != *value;
            *value = nv;
            changed
        }
        WidgetKind::Checkbox { checked, .. } => {
            let nv = v != 0;
            let changed = nv != *checked;
            *checked = nv;
            changed
        }
        WidgetKind::Spinbox { min, max, value, .. } => {
            let nv = v.clamp(*min, *max);
            let changed = nv != *value;
            *value = nv;
            changed
        }
        WidgetKind::Led { bright, .. } => {
            let nv = v.clamp(0, 255) as u8;
            let changed = nv != *bright;
            *bright = nv;
            changed
        }
        WidgetKind::Roller { items, selected, .. } | WidgetKind::Dropdown { items, selected, .. } => {
            if items.is_empty() {
                false
            } else {
                let nv = (v.max(0) as usize).min(items.len() - 1);
                let changed = nv != *selected;
                *selected = nv;
                changed
            }
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
