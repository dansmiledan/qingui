use crate::arena::ObjRef;
use crate::canvas::Canvas;
use embedded_graphics::pixelcolor::{PixelColor, Rgb888};
use crate::geometry::{Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Builder for the Spinner widget.
pub type SpinnerBuilder<C = embedded_graphics::pixelcolor::Rgb888> = WidgetBuilder<SpinnerCfg, C>;

/// Spinner configuration: arc line width and rotation period.
pub struct SpinnerCfg {
    line_width: i32,
    period_ms: u64,
}

impl SpinnerCfg {
    /// Creates a builder (default 32x32, transparent bg).
    pub fn new<C: PixelColor + From<Rgb888>>() -> WidgetBuilder<SpinnerCfg, C> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: SpinnerCfg { line_width: 3, period_ms: 1800 } }
    }
}

impl<C> WidgetBuilder<SpinnerCfg, C> {
    /// Sets the arc line width in pixels (default 3).
    pub fn line_width(mut self, w: i32) -> Self {
        self.cfg.line_width = w;
        self
    }
    /// Sets the rotation period in ms (default 1800).
    pub fn period_ms(mut self, ms: u64) -> Self {
        self.cfg.period_ms = ms;
        self
    }
}

impl<C: PixelColor + From<Rgb888>> WidgetCfg<C> for SpinnerCfg {
    fn default_style() -> Style<C> {
        Style::default()
    }

    fn build(self, ui: &mut Ui<C>, parent: ObjRef, mut common: CommonBuilder<C>) -> ObjRef {
        let (w, h) = common.size.unwrap_or((32, 32));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(SpinnerState { line_width: self.line_width, period_ms: self.period_ms }),
        );
        let s = common.style.take().unwrap_or_else(<Self as WidgetCfg<C>>::default_style);
        ui.set_style(r, s);
        common.apply_tail(ui, r);
        r
    }
}

/// Spinner state: geometry/timing resolved at build time; the widget only rotates with time.
pub struct SpinnerState {
    pub line_width: i32,
    pub period_ms: u64,
}

impl SpinnerState {
    fn draw_arc_ind<C: PixelColor + From<Rgb888>>(&self, ctx: &WidgetCtx<'_, C>, d: &mut Canvas<'_, C>, clip: Rect) {
        let abs = ctx.abs;
        let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
        let r = abs.w.min(abs.h) / 2 - 2;
        if r <= 0 {
            return;
        }
        // Continuous rotation start + triangle-wave sweep length (smooth expanding/contracting, no jumps)
        let period = self.period_ms.max(1);
        let start = ((ctx.now % period) * 360 / period) as i32;
        let phase = (ctx.now / 7) as i32 % 300;
        let tri = if phase < 150 { phase } else { 300 - phase };
        let sweep = 60 + tri;
        d.draw_arc(c, r, self.line_width, start, start + sweep, Rgb888::new(80, 140, 255).into(), clip);
    }
}

impl<C: PixelColor + From<Rgb888>> super::Widget<C> for SpinnerState {
    fn draw(&self, ctx: &WidgetCtx<'_, C>, c: &mut super::Canvas<'_, C>, clip: Rect) { self.draw_arc_ind(ctx, c, clip) }
    // Spinner spins forever
    fn tick(&mut self, _ui: &mut Ui<C>, _obj: ObjRef, _now: u64) -> super::TickOut { super::TickOut::ACTIVE }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
