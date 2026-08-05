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
#[derive(Clone, PartialEq, Debug)]
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

/// Runs one flex layout pass on `container` (directly modifies the x/y of child rects).
pub fn layout_flex(ui: &mut Ui, container: ObjRef, f: &Flex) {
    let kids: Vec<ObjRef> = ui
        .children(container)
        .into_iter()
        .filter(|&k| !ui.is_hidden(k) && !ui.is_ignore_layout(k))
        .collect();
    if kids.is_empty() {
        return;
    }
    let style = ui.resolved_style(container);
    let origin_x = style.pad_left;
    let origin_y = style.pad_top;
    let area_w = ui.rect(container).w - style.pad_left - style.pad_right;
    let area_h = ui.rect(container).h - style.pad_top - style.pad_bottom;

    let is_row = matches!(f.dir, FlexDir::Row | FlexDir::RowReverse);
    let reverse = matches!(f.dir, FlexDir::RowReverse | FlexDir::ColumnReverse);
    let mut order = kids.clone();
    if reverse {
        order.reverse();
    }

    // Basis sizes (with sizing strategy; Grow first takes min, remaining space is allocated later)
    let area_main = if is_row { area_w } else { area_h };
    let area_cross_total = if is_row { area_h } else { area_w };
    let mut main_sz: Vec<i32> = Vec::with_capacity(order.len());
    let mut cross_sz: Vec<i32> = Vec::with_capacity(order.len());
    let mut main_grow: Vec<Option<Sizing>> = Vec::with_capacity(order.len());
    let mut cross_grow: Vec<Option<Sizing>> = Vec::with_capacity(order.len());
    let mut aspect: Vec<Option<u32>> = Vec::with_capacity(order.len());
    for &k in &order {
        let st = ui.resolved_style(k);
        let r = ui.rect(k);
        let (content_m, content_c) = if is_row { (r.w, r.h) } else { (r.h, r.w) };
        let (sm, sc) = if is_row { (st.sizing_w, st.sizing_h) } else { (st.sizing_h, st.sizing_w) };
        main_sz.push(axis_basis(sm, content_m, area_main));
        cross_sz.push(axis_basis(sc, content_c, area_cross_total));
        main_grow.push(sm);
        cross_grow.push(sc);
        aspect.push(st.aspect_ratio);
    }

    // Split into lines
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut cur_main = 0i32;
    for i in 0..order.len() {
        let m = main_sz[i];
        let need = if cur.is_empty() { m } else { cur_main + f.gap + m };
        if f.wrap && !cur.is_empty() && need > area_main {
            lines.push(core::mem::take(&mut cur));
            cur_main = 0;
        }
        cur_main = if cur.is_empty() { m } else { cur_main + f.gap + m };
        cur.push(i);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }

    // Line heights (cross-axis sizes)
    let line_cross: Vec<i32> = lines
        .iter()
        .map(|l| l.iter().map(|&i| cross_sz[i]).max().unwrap_or(0))
        .collect();
    let total_cross: i32 = line_cross.iter().sum::<i32>() + f.gap * (lines.len() as i32 - 1).max(0);

    // track alignment: cross-axis distribution between lines
    let (mut cross_pos, track_gap) = distribute(total_cross, area_cross_total, f.track, lines.len() as i32, f.gap);

    for (li, line) in lines.iter().enumerate() {
        // Grow children split the main-axis space remaining on the line
        let grow_idx: Vec<usize> = line
            .iter()
            .copied()
            .filter(|&i| matches!(main_grow[i], Some(Sizing::Grow { .. })))
            .collect();
        if !grow_idx.is_empty() {
            let used: i32 = line.iter().map(|&i| main_sz[i]).sum::<i32>()
                + f.gap * (line.len() as i32 - 1).max(0);
            let free = (area_main - used).max(0);
            let share = free / grow_idx.len() as i32;
            for &i in &grow_idx {
                if let Some(Sizing::Grow { min, max }) = main_grow[i] {
                    main_sz[i] = (main_sz[i] + share).clamp(min, max);
                }
            }
        }
        // Cross-axis Grow fills the container's cross axis
        for &i in line {
            if let Some(Sizing::Grow { min, max }) = cross_grow[i] {
                cross_sz[i] = area_cross_total.clamp(min, max);
            }
        }
        // Aspect ratio: derive the cross size from the final main size (takes priority over cross-axis sizing)
        for &i in line {
            if let Some(ratio) = aspect[i] {
                if ratio > 0 {
                    cross_sz[i] = (main_sz[i] as i64 * 1000 / ratio as i64) as i32;
                }
            }
        }
        let line_main: i32 = {
            let sum: i32 = line.iter().map(|&i| main_sz[i]).sum();
            sum + f.gap * (line.len() as i32 - 1).max(0)
        };
        let (mut main_pos, item_gap) = distribute(line_main, area_main, f.main, line.len() as i32, f.gap);
        for &i in line {
            let m = main_sz[i];
            let c = cross_sz[i];
            let lc = line_cross[li];
            let cross_off = align_offset(c, lc, f.cross);
            let (x, y) = if is_row {
                (origin_x + main_pos, origin_y + cross_pos + cross_off)
            } else {
                (origin_x + cross_pos + cross_off, origin_y + main_pos)
            };
            // Write back when sizing changes the size (transition animations are handled by layout_resize/layout_move)
            let (fw, fh) = if is_row { (m, c) } else { (c, m) };
            ui.layout_resize(order[i], fw, fh);
            ui.layout_move(order[i], x, y);
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
        if *span == 1 {
            if let Some(Track::Content) = tracks.get(*start as usize) {
                sizes[*start as usize] = sizes[*start as usize].max(*size);
            }
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
pub fn layout_grid(ui: &mut Ui, container: ObjRef, g: &Grid) {
    let style = ui.resolved_style(container);
    let area_w = ui.rect(container).w - style.pad_left - style.pad_right;
    let area_h = ui.rect(container).h - style.pad_top - style.pad_bottom;
    let kids: Vec<ObjRef> = ui.children(container).into_iter().filter(|&k| !ui.is_hidden(k) && !ui.is_ignore_layout(k)).collect();

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
        let st = ui.resolved_style(k);
        let cur = ui.rect(k);
        let mut fw = axis_in_cell(st.sizing_w, cur.w, cw);
        let mut fh = axis_in_cell(st.sizing_h, cur.h, ch);
        // Aspect ratio: fit proportionally within the cell bounds
        if let Some(ratio) = st.aspect_ratio {
            if ratio > 0 {
                fh = (fw as i64 * 1000 / ratio as i64) as i32;
                if fh > ch {
                    fh = ch;
                    fw = (ch as i64 * ratio as i64 / 1000) as i32;
                }
            }
        }
        ui.layout_resize(k, fw, fh);
        let x = style.pad_left + track_offset(&col_px, ci, g.col_gap);
        let y = style.pad_top + track_offset(&row_px, ri, g.row_gap);
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
