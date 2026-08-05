use core::any::Any;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::input::Key;
use crate::ui::Ui;

use super::{TickOut, WidgetCtx};

/// User-defined widget: mounted as `WidgetKind::Custom` via `Ui::create_custom`,
/// participating in drawing/per-frame/key handling like built-in widgets.
///
/// Note: while `on_key` is being called, this node's kind is "taken out" (the node holds a placeholder `Obj`),
/// so modify your own state directly on `self`; operations on other nodes are unrestricted.
pub trait Widget {
    /// Content drawing (background/border/opa are handled uniformly by Ui)
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect);
    /// Per-frame progress: returns the active state (no per-frame behavior by default)
    fn tick(&mut self, _now: u64) -> TickOut {
        TickOut::IDLE
    }
    /// Key handling: returns true if consumed (not consumed by default, falls through to default focus move/Clicked)
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, _key: Key) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// State wrapper for the Custom variant: delegates the trait object into `WidgetBehavior` so the macro treats it uniformly.
/// Not `Clone` (trait objects cannot be cloned), so `WidgetKind` no longer derives `Clone`.
pub struct CustomState(pub alloc::boxed::Box<dyn Widget>);

impl super::WidgetBehavior for CustomState {
    fn draw(&self, ctx: &super::WidgetCtx, d: &mut DrawBuf, clip: Rect) { self.0.draw(ctx, d, clip) }
    fn tick(&mut self, now: u64) -> super::TickOut { self.0.tick(now) }
}
