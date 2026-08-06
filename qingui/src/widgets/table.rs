use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::{Color, Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
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

/// Builder for the Table widget.
pub type TableBuilder = WidgetBuilder<TableCfg>;

/// Table configuration: grid dimensions and pre-filled cell contents.
pub struct TableCfg {
    cols: u8,
    rows: u8,
    cells: Vec<String>,
}

impl TableCfg {
    /// Creates a builder with the given grid dimensions (default cols*CELL_W x rows*CELL_H, transparent bg + white text).
    pub fn new(cols: u8, rows: u8) -> WidgetBuilder<TableCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: TableCfg {
                cols, rows,
                cells: alloc::vec![String::new(); cols as usize * rows as usize],
            },
        }
    }
}

impl WidgetBuilder<TableCfg> {
    /// Pre-fills a cell's content (out-of-bounds is ignored)
    pub fn cell(mut self, row: u8, col: u8, text: &str) -> Self {
        if row < self.cfg.rows && col < self.cfg.cols {
            self.cfg.cells[row as usize * self.cfg.cols as usize + col as usize] = text.into();
        }
        self
    }
}

impl WidgetCfg for TableCfg {
    fn default_style() -> Style {
        let mut s = Style::default();
        s.bg_opa = Some(0);
        s.text_color = Some(Color::WHITE);
        s
    }

    fn build(self, ui: &mut Ui, parent: ObjRef, mut common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((self.cols as i32 * CELL_W, self.rows as i32 * CELL_H));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            WidgetKind::Table(TableState { cols: self.cols, rows: self.rows, cells: self.cells }),
        );
        let mut s = common.style.take().unwrap_or_else(Self::default_style);
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0);
        }
        if s.text_color.is_none() {
            s.text_color = Some(Color::WHITE);
        }
        ui.set_style(r, s);
        common.apply_tail(ui, r);
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
