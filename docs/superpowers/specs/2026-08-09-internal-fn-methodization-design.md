# 内部函数方法化重构 设计

日期：2026-08-09
状态：已批准（brainstorming 结论）

## 背景

widget 内部大量 `pub(crate)` 自由函数参数过多（`draw`/`select`/`ensure_visible` 等达 8-9 个），上一轮为通过 clippy 加了 5 处 `#[allow(clippy::too_many_arguments)]`。这些函数的参数绝大多数本就是 State 的字段，且每个函数只有一个调用点（对应 State 的 `Widget` impl 或同模块内部），应收编为方法。canvas/draw.rs 的光栅化助手同理：调用点在每个图元的绘制循环前已预计算好不变量，打包成几何 struct 后参数消失。

## 范围

- **做**：全部 widget 的内部自由函数方法化；`draw.rs` 的 `line_row_span`/`line_sdf_cov16`/`arc_cov16` 打包。
- **不做**：公开 API 不变；行为不变（纯代码移动）；`ListFx` 结构不变（避免 pub-struct 字段破坏面，沿用上次终审结论）；`lerp_t`/`circle_cov16`/`isqrt`/`div_*`/`dir_vec`（≤3 参）不动；spinner 弧形参数等逻辑常数不动。

## Widget 侧：自由函数 → State 私有方法

统一模式：函数体原样移入 `impl XState`（保持 `pub(crate)`，因同 crate 的 `Ui*Ext`/测试可能调用 select 类方法），签名删除所有 State 字段参数：

| 文件 | 现状（参数数） | 改后 |
|---|---|---|
| spinner.rs | `draw(line_width, period_ms, ctx, d, clip)` 5 | `SpinnerState::draw_arc_ind(&self, ctx, d, clip)` 3 |
| roller.rs | `draw(items, selected, sel_from, row_h, dur, ctx, d, clip)` 8；`sel_f` 4；`fx_active` 3；`select` 6 | `RollerState` 方法：`draw_rows` 3、`sel_f(now)` 1、`fx_active(now)` 1、`select(idx, now)` 2 |
| list.rs | `draw` 9；`select` 8→`select(idx, vis_h, now)` 3；`ensure_visible` 7→`(vis_h, now)` 2；`insert` 6→`(idx, text, now)` 3；`remove` 5→`(now) -> bool` 1 | `ListState` 方法 |
| arc.rs | `draw(min, max, value, track_w, start_deg, sweep_deg, ctx, d, clip)` 9 | `ArcState::draw_dial(&self, ctx, d, clip)` 3 |
| slider.rs | `draw(min, max, value, knob_w, ctx, d, clip)` 7 | `SliderState::draw_track(&self, ctx, d, clip)` 3 |
| checkbox.rs | `draw(text, checked, box_size, gap, ctx, d, clip)` 7 | `CheckboxState::draw_box(&self, ctx, d, clip)` 3 |
| table.rs | `draw(cols, rows, cells, cell_w, cell_h, ctx, d, clip)` 8 | `TableState::draw_grid(&self, ctx, d, clip)` 3 |
| dropdown.rs | `draw(items, selected, ctx, d, clip)` 5 | `DropdownState::draw_label(&self, ctx, d, clip)` 3 |
| chart.rs | `draw(s: &ChartState, ctx, d, clip)` 3 | `ChartState::draw_series(&self, ctx, d, clip)` 3（形态统一） |
| list.rs | `ListFx::active(now, dur)` / `prune(now, dur)` | 保持方法 + `dur` 参数不变 |

命名规则：draw 类方法用动词性私名（`draw_rows`/`draw_dial`/`draw_track`/`draw_box`/`draw_grid`/`draw_label`/`draw_series`/`draw_arc_ind`），避免与 trait 方法 `Widget::draw` 混淆；trait impl 变为一行转发。`lerp_t(start, now, dur)` 保持自由函数。

删除不再需要的全部 `#[allow(clippy::too_many_arguments)]`（roller.rs 1 处、list.rs 2 处、arc.rs 1 处、table.rs 1 处、draw.rs 2 处，共 7 处——方法化后参数均 ≤7）。

## canvas/draw.rs 侧：几何 struct

```rust
/// Thick-segment geometry with the per-line invariants precomputed once
/// (replaces the line_row_span/line_sdf_cov16 free functions' 9-arg calls).
pub(crate) struct ThickLine {
    x0: i32, y0: i32, x1: i32, y1: i32,
    dx: i32, dy: i32, len2: i32, rm: i64,
    ux: i64, uy: i64, len2_64: i64, inv_len: i64, r16: i64,
}

impl ThickLine {
    pub(crate) fn new(p1: Point, p2: Point, width: i32) -> Self { /* 原 draw_line_thick 前半段 */ }
    /// 原 line_row_span
    pub(crate) fn row_span(&self, y: i32) -> Option<(i32, i32)> { /* 原函数体，读 self */ }
    /// 原 line_sdf_cov16
    pub(crate) fn cov16(&self, px: i32, py: i32) -> i32 { /* 原函数体，读 self */ }
}
```

`canvas.rs::draw_line_thick` 改为：`let line = ThickLine::new(p1, p2, width);` 循环中 `line.row_span(y)` / `line.cov16(x, y)`。

`arc_cov16(dx, dy, outer, inner, s, e, and_mode)`（7 参）→

```rust
/// Annular-arc coverage parameters (replaces arc_cov16's 7-arg calls).
pub(crate) struct ArcGeom {
    outer: i32, inner: i32, s: (i32, i32), e: (i32, i32), and_mode: bool,
}
impl ArcGeom {
    pub(crate) fn cov16(&self, dx: i32, dy: i32) -> i32 { /* 原函数体 */ }
}
```

`canvas.rs:415` 调用点：循环前构造一次 `ArcGeom`，循环内 `geom.cov16(dx, dy)`。

## 实现要点

- 严格纯移动：除签名和 `self.` 前缀外不改任何表达式；特别是不动 roller/list 动画插值、chart 的 x-strip 裁剪（上两个特性的成果）。
- 借用注意：`UiListExt`/`UiRollerExt` 闭包内方法调用直接 `s.select(...)`；`on_key` 中先 `let now = ui.time();` 再 `self.select(...)`。
- `ListFx::active/prune` 的 `dur` 参数保留（`fx_dur` 在 `ListState` 上，不移字段）。
- 每步 `cargo test` 全绿；`cargo clippy --all-targets` 不新增警告（canvas.rs 的 4 个既有 deny 错误为基线问题，另案处理）。

## 测试

- 行为不变的重构：现有测试套件（含 `widget_props.rs`、chart 鼓包回归测试）必须全部原样通过，不改任何测试文件。
- 每个重构单元完成后单独 `cargo test` 验证。
