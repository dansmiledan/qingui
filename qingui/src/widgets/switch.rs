use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::EventKind;
use crate::geometry::{Color, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{WidgetCtx, WidgetKind};

/// Switch widget state.
#[derive(Clone)]
pub struct SwitchState {
    pub on: bool,
}

impl SwitchState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: super::KeyCtx) -> super::KeyOutcome {
        if key == Key::Enter { self.on = !self.on; super::KeyOutcome::ValueChanged } else { super::KeyOutcome::Pass }
    }
}

pub(crate) fn draw(on: bool, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let tc = if on { Color::rgb(60, 180, 90) } else { Color::rgb(90, 90, 90) };
    d.fill_rounded(abs, abs.h / 2, tc, ctx.ap(255), clip);
    let k = abs.h - 4;
    let kx = if on { abs.right() - k - 2 } else { abs.x + 2 };
    d.fill_rounded(Rect::new(kx, abs.y + 2, k, k), k / 2, Color::WHITE, ctx.ap(255), clip);
}

/// Builder for the Switch widget.
pub type SwitchBuilder = WidgetBuilder<SwitchCfg>;

/// Switch configuration: initial on/off state.
pub struct SwitchCfg {
    on: bool,
}

impl SwitchCfg {
    /// Creates a builder with the switch initially off.
    pub fn new() -> WidgetBuilder<SwitchCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: SwitchCfg { on: false } }
    }
}

impl WidgetBuilder<SwitchCfg> {
    /// Sets the initial on/off state.
    pub fn checked(mut self, on: bool) -> Self {
        self.cfg.on = on;
        self
    }
}

impl WidgetCfg for SwitchCfg {
    fn default_style() -> Style {
        crate::style::theme_switch()
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((40, 20));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(WidgetKind::Switch(SwitchState { on: self.on })));
        ui.set_style(r, common.style.take().unwrap_or_else(Self::default_style));
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

impl UiSwitchExt for Ui {
    fn toggle_switch(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
        if let Some(s) = self.kind_mut(obj).and_then(|k| k.as_kind_mut()?.as_switch_mut()) {
            s.on = !s.on;
        }
        self.invalidate_obj(obj);
        self.send_event(obj, EventKind::ValueChanged);
    }
}

impl super::WidgetBehavior for SwitchState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.on, ctx, d, clip) }
    fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome { self.on_key(key, ctx) }
    fn value(&self) -> i32 { self.on as i32 }
}
