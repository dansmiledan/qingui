#![no_std]
extern crate alloc;

pub mod arena;
pub mod anim;
pub mod dirty;
pub mod display;
pub mod draw;
pub mod event;
pub mod font;
pub mod focus;
pub mod geometry;
pub mod input;
pub mod render;
pub mod layout;
pub mod node;
pub mod style;
pub mod ui;
pub mod widgets;
pub use arena::ObjRef;
pub use event::EventKind;
pub use geometry::{Color, Point, Rect};
pub use ui::Ui;
pub use widgets::custom::Widget;
pub use widgets::TickOut;

/// 各 widget 扩展 trait 汇总:一行引入全部 widget 专属 API
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
