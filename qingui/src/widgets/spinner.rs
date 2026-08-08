use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{WidgetCtx, WidgetKind};

pub(crate) fn draw(ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 2;
    if r <= 0 {
        return;
    }
    // Continuous rotation start + triangle-wave sweep length (smooth expanding/contracting, no jumps)
    let start = (ctx.now / 5) as i32 % 360;
    let phase = (ctx.now / 7) as i32 % 300;
    let tri = if phase < 150 { phase } else { 300 - phase };
    let sweep = 60 + tri;
    d.draw_arc(c, r, 3, start, start + sweep, Color::rgb(80, 140, 255), ctx.ap(255), clip);
}

/// Builder for the Spinner widget.
pub type SpinnerBuilder = WidgetBuilder<SpinnerCfg>;

/// Spinner configuration: no widget-specific fields.
pub struct SpinnerCfg;

impl SpinnerCfg {
    /// Creates a builder (default 32x32, transparent bg).
    pub fn new() -> WidgetBuilder<SpinnerCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: SpinnerCfg }
    }
}

impl WidgetCfg for SpinnerCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_opa = Some(0);
        s
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((32, 32));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(WidgetKind::Spinner(SpinnerState)));
        let mut s = common.style.take().unwrap_or_else(Self::default_style);
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0);
        }
        ui.set_style(r, s);
        common.apply_tail(ui, r);
        r
    }
}

/// Placeholder state: Spinner carries no data, it only keeps the macro treating all variants uniformly
pub struct SpinnerState;

impl super::WidgetBehavior for SpinnerState {
    fn draw(&self, ctx: &super::WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(ctx, d, clip) }
    // Spinner spins forever
    fn tick(&mut self, _now: u64) -> super::TickOut { super::TickOut::ACTIVE }
}
