use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::canvas::Canvas;
use crate::geometry::{Color, Point, Rect};
use crate::style::Style;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetCtx;

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
    pub cell_w: i32,
    pub cell_h: i32,
}

/// Builder for the Table widget.
pub type TableBuilder = WidgetBuilder<TableCfg>;

/// Table configuration: grid dimensions and pre-filled cell contents.
pub struct TableCfg {
    cols: u8,
    rows: u8,
    cells: Vec<String>,
    cell_w: i32,
    cell_h: i32,
}

impl TableCfg {
    /// Creates a builder with the given grid dimensions (default cols*CELL_W x rows*CELL_H, transparent bg + white text).
    pub fn new(cols: u8, rows: u8) -> WidgetBuilder<TableCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: TableCfg {
                cols, rows,
                cells: alloc::vec![String::new(); cols as usize * rows as usize],
                cell_w: CELL_W,
                cell_h: CELL_H,
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
    /// Sets the cell width in pixels (default `CELL_W` = 60).
    pub fn cell_w(mut self, w: i32) -> Self {
        self.cfg.cell_w = w;
        self
    }
    /// Sets the cell height in pixels (default `CELL_H` = 16).
    pub fn cell_h(mut self, h: i32) -> Self {
        self.cfg.cell_h = h;
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
        let (w, h) = common.size.unwrap_or((self.cols as i32 * self.cell_w, self.rows as i32 * self.cell_h));
        let r = ui.insert_node(
            parent,
            Rect::new(0, 0, w, h),
            alloc::boxed::Box::new(TableState { cols: self.cols, rows: self.rows, cells: self.cells, cell_w: self.cell_w, cell_h: self.cell_h }),
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

impl TableState {
    fn draw_grid(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
        let abs = ctx.abs;
        let lclip = abs.intersect(&clip).unwrap_or(clip);
        let line_c = Color::rgb(70, 70, 90);
        let ap = ctx.ap(255);
        // Grid lines (the bottom/right edges are pulled 1px inside the half-open interval boundary)
        for c in 0..=self.cols as i32 {
            let x = (abs.x + c * self.cell_w).min(abs.right() - 1);
            d.draw_line(Point { x, y: abs.y }, Point { x, y: abs.bottom() }, 1, line_c, ap, lclip);
        }
        for r in 0..=self.rows as i32 {
            let y = (abs.y + r * self.cell_h).min(abs.bottom() - 1);
            d.draw_line(Point { x: abs.x, y }, Point { x: abs.right(), y }, 1, line_c, ap, lclip);
        }
        // Cell text
        for r in 0..self.rows as usize {
            for c in 0..self.cols as usize {
                let idx = r * self.cols as usize + c;
                if let Some(text) = self.cells.get(idx)
                    && !text.is_empty()
                {
                    d.draw_text_opa(
                        Point { x: abs.x + c as i32 * self.cell_w + 4, y: abs.y + r as i32 * self.cell_h + 4 },
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

impl super::Widget for TableState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { self.draw_grid(ctx, c, clip) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
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
