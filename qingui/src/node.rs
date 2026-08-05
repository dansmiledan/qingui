use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::geometry::Rect;

/// The kind of widget a node holds.
pub use crate::widgets::WidgetKind;

/// Overlay draw hook: called after the widget draws its own content, with
/// (draw buffer, widget absolute rect, clip rect, current time ms).
pub type DrawHook = alloc::boxed::Box<dyn FnMut(&mut crate::draw::DrawBuf, Rect, Rect, u64)>;
/// Per-frame hook: returning `true` means still active (dirties the node and keeps the
/// timer handler awake).
pub type TickHook = alloc::boxed::Box<dyn FnMut(&mut crate::ui::Ui, ObjRef, u64) -> bool>;

bitflags::bitflags! {
    /// Object state (mirrors LVGL's state).
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    pub struct State: u8 {
        const PRESSED = 1 << 0;
        const FOCUSED = 1 << 1;
        const DISABLED = 1 << 2;
        const EDITED = 1 << 3;
        /// Selected state: used for list item/entry highlighting; takes priority below pressed/focused.
        const SELECTED = 1 << 4;
    }

    /// Object flags.
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    pub struct Flag: u8 {
        const HIDDEN = 1 << 0;
        const CLICKABLE = 1 << 1;
        /// Floating object: excluded from the parent container's layout (mirrors LVGL
        /// IGNORE_LAYOUT); used for popups and overlays.
        const IGNORE_LAYOUT = 1 << 2;
        /// Viewport clipping: the subtree is drawn clipped to this object's rect (for scroll containers).
        const CLIP_CHILDREN = 1 << 3;
    }
}

/// A node in the widget tree: geometry, style, state, and widget behavior.
pub struct Node {
    /// Parent object, or `None` for the screen root.
    pub parent: Option<ObjRef>,
    /// Direct children, in paint/layout order.
    pub children: Vec<ObjRef>,
    /// Local rect relative to the parent's content origin.
    pub rect: Rect,
    /// UI state flags (pressed/focused/etc.).
    pub state: State,
    /// Behavior flags (hidden/clickable/etc.).
    pub flags: Flag,
    /// The widget behavior carried by this node.
    pub kind: WidgetKind,
    /// Base style.
    pub style: crate::style::Style,
    /// Style overlay while pressed.
    pub style_pressed: crate::style::Style,
    /// Style overlay while focused.
    pub style_focused: crate::style::Style,
    /// Style overlay while selected.
    pub style_selected: crate::style::Style,
    /// Node opacity multiplier (0..=255) applied to everything it draws.
    pub opa: u8,
    /// Registered event callbacks, in order.
    pub events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
    /// Overlay draw hook, drawn after the widget's own content.
    pub draw_hook: Option<DrawHook>,
    /// Per-frame hook.
    pub tick_hook: Option<TickHook>,
    /// Grid column (start, span).
    pub grid_col: (u8, u8),
    /// Grid row (start, span).
    pub grid_row: (u8, u8),
    /// Visual translation offset: applied to the whole subtree at render time, does not
    /// participate in layout (mirrors LVGL translate_x/y).
    pub translate: crate::geometry::Point,
    /// Floating anchor: (target object, attach mode). Setting this also marks the object as IGNORE_LAYOUT.
    pub floating: Option<(ObjRef, crate::layout::Attach)>,
    /// Stacking order: siblings are stably sorted by z_index at render time (larger = on top).
    pub z_index: i16,
    /// Whether the node has been through one layout pass (the first layout does not animate transitions).
    pub laid_out: bool,
}

impl Node {
    /// Creates a node with the given parent, local rect, and widget kind.
    pub fn new(parent: Option<ObjRef>, rect: Rect, kind: WidgetKind) -> Self {
        Self {
            parent,
            children: Vec::new(),
            rect,
            state: State::empty(),
            flags: Flag::empty(),
            kind,
            style: crate::style::Style::default(),
            style_pressed: crate::style::Style::default(),
            style_focused: crate::style::Style::default(),
            style_selected: crate::style::Style::default(),
            opa: 255,
            events: Vec::new(),
            draw_hook: None,
            tick_hook: None,
            grid_col: (0, 1),
            grid_row: (0, 1),
            translate: crate::geometry::Point::default(),
            floating: None,
            z_index: 0,
            laid_out: false,
        }
    }
}
