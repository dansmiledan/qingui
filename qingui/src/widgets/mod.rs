use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::input::Key;
use crate::style::ResolvedStyle;
use crate::arena::ObjRef;
use crate::ui::Ui;

// Temporary alias: Task 21 renames DrawBuf to Canvas and switches this to a re-export.
use crate::draw::DrawBuf as Canvas;

pub(crate) mod builder;
pub use builder::{Layout, WidgetBuilder}; // public return type (XxxCfg::new returns it)

pub mod arc;
pub mod bar;
pub mod button;
pub mod canvas;
pub mod chart;
pub mod checkbox;
pub mod custom;
pub mod dropdown;
pub mod flexbox;
pub mod gridbox;
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
pub enum KeyOutcome {
    Pass,          // not consumed → fall through to default (focus move / Clicked)
    Consumed,      // consumed, dirty the node
    ValueChanged,  // consumed, dirty the node and send a ValueChanged event
    EnterEdit,     // enter the EDITED state
    ExitEdit,      // leave the EDITED state and dirty the node
    /// Widget-specific side effect, executed later: a static exec fn provided by the widget file + an i32 payload.
    /// Ui calls `f(self, obj, p)` after putting the kind back in the arena (clean window, no placeholder); treated as consumed.
    Deferred(fn(&mut Ui, ObjRef, i32), i32),
}

/// Measure context: read-only inputs for intrinsic content sizing.
pub struct MeasureCtx {
    /// Resolved font (node style font or the Ui default).
    pub font: &'static embedded_graphics::mono_font::MonoFont<'static>,
    /// The node's current size (layout treats it as content size today).
    pub cur: (i32, i32),
}

/// The single widget behavior interface. Node owns common data; the trait object
/// owns behavior and widget-specific data (reached via `as_any` downcast).
///
/// `draw`/`measure` take `&self` and never leave the arena. `layout`/`tick`/`on_key`
/// take `&mut self` and are called via take-out (the node temporarily holds a
/// `NoopWidget` placeholder), so they receive `&mut Ui` and may operate on any
/// other node; rules while taken out:
/// - mutate your own state directly on `self`;
/// - `ui.update(self_obj, ...)` is a silent no-op (your kind is not in the arena);
/// - deleting your own node is allowed (Ui treats the outcome as consumed).
pub trait Widget {
    /// Content drawing (background/border/opa are handled uniformly by Ui). Default: draws nothing.
    fn draw(&self, _ctx: &WidgetCtx, _c: &mut Canvas, _clip: Rect) {}
    /// Intrinsic content size; `(0, 0)` means "no intrinsic size" (layout uses the current rect).
    fn measure(&self, _ctx: &MeasureCtx) -> (i32, i32) { (0, 0) }
    /// Lays out direct children. Default: manual positioning (children keep their rects).
    fn layout(&mut self, _ui: &mut Ui, _obj: ObjRef) {}
    /// Per-frame progress. Default: idle.
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, _now: u64) -> TickOut { TickOut::IDLE }
    /// Key handling. Default: not consumed (falls through to focus move / Clicked).
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, _key: Key) -> KeyOutcome { KeyOutcome::Pass }
    /// Property-animation Value channel.
    fn value(&self) -> i32 { 0 }
    fn set_value(&mut self, _v: i32) -> bool { false }
    fn set_range(&mut self, _min: i32, _max: i32) {}
    /// Draw overflow beyond the node rect (knobs, etc.), for dirty-area expansion.
    fn overflow(&self) -> i32 { 0 }
    fn as_any(&self) -> &dyn core::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any;
}

/// Zero-sized placeholder swapped in during take-out (Box of a ZST does not allocate).
pub struct NoopWidget;

impl Widget for NoopWidget {
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
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
            /// Read-only counterpart of `downcast_mut` (used by `Ui::widget` during migration).
            pub(crate) fn downcast_ref<T: 'static>(&self) -> Option<&T> {
                $(
                    if core::any::TypeId::of::<T>() == core::any::TypeId::of::<$state>() {
                        if let WidgetKind::$variant(s) = self {
                            return (wref!($store, s) as &dyn core::any::Any).downcast_ref::<T>();
                        }
                    }
                )+
                None
            }
        }
    };
}

define_widgets! {
    ItemList(itemlist::ItemListState, as_itemlist, as_itemlist_mut, boxed),
    List(list::ListState, as_list, as_list_mut, boxed),
    Chart(chart::ChartState, as_chart, as_chart_mut, inline),
    Spinner(spinner::SpinnerState, as_spinner, as_spinner_mut, inline),
    Msgbox(msgbox::MsgboxState, as_msgbox, as_msgbox_mut, inline),
    Table(table::TableState, as_table, as_table_mut, inline),
    Spinbox(spinbox::SpinboxState, as_spinbox, as_spinbox_mut, inline),
    Roller(roller::RollerState, as_roller, as_roller_mut, boxed),
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

/// Compatibility shim: the legacy enum boxes itself as a trait object while
/// widgets are migrated one by one. Deleted together with the enum (Task 22).
impl Widget for WidgetKind {
    fn draw(&self, ctx: &WidgetCtx, c: &mut Canvas, clip: Rect) {
        WidgetKind::draw(self, ctx, c, clip);
    }
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, now: u64) -> TickOut {
        WidgetKind::tick(self, now)
    }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> KeyOutcome {
        let ctx = KeyCtx {
            edited: ui.state(obj).contains(crate::node::State::EDITED),
            vis_h: ui.rect(obj).h,
            now: ui.time(),
        };
        // Legacy Custom variant: user state already received `&mut Ui` — keep that path.
        if let Some(w) = self.as_custom_mut() {
            return if w.on_key(ui, obj, key) { KeyOutcome::Consumed } else { KeyOutcome::Pass };
        }
        WidgetKind::on_key(self, key, ctx)
    }
    fn layout(&mut self, ui: &mut Ui, obj: ObjRef) {
        // Legacy container variants keep their fixed arrangement until their own
        // migration task (Msgbox: Task 19).
        if let WidgetKind::Msgbox(_) = self {
            crate::layout::layout_flex(ui, obj, &msgbox::ROOT_FLEX);
        }
    }
    fn value(&self) -> i32 { WidgetKind::value(self) }
    fn set_value(&mut self, v: i32) -> bool { WidgetKind::set_value(self, v) }
    fn set_range(&mut self, min: i32, max: i32) { WidgetKind::set_range(self, min, max) }
    fn overflow(&self) -> i32 { WidgetKind::overflow(self) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}

/// Migration helpers on the trait object: reach the boxed legacy `WidgetKind`
/// enum while widgets are migrated one by one. Deleted with the enum (Task 22).
impl dyn Widget {
    pub(crate) fn as_kind(&self) -> Option<&WidgetKind> {
        self.as_any().downcast_ref()
    }
    pub(crate) fn as_kind_mut(&mut self) -> Option<&mut WidgetKind> {
        self.as_any_mut().downcast_mut()
    }
}
