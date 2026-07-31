use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::input::Key;
use crate::style::ResolvedStyle;

pub mod arc;
pub mod bar;
pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod custom;
pub mod dropdown;
pub mod label;
pub mod led;
pub mod list;
pub mod msgbox;
pub mod obj;
pub mod roller;
pub mod slider;
pub mod spinbox;
pub mod spinner;
pub mod switch;
pub mod table;

pub enum WidgetKind {
    Obj,
    Label(label::LabelState),
    Button(button::ButtonState),
    Slider(slider::SliderState),
    Switch(switch::SwitchState),
    Bar(bar::BarState),
    List(list::ListState),
    Arc(arc::ArcState),
    Checkbox(checkbox::CheckboxState),
    Spinner,
    Msgbox(msgbox::MsgboxState),
    Led(led::LedState),
    Table(table::TableState),
    Spinbox(spinbox::SpinboxState),
    Roller(roller::RollerState),
    Dropdown(dropdown::DropdownState),
    /// 用户自定义 widget（逃生舱；不可 Clone，故 WidgetKind 不再 derive Clone）
    Custom(alloc::boxed::Box<dyn custom::Widget>),
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

/// 每帧效果推进结果：redraw = 本帧需重绘；active = 效果仍活动（保持唤醒）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TickOut {
    pub redraw: bool,
    pub active: bool,
}

impl TickOut {
    pub const IDLE: Self = Self { redraw: false, active: false };
    pub const ACTIVE: Self = Self { redraw: true, active: true };
}

/// 按键处理上下文（由 Ui 从节点/自身状态收集后传入）
pub(crate) struct KeyCtx {
    pub edited: bool, // 节点处于 EDITED 态
    pub vis_h: i32,   // 节点可视高度（滚动控件用）
    pub now: u64,
}

/// 按键处理结果：Ui 据此执行通用副作用（标脏/事件/EDITED 态/开下拉）
pub(crate) enum KeyOutcome {
    Pass,          // 未消费 → 走默认（移焦/Clicked）
    Consumed,      // 已消费，标脏
    ValueChanged,  // 已消费，标脏并发 ValueChanged 事件
    EnterEdit,     // 进入 EDITED 态
    ExitEdit,      // 退出 EDITED 态并标脏
    OpenDropdown,  // 打开下拉浮层
}

impl WidgetKind {
    pub(crate) fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
        match self {
            WidgetKind::Obj => {}
            WidgetKind::Label(s) => label::draw(&s.text, ctx, d, clip),
            WidgetKind::Button(s) => button::draw(&s.text, ctx, d, clip),
            WidgetKind::Slider(s) => slider::draw(s.min, s.max, s.value, ctx, d, clip),
            WidgetKind::Switch(s) => switch::draw(s.on, ctx, d, clip),
            WidgetKind::Bar(s) => bar::draw(s.min, s.max, s.value, ctx, d, clip),
            WidgetKind::List(s) => list::draw(&s.items, s.selected, s.scroll, &s.fx, ctx, d, clip),
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
            WidgetKind::Custom(w) => w.draw(ctx, d, clip),
        }
    }

    /// 控件绘制超出自身矩形的最大距离（用于标脏外扩，对齐 LVGL ext_draw_size）
    pub(crate) fn overflow(&self) -> i32 {
        match self {
            // Slider 旋钮 ±4px 横向 ±2px 纵向；Arc 旋钮超出边缘 ~3px
            WidgetKind::Slider(_) | WidgetKind::Arc(_) => 4,
            _ => 0,
        }
    }

    /// 控件的当前值（Switch/Checkbox：on=1/off=0；Roller/Dropdown：选中索引；无值控件返回 0）
    pub(crate) fn value(&self) -> i32 {
        match self {
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
    pub(crate) fn set_value(&mut self, v: i32) -> bool {
        fn clamp_val(min: i32, max: i32, value: &mut i32, v: i32) -> bool {
            let nv = v.clamp(min, max);
            let changed = nv != *value;
            *value = nv;
            changed
        }
        fn select_clamp(len: usize, selected: &mut usize, v: i32) -> bool {
            if len == 0 { return false; }
            let nv = (v.max(0) as usize).min(len - 1);
            let changed = nv != *selected;
            *selected = nv;
            changed
        }
        match self {
            WidgetKind::Slider(s) => clamp_val(s.min, s.max, &mut s.value, v),
            WidgetKind::Bar(s) => clamp_val(s.min, s.max, &mut s.value, v),
            WidgetKind::Arc(s) => clamp_val(s.min, s.max, &mut s.value, v),
            WidgetKind::Spinbox(s) => clamp_val(s.min, s.max, &mut s.value, v),
            WidgetKind::Checkbox(s) => {
                let nv = v != 0;
                let c = nv != s.checked;
                s.checked = nv;
                c
            }
            WidgetKind::Led(s) => {
                let nv = v.clamp(0, 255) as u8;
                let c = nv != s.bright;
                s.bright = nv;
                c
            }
            WidgetKind::Roller(s) => select_clamp(s.items.len(), &mut s.selected, v),
            WidgetKind::Dropdown(s) => select_clamp(s.items.len(), &mut s.selected, v),
            _ => false,
        }
    }

    /// 设置控件 range（值随之 clamp）
    pub(crate) fn set_range(&mut self, min: i32, max: i32) {
        match self {
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

    /// 每帧效果推进（fx/自转）。默认无逐帧行为。
    pub(crate) fn tick(&mut self, now: u64) -> TickOut {
        match self {
            WidgetKind::List(s) => s.tick(now),
            WidgetKind::Roller(s) => s.tick(now),
            // Spinner 永远自转
            WidgetKind::Spinner => TickOut::ACTIVE,
            WidgetKind::Custom(w) => w.tick(now),
            _ => TickOut::IDLE,
        }
    }

    /// 按键处理（无 &mut Ui：只改自身状态，副作用由 Ui 按 KeyOutcome 执行）
    pub(crate) fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
        match self {
            WidgetKind::Slider(s) => s.on_key(key, ctx),
            WidgetKind::Spinbox(s) => s.on_key(key, ctx),
            WidgetKind::Switch(s) => s.on_key(key, ctx),
            WidgetKind::Checkbox(s) => s.on_key(key, ctx),
            WidgetKind::List(s) => s.on_key(key, ctx),
            WidgetKind::Roller(s) => s.on_key(key, ctx),
            WidgetKind::Dropdown(s) => s.on_key(key, ctx),
            _ => KeyOutcome::Pass,
        }
    }

    pub fn as_list(&self) -> Option<&list::ListState> {
        match self { WidgetKind::List(s) => Some(s), _ => None }
    }
    pub fn as_list_mut(&mut self) -> Option<&mut list::ListState> {
        match self { WidgetKind::List(s) => Some(s), _ => None }
    }
    pub fn as_roller(&self) -> Option<&roller::RollerState> {
        match self { WidgetKind::Roller(s) => Some(s), _ => None }
    }
    pub fn as_roller_mut(&mut self) -> Option<&mut roller::RollerState> {
        match self { WidgetKind::Roller(s) => Some(s), _ => None }
    }
    pub fn as_dropdown(&self) -> Option<&dropdown::DropdownState> {
        match self { WidgetKind::Dropdown(s) => Some(s), _ => None }
    }
    pub fn as_dropdown_mut(&mut self) -> Option<&mut dropdown::DropdownState> {
        match self { WidgetKind::Dropdown(s) => Some(s), _ => None }
    }
    pub fn as_table(&self) -> Option<&table::TableState> {
        match self { WidgetKind::Table(s) => Some(s), _ => None }
    }
    pub fn as_table_mut(&mut self) -> Option<&mut table::TableState> {
        match self { WidgetKind::Table(s) => Some(s), _ => None }
    }
    pub fn as_checkbox(&self) -> Option<&checkbox::CheckboxState> {
        match self { WidgetKind::Checkbox(s) => Some(s), _ => None }
    }
    pub fn as_checkbox_mut(&mut self) -> Option<&mut checkbox::CheckboxState> {
        match self { WidgetKind::Checkbox(s) => Some(s), _ => None }
    }
    pub fn as_switch(&self) -> Option<&switch::SwitchState> {
        match self { WidgetKind::Switch(s) => Some(s), _ => None }
    }
    pub fn as_switch_mut(&mut self) -> Option<&mut switch::SwitchState> {
        match self { WidgetKind::Switch(s) => Some(s), _ => None }
    }
    pub fn as_msgbox(&self) -> Option<&msgbox::MsgboxState> {
        match self { WidgetKind::Msgbox(s) => Some(s), _ => None }
    }
    pub fn as_msgbox_mut(&mut self) -> Option<&mut msgbox::MsgboxState> {
        match self { WidgetKind::Msgbox(s) => Some(s), _ => None }
    }
    pub fn as_spinbox(&self) -> Option<&spinbox::SpinboxState> {
        match self { WidgetKind::Spinbox(s) => Some(s), _ => None }
    }
    pub fn as_spinbox_mut(&mut self) -> Option<&mut spinbox::SpinboxState> {
        match self { WidgetKind::Spinbox(s) => Some(s), _ => None }
    }
    pub fn as_label(&self) -> Option<&label::LabelState> {
        match self { WidgetKind::Label(s) => Some(s), _ => None }
    }
    pub fn as_label_mut(&mut self) -> Option<&mut label::LabelState> {
        match self { WidgetKind::Label(s) => Some(s), _ => None }
    }
    pub fn as_button(&self) -> Option<&button::ButtonState> {
        match self { WidgetKind::Button(s) => Some(s), _ => None }
    }
    pub fn as_button_mut(&mut self) -> Option<&mut button::ButtonState> {
        match self { WidgetKind::Button(s) => Some(s), _ => None }
    }
    pub fn as_led(&self) -> Option<&led::LedState> {
        match self { WidgetKind::Led(s) => Some(s), _ => None }
    }
    pub fn as_led_mut(&mut self) -> Option<&mut led::LedState> {
        match self { WidgetKind::Led(s) => Some(s), _ => None }
    }
    pub fn as_slider(&self) -> Option<&slider::SliderState> {
        match self { WidgetKind::Slider(s) => Some(s), _ => None }
    }
    pub fn as_slider_mut(&mut self) -> Option<&mut slider::SliderState> {
        match self { WidgetKind::Slider(s) => Some(s), _ => None }
    }
    pub fn as_bar(&self) -> Option<&bar::BarState> {
        match self { WidgetKind::Bar(s) => Some(s), _ => None }
    }
    pub fn as_bar_mut(&mut self) -> Option<&mut bar::BarState> {
        match self { WidgetKind::Bar(s) => Some(s), _ => None }
    }
    pub fn as_arc(&self) -> Option<&arc::ArcState> {
        match self { WidgetKind::Arc(s) => Some(s), _ => None }
    }
    pub fn as_arc_mut(&mut self) -> Option<&mut arc::ArcState> {
        match self { WidgetKind::Arc(s) => Some(s), _ => None }
    }
    pub(crate) fn as_custom(&self) -> Option<&dyn custom::Widget> {
        match self { WidgetKind::Custom(w) => Some(w.as_ref()), _ => None }
    }
    pub(crate) fn as_custom_mut(&mut self) -> Option<&mut dyn custom::Widget> {
        match self { WidgetKind::Custom(w) => Some(w.as_mut()), _ => None }
    }
}
