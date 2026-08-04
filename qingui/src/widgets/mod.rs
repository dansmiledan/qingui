use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::input::Key;
use crate::style::ResolvedStyle;
use crate::arena::ObjRef;
use crate::ui::Ui;

pub mod arc;
pub mod bar;
pub mod button;
pub mod canvas;
pub mod chart;
pub mod checkbox;
pub mod custom;
pub mod dropdown;
pub mod image;
pub mod itemlist;
pub mod label;
pub mod led;
pub mod list;
pub mod msgbox;
pub mod obj;
pub mod roller;
pub mod scrollview;
pub mod slider;
pub mod spinbox;
pub mod spinner;
pub mod switch;
pub mod table;

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
    /// 特异副作用延迟执行：widget 文件提供的静态执行函数 + i32 载荷。
    /// Ui 在把 kind 放回 arena 后调用 f(self, obj, p)（干净窗口，无占位），视为已消费。
    Deferred(fn(&mut Ui, ObjRef, i32), i32),
    /// 滚动容器滚动(步进 ±px),由 Ui 执行(clamp + translate)
    ScrollBy(i32),
}

/// 控件行为接口:draw 必须实现(新 widget 忘了画会编译错),
/// 其余行为大多数控件没有,给默认空实现。
pub(crate) trait WidgetBehavior {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect);
    fn tick(&mut self, _now: u64) -> TickOut { TickOut::IDLE }
    fn on_key(&mut self, _key: Key, _ctx: KeyCtx) -> KeyOutcome { KeyOutcome::Pass }
    fn value(&self) -> i32 { 0 }
    fn set_value(&mut self, _v: i32) -> bool { false }
    fn set_range(&mut self, _min: i32, _max: i32) {}
    fn overflow(&self) -> i32 { 0 }
}

/// set_value 共用:clamp 到 [min,max],返回是否有变化
pub(crate) fn clamp_val(min: i32, max: i32, value: &mut i32, v: i32) -> bool {
    let nv = v.clamp(min, max);
    let changed = nv != *value;
    *value = nv;
    changed
}

/// 选择型控件共用:clamp 到 [0,len),返回是否有变化
pub(crate) fn select_clamp(len: usize, selected: &mut usize, v: i32) -> bool {
    if len == 0 { return false; }
    let nv = (v.max(0) as usize).min(len - 1);
    let changed = nv != *selected;
    *selected = nv;
    changed
}

/// 声明式注册 widget:生成 enum、行为分发、as_xxx 访问器、downcast。
/// 每加一个 widget 只需在此处加一行。
macro_rules! define_widgets {
    ($($variant:ident($state:ty, $as:ident, $as_mut:ident)),+ $(,)?) => {
        pub enum WidgetKind {
            $( $variant($state), )+
        }

        impl WidgetKind {
            pub(crate) fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::draw(s, ctx, d, clip), )+ }
            }
            pub(crate) fn overflow(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::overflow(s), )+ }
            }
            pub(crate) fn value(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::value(s), )+ }
            }
            pub(crate) fn set_value(&mut self, v: i32) -> bool {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_value(s, v), )+ }
            }
            pub(crate) fn set_range(&mut self, min: i32, max: i32) {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_range(s, min, max), )+ }
            }
            pub(crate) fn tick(&mut self, now: u64) -> TickOut {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::tick(s, now), )+ }
            }
            pub(crate) fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::on_key(s, key, ctx), )+ }
            }
            $(
                pub fn $as(&self) -> Option<&$state> {
                    match self { WidgetKind::$variant(s) => Some(s), _ => None }
                }
                pub fn $as_mut(&mut self) -> Option<&mut $state> {
                    match self { WidgetKind::$variant(s) => Some(s), _ => None }
                }
            )+
            /// 按类型下发 &mut 状态(Ui::update 用);TypeId 比对 + Any downcast
            pub(crate) fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
                $(
                    if core::any::TypeId::of::<T>() == core::any::TypeId::of::<$state>() {
                        if let WidgetKind::$variant(s) = self {
                            return (s as &mut dyn core::any::Any).downcast_mut::<T>();
                        }
                    }
                )+
                None
            }
        }
    };
}

define_widgets! {
    Obj(obj::ObjState, as_obj, as_obj_mut),
    ItemList(itemlist::ItemListState, as_itemlist, as_itemlist_mut),
    Label(label::LabelState, as_label, as_label_mut),
    Button(button::ButtonState, as_button, as_button_mut),
    Slider(slider::SliderState, as_slider, as_slider_mut),
    Switch(switch::SwitchState, as_switch, as_switch_mut),
    Bar(bar::BarState, as_bar, as_bar_mut),
    List(list::ListState, as_list, as_list_mut),
    Arc(arc::ArcState, as_arc, as_arc_mut),
    Checkbox(checkbox::CheckboxState, as_checkbox, as_checkbox_mut),
    Chart(chart::ChartState, as_chart, as_chart_mut),
    Spinner(spinner::SpinnerState, as_spinner, as_spinner_mut),
    Msgbox(msgbox::MsgboxState, as_msgbox, as_msgbox_mut),
    Led(led::LedState, as_led, as_led_mut),
    Table(table::TableState, as_table, as_table_mut),
    Spinbox(spinbox::SpinboxState, as_spinbox, as_spinbox_mut),
    Roller(roller::RollerState, as_roller, as_roller_mut),
    ScrollView(scrollview::ScrollViewState, as_scrollview, as_scrollview_mut),
    Dropdown(dropdown::DropdownState, as_dropdown, as_dropdown_mut),
    Image(image::ImageState, as_image, as_image_mut),
    Custom(custom::CustomState, as_custom_state, as_custom_state_mut),
}

impl WidgetKind {
    pub(crate) fn as_custom(&self) -> Option<&dyn custom::Widget> {
        match self { WidgetKind::Custom(s) => Some(s.0.as_ref()), _ => None }
    }
    pub(crate) fn as_custom_mut(&mut self) -> Option<&mut dyn custom::Widget> {
        match self { WidgetKind::Custom(s) => Some(s.0.as_mut()), _ => None }
    }
}
