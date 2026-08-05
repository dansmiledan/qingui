use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

/// Cell width in pixels.
pub const CELL_W: i32 = 60;
/// Cell height in pixels.
pub const CELL_H: i32 = 16;

/// Table widget state: a grid of cell strings.
#[derive(Clone)]
pub struct TableState {
    pub cols: u8,
    pub rows: u8,
    pub cells: Vec<String>,
}

pub(crate) fn draw(cols: u8, rows: u8, cells: &[String], ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    let lclip = abs.intersect(&clip).unwrap_or(clip);
    let line_c = Color::rgb(70, 70, 90);
    let ap = ctx.ap(255);
    // Grid lines (the bottom/right edges are pulled 1px inside the half-open interval boundary)
    for c in 0..=cols as i32 {
        let x = (abs.x + c * CELL_W).min(abs.right() - 1);
        d.draw_line(Point { x, y: abs.y }, Point { x, y: abs.bottom() }, 1, line_c, ap, lclip);
    }
    for r in 0..=rows as i32 {
        let y = (abs.y + r * CELL_H).min(abs.bottom() - 1);
        d.draw_line(Point { x: abs.x, y }, Point { x: abs.right(), y }, 1, line_c, ap, lclip);
    }
    // Cell text
    for r in 0..rows as usize {
        for c in 0..cols as usize {
            let idx = r * cols as usize + c;
            if let Some(text) = cells.get(idx) {
                if !text.is_empty() {
                    d.draw_text_opa(
                        Point { x: abs.x + c as i32 * CELL_W + 4, y: abs.y + r as i32 * CELL_H + 4 },
                        ctx.resolved.font,
                        text,
                        ctx.resolved.text_color,
                        ap,
                        lclip,
                    );
                }
            }
        }
    }
}

/// Table builder: default cols*60 x rows*16, transparent bg + white text; cell() pre-fills cells
pub struct TableBuilder {
    cols: u8,
    rows: u8,
    cells: Vec<String>,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<crate::layout::Sizing>, Option<crate::layout::Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl TableBuilder {
    /// Creates a builder with the given grid dimensions.
    pub fn new(cols: u8, rows: u8) -> Self {
        Self {
            cols, rows,
            cells: alloc::vec![String::new(); cols as usize * rows as usize],
            size: None, style: None, sizing: None, transition: None, events: Vec::new(),
        }
    }
    /// Pre-fills a cell's content (out-of-bounds is ignored)
    pub fn cell(mut self, row: u8, col: u8, text: &str) -> Self {
        if row < self.rows && col < self.cols {
            self.cells[row as usize * self.cols as usize + col as usize] = text.into();
        }
        self
    }
    /// Sets the widget size.
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }
    /// Sets the style.
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }
    /// Sets the width/height sizing.
    pub fn sizing(mut self, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) -> Self {
        self.sizing = Some((w, h));
        self
    }
    /// Sets the transition duration and easing.
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    /// Registers an event callback.
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }

    /// Builds the widget into the parent node.
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((self.cols as i32 * CELL_W, self.rows as i32 * CELL_H));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Table(TableState { cols: self.cols, rows: self.rows, cells: self.cells }),
        );
        let mut s = self.style.unwrap_or_default();
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0);
        }
        if s.text_color.is_none() {
            s.text_color = Some(Color::WHITE);
        }
        ui.set_style(r, s);
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}

impl super::WidgetBehavior for TableState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(self.cols, self.rows, &self.cells, ctx, d, clip) }
}

/// Table-specific API (brought in via prelude or an explicit use)
pub trait UiTableExt {
    /// Sets a cell's text (out-of-bounds cells are ignored).
    fn table_set_cell(&mut self, obj: ObjRef, row: u8, col: u8, text: &str);
}

impl UiTableExt for Ui {
    fn table_set_cell(&mut self, obj: ObjRef, row: u8, col: u8, text: &str) {
        self.update::<TableState, _>(obj, |s| {
            if row < s.rows && col < s.cols {
                s.cells[row as usize * s.cols as usize + col as usize] = text.into();
            }
        });
    }
}
