use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{WidgetCtx, WidgetKind};

/// Button widget state.
#[derive(Clone)]
pub struct ButtonState {
    pub text: alloc::string::String,
}

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let (tw, th) = crate::font::text_size(ctx.resolved.font, text);
    let p = Point {
        x: ctx.abs.x + (ctx.abs.w - tw) / 2,
        y: ctx.abs.y + (ctx.abs.h - th) / 2,
    };
    d.draw_text_opa(p, ctx.resolved.font, text, ctx.resolved.text_color, ctx.ap(255), clip);
}

/// Builder for the Button widget.
pub type ButtonBuilder = WidgetBuilder<ButtonCfg>;

/// Button configuration: label text.
pub struct ButtonCfg {
    text: alloc::string::String,
}

impl ButtonCfg {
    /// Creates a builder with the given label text.
    pub fn new(text: &str) -> WidgetBuilder<ButtonCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ButtonCfg { text: text.into() } }
    }
}

impl WidgetCfg for ButtonCfg {
    fn default_style() -> Style {
        crate::style::theme_button()
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or_else(|| {
            let font = crate::font::measure_font(common.style.as_ref(), ui);
            let (tw, th) = crate::font::text_size(font, &self.text);
            (tw + 24, th + 12)
        });
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Button(ButtonState { text: self.text }));
        ui.set_style(r, common.style.take().unwrap_or_else(Self::default_style));
        ui.set_style_pressed(r, common.style_pressed.take().unwrap_or_else(crate::style::theme_button_pressed));
        ui.set_style_focused(r, common.style_focused.take().unwrap_or_else(crate::style::theme_button_focused));
        if let Some(n) = ui.arena.get_mut(r) {
            n.flags |= crate::node::Flag::CLICKABLE;
        }
        common.apply_tail(ui, r);
        r
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, text: &str) -> ObjRef {
    ButtonCfg::new(text).build(ui, parent)
}

impl super::WidgetBehavior for ButtonState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(&self.text, ctx, d, clip) }
}
