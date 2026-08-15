use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::ui::Ui;

/// Axis sizing strategy (modeled after Clay's FIT/GROW/FIXED/PERCENT model).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sizing {
    /// Content size (with min/max constraints); the default behavior when no sizing is set.
    Fit { min: i32, max: i32 },
    /// Fixed size.
    Fixed(i32),
    /// Share the parent container's remaining space (with constraints).
    Grow { min: i32, max: i32 },
    /// A percentage of the parent container's size (0-100).
    Percent(i32),
}

impl Sizing {
    /// `Grow` with no constraints: takes whatever space is left.
    pub const GROW: Sizing = Sizing::Grow { min: 0, max: i32::MAX };
    /// `Fit` with no constraints: natural content size.
    pub const FIT: Sizing = Sizing::Fit { min: 0, max: i32::MAX };
}

/// Floating-layer anchoring (mirrors Clay's floating attachTo).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attach {
    /// Centered on the target.
    Center,
    /// Outside the target's top edge, horizontally centered.
    Top,
    /// Outside the target's bottom edge, horizontally centered.
    Bottom,
    /// Outside the target's left edge, vertically centered.
    Left,
    /// Outside the target's right edge, vertically centered.
    Right,
}

/// Basis size (`Grow` first takes `min`, remaining space is allocated later; `parent` is used for `Percent`).
fn axis_basis(s: Option<Sizing>, content: i32, parent: i32) -> i32 {
    match s {
        None => content,
        Some(Sizing::Fit { min, max }) => content.clamp(min, max),
        Some(Sizing::Fixed(v)) => v,
        Some(Sizing::Grow { min, .. }) => min,
        Some(Sizing::Percent(p)) => parent * p.clamp(0, 100) / 100,
    }
}

/// Final size within a cell/target size (Grid: `Grow` = fill the cell).
fn axis_in_cell(s: Option<Sizing>, content: i32, cell: i32) -> i32 {
    match s {
        None => content,
        Some(Sizing::Fit { min, max }) => content.clamp(min, max),
        Some(Sizing::Fixed(v)) => v,
        Some(Sizing::Grow { min, max }) => cell.clamp(min, max),
        Some(Sizing::Percent(p)) => cell * p.clamp(0, 100) / 100,
    }
}

/// Flex layout direction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlexDir {
    /// Left to right.
    Row,
    /// Top to bottom.
    Column,
    /// Right to left.
    RowReverse,
    /// Bottom to top.
    ColumnReverse,
}

/// Alignment along a flex axis.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    /// Pack toward the start.
    Start,
    /// Center the items.
    Center,
    /// Pack toward the end.
    End,
    /// Distribute free space between items.
    SpaceBetween,
    /// Distribute free space around each item.
    SpaceAround,
    /// Distribute free space evenly including the ends.
    SpaceEvenly,
}

/// Flex layout parameters applied to a container.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flex {
    /// Main axis direction.
    pub dir: FlexDir,
    /// Whether items wrap onto multiple lines when the main axis overflows.
    pub wrap: bool,
    /// Alignment along the main axis.
    pub main: Align,
    /// Alignment of items along the cross axis within a line.
    pub cross: Align,
    /// Alignment of lines along the cross axis.
    pub track: Align,
    /// Spacing between items, in pixels.
    pub gap: i32,
}

/// Per-child flex inputs, gathered in one pass (a single allocation instead
/// of five parallel Vecs).
struct Kid {
    main_sz: i32,
    cross_sz: i32,
    sizing_m: Option<Sizing>,
    sizing_c: Option<Sizing>,
    aspect: Option<u32>,
}

/// Runs one flex layout pass on `container` (directly modifies the x/y of child rects).
/// `content` is the container's content box in its local coordinate space
/// (origin = padding offsets, size = rect minus padding), computed by Ui.
pub fn layout_flex<C: crate::pixel::PixelFormat>(ui: &mut Ui<C>, container: ObjRef, f: &Flex, content: crate::geometry::Rect) {
    // Filtered child list, gathered without cloning the children Vec.
    let mut order: Vec<ObjRef> = Vec::new();
    if let Some(n) = ui.arena.get(container) {
        for &k in &n.children {
            if !ui.is_hidden(k) && !ui.is_ignore_layout(k) {
                order.push(k);
            }
        }
    }
    if order.is_empty() {
        return;
    }
    let origin_x = content.x;
    let origin_y = content.y;
    let area_w = content.w;
    let area_h = content.h;

    let is_row = matches!(f.dir, FlexDir::Row | FlexDir::RowReverse);
    let reverse = matches!(f.dir, FlexDir::RowReverse | FlexDir::ColumnReverse);
    if reverse {
        order.reverse();
    }

    // Basis sizes (with sizing strategy; Grow first takes min, remaining space is allocated later)
    let area_main = if is_row { area_w } else { area_h };
    let area_cross_total = if is_row { area_h } else { area_w };
    let mut info: Vec<Kid> = Vec::with_capacity(order.len());
    for &k in &order {
        let r = ui.rect(k);
        let (sw, sh, aspect) = ui.arena.get(k).map(|n| (n.item_props.sizing_w, n.item_props.sizing_h, n.item_props.aspect_ratio)).unwrap_or((None, None, None));
        let (content_m, content_c) = if is_row { (r.w, r.h) } else { (r.h, r.w) };
        let (sm, sc) = if is_row { (sw, sh) } else { (sh, sw) };
        info.push(Kid {
            main_sz: axis_basis(sm, content_m, area_main),
            cross_sz: axis_basis(sc, content_c, area_cross_total),
            sizing_m: sm,
            sizing_c: sc,
            aspect,
        });
    }

    // Split into lines as index ranges into `order` (no per-line Vec allocs).
    let mut lines: Vec<(usize, usize)> = Vec::new();
    let mut cur_start = 0usize;
    let mut cur_main = 0i32;
    for i in 0..order.len() {
        let m = info[i].main_sz;
        let empty = cur_start == i;
        let need = if empty { m } else { cur_main + f.gap + m };
        if f.wrap && !empty && need > area_main {
            lines.push((cur_start, i));
            cur_main = m;
            cur_start = i;
        } else {
            cur_main = need;
        }
    }
    if cur_start < order.len() {
        lines.push((cur_start, order.len()));
    }

    // Line heights (cross-axis sizes)
    let line_cross: Vec<i32> = lines
        .iter()
        .map(|&(s, e)| info[s..e].iter().map(|k| k.cross_sz).max().unwrap_or(0))
        .collect();
    let total_cross: i32 = line_cross.iter().sum::<i32>() + f.gap * (lines.len() as i32 - 1).max(0);

    // track alignment: cross-axis distribution between lines
    let (mut cross_pos, track_gap) = distribute(total_cross, area_cross_total, f.track, lines.len() as i32, f.gap);

    for (li, &(ls, le)) in lines.iter().enumerate() {
        let line = &info[ls..le];
        // Grow children split the main-axis space remaining on the line
        let grow_count = line.iter().filter(|k| matches!(k.sizing_m, Some(Sizing::Grow { .. }))).count();
        if grow_count > 0 {
            let used: i32 = line.iter().map(|k| k.main_sz).sum::<i32>()
                + f.gap * (line.len() as i32 - 1).max(0);
            let free = (area_main - used).max(0);
            let share = free / grow_count as i32;
            for k in &mut info[ls..le] {
                if let Some(Sizing::Grow { min, max }) = k.sizing_m {
                    k.main_sz = (k.main_sz + share).clamp(min, max);
                }
            }
        }
        // Cross-axis Grow fills the container's cross axis
        for k in &mut info[ls..le] {
            if let Some(Sizing::Grow { min, max }) = k.sizing_c {
                k.cross_sz = area_cross_total.clamp(min, max);
            }
        }
        // Aspect ratio: derive the cross size from the final main size (takes priority over cross-axis sizing)
        for k in &mut info[ls..le] {
            if let Some(ratio) = k.aspect
                && ratio > 0
            {
                k.cross_sz = (k.main_sz as i64 * 1000 / ratio as i64) as i32;
            }
        }
        let line = &info[ls..le];
        let line_main: i32 = {
            let sum: i32 = line.iter().map(|k| k.main_sz).sum();
            sum + f.gap * (line.len() as i32 - 1).max(0)
        };
        let (mut main_pos, item_gap) = distribute(line_main, area_main, f.main, line.len() as i32, f.gap);
        for (i, k) in line.iter().enumerate() {
            let m = k.main_sz;
            let c = k.cross_sz;
            let lc = line_cross[li];
            let cross_off = align_offset(c, lc, f.cross);
            let (x, y) = if is_row {
                (origin_x + main_pos, origin_y + cross_pos + cross_off)
            } else {
                (origin_x + cross_pos + cross_off, origin_y + main_pos)
            };
            // Write back when sizing changes the size (transition animations are handled by layout_resize/layout_move)
            let (fw, fh) = if is_row { (m, c) } else { (c, m) };
            ui.layout_resize(order[ls + i], fw, fh);
            ui.layout_move(order[ls + i], x, y);
            main_pos += m + item_gap;
        }
        cross_pos += line_cross[li] + track_gap;
    }
}

/// Computes the start offset and item spacing: places the total `content` length inside `area`
/// according to `align`.
fn distribute(content: i32, area: i32, align: Align, count: i32, gap: i32) -> (i32, i32) {
    let free = (area - content).max(0);
    match align {
        Align::Start => (0, gap),
        Align::Center => (free / 2, gap),
        Align::End => (free, gap),
        Align::SpaceBetween => {
            if count > 1 {
                (0, gap + free / (count - 1))
            } else {
                (0, gap)
            }
        }
        Align::SpaceAround => {
            let g = free / count.max(1);
            (g / 2, gap + g)
        }
        Align::SpaceEvenly => {
            let g = free / (count + 1);
            (g, gap + g)
        }
    }
}

fn align_offset(item: i32, line: i32, align: Align) -> i32 {
    match align {
        Align::Start => 0,
        Align::Center => (line - item) / 2,
        Align::End => line - item,
        _ => 0, // Space* has no meaning for a single item
    }
}

/// Grid track sizing: a fixed pixel width, a flexible fraction, or content width.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    /// Fixed pixel size.
    Px(i32),
    /// Flexible fraction of the remaining space.
    Fr(u8),
    /// Sized to the largest single-cell child.
    Content,
}

/// Grid layout parameters applied to a container.
#[derive(Clone, PartialEq, Debug)]
pub struct Grid {
    /// Column tracks, left to right.
    pub cols: Vec<Track>,
    /// Row tracks, top to bottom.
    pub rows: Vec<Track>,
    /// Spacing between columns, in pixels.
    pub col_gap: i32,
    /// Spacing between rows, in pixels.
    pub row_gap: i32,
}

/// Solves track sizes: returns the pixel size of every track.
fn solve_tracks(
    tracks: &[Track],
    child_sizes: &[(u8, u8, i32)], // (start, span, size); only span=1 children feed Content
    gap: i32,
    area: i32,
) -> Vec<i32> {
    let mut sizes: Vec<i32> = tracks
        .iter()
        .map(|t| match t {
            Track::Px(p) => *p,
            Track::Fr(_) | Track::Content => 0,
        })
        .collect();
    // Content: take the largest size among the track's span=1 children
    for (start, span, size) in child_sizes {
        if *span == 1
            && let Some(Track::Content) = tracks.get(*start as usize)
        {
            sizes[*start as usize] = sizes[*start as usize].max(*size);
        }
    }
    let fixed: i32 = sizes.iter().sum::<i32>() + gap * (tracks.len() as i32 - 1).max(0);
    let remaining = (area - fixed).max(0);
    let fr_total: u32 = tracks
        .iter()
        .filter_map(|t| if let Track::Fr(w) = t { Some(*w as u32) } else { None })
        .sum();
    if fr_total > 0 {
        let mut used = 0i32;
        let last_fr = tracks.iter().rposition(|t| matches!(t, Track::Fr(_)));
        for (i, t) in tracks.iter().enumerate() {
            if let Track::Fr(w) = t {
                if Some(i) == last_fr {
                    sizes[i] = remaining - used; // last track absorbs the rounding error
                } else {
                    sizes[i] = remaining * *w as i32 / fr_total as i32;
                    used += sizes[i];
                }
            }
        }
    }
    sizes
}

fn track_offset(sizes: &[i32], idx: u8, gap: i32) -> i32 {
    sizes[..idx as usize].iter().sum::<i32>() + gap * idx as i32
}

/// Runs one grid layout pass on `container`: positions every child in its grid cell.
/// `content` is the container's content box in its local coordinate space
/// (origin = padding offsets, size = rect minus padding), computed by Ui.
pub fn layout_grid<C: crate::pixel::PixelFormat>(ui: &mut Ui<C>, container: ObjRef, g: &Grid, content: crate::geometry::Rect) {
    let area_w = content.w;
    let area_h = content.h;
    let mut kids: Vec<ObjRef> = Vec::new();
    if let Some(n) = ui.arena.get(container) {
        for &k in &n.children {
            if !ui.is_hidden(k) && !ui.is_ignore_layout(k) {
                kids.push(k);
            }
        }
    }

    let col_sizes_in: Vec<(u8, u8, i32)> = kids
        .iter()
        .map(|&k| {
            let (c, s) = ui.grid_cell(k).0;
            (c, s, ui.rect(k).w)
        })
        .collect();
    let row_sizes_in: Vec<(u8, u8, i32)> = kids
        .iter()
        .map(|&k| {
            let (r, s) = ui.grid_cell(k).1;
            (r, s, ui.rect(k).h)
        })
        .collect();

    let col_px = solve_tracks(&g.cols, &col_sizes_in, g.col_gap, area_w);
    let row_px = solve_tracks(&g.rows, &row_sizes_in, g.row_gap, area_h);

    for &k in &kids {
        let ((ci, cs), (ri, rs)) = ui.grid_cell(k);
        // Cell size (including span and gaps)
        let span_w = |i: u8, s: u8| -> i32 {
            let mut w = 0;
            for t in i..(i + s) {
                w += col_px.get(t as usize).copied().unwrap_or(0);
            }
            w + g.col_gap * (s as i32 - 1)
        };
        let span_h = |i: u8, s: u8| -> i32 {
            let mut h = 0;
            for t in i..(i + s) {
                h += row_px.get(t as usize).copied().unwrap_or(0);
            }
            h + g.row_gap * (s as i32 - 1)
        };
        let (cw, ch) = (span_w(ci, cs), span_h(ri, rs));
        // The sizing strategy decides each child's size inside its cell
        let (sw, sh, aspect) = ui.arena.get(k).map(|n| (n.item_props.sizing_w, n.item_props.sizing_h, n.item_props.aspect_ratio)).unwrap_or((None, None, None));
        let cur = ui.rect(k);
        let mut fw = axis_in_cell(sw, cur.w, cw);
        let mut fh = axis_in_cell(sh, cur.h, ch);
        // Aspect ratio: fit proportionally within the cell bounds
        if let Some(ratio) = aspect
            && ratio > 0
        {
            fh = (fw as i64 * 1000 / ratio as i64) as i32;
            if fh > ch {
                fh = ch;
                fw = (ch as i64 * ratio as i64 / 1000) as i32;
            }
        }
        ui.layout_resize(k, fw, fh);
        let x = content.x + track_offset(&col_px, ci, g.col_gap);
        let y = content.y + track_offset(&row_px, ri, g.row_gap);
        ui.layout_move(k, x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn axis_basis_variants() {
        assert_eq!(axis_basis(None, 5, 100), 5);
        assert_eq!(axis_basis(Some(Sizing::Fixed(10)), 5, 100), 10);
        assert_eq!(axis_basis(Some(Sizing::Fit { min: 3, max: 8 }), 10, 100), 8);
        assert_eq!(axis_basis(Some(Sizing::Fit { min: 3, max: 8 }), 1, 100), 3);
        assert_eq!(axis_basis(Some(Sizing::Grow { min: 4, max: 100 }), 5, 100), 4);
        assert_eq!(axis_basis(Some(Sizing::Percent(50)), 0, 200), 100);
    }

    #[test]
    fn axis_in_cell_grow_fills() {
        assert_eq!(axis_in_cell(Some(Sizing::Grow { min: 0, max: 100 }), 5, 50), 50);
        assert_eq!(axis_in_cell(Some(Sizing::Grow { min: 80, max: 100 }), 5, 50), 80);
    }

    #[test]
    fn distribute_alignments() {
        assert_eq!(distribute(100, 200, Align::Start, 2, 4), (0, 4));
        assert_eq!(distribute(100, 200, Align::Center, 2, 4), (50, 4));
        assert_eq!(distribute(100, 200, Align::End, 2, 4), (100, 4));
        assert_eq!(distribute(100, 200, Align::SpaceBetween, 2, 4), (0, 104));
        assert_eq!(distribute(100, 200, Align::SpaceEvenly, 2, 4), (33, 37));
    }

    #[test]
    fn align_offset_cases() {
        assert_eq!(align_offset(10, 100, Align::Start), 0);
        assert_eq!(align_offset(10, 100, Align::Center), 45);
        assert_eq!(align_offset(10, 100, Align::End), 90);
        assert_eq!(align_offset(10, 100, Align::SpaceBetween), 0);
    }

    #[test]
    fn track_offset_accumulates() {
        assert_eq!(track_offset(&[10, 20, 30], 2, 4), 38);
        assert_eq!(track_offset(&[10], 0, 4), 0);
    }

    #[test]
    fn solve_tracks_fr_consumes_remaining() {
        let tracks = [Track::Px(10), Track::Fr(1), Track::Fr(2)];
        assert_eq!(solve_tracks(&tracks, &[], 0, 100), vec![10, 30, 60]);
    }

    #[test]
    fn solve_tracks_content_sizes() {
        let tracks = [Track::Content, Track::Fr(1)];
        let child_sizes = [(0u8, 1u8, 25i32)];
        assert_eq!(solve_tracks(&tracks, &child_sizes, 0, 100), vec![25, 75]);
    }

    #[test]
    fn distribute_space_around() {
        // free=100, count=2, gap=4 → g=50 → (g/2, gap+g) = (25, 54)
        assert_eq!(distribute(100, 200, Align::SpaceAround, 2, 4), (25, 54));
    }

    #[test]
    fn solve_tracks_last_fr_eats_rounding() {
        // 100 is not divisible by 3: the plain formula gives ~33 each, the last track absorbs
        // the rounding error → 34
        let tracks = [Track::Fr(1), Track::Fr(1), Track::Fr(1)];
        assert_eq!(solve_tracks(&tracks, &[], 0, 100), vec![33, 33, 34]);
    }
}
