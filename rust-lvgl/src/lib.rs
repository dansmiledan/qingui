#![no_std]
extern crate alloc;

pub mod arena;
pub mod anim;
pub mod dirty;
pub mod display;
pub mod draw;
pub mod event;
pub mod font;
pub mod geometry;
pub mod input;
pub mod node;
pub mod style;
pub mod ui;
pub use arena::ObjRef;
pub use event::EventKind;
pub use geometry::{Color, Point, Rect};
pub use ui::Ui;
