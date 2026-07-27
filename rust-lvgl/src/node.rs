use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::geometry::Rect;

pub mod state {
    pub const PRESSED: u8 = 1 << 0;
    pub const FOCUSED: u8 = 1 << 1;
    pub const DISABLED: u8 = 1 << 2;
    pub const EDITED: u8 = 1 << 3;
}

pub mod flag {
    pub const HIDDEN: u8 = 1 << 0;
    pub const CLICKABLE: u8 = 1 << 1;
}

pub enum WidgetKind {
    Obj,
    Label { text: alloc::string::String },
    Button { text: alloc::string::String },
    Slider { min: i32, max: i32, value: i32 },
    Switch { on: bool },
    Bar { min: i32, max: i32, value: i32 },
    List { items: Vec<alloc::string::String>, selected: usize, scroll: i32 },
}

pub struct Node {
    pub parent: Option<ObjRef>,
    pub children: Vec<ObjRef>,
    pub rect: Rect, // 相对父内容原点的本地坐标
    pub state: u8,
    pub flags: u8,
    pub kind: WidgetKind,
    pub style: crate::style::Style,
    pub style_pressed: crate::style::Style,
    pub style_focused: crate::style::Style,
    pub opa: u8,
    pub events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
    pub grid_col: (u8, u8),
    pub grid_row: (u8, u8),
}

impl Node {
    pub fn new(parent: Option<ObjRef>, rect: Rect, kind: WidgetKind) -> Self {
        Self {
            parent,
            children: Vec::new(),
            rect,
            state: 0,
            flags: 0,
            kind,
            style: crate::style::Style::default(),
            style_pressed: crate::style::Style::default(),
            style_focused: crate::style::Style::default(),
            opa: 255,
            events: Vec::new(),
            grid_col: (0, 1),
            grid_row: (0, 1),
        }
    }
}
