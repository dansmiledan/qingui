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
        }
    }
}
