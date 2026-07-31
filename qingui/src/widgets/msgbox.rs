use alloc::boxed::Box;

use crate::arena::ObjRef;
use crate::event::EventKind;
use crate::geometry::Rect;
use crate::layout::{Align, Attach, Flex, FlexDir};
use crate::style::Layout;
use crate::ui::Ui;
use super::WidgetKind;

#[derive(Clone)]
pub struct MsgboxState {
    pub selected: i32,
}

/// 模态消息框：标题 + 文本 + 按钮行，浮层居中并锁定焦点。
/// 按钮点击后关闭：根对象收到 `EventKind::ValueChanged`，
/// 用 `ObjRef::msgbox_selected` 读取点击的按钮索引（Esc 关闭为 -1）。
/// Msgbox 构建器：模态消息框（标题 + 文本 + 按钮行）
pub struct MsgboxBuilder {
    title: alloc::string::String,
    text: alloc::string::String,
    buttons: alloc::vec::Vec<alloc::string::String>,
}

impl MsgboxBuilder {
    pub fn new(title: &str, text: &str) -> Self {
        Self {
            title: title.into(),
            text: text.into(),
            buttons: alloc::vec::Vec::new(),
        }
    }
    pub fn buttons(mut self, buttons: &[&str]) -> Self {
        self.buttons = buttons.iter().map(|s| (*s).into()).collect();
        self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let refs: alloc::vec::Vec<&str> = self.buttons.iter().map(|s| s.as_str()).collect();
        create(ui, parent, &self.title, &self.text, &refs)
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, title: &str, text: &str, buttons: &[&str]) -> ObjRef {
    let root = ui.insert_node(parent, Rect::new(0, 0, 200, 110), WidgetKind::Msgbox(MsgboxState { selected: -1 }));
    ui.set_floating(root, parent, Attach::Center);
    // 样式：对话框 + 列布局
    ui.set_style(root,
        crate::style::theme_obj()
            .border(crate::geometry::Color::WHITE, 2)
            .pad(12, 12, 10, 10)
            .layout(Layout::Flex(Flex {
                dir: FlexDir::Column, wrap: false,
                main: Align::Start, cross: Align::Center, track: Align::Start, gap: 8,
            })),
    );
    let t = crate::widgets::label::create(ui, root, title);
    ui.set_style(t, crate::style::Style::new().text_color(crate::geometry::Color::rgb(255, 200, 60)));
    let _msg = crate::widgets::label::create(ui, root, text);
    // 按钮行
    let row = ui.insert_node(root, Rect::default(), WidgetKind::Obj);
    let mut rs = crate::style::Style::default();
    rs.bg_opa = Some(0);
    rs.layout = Some(Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Start, track: Align::Start, gap: 12,
    }));
    ui.set_style(row, rs);
    for (i, b) in buttons.iter().enumerate() {
        let btn = crate::widgets::button::create(ui, row, b);
        ui.group_add(btn);
        // 点击：记录索引 → 通知 → 解锁并删除
        ui.add_event_cb(btn, EventKind::Clicked, Box::new(move |ui, _x, _| {
            if let Some(n) = ui.arena.get_mut(root) {
                if let WidgetKind::Msgbox(s) = &mut n.kind {
                    s.selected = i as i32;
                }
            }
            let root = root;
            ui.send_event(root, EventKind::ValueChanged);
            ui.clear_modal();
            ui.delete(root);
        }));
        // Esc：selected 保持 -1，直接关闭
        ui.add_event_cb(btn, EventKind::Key(crate::input::Key::Esc), Box::new(move |ui, _x, k| {
            if k == EventKind::Key(crate::input::Key::Esc) {
                ui.send_event(root, EventKind::ValueChanged);
                ui.clear_modal();
                ui.delete(root);
            }
        }));
    }
    ui.set_modal(root);
    root
}
