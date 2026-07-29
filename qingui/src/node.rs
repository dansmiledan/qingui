use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::geometry::Rect;

pub use crate::widgets::WidgetKind;

bitflags::bitflags! {
    /// 对象状态（对齐 LVGL 的 state）
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    pub struct State: u8 {
        const PRESSED = 1 << 0;
        const FOCUSED = 1 << 1;
        const DISABLED = 1 << 2;
        const EDITED = 1 << 3;
    }

    /// 对象标志位
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    pub struct Flag: u8 {
        const HIDDEN = 1 << 0;
        const CLICKABLE = 1 << 1;
        /// 浮动对象：不参与父容器的布局（对齐 LVGL IGNORE_LAYOUT），弹窗/悬浮层用
        const IGNORE_LAYOUT = 1 << 2;
    }
}

pub struct Node {
    pub parent: Option<ObjRef>,
    pub children: Vec<ObjRef>,
    pub rect: Rect, // 相对父内容原点的本地坐标
    pub state: State,
    pub flags: Flag,
    pub kind: WidgetKind,
    pub style: crate::style::Style,
    pub style_pressed: crate::style::Style,
    pub style_focused: crate::style::Style,
    pub opa: u8,
    pub events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
    pub grid_col: (u8, u8),
    pub grid_row: (u8, u8),
    /// 视觉平移偏移：子树整体在渲染时叠加，不参与布局（对齐 LVGL translate_x/y）
    pub translate: crate::geometry::Point,
    /// 浮层锚定：(目标对象, 锚定方式)。设置后对象同时视为 IGNORE_LAYOUT
    pub floating: Option<(ObjRef, crate::layout::Attach)>,
    /// 叠放次序：渲染时兄弟节点按 z_index 稳定排序（大者在上）
    pub z_index: i16,
    /// 是否已经历过一次布局（首次布局不做过渡动画）
    pub laid_out: bool,
}

impl Node {
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
            opa: 255,
            events: Vec::new(),
            grid_col: (0, 1),
            grid_row: (0, 1),
            translate: crate::geometry::Point::default(),
            floating: None,
            z_index: 0,
            laid_out: false,
        }
    }
}
