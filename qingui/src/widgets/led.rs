use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect, blend};
use crate::pixel::PixelFormat;
use crate::style::Style;
use crate::ui::Ui;
use embedded_graphics::pixelcolor::RgbColor;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Led widget state.
#[derive(Clone)]
pub struct LedState {
    pub color: Color,
    pub bright: u8,
}

pub(crate) fn draw<C: PixelFormat>(color: Color, bright: u8, ctx: &WidgetCtx, d: &mut Canvas<'_, C>, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 1;
    if r <= 0 {
        return;
    }
    // Brightness: gradient from black to the solid color
    let on = blend(Color::BLACK, color, bright);
    d.fill_circle(c, r, on, clip);
    d.draw_circle(c, r, 1, Color::new(90, 90, 100), clip);
}

/// Builder for the Led widget.
pub type LedBuilder<C = crate::geometry::Color> = WidgetBuilder<LedCfg, C>;

/// Led configuration: base color and optional initial brightness.
pub struct LedCfg {
    color: Color,
    bright: Option<u8>,
}

impl LedCfg {
    /// Creates a builder with the given LED color (default 16x16, transparent bg).
    pub fn new<C: PixelFormat>(color: Color) -> WidgetBuilder<LedCfg, C> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: LedCfg { color, bright: None } }
    }
}

impl<C> WidgetBuilder<LedCfg, C> {
    /// Sets the initial brightness (0..=255, default 255).
    pub fn bright(mut self, bright: u8) -> Self {
        self.cfg.bright = Some(bright);
        self
    }
}

impl<C: PixelFormat> WidgetCfg<C> for LedCfg {
    fn default_style() -> Style {
        Style::default()
    }

    fn build(self, ui: &mut Ui<C>, parent: ObjRef, mut common: CommonBuilder<C>) -> ObjRef {
        let (w, h) = common.size.unwrap_or((16, 16));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(LedState { color: self.color, bright: self.bright.unwrap_or(255) }),
        );
        let s = common.style.take().unwrap_or_else(<Self as WidgetCfg<C>>::default_style);
        ui.set_style(r, s);
        common.apply_tail(ui, r);
        r
    }
}

impl<C: PixelFormat> super::Widget<C> for LedState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas<'_, C>, clip: Rect) { draw(self.color, self.bright, ctx, c, clip) }
    fn value(&self) -> i32 { self.bright as i32 }
    fn set_value(&mut self, v: i32) -> bool {
        let nv = v.clamp(0, 255) as u8;
        let c = nv != self.bright;
        self.bright = nv;
        c
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
