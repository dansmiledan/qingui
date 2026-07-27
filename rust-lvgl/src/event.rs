use alloc::boxed::Box;
use crate::arena::ObjRef;
use crate::input::Key;
use crate::ui::Ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    Clicked,
    ValueChanged,
    Focused,
    Defocused,
    Key(Key),
}

pub type EventCb = Box<dyn FnMut(&mut Ui, ObjRef, EventKind)>;
