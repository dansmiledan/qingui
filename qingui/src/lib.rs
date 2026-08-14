#![no_std]
extern crate alloc;

pub mod arena;
pub mod anim;
pub mod canvas;
pub mod dirty;
pub mod display;
pub(crate) mod draw;
pub mod event;
pub mod font;
pub mod focus;
pub mod geometry;
pub mod input;
pub mod render;
pub mod layout;
pub mod node;
pub mod pixel;
pub mod style;
pub mod ui;
pub mod widgets;
/// Handle referencing an object stored in the arena.
pub use arena::ObjRef;
/// Event kinds delivered to widget event callbacks.
pub use event::EventKind;
/// Core geometry and color types.
pub use geometry::{Color, Point, Rect};
/// Framebuffer pixel format bridge between internal RGB888 `Color` and e-g pixel types.
pub use pixel::PixelFormat;
/// Main UI state and entry point for all object operations.
pub use ui::Ui;
/// The widget behavior trait: implement it and mount via `Ui::create_widget` (the single extension point).
pub use widgets::Widget;
/// Per-frame effect result reported by a widget's tick.
pub use widgets::TickOut;

/// Aggregates all widget extension traits: brings in every widget-specific API with one import
pub mod prelude {
    pub use crate::widgets::chart::UiChartExt;
    pub use crate::widgets::checkbox::UiCheckboxExt;
    pub use crate::widgets::itemlist::UiItemListExt;
    pub use crate::widgets::label::UiTextExt;
    pub use crate::widgets::list::UiListExt;
    pub use crate::widgets::msgbox::UiMsgboxExt;
    pub use crate::widgets::roller::UiRollerExt;
    pub use crate::widgets::scrollview::UiScrollViewExt;
    pub use crate::widgets::switch::UiSwitchExt;
    pub use crate::widgets::table::UiTableExt;
}
