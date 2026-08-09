use crate::geometry::Rect;
use crate::input::Key;
use crate::style::ResolvedStyle;
use crate::arena::ObjRef;
use crate::ui::Ui;

// Re-exported so user widgets can name the canvas type in `Widget::draw`.
pub use crate::canvas::Canvas;

pub(crate) mod builder;
pub use builder::{Layout, WidgetBuilder}; // public return type (XxxCfg::new returns it)

pub mod arc;
pub mod bar;
pub mod button;
pub mod chart;
pub mod checkbox;
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

/// Key handling result: Ui performs the common side effects (dirtying/events/EDITED state)
pub enum KeyOutcome {
    Pass,          // not consumed → fall through to default (focus move / Clicked)
    Consumed,      // consumed, dirty the node
    ValueChanged,  // consumed, dirty the node and send a ValueChanged event
    EnterEdit,     // enter the EDITED state
    ExitEdit,      // leave the EDITED state and dirty the node
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
    /// `content` is the node's content box in the node's LOCAL coordinate space
    /// (origin = padding offsets, size = rect minus padding), computed by Ui;
    /// the layout positions children purely within it.
    fn layout(&mut self, _ui: &mut Ui, _obj: ObjRef, _content: Rect) {}
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
