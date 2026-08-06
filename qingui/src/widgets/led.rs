use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{WidgetCtx, WidgetKind};

/// Led widget state.
#[derive(Clone)]
pub struct LedState {
    pub color: Color,
    pub bright: u8,
}

pub(crate) fn draw(color: Color, bright: u8, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 1;
    if r <= 0 {
        return;
    }
    // Brightness: gradient from black to the solid color
    let on = Color::BLACK.blend(color, bright);
    d.fill_circle(c, r, on, ctx.ap(255), clip);
    d.draw_circle(c, r, 1, Color::rgb(90, 90, 100), ctx.ap(255), clip);
}

/// Builder for the Led widget.
pub type LedBuilder = WidgetBuilder<LedCfg>;

/// Led configuration: base color and optional initial brightness.
pub struct LedCfg {
    color: Color,
    bright: Option<u8>,
}

impl LedCfg {
    /// Creates a builder with the given LED color (default 16x16, transparent bg).
    pub fn new(color: Color) -> WidgetBuilder<LedCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: LedCfg { color, bright: None } }
    }
}

impl WidgetBuilder<LedCfg> {
    /// Sets the initial brightness (0..=255, default 255).
    pub fn bright(mut self, bright: u8) -> Self {
        self.cfg.bright = Some(bright);
        self
    }
}

impl WidgetCfg for LedCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_opa = Some(0);
        s
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((16, 16));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Led(LedState { color: self.color, bright: self.bright.unwrap_or(255) }),
        );
        let mut s = common.style.take().unwrap_or_else(Self::default_style);
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0);
        }
        ui.set_style(r, s);
        common.apply_tail(ui, r);
        r
    }
}

impl super::WidgetBehavior for LedState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.color, self.bright, ctx, d, clip) }
    fn value(&self) -> i32 { self.bright as i32 }
    fn set_value(&mut self, v: i32) -> bool {
        let nv = v.clamp(0, 255) as u8;
        let c = nv != self.bright;
        self.bright = nv;
        c
    }
}
