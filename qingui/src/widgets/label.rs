use alloc::string::String;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{MeasureCtx, WidgetCtx};

/// Label widget state.
#[derive(Clone)]
pub struct LabelState {
    pub text: String,
}

pub(crate) fn draw(text: &str, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    d.draw_text_opa(
        Point { x: ctx.abs.x, y: ctx.abs.y },
        ctx.resolved.font,
        text,
        ctx.resolved.text_color,
        ctx.ap(255),
        clip,
    );
}

/// Builder for the Label widget.
pub type LabelBuilder = WidgetBuilder<LabelCfg>;

/// Label configuration: text content.
pub struct LabelCfg {
    text: String,
}

impl LabelCfg {
    /// Creates a builder with the given text.
    pub fn new(text: &str) -> WidgetBuilder<LabelCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: LabelCfg { text: text.into() } }
    }
}

impl WidgetCfg for LabelCfg {
    fn default_style() -> Style {
        crate::style::theme_label()
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or_else(|| {
            let font = crate::font::measure_font(common.style.as_ref(), ui);
            crate::font::text_size(font, &self.text)
        });
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(LabelState { text: self.text }));
        ui.set_style(r, common.style.take().unwrap_or_else(Self::default_style));
        common.apply_tail(ui, r);
        r
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, text: &str) -> ObjRef {
    LabelCfg::new(text).build(ui, parent)
}

pub(crate) fn set_text(ui: &mut Ui, obj: ObjRef, text: &str) {
    ui.invalidate_obj(obj);
    let font = crate::font::measure_font(ui.arena.get(obj).map(|n| &n.style), ui);
    let (w, h) = crate::font::text_size(font, text);
    if ui.update::<LabelState, _>(obj, |s| { s.text = text.into(); }).is_some() {
        if let Some(n) = ui.arena.get_mut(obj) {
            n.rect.w = w;
            n.rect.h = h;
        }
    }
    ui.invalidate_obj(obj);
    ui.layout_dirty = true;
}

pub(crate) fn text(ui: &Ui, obj: ObjRef) -> String {
    ui.widget::<LabelState>(obj).map(|s| s.text.clone()).unwrap_or_default()
}

impl super::Widget for LabelState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(&self.text, ctx, c, clip) }
    fn measure(&self, ctx: &MeasureCtx) -> (i32, i32) {
        if self.text.is_empty() { return ctx.cur; }
        crate::font::text_size(ctx.font, &self.text)
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}

/// Text API (brought in via prelude or an explicit use)
pub trait UiTextExt {
    /// Sets the label's text (also resizes the node to fit).
    fn set_text(&mut self, obj: ObjRef, text: &str);
    /// Returns the label's current text.
    fn text(&self, obj: ObjRef) -> String;
}

impl UiTextExt for Ui {
    fn set_text(&mut self, obj: ObjRef, text: &str) {
        set_text(self, obj, text);
    }

    fn text(&self, obj: ObjRef) -> String {
        text(self, obj)
    }
}
