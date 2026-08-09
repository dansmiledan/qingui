use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::layout::Sizing;

/// Overlay draw hook: called after the widget draws its own content, with
/// (draw buffer, widget absolute rect, clip rect, current time ms).
pub type DrawHook = alloc::boxed::Box<dyn FnMut(&mut crate::canvas::Canvas, Rect, Rect, u64)>;
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

/// Constraints the PARENT's layout interprets for this child.
/// Shared fields are understood by the built-in layouts (flex/grid);
/// `specific` carries algorithm-specific or third-party constraints.
#[derive(Default)]
pub enum ItemSpecific {
    /// No algorithm-specific constraints (default).
    #[default]
    None,
    /// Grid cell placement: (start, span) per axis.
    Grid { col: (u8, u8), row: (u8, u8) },
    /// Third-party layout constraints: heap-allocated, type-erased,
    /// follows the child's lifecycle (no parent-side table to clean up).
    Custom(alloc::boxed::Box<dyn core::any::Any>),
}

/// Constraints the PARENT's layout interprets for this child.
/// Shared fields are understood by the built-in layouts (flex/grid);
/// `specific` carries algorithm-specific or third-party constraints.
#[derive(Default)]
pub struct ItemProps {
    /// Width sizing strategy (None = content size).
    pub sizing_w: Option<Sizing>,
    /// Height sizing strategy (None = content size).
    pub sizing_h: Option<Sizing>,
    /// Aspect ratio (per-mille: 1000 = 1:1).
    pub aspect_ratio: Option<u32>,
    /// Algorithm-specific constraints.
    pub specific: ItemSpecific,
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
    pub kind: alloc::boxed::Box<dyn crate::widgets::Widget>,
    /// Base style.
    pub style: crate::style::Style,
    /// Style overlay while pressed.
    pub style_pressed: Option<alloc::boxed::Box<crate::style::Style>>,
    /// Style overlay while focused.
    pub style_focused: Option<alloc::boxed::Box<crate::style::Style>>,
    /// Style overlay while selected.
    pub style_selected: Option<alloc::boxed::Box<crate::style::Style>>,
    /// Registered event callbacks, in order.
    pub events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
    /// Overlay draw hook, drawn after the widget's own content.
    pub draw_hook: Option<DrawHook>,
    /// Per-frame hook.
    pub tick_hook: Option<TickHook>,
    /// Padding (l, r, t, b): layout input, content origin offset.
    pub pad: (i32, i32, i32, i32),
    /// Layout transition: (duration ms, easing).
    pub transition: Option<(u32, crate::anim::Easing)>,
    /// Per-child layout constraints consumed by the parent (sizing, aspect, specific).
    pub item_props: ItemProps,
    /// Visual translation offset: applied to the whole subtree at render time, does not
    /// participate in layout (mirrors LVGL translate_x/y).
    pub translate: crate::geometry::Point,
    /// Floating anchor: (target object, attach mode). Setting this also marks the object as IGNORE_LAYOUT.
    pub floating: Option<(ObjRef, crate::layout::Attach)>,
    /// Whether the node has been through one layout pass (the first layout does not animate transitions).
    pub laid_out: bool,
}

impl Node {
    /// Creates a node with the given parent, local rect, and widget kind.
    pub fn new(parent: Option<ObjRef>, rect: Rect, kind: alloc::boxed::Box<dyn crate::widgets::Widget>) -> Self {
        Self {
            parent,
            children: Vec::new(),
            rect,
            state: State::empty(),
            flags: Flag::empty(),
            kind,
            style: crate::style::Style::default(),
            style_pressed: None,
            style_focused: None,
            style_selected: None,
            events: Vec::new(),
            draw_hook: None,
            tick_hook: None,
            pad: (0, 0, 0, 0),
            transition: None,
            item_props: ItemProps::default(),
            translate: crate::geometry::Point::default(),
            floating: None,
            laid_out: false,
        }
    }
}
