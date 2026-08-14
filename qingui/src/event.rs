use alloc::boxed::Box;
use crate::arena::ObjRef;
use crate::input::Key;
use crate::ui::Ui;

/// Kinds of events delivered to widget event callbacks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    /// The widget was activated (e.g. Enter pressed on a focused button).
    Clicked,
    /// The widget's value changed.
    ValueChanged,
    /// The widget gained focus.
    Focused,
    /// The widget lost focus.
    Defocused,
    /// A key was pressed while the widget was focused.
    Key(Key),
}

/// Event callback: invoked with the UI, the source object, and the event kind.
// NOTE: stays non-generic until Task 5 genericizes `Ui`; a type alias cannot carry
// an unused `C` (E0091), so `EventCb<C>` = `FnMut(&mut Ui<C>, ...)` lands with `Ui<C>`.
pub type EventCb = Box<dyn FnMut(&mut Ui, ObjRef, EventKind)>;
