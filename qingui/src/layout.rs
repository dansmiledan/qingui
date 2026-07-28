use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::ui::Ui;

/// 轴尺寸策略（借鉴 Clay 的 FIT/GROW/FIXED/PERCENT 模型）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sizing {
    /// 内容尺寸（带 min/max 约束）；未设置 sizing 时的默认行为
    Fit { min: i32, max: i32 },
    /// 固定尺寸
    Fixed(i32),
    /// 瓜分父容器剩余空间（带约束）
    Grow { min: i32, max: i32 },
    /// 父容器尺寸的百分比（0-100）
    Percent(i32),
}

impl Sizing {
    pub const GROW: Sizing = Sizing::Grow { min: 0, max: i32::MAX };
    pub const FIT: Sizing = Sizing::Fit { min: 0, max: i32::MAX };
}

/// 浮层锚定方式（对齐 Clay 的 floating attachTo）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attach {
    /// 与目标中心对齐
    Center,
    /// 目标上边缘外侧，水平居中
    Top,
    /// 目标下边缘外侧，水平居中
    Bottom,
    /// 目标左边缘外侧，垂直居中
    Left,
    /// 目标右边缘外侧，垂直居中
    Right,
}

/// 基础尺寸（Grow 先取 min，剩余空间稍后分配；parent 用于 Percent）
fn axis_basis(s: Option<Sizing>, content: i32, parent: i32) -> i32 {
    match s {
        None => content,
        Some(Sizing::Fit { min, max }) => content.clamp(min, max),
        Some(Sizing::Fixed(v)) => v,
        Some(Sizing::Grow { min, .. }) => min,
        Some(Sizing::Percent(p)) => parent * p.clamp(0, 100) / 100,
    }
}

/// 单元格/目标尺寸下的最终尺寸（Grid：Grow = 填满单元格）
fn axis_in_cell(s: Option<Sizing>, content: i32, cell: i32) -> i32 {
    match s {
        None => content,
        Some(Sizing::Fit { min, max }) => content.clamp(min, max),
        Some(Sizing::Fixed(v)) => v,
        Some(Sizing::Grow { min, max }) => cell.clamp(min, max),
        Some(Sizing::Percent(p)) => cell * p.clamp(0, 100) / 100,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlexDir {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Flex {
    pub dir: FlexDir,
    pub wrap: bool,
    pub main: Align,
    pub cross: Align,
    pub track: Align,
    pub gap: i32,
}

/// 对容器 container 执行一次 flex 布局（直接修改子对象 rect 的 x/y）
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

    // 尺寸基础值（含 sizing 策略；Grow 先取 min，剩余空间稍后分配）
    let area_main = if is_row { area_w } else { area_h };
    let area_cross_total = if is_row { area_h } else { area_w };
    let mut main_sz: Vec<i32> = Vec::with_capacity(order.len());
    let mut cross_sz: Vec<i32> = Vec::with_capacity(order.len());
    let mut main_grow: Vec<Option<Sizing>> = Vec::with_capacity(order.len());
    let mut cross_grow: Vec<Option<Sizing>> = Vec::with_capacity(order.len());
    for &k in &order {
        let st = ui.resolved_style(k);
        let r = ui.rect(k);
        let (content_m, content_c) = if is_row { (r.w, r.h) } else { (r.h, r.w) };
        let (sm, sc) = if is_row { (st.sizing_w, st.sizing_h) } else { (st.sizing_h, st.sizing_w) };
        main_sz.push(axis_basis(sm, content_m, area_main));
        cross_sz.push(axis_basis(sc, content_c, area_cross_total));
        main_grow.push(sm);
        cross_grow.push(sc);
    }

    // 分行
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

    // 行高（交叉轴尺寸）
    let line_cross: Vec<i32> = lines
        .iter()
        .map(|l| l.iter().map(|&i| cross_sz[i]).max().unwrap_or(0))
        .collect();
    let total_cross: i32 = line_cross.iter().sum::<i32>() + f.gap * (lines.len() as i32 - 1).max(0);

    // track 对齐：行间交叉轴分布
    let (mut cross_pos, track_gap) = distribute(total_cross, area_cross_total, f.track, lines.len() as i32, f.gap);

    for (li, line) in lines.iter().enumerate() {
        // Grow 子对象平分主轴剩余空间
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
        // 交叉轴 Grow 撑满容器交叉轴
        for &i in line {
            if let Some(Sizing::Grow { min, max }) = cross_grow[i] {
                cross_sz[i] = area_cross_total.clamp(min, max);
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
            // sizing 改变尺寸时写回（set_size 会标布局脏，pass 结束后统一清除）
            let (fw, fh) = if is_row { (m, c) } else { (c, m) };
            let cur = ui.rect(order[i]);
            if cur.w != fw || cur.h != fh {
                ui.set_size(order[i], fw, fh);
            }
            ui.set_pos(order[i], x, y);
            main_pos += m + item_gap;
        }
        cross_pos += line_cross[li] + track_gap;
    }
}

/// 计算起始位置与项间距：把 content 总长按 align 放入 area
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
        _ => 0, // Space* 对单 item 无意义
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    Px(i32),
    Fr(u8),
    Content,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Grid {
    pub cols: Vec<Track>,
    pub rows: Vec<Track>,
    pub col_gap: i32,
    pub row_gap: i32,
}

/// 轨道尺寸求解：返回每条轨道的像素尺寸
fn solve_tracks(
    tracks: &[Track],
    child_sizes: &[(u8, u8, i32)], // (起始, 跨度, 尺寸)，仅 span=1 参与 Content
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
    // Content：取该轨道 span=1 子对象最大尺寸
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
                    sizes[i] = remaining - used; // 最后一条吃掉取整误差
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
        // 单元格尺寸（含 span 与 gap）
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
        // sizing 策略决定子对象在单元格内的尺寸
        let st = ui.resolved_style(k);
        let cur = ui.rect(k);
        let fw = axis_in_cell(st.sizing_w, cur.w, cw);
        let fh = axis_in_cell(st.sizing_h, cur.h, ch);
        if cur.w != fw || cur.h != fh {
            ui.set_size(k, fw, fh);
        }
        let x = style.pad_left + track_offset(&col_px, ci, g.col_gap);
        let y = style.pad_top + track_offset(&row_px, ri, g.row_gap);
        ui.set_pos(k, x, y);
    }
}
