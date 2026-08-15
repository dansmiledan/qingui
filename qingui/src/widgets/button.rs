use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Point, Rect};
use crate::pixel::PixelFormat;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Button widget state.
#[derive(Clone)]
pub struct ButtonState {
    pub text: alloc::string::String,
}

pub(crate) fn draw<C: PixelFormat>(text: &str, ctx: &WidgetCtx, d: &mut Canvas<'_, C>, clip: Rect) {
    let (tw, th) = crate::font::text_size(ctx.resolved.font, text);
    let p = Point {
        x: ctx.abs.x + (ctx.abs.w - tw) / 2,
        y: ctx.abs.y + (ctx.abs.h - th) / 2,
    };
    d.draw_text_opa(p, ctx.resolved.font, text, ctx.resolved.text_color, ctx.ap(255), clip);
}

/// Builder for the Button widget.
pub type ButtonBuilder<C = crate::geometry::Color> = WidgetBuilder<ButtonCfg, C>;

/// Button configuration: label text and the default content padding.
pub struct ButtonCfg {
    text: alloc::string::String,
    content_pad: (i32, i32),
}

impl ButtonCfg {
    /// Creates a builder with the given label text.
    pub fn new<C: PixelFormat>(text: &str) -> WidgetBuilder<ButtonCfg, C> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ButtonCfg { text: text.into(), content_pad: (24, 12) } }
    }
}

impl<C> WidgetBuilder<ButtonCfg, C> {
    /// Sets the padding added to the text size for the default widget size (default (24, 12)).
    pub fn content_pad(mut self, x: i32, y: i32) -> Self {
        self.cfg.content_pad = (x, y);
        self
    }
}

impl<C: PixelFormat> WidgetCfg<C> for ButtonCfg {
    fn default_style() -> Style {
        crate::style::theme_button()
    }

    fn build(self, ui: &mut Ui<C>, parent: ObjRef, mut common: CommonBuilder<C>) -> ObjRef {
        let (w, h) = common.size.unwrap_or_else(|| {
            let font = crate::font::measure_font(common.style.as_ref(), ui);
            let (tw, th) = crate::font::text_size(font, &self.text);
            (tw + self.content_pad.0, th + self.content_pad.1)
        });
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(ButtonState { text: self.text }));
        ui.set_style(r, common.style.take().unwrap_or_else(<Self as WidgetCfg<C>>::default_style));
        ui.set_style_focused(r, common.style_focused.take().unwrap_or_else(crate::style::theme_button_focused));
        if let Some(n) = ui.arena.get_mut(r) {
            n.flags |= crate::node::Flag::CLICKABLE;
        }
        common.apply_tail(ui, r);
        r
    }
}

pub(crate) fn create<C: PixelFormat>(ui: &mut Ui<C>, parent: ObjRef, text: &str) -> ObjRef {
    ButtonCfg::new(text).build(ui, parent)
}

impl<C: PixelFormat> super::Widget<C> for ButtonState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas<'_, C>, clip: Rect) { draw(&self.text, ctx, c, clip) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
