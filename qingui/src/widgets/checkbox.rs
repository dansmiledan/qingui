use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::EventKind;
use crate::geometry::{Color, Point, Rect};
use crate::input::Key;
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

const BOX: i32 = 12;

/// Checkbox widget state.
#[derive(Clone)]
pub struct CheckboxState {
    pub text: alloc::string::String,
    pub checked: bool,
}

pub(crate) fn draw(text: &str, checked: bool, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let ap = |b: u8| ctx.ap(b);
    let by = abs.y + (abs.h - BOX) / 2;
    let brect = Rect::new(abs.x, by, BOX, BOX);
    // Box
    d.draw_border(brect, 1, 2, Color::rgb(150, 150, 160), ap(255), clip);
    if checked {
        // Check mark: two lines
        let p1 = Point { x: abs.x + 2, y: by + 6 };
        let p2 = Point { x: abs.x + 5, y: by + 9 };
        let p3 = Point { x: abs.x + 10, y: by + 3 };
        d.draw_line(p1, p2, 2, Color::rgb(80, 140, 255), ap(255), clip);
        d.draw_line(p2, p3, 2, Color::rgb(80, 140, 255), ap(255), clip);
    }
    d.draw_text_opa(
        Point { x: abs.x + BOX + 6, y: abs.y + (abs.h - crate::font::line_height(ctx.resolved.font)) / 2 },
        ctx.resolved.font,
        text,
        ctx.resolved.text_color,
        ap(255),
        clip,
    );
}

/// Builder for the Checkbox widget.
pub type CheckboxBuilder = WidgetBuilder<CheckboxCfg>;

/// Checkbox configuration: label text and initial checked state.
pub struct CheckboxCfg {
    text: alloc::string::String,
    checked: bool,
}

impl CheckboxCfg {
    /// Creates a builder with the given label text.
    pub fn new(text: &str) -> WidgetBuilder<CheckboxCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: CheckboxCfg { text: text.into(), checked: false } }
    }
}

impl WidgetBuilder<CheckboxCfg> {
    /// Sets the initial checked state.
    pub fn checked(mut self, on: bool) -> Self {
        self.cfg.checked = on;
        self
    }
}

impl WidgetCfg for CheckboxCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_opa = Some(0);
        s.text_color = Some(Color::WHITE);
        s
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or_else(|| {
            let font = crate::font::measure_font(common.style.as_ref(), ui);
            let (tw, _) = crate::font::text_size(font, &self.text);
            (BOX + 6 + tw, 16)
        });
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(CheckboxState { text: self.text, checked: self.checked }),
        );
        let base = common.style.take().unwrap_or_else(Self::default_style);
        ui.set_style(r, base.clone());
        let focused = common.style_focused.take().unwrap_or_else(|| {
            let mut s = base;
            s.border_color = Some(Color::WHITE);
            s.border_width = Some(1);
            s
        });
        ui.set_style_focused(r, focused);
        common.apply_tail(ui, r);
        r
    }
}

/// Checkbox toggle API (brought in via prelude or an explicit use)
pub trait UiCheckboxExt {
    /// Flips the checkbox's checked state and sends a ValueChanged event.
    fn toggle_checkbox(&mut self, obj: ObjRef);
}

impl UiCheckboxExt for Ui {
    fn toggle_checkbox(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
        self.update::<CheckboxState, _>(obj, |s| { s.checked = !s.checked; });
        self.invalidate_obj(obj);
        self.send_event(obj, EventKind::ValueChanged);
    }
}

impl super::Widget for CheckboxState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(&self.text, self.checked, ctx, c, clip) }
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, key: Key) -> super::KeyOutcome {
        if key == Key::Enter { self.checked = !self.checked; super::KeyOutcome::ValueChanged } else { super::KeyOutcome::Pass }
    }
    fn value(&self) -> i32 { self.checked as i32 }
    fn set_value(&mut self, v: i32) -> bool {
        let nv = v != 0;
        let c = nv != self.checked;
        self.checked = nv;
        c
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
