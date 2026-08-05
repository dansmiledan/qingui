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

/// Widget drawing context: the common parts (background/border) are handled by `Ui::draw_node`,
/// each widget's `draw` only paints its own content.
pub struct WidgetCtx<'a> {
    pub abs: Rect,
    pub resolved: &'a ResolvedStyle,
    pub edited: bool,
    pub opa: u8, // node opa 0..=255
    pub now: u64, // current time (ms), for interpolating internal widget effects
}

impl WidgetCtx<'_> {
    /// Opacity after compositing with the node's opa
    pub fn ap(&self, base: u8) -> u8 {
        (base as u32 * self.opa as u32 / 255) as u8
    }
}

/// Per-frame effect progress result: `redraw` = needs repaint this frame; `active` = effect still running (keeps the widget awake)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TickOut {
    pub redraw: bool,
    pub active: bool,
}

impl TickOut {
    /// No repaint and no active effect
    pub const IDLE: Self = Self { redraw: false, active: false };
    /// Repaints and keeps the effect active
    pub const ACTIVE: Self = Self { redraw: true, active: true };
}

/// Key handling context (collected by Ui from the node/its own state before dispatch)
pub(crate) struct KeyCtx {
    pub edited: bool, // node is in the EDITED state
    pub vis_h: i32,   // node visible height (used by scrolling widgets)
    pub now: u64,
}

/// Key handling result: Ui performs the common side effects (dirtying/events/EDITED state);
/// widget-specific side effects run via `Deferred` in the widget file
pub(crate) enum KeyOutcome {
    Pass,          // not consumed → fall through to default (focus move / Clicked)
    Consumed,      // consumed, dirty the node
    ValueChanged,  // consumed, dirty the node and send a ValueChanged event
    EnterEdit,     // enter the EDITED state
    ExitEdit,      // leave the EDITED state and dirty the node
    /// Widget-specific side effect, executed later: a static exec fn provided by the widget file + an i32 payload.
    /// Ui calls `f(self, obj, p)` after putting the kind back in the arena (clean window, no placeholder); treated as consumed.
    Deferred(fn(&mut Ui, ObjRef, i32), i32),
}

/// Widget behavior interface: `draw` must be implemented (a new widget that forgets to paint fails to compile);
/// most widgets lack the other behaviors, so they get default no-op implementations.
pub(crate) trait WidgetBehavior {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect);
    fn tick(&mut self, _now: u64) -> TickOut { TickOut::IDLE }
    fn on_key(&mut self, _key: Key, _ctx: KeyCtx) -> KeyOutcome { KeyOutcome::Pass }
    fn value(&self) -> i32 { 0 }
    fn set_value(&mut self, _v: i32) -> bool { false }
    fn set_range(&mut self, _min: i32, _max: i32) {}
    fn overflow(&self) -> i32 { 0 }
}

/// Shared for `set_value`: clamp to `[min, max]`, return whether the value changed
pub(crate) fn clamp_val(min: i32, max: i32, value: &mut i32, v: i32) -> bool {
    let nv = v.clamp(min, max);
    let changed = nv != *value;
    *value = nv;
    changed
}

/// Shared for selection widgets: clamp to `[0, len)`, return whether the value changed
pub(crate) fn select_clamp(len: usize, selected: &mut usize, v: i32) -> bool {
    if len == 0 { return false; }
    let nv = (v.max(0) as usize).min(len - 1);
    let changed = nv != *selected;
    *selected = nv;
    changed
}

/// Variant storage: inline = inlined state, boxed = heap-allocated (large states to
/// avoid the "largest-variant tax").
macro_rules! wtype {
    (inline, $state:ty) => { $state };
    (boxed,  $state:ty) => { alloc::boxed::Box<$state> };
}

/// Private deref helpers: expand to a `&T`/`&mut T` for both inline (`s: &T`) and
/// boxed (`s: &Box<T>`) payloads without adding any public trait impls.
macro_rules! wref {
    (inline, $s:expr) => { $s };
    (boxed,  $s:expr) => { &**$s };
}
macro_rules! wmut {
    (inline, $s:expr) => { $s };
    (boxed,  $s:expr) => { &mut **$s };
}

/// Declaratively registers a widget: generates the enum, behavior dispatch, `as_xxx` accessors, and downcast.
/// Adding a widget requires just one line here.
macro_rules! define_widgets {
    ($($variant:ident($state:ty, $as:ident, $as_mut:ident, $store:ident)),+ $(,)?) => {
        /// Discriminated widget state: one variant per registered widget type.
        pub enum WidgetKind {
            $( $variant(wtype!($store, $state)), )+
        }

        impl WidgetKind {
            pub(crate) fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
                    match self { $( WidgetKind::$variant(s) => WidgetBehavior::draw(wref!($store, s), ctx, d, clip), )+ }
            }
            pub(crate) fn overflow(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::overflow(wref!($store, s)), )+ }
            }
            pub(crate) fn value(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::value(wref!($store, s)), )+ }
            }
            pub(crate) fn set_value(&mut self, v: i32) -> bool {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_value(wmut!($store, s), v), )+ }
            }
            pub(crate) fn set_range(&mut self, min: i32, max: i32) {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_range(wmut!($store, s), min, max), )+ }
            }
            pub(crate) fn tick(&mut self, now: u64) -> TickOut {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::tick(wmut!($store, s), now), )+ }
            }
            pub(crate) fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::on_key(wmut!($store, s), key, ctx), )+ }
            }
            $(
                /// Returns `Some(&$state)` if this widget is a `$variant`.
                pub fn $as(&self) -> Option<&$state> {
                    match self { WidgetKind::$variant(s) => Some(wref!($store, s)), _ => None }
                }
                /// Returns `Some(&mut $state)` if this widget is a `$variant`.
                pub fn $as_mut(&mut self) -> Option<&mut $state> {
                    match self { WidgetKind::$variant(s) => Some(wmut!($store, s)), _ => None }
                }
            )+
            /// Hands out `&mut` state by type (used by `Ui::update`); TypeId comparison + Any downcast
            pub(crate) fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
                $(
                    if core::any::TypeId::of::<T>() == core::any::TypeId::of::<$state>() {
                        if let WidgetKind::$variant(s) = self {
                            return (wmut!($store, s) as &mut dyn core::any::Any).downcast_mut::<T>();
                        }
                    }
                )+
                None
            }
        }
    };
}

define_widgets! {
    Obj(obj::ObjState, as_obj, as_obj_mut, inline),
    ItemList(itemlist::ItemListState, as_itemlist, as_itemlist_mut, boxed),
    Label(label::LabelState, as_label, as_label_mut, inline),
    Button(button::ButtonState, as_button, as_button_mut, inline),
    Slider(slider::SliderState, as_slider, as_slider_mut, inline),
    Switch(switch::SwitchState, as_switch, as_switch_mut, inline),
    Bar(bar::BarState, as_bar, as_bar_mut, inline),
    List(list::ListState, as_list, as_list_mut, boxed),
    Arc(arc::ArcState, as_arc, as_arc_mut, inline),
    Checkbox(checkbox::CheckboxState, as_checkbox, as_checkbox_mut, inline),
    Chart(chart::ChartState, as_chart, as_chart_mut, inline),
    Spinner(spinner::SpinnerState, as_spinner, as_spinner_mut, inline),
    Msgbox(msgbox::MsgboxState, as_msgbox, as_msgbox_mut, inline),
    Led(led::LedState, as_led, as_led_mut, inline),
    Table(table::TableState, as_table, as_table_mut, inline),
    Spinbox(spinbox::SpinboxState, as_spinbox, as_spinbox_mut, inline),
    Roller(roller::RollerState, as_roller, as_roller_mut, boxed),
    ScrollView(scrollview::ScrollViewState, as_scrollview, as_scrollview_mut, inline),
    Dropdown(dropdown::DropdownState, as_dropdown, as_dropdown_mut, inline),
    Image(image::ImageState, as_image, as_image_mut, inline),
    Custom(custom::CustomState, as_custom_state, as_custom_state_mut, inline),
}

impl WidgetKind {
    pub(crate) fn as_custom(&self) -> Option<&dyn custom::Widget> {
        match self { WidgetKind::Custom(s) => Some(s.0.as_ref()), _ => None }
    }
    pub(crate) fn as_custom_mut(&mut self) -> Option<&mut dyn custom::Widget> {
        match self { WidgetKind::Custom(s) => Some(s.0.as_mut()), _ => None }
    }
}
