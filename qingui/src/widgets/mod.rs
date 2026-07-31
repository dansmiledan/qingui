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
    Label(label::LabelState),
    Button(button::ButtonState),
    Slider(slider::SliderState),
    Switch(switch::SwitchState),
    Bar(bar::BarState),
    List(list::ListState),
    /// 自定义绘制控件：cb 为 Ui 回调注册表中的索引（Task 5 删除）
    Canvas { cb: usize },
    Arc(arc::ArcState),
    Checkbox(checkbox::CheckboxState),
    Spinner,
    Msgbox(msgbox::MsgboxState),
    Led(led::LedState),
    Table(table::TableState),
    Spinbox(spinbox::SpinboxState),
    Roller(roller::RollerState),
    Dropdown(dropdown::DropdownState),
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
        WidgetKind::Label(s) => label::draw(&s.text, ctx, d, clip),
        WidgetKind::Button(s) => button::draw(&s.text, ctx, d, clip),
        WidgetKind::Slider(s) => slider::draw(s.min, s.max, s.value, ctx, d, clip),
        WidgetKind::Switch(s) => switch::draw(s.on, ctx, d, clip),
        WidgetKind::Bar(s) => bar::draw(s.min, s.max, s.value, ctx, d, clip),
        WidgetKind::List(s) => list::draw(&s.items, s.selected, s.scroll, &s.fx, ctx, d, clip),
        // Canvas 由 Ui::draw_node 单独处理（回调在 Ui 的注册表中）
        WidgetKind::Canvas { .. } => {}
        WidgetKind::Arc(s) => arc::draw(s.min, s.max, s.value, ctx, d, clip),
        WidgetKind::Checkbox(s) => checkbox::draw(&s.text, s.checked, ctx, d, clip),
        WidgetKind::Spinner => spinner::draw(ctx, d, clip),
        // Msgbox 是普通容器（子对象正常绘制）
        WidgetKind::Msgbox(_) => {}
        WidgetKind::Led(s) => led::draw(s.color, s.bright, ctx, d, clip),
        WidgetKind::Table(s) => table::draw(s.cols, s.rows, &s.cells, ctx, d, clip),
        WidgetKind::Spinbox(s) => spinbox::draw(s.min, s.max, s.value, s.digits, s.cursor, ctx, d, clip),
        WidgetKind::Roller(s) => roller::draw(&s.items, s.selected, s.sel_from, ctx, d, clip),
        WidgetKind::Dropdown(s) => dropdown::draw(&s.items, s.selected, ctx, d, clip),
    }
}

/// 控件绘制超出自身矩形的最大距离（用于标脏外扩，对齐 LVGL ext_draw_size）
pub(crate) fn overflow_of(kind: &WidgetKind) -> i32 {
    match kind {
        // Slider 旋钮 ±4px 横向 ±2px 纵向；Arc 旋钮超出边缘 ~3px
        WidgetKind::Slider(_) | WidgetKind::Arc(_) => 4,
        _ => 0,
    }
}

/// 控件的当前值（Switch/Checkbox：on=1/off=0；Roller/Dropdown：选中索引；无值控件返回 0）
pub(crate) fn value_of(kind: &WidgetKind) -> i32 {
    match kind {
        WidgetKind::Slider(s) => s.value,
        WidgetKind::Bar(s) => s.value,
        WidgetKind::Arc(s) => s.value,
        WidgetKind::Switch(s) => s.on as i32,
        WidgetKind::Checkbox(s) => s.checked as i32,
        WidgetKind::Spinbox(s) => s.value,
        WidgetKind::Led(s) => s.bright as i32,
        WidgetKind::Roller(s) => s.selected as i32,
        WidgetKind::Dropdown(s) => s.selected as i32,
        _ => 0,
    }
}

/// 设置控件值（clamp 到 range），返回是否有变化
pub(crate) fn set_value_of(kind: &mut WidgetKind, v: i32) -> bool {
    match kind {
        WidgetKind::Slider(s) => {
            let nv = v.clamp(s.min, s.max);
            let changed = nv != s.value;
            s.value = nv;
            changed
        }
        WidgetKind::Bar(s) => {
            let nv = v.clamp(s.min, s.max);
            let changed = nv != s.value;
            s.value = nv;
            changed
        }
        WidgetKind::Arc(s) => {
            let nv = v.clamp(s.min, s.max);
            let changed = nv != s.value;
            s.value = nv;
            changed
        }
        WidgetKind::Checkbox(s) => {
            let nv = v != 0;
            let changed = nv != s.checked;
            s.checked = nv;
            changed
        }
        WidgetKind::Spinbox(s) => {
            let nv = v.clamp(s.min, s.max);
            let changed = nv != s.value;
            s.value = nv;
            changed
        }
        WidgetKind::Led(s) => {
            let nv = v.clamp(0, 255) as u8;
            let changed = nv != s.bright;
            s.bright = nv;
            changed
        }
        WidgetKind::Roller(s) => {
            if s.items.is_empty() {
                false
            } else {
                let nv = (v.max(0) as usize).min(s.items.len() - 1);
                let changed = nv != s.selected;
                s.selected = nv;
                changed
            }
        }
        WidgetKind::Dropdown(s) => {
            if s.items.is_empty() {
                false
            } else {
                let nv = (v.max(0) as usize).min(s.items.len() - 1);
                let changed = nv != s.selected;
                s.selected = nv;
                changed
            }
        }
        _ => false,
    }
}

/// 设置控件 range（值随之 clamp）
pub(crate) fn set_range_of(kind: &mut WidgetKind, min: i32, max: i32) {
    match kind {
        WidgetKind::Slider(s) => {
            s.min = min;
            s.max = max;
            s.value = s.value.clamp(min, max);
        }
        WidgetKind::Bar(s) => {
            s.min = min;
            s.max = max;
            s.value = s.value.clamp(min, max);
        }
        _ => {}
    }
}
