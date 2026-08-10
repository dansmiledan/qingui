# 内部函数方法化重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把参数过多的 `pub(crate)` 自由函数收编为 State 方法 / 几何 struct 方法，删除全部 7 处 `#[allow(clippy::too_many_arguments)]`，行为与公开 API 零变化。

**Architecture:** Widget 侧——自由函数的参数本就全是 State 字段且只有一个调用点，整体移入 `impl XState`；canvas/draw.rs 侧——把调用点预计算的不变量打包成 `ThickLine`/`ArcGeom`，9 参调用变 1-2 参。spec：`docs/superpowers/specs/2026-08-09-internal-fn-methodization-design.md`。

**Tech Stack:** Rust（no_std crate，lib 内只能 `alloc::`），cargo test 集成测试在 `qingui/tests/`。

## Global Constraints

- 工作目录命令统一在 `qingui/` 下执行（`cd qingui && cargo test`）。
- **纯重构**：除签名、`self.` 前缀和调用点外，不改变任何表达式与逻辑；公开 API、渲染行为、动画时序全部不变。
- 不允许修改任何测试文件；现有测试套件（含 `widget_props.rs`、chart 鼓包回归）必须原样通过。
- 代码注释、commit message 用英文，Conventional Commits；只本地提交，绝不 push（commit 已批量预授权）。
- 完成后 `grep -rn "too_many_arguments" qingui/src` 必须无结果（7 处 allow 全删）。
- `cargo clippy --all-targets` 不新增警告（canvas.rs 的 4 个既有 deny 错误与若干 `field_reassign_with_default` 是基线，不处理）。
- 每个 Task 一个 commit。

---

### Task 1: draw.rs — `ThickLine` + `ArcGeom`

**Files:**
- Modify: `qingui/src/draw.rs`（`line_row_span`、`line_sdf_cov16`、`arc_cov16` 方法化，删 2 处 allow）
- Modify: `qingui/src/canvas.rs`（`draw_line_thick` 与圆弧绘制循环的调用点）

**Interfaces:**
- Produces:
  - `pub(crate) struct ThickLine` + `ThickLine::new(p1: Point, p2: Point, width: i32) -> Self`、`row_span(&self, y: i32) -> Option<(i32, i32)>`、`cov16(&self, px: i32, py: i32) -> i32`
  - `pub(crate) struct ArcGeom` + `ArcGeom { outer, inner, s, e, and_mode }`（pub(crate) 字段，直接结构体字面量构造）、`cov16(&self, dx: i32, dy: i32) -> i32`
- Consumes: 无（第一个任务）。

- [ ] **Step 1: 基线验证**

Run: `cd qingui && cargo test 2>&1 | tail -3`
Expected: 全绿（记录此基线）。

- [ ] **Step 2: draw.rs 重构**

1. 新增 `ThickLine`（放在 `line_sdf_cov16` 原位置）：

```rust
/// Thick-segment geometry with the per-line invariants precomputed once
/// (the capsule: segment swept with a disk of radius `rm`; see `row_span`).
pub(crate) struct ThickLine {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    dx: i32,
    dy: i32,
    len2: i32,
    rm: i64,
    ux: i64,
    uy: i64,
    len2_64: i64,
    inv_len: i64,
    r16: i64,
}

impl ThickLine {
    /// Builds the geometry for a `width`-thick segment p1→p2 (invariants from the old draw_line_thick preamble).
    pub(crate) fn new(p1: crate::geometry::Point, p2: crate::geometry::Point, width: i32) -> Self {
        let (x0, y0) = (p1.x, p1.y);
        let (x1, y1) = (p2.x, p2.y);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = dx * dx + dy * dy;
        let r = width / 2;
        let rm = (r + 1) as i64; // half-width + 1px AA margin
        let r16 = (width * 16 / 2) as i64;
        let (ux, uy) = (16 * dx as i64, 16 * dy as i64);
        let len2_64 = len2 as i64 * 256;
        let inv_len = (1i64 << 32) / isqrt(len2_64 as u64) as i64;
        Self { x0, y0, x1, y1, dx, dy, len2, rm, ux, uy, len2_64, inv_len, r16 }
    }

    /// 原 line_row_span 的文档注释与函数体，签名替换为：
    pub(crate) fn row_span(&self, y: i32) -> Option<(i32, i32)> {
        // 函数体与 line_row_span 逐行相同，参数 x0/y0/x1/y1/dx/dy/len2/rm 全部改为 self.*
    }

    /// 原 line_sdf_cov16 的文档注释与函数体，签名替换为：
    pub(crate) fn cov16(&self, px: i32, py: i32) -> i32 {
        // 函数体与 line_sdf_cov16 逐行相同，参数 x0/y0/ux/uy/len2_64/inv_len/r16 全部改为 self.*
    }
}
```

注意：`len2 == 0`（退化点）的保护在调用方 `draw_line_thick` 已提前 return，`new` 中 `isqrt(0)=0` 会导致除零——把 `len2 == 0` 的处理保留在调用方（现状即如此），`new` 不加分支，但文档注释说明“caller must reject degenerate zero-length segments”。

2. 删除 `line_row_span`、`line_sdf_cov16` 两个自由函数及其上的 `#[allow(clippy::too_many_arguments)]`（共 2 处）。
3. `arc_cov16` 方法化：

```rust
/// Annular-arc coverage parameters (supersampled 4x4; see `cov16`).
pub(crate) struct ArcGeom {
    pub(crate) outer: i32,
    pub(crate) inner: i32,
    pub(crate) s: (i32, i32),
    pub(crate) e: (i32, i32),
    pub(crate) and_mode: bool,
}

impl ArcGeom {
    // 原 arc_cov16 的文档注释与函数体，outer/inner/s/e/and_mode 改为 self.*
    pub(crate) fn cov16(&self, dx: i32, dy: i32) -> i32 { /* 原函数体 */ }
}
```

- [ ] **Step 3: canvas.rs 调用点更新**

1. `draw_line_thick`：删除前半段的 dx/dy/len2/r/rm/r16/ux/uy/len2_64/inv_len 局部计算（`len2 == 0` 退化分支保留——仍需 `dx`/`dy` 或直接算 `(x1-x0)`；保留 `(x0, y0)`/`(x1, y1)` 局部供 `miny`/`maxy` 用），改为：

```rust
let (x0, y0) = (p1.x, p1.y);
let (x1, y1) = (p2.x, p2.y);
if (x1 - x0) * (x1 - x0) + (y1 - y0) * (y1 - y0) == 0 {
    // Degenerate point: single stamped pixel.
    self.put_clipped(x0, y0, c, opa, clip);
    return;
}
let line = ThickLine::new(p1, p2, width);
let r = width / 2;
// ... miny/maxy 保持用 y0/y1/r
// 循环内：line.row_span(y)、line.cov16(x, y)
```

`use crate::draw::{...}` 导入列表中 `line_row_span, line_sdf_cov16` 换成 `ThickLine`（其余不动）。

2. 圆弧覆盖调用点（canvas.rs:415 附近，`draw_arc` 的像素循环）：循环前构造 `let geom = ArcGeom { outer: radius, inner, s, e, and_mode };`，循环内 `arc_cov16(dx, dy, radius, inner, s, e, and_mode)` → `geom.cov16(dx, dy)`。导入 `arc_cov16` 换成 `ArcGeom`。

- [ ] **Step 4: 验证**

Run: `cd qingui && cargo test 2>&1 | tail -3 && cargo clippy --all-targets 2>&1 | grep -E "draw\.rs|canvas\.rs" | grep -v absurd_extreme | grep -v precedence | head -5`
Expected: 测试全绿；无新增 draw.rs/canvas.rs 警告（`absurd_extreme_comparisons`/`precedence` 为既有基线）。

- [ ] **Step 5: Commit**

```bash
git add qingui/src/draw.rs qingui/src/canvas.rs
git commit -m "refactor: pack thick-line and arc rasterization invariants into geometry structs"
```

---

### Task 2: spinner / chart / dropdown — draw 方法化

**Files:**
- Modify: `qingui/src/widgets/spinner.rs`、`qingui/src/widgets/chart.rs`、`qingui/src/widgets/dropdown.rs`

**Interfaces:**
- Produces: `SpinnerState::draw_arc_ind(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`、`ChartState::draw_series(&self, ctx, d, clip)`、`DropdownState::draw_label(&self, ctx, d, clip)`（均 `pub(crate)` 不需要——同模块私有即可，trait impl 一行转发）。

- [ ] **Step 1: 基线验证**

Run: `cd qingui && cargo test --test widget_props --test chart 2>&1 | tail -4`
Expected: 全绿。

- [ ] **Step 2: 三个文件各自做同一变换**

以 spinner 为例（chart/dropdown 完全同构）：

1. 删除自由函数 `pub(crate) fn draw(line_width: i32, period_ms: u64, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`，函数体移入：

```rust
impl SpinnerState {
    fn draw_arc_ind(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
        // 原函数体；line_width → self.line_width，period_ms → self.period_ms
    }
}
```

2. `Widget::draw` 改为 `fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { self.draw_arc_ind(ctx, c, clip) }`。

chart：自由函数 `draw(s: &ChartState, ctx, d, clip)` → `impl ChartState { fn draw_series(&self, ctx, d, clip) }`，体内 `s.` → `self.`。dropdown：`draw(items, selected, ctx, d, clip)` → `impl DropdownState { fn draw_label(&self, ctx, d, clip) }`，体内 `items` → `self.items`、`selected` → `self.selected`。

- [ ] **Step 3: 验证**

Run: `cd qingui && cargo test 2>&1 | tail -3`
Expected: 全绿。

- [ ] **Step 4: Commit**

```bash
git add qingui/src/widgets/spinner.rs qingui/src/widgets/chart.rs qingui/src/widgets/dropdown.rs
git commit -m "refactor: methodize spinner, chart and dropdown draw helpers"
```

---

### Task 3: arc / slider / table — draw 方法化 + 删 2 处 allow

**Files:**
- Modify: `qingui/src/widgets/arc.rs`、`qingui/src/widgets/slider.rs`、`qingui/src/widgets/table.rs`

**Interfaces:**
- Produces: `ArcState::draw_dial(&self, ctx, d, clip)`、`SliderState::draw_track(&self, ctx, d, clip)`、`TableState::draw_grid(&self, ctx, d, clip)`。

- [ ] **Step 1: 基线验证**

Run: `cd qingui && cargo test --test widget_props 2>&1 | tail -2`
Expected: 全绿。

- [ ] **Step 2: 三个文件各自做同一变换**

同 Task 2 的模式：自由函数 `draw(...)` 移入对应 `impl XState` 私有方法（arc→`draw_dial`、slider→`draw_track`、table→`draw_grid`），所有字段参数改 `self.*`，trait impl 一行转发。**同时删除 arc.rs 和 table.rs 里 `draw` 上的 `#[allow(clippy::too_many_arguments)]`**（slider.rs 的 draw 无 allow）。

- [ ] **Step 3: 验证**

Run: `cd qingui && cargo test 2>&1 | tail -3 && grep -c "too_many_arguments" qingui/src/widgets/arc.rs qingui/src/widgets/table.rs`
Expected: 全绿；grep 计数均为 0。

- [ ] **Step 4: Commit**

```bash
git add qingui/src/widgets/arc.rs qingui/src/widgets/slider.rs qingui/src/widgets/table.rs
git commit -m "refactor: methodize arc, slider and table draw helpers"
```

---

### Task 4: checkbox — draw 方法化

**Files:**
- Modify: `qingui/src/widgets/checkbox.rs`

**Interfaces:**
- Produces: `CheckboxState::draw_box(&self, ctx, d, clip)`。

- [ ] **Step 1: 基线验证**

Run: `cd qingui && cargo test --test widget_props 2>&1 | tail -2`
Expected: 全绿。

- [ ] **Step 2: 变换**

自由函数 `draw(text, checked, box_size, gap, ctx, d, clip)` → `impl CheckboxState { fn draw_box(&self, ctx, d, clip) }`，体内 `text` → `&self.text`、`checked` → `self.checked`、`box_size`/`gap` → `self.*`；trait impl 一行转发。注意 `draw` 内 `sc` 闭包引用 `box_size`：改 `self.box_size` 时注意闭包捕获（`let box_size = self.box_size;` 放函数开头可保持闭包不变）。

- [ ] **Step 3: 验证**

Run: `cd qingui && cargo test 2>&1 | tail -3`
Expected: 全绿。

- [ ] **Step 4: Commit**

```bash
git add qingui/src/widgets/checkbox.rs
git commit -m "refactor: methodize checkbox draw helper"
```

---

### Task 5: roller — 全部助手方法化 + 删 1 处 allow

**Files:**
- Modify: `qingui/src/widgets/roller.rs`

**Interfaces:**
- Produces（`RollerState` 方法，可见性与原函数一致）:
  - `fn draw_rows(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`（原 `draw`，8 参，含 allow——allow 删除）
  - `fn sel_f(&self, now: u64) -> f32`（原 4 参自由函数）
  - `pub(crate) fn fx_active(&self, now: u64) -> bool`（原 3 参）
  - `pub(crate) fn select(&mut self, idx: usize, now: u64)`（原 6 参）

- [ ] **Step 1: 基线验证**

Run: `cd qingui && cargo test --test widget_props --test roller_ghost 2>&1 | tail -4`
Expected: 全绿。

- [ ] **Step 2: 变换**

1. `draw`（带 `#[allow(clippy::too_many_arguments)]`）→ `impl RollerState { fn draw_rows(&self, ctx, d, clip) }`：体内 `items` → `self.items`（迭代用 `self.items.iter()`）、`selected`/`sel_from`/`row_h` → `self.*`、`sel_f(selected, sel_from, ctx.now, dur)` → `self.sel_f(ctx.now)`；删 allow。
2. `sel_f(selected, sel_from, now, dur)` → `fn sel_f(&self, now: u64) -> f32`：`self.selected as f32`、`self.sel_from`、`self.roll_dur`。
3. `fx_active(sel_from, now, dur)` → `pub(crate) fn fx_active(&self, now: u64) -> bool`：`self.sel_from`、`self.roll_dur`。
4. `select(items, selected, sel_from, idx, now, dur)` → `pub(crate) fn select(&mut self, idx: usize, now: u64)`：体内直接操作 `self.items`/`self.selected`/`self.sel_from`，`sel_f(*selected, *sel_from, now, dur)` → `self.sel_f(now)`。
5. `Widget` impl 更新：`draw` → `self.draw_rows(ctx, c, clip)`；`tick` 中 `fx_active(self.sel_from, now, self.roll_dur)` → `self.fx_active(now)`；`on_key` 中：
```rust
let next = (self.selected as i32 + dir).clamp(0, self.items.len().saturating_sub(1) as i32);
let now = ui.time();
self.select(next as usize, now);
```

- [ ] **Step 3: 验证**

Run: `cd qingui && cargo test 2>&1 | tail -3 && grep -c "too_many_arguments" qingui/src/widgets/roller.rs`
Expected: 全绿；计数 0。

- [ ] **Step 4: Commit**

```bash
git add qingui/src/widgets/roller.rs
git commit -m "refactor: methodize roller draw and animation helpers"
```

---

### Task 6: list — 全部助手方法化 + 删 2 处 allow

**Files:**
- Modify: `qingui/src/widgets/list.rs`

**Interfaces:**
- Produces（`ListState` 方法，可见性与原函数一致）:
  - `fn draw_rows(&self, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`（原 `draw`，9 参，含 allow——删）
  - `pub(crate) fn select(&mut self, idx: usize, vis_h: i32, now: u64)`（原 8 参，含 allow——删）
  - `pub(crate) fn ensure_visible(&mut self, vis_h: i32, now: u64)`（原 7 参）
  - `pub(crate) fn insert(&mut self, idx: usize, text: &str, now: u64)`（原 6 参）
  - `pub(crate) fn remove(&mut self, now: u64) -> bool`（原 5 参）
  - `ListFx::active(now, dur)` / `prune(now, dur)` 与自由函数 `lerp_t(start, now, dur)` **保持不变**
- Consumes: 无（dropdown 已不直接调这些函数）。

- [ ] **Step 1: 基线验证**

Run: `cd qingui && cargo test --test widget_props --test list_fx --test list_nav 2>&1 | tail -6`
Expected: 全绿。

- [ ] **Step 2: 变换**

1. `draw` → `fn draw_rows(&self, ctx, d, clip)`：`items` → `self.items`、`selected`/`scroll` → `self.*`、`fx` → `&self.fx`、`row_h` → `self.row_h`、`lerp_t(start, now, fx_dur)` → `lerp_t(start, now, self.fx_dur)`（lerp_t 保持自由函数）；删 allow。
2. `select` → `pub(crate) fn select(&mut self, idx, vis_h, now)`：体内字段改 `self.*`，`ensure_visible(...)` → `self.ensure_visible(vis_h, now)`；删 allow。
3. `ensure_visible` → `pub(crate) fn ensure_visible(&mut self, vis_h, now)`：`selected` → `self.selected`、`item_count` 由 `self.items.len()` 得、`scroll` → `self.scroll`、`fx` → `self.fx`。
4. `insert` → `pub(crate) fn insert(&mut self, idx, text, now)`：`items` → `self.items`、`fx` → `self.fx`。
5. `remove` → `pub(crate) fn remove(&mut self, now) -> bool`：`items`/`fx`/`selected` → `self.*`。
6. `Widget` impl：`draw` → `self.draw_rows(ctx, c, clip)`；`on_key` 中 `select(...)` → `self.select(idx, vis_h, now)`。
7. `UiListExt` 闭包更新（字段拆分传参改为整体方法调用，借用更宽松）：
   - `list_select`：`s.select(idx, vis_h, now);`
   - `list_insert`：保留 selected 下移的前置逻辑，`insert(&mut s.items, &mut s.fx, idx, text, now, s.row_h)` → `s.insert(idx, text, now);`
   - `list_remove`：`remove(&mut s.items, &mut s.fx, &mut s.selected, now, s.row_h)` → `let ok = s.remove(now);`，`ensure_visible(...)` → `s.ensure_visible(vis_h, now);`

- [ ] **Step 3: 验证**

Run: `cd qingui && cargo test 2>&1 | tail -3 && grep -c "too_many_arguments" qingui/src/widgets/list.rs`
Expected: 全绿；计数 0。

- [ ] **Step 4: Commit**

```bash
git add qingui/src/widgets/list.rs
git commit -m "refactor: methodize list draw, selection and edit helpers"
```

---

### Task 7: 收尾验证

**Files:**
- 无修改（仅验证；若发现残留 allow 或签名不一致，回对应任务修复）

- [ ] **Step 1: 全量检查**

Run:
```bash
cd qingui && cargo test 2>&1 | grep -cE "test result: ok" && cargo test 2>&1 | grep -E "FAILED|error\[" | head -3
grep -rn "too_many_arguments" qingui/src || echo "no allows left"
cargo clippy --all-targets 2>&1 | grep -c "too_many_arguments" || true
```
Expected: 测试全绿、无 FAILED；`too_many_arguments` 在源码与 clippy 输出中均无结果（canvas.rs 既有 `absurd_extreme_comparisons`/`precedence` 与若干 `field_reassign_with_default` 为基线，不动）。

- [ ] **Step 2: 无需 commit**（仅当有修正时提交，message 用 `refactor:` 前缀说明修正点）
