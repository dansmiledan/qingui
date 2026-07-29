use alloc::boxed::Box;

use crate::arena::ObjRef;
use crate::event::EventKind;
use crate::geometry::Rect;
use crate::layout::{Align, Attach, Flex, FlexDir};
use crate::style::Layout;
use crate::ui::Ui;
use super::WidgetKind;

/// 模态消息框：标题 + 文本 + 按钮行，浮层居中并锁定焦点。
/// 按钮点击后关闭：根对象收到 `EventKind::ValueChanged`，
/// 用 `Ui::msgbox_selected` 读取点击的按钮索引（Esc 关闭为 -1）。
pub(crate) fn create(ui: &mut Ui, parent: ObjRef, title: &str, text: &str, buttons: &[&str]) -> ObjRef {
    let root = ui.insert_node(parent, Rect::new(0, 0, 200, 110), WidgetKind::Msgbox { selected: -1 });
    ui.set_floating(root, parent, Attach::Center);
    // 样式：对话框 + 列布局
    ui.widget(root).style(
        crate::style::theme_obj()
            .border(crate::geometry::Color::WHITE, 2)
            .pad(12, 12, 10, 10)
            .layout(Layout::Flex(Flex {
                dir: FlexDir::Column, wrap: false,
                main: Align::Start, cross: Align::Center, track: Align::Start, gap: 8,
            })),
    );
    let t = ui.create_label(root, title);
    ui.widget(t).style(crate::style::Style::new().text_color(crate::geometry::Color::rgb(255, 200, 60)));
    let _msg = ui.create_label(root, text);
    // 按钮行
    let row = ui.create_obj(root);
    let mut rs = crate::style::Style::default();
    rs.bg_opa = Some(0);
    rs.layout = Some(Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Start, track: Align::Start, gap: 12,
    }));
    ui.set_style(row, rs);
    for (i, b) in buttons.iter().enumerate() {
        let btn = ui.create_button(row, b);
        ui.group_add(btn);
        // 点击：记录索引 → 通知 → 解锁并删除
        ui.add_event_cb(btn, EventKind::Clicked, Box::new(move |ui, _x, _| {
            if let Some(n) = ui.arena.get_mut(root) {
                if let WidgetKind::Msgbox { selected } = &mut n.kind {
                    *selected = i as i32;
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
