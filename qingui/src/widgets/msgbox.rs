use alloc::boxed::Box;

use crate::arena::ObjRef;
use crate::event::EventKind;
use crate::geometry::Rect;
use crate::layout::{Align, Attach, Flex, FlexDir, Layout};
use crate::ui::Ui;
use super::WidgetKind;

/// Msgbox widget state: index of the clicked button (-1 if none).
#[derive(Clone)]
pub struct MsgboxState {
    pub selected: i32,
}

/// Modal message box: title + text + button row, floating centered with focus locked.
/// Closes on button click: the root object receives `EventKind::ValueChanged`;
/// read the clicked button index with `ObjRef::msgbox_selected` (Esc close is -1).
/// Msgbox builder: modal message box (title + text + button row)
pub struct MsgboxBuilder {
    title: alloc::string::String,
    text: alloc::string::String,
    buttons: alloc::vec::Vec<alloc::string::String>,
}

impl MsgboxBuilder {
    /// Creates a builder with the given title and message text.
    pub fn new(title: &str, text: &str) -> Self {
        Self {
            title: title.into(),
            text: text.into(),
            buttons: alloc::vec::Vec::new(),
        }
    }
    /// Sets the button labels.
    pub fn buttons(mut self, buttons: &[&str]) -> Self {
        self.buttons = buttons.iter().map(|s| (*s).into()).collect();
        self
    }

    /// Builds the message box into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let refs: alloc::vec::Vec<&str> = self.buttons.iter().map(|s| s.as_str()).collect();
        create(ui, parent, &self.title, &self.text, &refs)
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, title: &str, text: &str, buttons: &[&str]) -> ObjRef {
    let root = ui.insert_node(parent, Rect::new(0, 0, 200, 110), WidgetKind::Msgbox(MsgboxState { selected: -1 }));
    ui.set_floating(root, parent, Attach::Center);
    ui.move_to_front(root); // popups draw on top (children order is the stacking order)
    // Style: dialog + column layout
    ui.set_style(root,
        crate::style::theme_obj()
            .border(crate::geometry::Color::WHITE, 2),
    );
    ui.set_pad(root, (12, 12, 10, 10));
    ui.set_layout(root, Layout::Flex(Flex {
        dir: FlexDir::Column, wrap: false,
        main: Align::Start, cross: Align::Center, track: Align::Start, gap: 8,
    }));
    let t = crate::widgets::label::create(ui, root, title);
    ui.set_style(t, crate::style::Style::new().text_color(crate::geometry::Color::rgb(255, 200, 60)));
    let _msg = crate::widgets::label::create(ui, root, text);
    // Button row
    let row = ui.insert_node(root, Rect::default(), WidgetKind::Obj(super::obj::ObjState));
    let mut rs = crate::style::Style::default();
    rs.bg_opa = Some(0);
    ui.set_style(row, rs);
    ui.set_layout(row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Start, track: Align::Start, gap: 12,
    }));
    for (i, b) in buttons.iter().enumerate() {
        let btn = crate::widgets::button::create(ui, row, b);
        ui.group_add(btn);
        // On click: record the index → notify → unlock and delete
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
        // Esc: selected stays -1, close directly
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

impl super::WidgetBehavior for MsgboxState {
    // Msgbox is an ordinary container (child objects are drawn normally)
    fn draw(&self, _ctx: &super::WidgetCtx, _d: &mut crate::draw::DrawBuf, _clip: Rect) {}
}

/// Msgbox-specific API (brought in via prelude or an explicit use)
pub trait UiMsgboxExt {
    /// Reads the clicked button index (-1 if none selected / closed with Esc)
    fn msgbox_selected(&self, obj: ObjRef) -> i32;
}

impl UiMsgboxExt for Ui {
    fn msgbox_selected(&self, obj: ObjRef) -> i32 {
        self.kind(obj).and_then(|k| k.as_msgbox()).map(|s| s.selected).unwrap_or(-1)
    }
}
