use alloc::boxed::Box;

use crate::arena::ObjRef;
use crate::event::EventKind;
use crate::geometry::Rect;
use crate::layout::{Align, Attach, Flex, FlexDir};
use crate::pixel::PixelFormat;
use crate::ui::Ui;

/// Msgbox widget state: index of the clicked button (-1 if none).
#[derive(Clone)]
pub struct MsgboxState {
    pub selected: i32,
}

/// The msgbox root's fixed arrangement (column flex, centered items), run by
/// `MsgboxState::layout`.
pub(crate) const ROOT_FLEX: Flex = Flex {
    dir: FlexDir::Column, wrap: false,
    main: Align::Start, cross: Align::Center, track: Align::Start, gap: 8,
};

/// Modal message box: title + text + button row, floating centered with focus locked.
/// Closes on button click: the root object receives `EventKind::ValueChanged`;
/// read the clicked button index with `ObjRef::msgbox_selected` (Esc close is -1).
/// Msgbox builder: modal message box (title + text + button row)
pub struct MsgboxBuilder {
    title: alloc::string::String,
    text: alloc::string::String,
    buttons: alloc::vec::Vec<alloc::string::String>,
    size: Option<(i32, i32)>,
}

impl MsgboxBuilder {
    /// Creates a builder with the given title and message text.
    pub fn new(title: &str, text: &str) -> Self {
        Self {
            title: title.into(),
            text: text.into(),
            buttons: alloc::vec::Vec::new(),
            size: None,
        }
    }
    /// Sets the button labels.
    pub fn buttons(mut self, buttons: &[&str]) -> Self {
        self.buttons = buttons.iter().map(|s| (*s).into()).collect();
        self
    }

    /// Sets the box size in pixels (default 200x110).
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }

    /// Builds the message box into the parent node.
    pub fn build<C: PixelFormat>(self, ui: &mut Ui<C>, parent: ObjRef) -> ObjRef {
        let refs: alloc::vec::Vec<&str> = self.buttons.iter().map(|s| s.as_str()).collect();
        create(ui, parent, &self.title, &self.text, &refs, self.size)
    }
}

pub(crate) fn create<C: PixelFormat>(ui: &mut Ui<C>, parent: ObjRef, title: &str, text: &str, buttons: &[&str], size: Option<(i32, i32)>) -> ObjRef {
    let (w, h) = size.unwrap_or((200, 110));
    let root = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(MsgboxState { selected: -1 }));
    ui.set_floating(root, parent, Attach::Center);
    ui.move_to_front(root); // popups draw on top (children order is the stacking order)
    // Style: dialog + column layout
    ui.set_style(root,
        crate::style::theme_obj()
            .border(crate::geometry::Color::WHITE, 2),
    );
    ui.set_pad(root, (12, 12, 10, 10));
    // The root's column flex is ROOT_FLEX, run by MsgboxState::layout.
    let t = crate::widgets::label::create(ui, root, title);
    ui.set_style(t, crate::style::Style::new().text_color(crate::geometry::Color::rgb(255, 200, 60)));
    let _msg = crate::widgets::label::create(ui, root, text);
    // Button row
    let row = ui.insert_node(root, Rect::default(), alloc::boxed::Box::new(super::obj::Manual));
    ui.set_style(row, crate::style::Style::default());
    ui.set_flex(row, Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Start, track: Align::Start, gap: 12,
    });
    for (i, b) in buttons.iter().enumerate() {
        let btn = crate::widgets::button::create(ui, row, b);
        ui.group_add(btn);
        // On click: record the index → notify → unlock and delete
        ui.add_event_cb(btn, EventKind::Clicked, Box::new(move |ui, _x, _| {
            ui.update::<MsgboxState, _>(root, |s| s.selected = i as i32);
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

impl<C: PixelFormat> super::Widget<C> for MsgboxState {
    // Msgbox is an ordinary container (child objects are drawn normally)
    fn draw(&self, _ctx: &super::WidgetCtx, _c: &mut super::Canvas<'_, C>, _clip: Rect) {}
    // The root's fixed column flex arrangement
    fn layout(&mut self, ui: &mut Ui<C>, obj: ObjRef, content: Rect) {
        crate::layout::layout_flex(ui, obj, &ROOT_FLEX, content);
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}

/// Msgbox-specific API (brought in via prelude or an explicit use)
pub trait UiMsgboxExt {
    /// Reads the clicked button index (-1 if none selected / closed with Esc)
    fn msgbox_selected(&self, obj: ObjRef) -> i32;
}

impl<C: PixelFormat> UiMsgboxExt for Ui<C> {
    fn msgbox_selected(&self, obj: ObjRef) -> i32 {
        self.widget::<MsgboxState>(obj).map(|s| s.selected).unwrap_or(-1)
    }
}
