use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::event::EventKind;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::pixel::PixelFormat;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

/// Switch widget state.
#[derive(Clone)]
pub struct SwitchState {
    pub on: bool,
}

pub(crate) fn draw<C: PixelFormat>(on: bool, ctx: &WidgetCtx, d: &mut Canvas<'_, C>, clip: Rect) {
    let abs = ctx.abs;
    let tc = if on { Color::rgb(60, 180, 90) } else { Color::rgb(90, 90, 90) };
    d.fill_rounded(abs, abs.h / 2, tc, ctx.ap(255), clip);
    let k = abs.h - 4;
    let kx = if on { abs.right() - k - 2 } else { abs.x + 2 };
    d.fill_rounded(Rect::new(kx, abs.y + 2, k, k), k / 2, Color::WHITE, ctx.ap(255), clip);
}

/// Builder for the Switch widget.
pub type SwitchBuilder<C = crate::geometry::Color> = WidgetBuilder<SwitchCfg, C>;

/// Switch configuration: initial on/off state.
pub struct SwitchCfg {
    on: bool,
}

impl SwitchCfg {
    /// Creates a builder with the switch initially off.
    pub fn new<C: PixelFormat>() -> WidgetBuilder<SwitchCfg, C> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: SwitchCfg { on: false } }
    }
}

impl<C> WidgetBuilder<SwitchCfg, C> {
    /// Sets the initial on/off state.
    pub fn checked(mut self, on: bool) -> Self {
        self.cfg.on = on;
        self
    }
}

impl<C: PixelFormat> WidgetCfg<C> for SwitchCfg {
    fn default_style() -> Style {
        crate::style::theme_switch()
    }

    fn build(self, ui: &mut Ui<C>, parent: ObjRef, mut common: CommonBuilder<C>) -> ObjRef {
        let (w, h) = common.size.unwrap_or((40, 20));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(SwitchState { on: self.on }));
        ui.set_style(r, common.style.take().unwrap_or_else(<Self as WidgetCfg<C>>::default_style));
        ui.set_style_focused(r, common.style_focused.take().unwrap_or_else(crate::style::theme_switch_focused));
        common.apply_tail(ui, r);
        r
    }
}

/// Switch toggle API (brought in via prelude or an explicit use)
pub trait UiSwitchExt {
    /// Flips the switch's on/off state and sends a ValueChanged event.
    fn toggle_switch(&mut self, obj: ObjRef);
}

impl<C: PixelFormat> UiSwitchExt for Ui<C> {
    fn toggle_switch(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
        self.update::<SwitchState, _>(obj, |s| { s.on = !s.on; });
        self.invalidate_obj(obj);
        self.send_event(obj, EventKind::ValueChanged);
    }
}

impl<C: PixelFormat> super::Widget<C> for SwitchState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas<'_, C>, clip: Rect) { draw(self.on, ctx, c, clip) }
    fn on_key(&mut self, _ui: &mut Ui<C>, _obj: ObjRef, key: Key) -> super::KeyOutcome {
        if key == Key::Enter { self.on = !self.on; super::KeyOutcome::ValueChanged } else { super::KeyOutcome::Pass }
    }
    fn value(&self) -> i32 { self.on as i32 }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
