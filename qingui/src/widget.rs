use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::event::{EventCb, EventKind};
use crate::layout::{Attach, Sizing};
use crate::style::{Layout, Style};
use crate::ui::Ui;

/// 链式配置包装：`ui.widget(obj).pos(10, 10).size(80, 30).style(s)`
/// 每个方法执行后返回自身以继续链式调用；用 `obj()` 取回句柄。
pub struct WidgetMut<'a> {
    ui: &'a mut Ui,
    obj: ObjRef,
}

impl<'a> WidgetMut<'a> {
    pub(crate) fn new(ui: &'a mut Ui, obj: ObjRef) -> Self {
        Self { ui, obj }
    }

    /// 取回对象句柄（结束链式调用）
    pub fn obj(self) -> ObjRef {
        self.obj
    }

    pub fn pos(self, x: i32, y: i32) -> Self {
        self.ui.set_pos(self.obj, x, y);
        self
    }
    pub fn size(self, w: i32, h: i32) -> Self {
        self.ui.set_size(self.obj, w, h);
        self
    }
    pub fn style(self, style: Style) -> Self {
        self.ui.set_style(self.obj, style);
        self
    }
    pub fn style_pressed(self, style: Style) -> Self {
        self.ui.set_style_pressed(self.obj, style);
        self
    }
    pub fn style_focused(self, style: Style) -> Self {
        self.ui.set_style_focused(self.obj, style);
        self
    }
    pub fn sizing(self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.ui.set_sizing(self.obj, w, h);
        self
    }
    pub fn aspect(self, ratio: Option<u32>) -> Self {
        self.ui.set_aspect(self.obj, ratio);
        self
    }
    pub fn transition(self, duration_ms: u32, easing: Easing) -> Self {
        self.ui.set_transition(self.obj, Some((duration_ms, easing)));
        self
    }
    pub fn hidden(self, hidden: bool) -> Self {
        self.ui.set_hidden(self.obj, hidden);
        self
    }
    pub fn translate(self, x: i32, y: i32) -> Self {
        self.ui.set_translate(self.obj, x, y);
        self
    }
    pub fn z_index(self, z: i16) -> Self {
        self.ui.set_z_index(self.obj, z);
        self
    }
    pub fn value(self, v: i32) -> Self {
        self.ui.set_value(self.obj, v);
        self
    }
    pub fn layout(self, layout: Layout) -> Self {
        self.ui.set_layout(self.obj, layout);
        self
    }
    pub fn grid_cell(self, col: (u8, u8), row: (u8, u8)) -> Self {
        self.ui.set_grid_cell(self.obj, col, row);
        self
    }
    pub fn ignore_layout(self, ignore: bool) -> Self {
        self.ui.set_ignore_layout(self.obj, ignore);
        self
    }
    pub fn floating(self, target: ObjRef, attach: Attach) -> Self {
        self.ui.set_floating(self.obj, target, attach);
        self
    }
    pub fn state(self, state: crate::node::State, on: bool) -> Self {
        self.ui.set_state(self.obj, state, on);
        self
    }
    pub fn on(self, kind: EventKind, cb: EventCb) -> Self {
        self.ui.add_event_cb(self.obj, kind, cb);
        self
    }
    pub fn group_add(self) -> Self {
        self.ui.group_add(self.obj);
        self
    }
}
