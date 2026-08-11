/// Input keys reported to the focused widget and used for focus navigation.
///
/// A rotary encoder maps onto this set directly: rotation CW -> `Down` (or `Next`),
/// rotation CCW -> `Up` (or `Prev`), push button -> `Enter`. Direction keys move the
/// focus between widgets by default; while a widget is in its inner (EDITED) mode they
/// operate the widget instead (a list's selection, a slider's value, ...).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    /// Navigate to the previous focusable.
    Prev,
    /// Navigate to the next focusable.
    Next,
    /// Up.
    Up,
    /// Down.
    Down,
    /// Left.
    Left,
    /// Right.
    Right,
    /// Activate: enters the focused widget's inner (EDITED) mode when it has one
    /// (list/roller/slider/...), fires a `Clicked` otherwise; while editing, confirms
    /// the widget (Click + exit). Esc leaves the inner mode without acting.
    Enter,
    /// Escape (cancel / close).
    Esc,
}
