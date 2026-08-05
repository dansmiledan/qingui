/// Input keys reported to the focused widget and used for focus navigation.
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
    /// Activate (confirm / click the focused widget).
    Enter,
    /// Escape (cancel / close).
    Esc,
}
