# 绘制原语优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 优化 `fill_rect`、`draw_line`、`fill_rounded` 三个最贵绘制原语：`fill_rect` 改 slice 批量填充，`draw_line` 改平行四边形逐行扫描替代 Bresenham+fill_circle stamp，`fill_rounded` 主体自动受益，保留 4×4 超采样 AA，视觉接近或更好，性能不慢即可（期望显著提升）。

**Architecture:** 全部改动集中在 `qingui/src/draw.rs` 与 `qingui/tests/draw.rs`。新增私有 `put_fast`（无边界检查，供已 intersect 的内部路径用）；`fill_rect` opa=255 走 slice `fill`、opa<255 走 `put_fast`；`draw_line` 用线段隐式方程（有向距离）逐行求 span，span 内像素用 4×4 超采样算覆盖率；`fill_rounded` 主体 3 次 `fill_rect` 自动提速、四角超采样逻辑不变。现有精确 ASCII 测试按 spec 三档决策处理（逐位一致→保留；仅 AA 边缘差异→重校准；超差异→算法 bug）。

**Tech Stack:** Rust（no_std 库 + 集成测试），`cargo test -p qingui`、`cargo bench -p qingui --bench time`、QEMU `--release` bench。

## Global Constraints

- **保留 4×4 超采样 AA**：`circle_cov16`/`arc_cov16` 的采样点与覆盖语义不变；`fill_rounded` 四角复用 `circle_cov16`。
- **像素级输出不必一致**：换算法可产生不同像素（视觉接近或更好即可）；现有精确 ASCII 测试按 spec §5 三档决策逐个处理。
- **零新第三方依赖**：只改 `qingui/src/draw.rs`、`qingui/tests/draw.rs`、`docs/BENCHMARK.md`。
- **不优化** draw_arc / draw_circle / fill_circle / draw_text / blit565（后续轮次）。
- **验收**：`cargo test -p qingui` 全绿；`cargo check -p qingui --all-targets` 无新 warning；host/QEMU bench 不慢于基线；人工视觉对比通过。
- **git**：只本地 commit，不 push；Commit message 英文（Conventional Commits）。
- **验证命令**：`cargo test -p qingui`、`cargo bench -p qingui --bench time`、`(cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)`。

---

### Task 1: `put_fast` 辅助 + `fill_rect` 批量填充

**Files:**
- Modify: `qingui/src/draw.rs`

**Interfaces:**
- Produces:
  - `DrawBuf::put_fast(&mut self, x: i32, y: i32, c: Color, opa: u8)` —— 私有（`pub(crate)` 级，无边界检查），供 `fill_rect` opa<255 分支与 Task 2 的 `draw_line` 内部 span 使用。
  - 新 `fill_rect` 行为：opa>=255 用 slice 批量填充；opa<255 用 `put_fast` 逐像素 blend。

- [ ] **Step 1: 写失败测试（新增性质测试）**

在 `qingui/tests/draw.rs` 末尾追加（`fill_rect` 全量填充 + 随机位置 + clip）：

```rust
#[test]
fn fill_rect_full_coverage_opa_255() {
    // Full-screen opaque fill must paint every pixel exactly (slice-fill path).
    let (mut px, area) = buf(5, 4);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 5 };
    d.fill_rect(area, Color::WHITE, 255, area);
    assert!(px.iter().all(|&c| c == Color::WHITE), "opaque fill must cover every pixel");
    assert_eq!(to_ascii(&d), "\
#####
#####
#####
#####");
}

#[test]
fn fill_rect_partial_opa_blends() {
    // Non-full opacity on a sub-rect blends over the black background.
    let (mut px, area) = buf(4, 3);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 4 };
    d.fill_rect(Rect::new(1, 1, 2, 2), Color::WHITE, 128, area);
    let at = |x: usize, y: usize| d.pixels[y * 4 + x];
    assert_eq!(at(1, 1), Color::WHITE.blend(Color::BLACK, 128).blend(Color::WHITE, 128), "blend arithmetic");
    // white over black at 128 opa -> mid-gray (~128)
    let mid = at(1, 1);
    assert!(mid.r > 60 && mid.r < 195, "mid-gray blend, got {}", mid.r);
    // corners untouched
    assert_eq!(at(0, 0), Color::BLACK);
}
```

> 说明：`Color::blend(self, over, opa)` 签名是 `self.blend(over, opa)`（geometry.rs:123），背景在上方 blend 前景。若 blend 断言的算术与实现不符，以"mid-gray 区间断言"为准（已含在测试内）。

- [ ] **Step 2: 运行测试确认通过/失败状态**

Run: `cargo test -p qingui --test draw fill_rect_full_coverage_opa_255 fill_rect_partial_opa_blends`
Expected: 当前实现两个测试均 PASS（现有 `fill_rect` 已能覆盖/混合）。本步骤确认基线行为，非 RED——因为这是重构优化，先有行为测试再做性能改动。

- [ ] **Step 3: 实现 `put_fast` + 新 `fill_rect`**

在 `draw.rs` 的 `put_clipped`（约 115 行）之后插入：

```rust
    /// Writes a pixel without bounds checking. The caller must ensure `(x, y)`
    /// lies inside the buffer area — used by internal paths that already
    /// clipped the region (e.g. `fill_rect` after intersecting with the area).
    fn put_fast(&mut self, x: i32, y: i32, c: Color, opa: u8) {
        let lx = x - self.area.x;
        let ly = y - self.area.y;
        let idx = (ly * self.stride + lx) as usize;
        if opa >= 255 {
            self.pixels[idx] = c;
        } else if opa > 0 {
            self.pixels[idx] = self.pixels[idx].blend(c, opa);
        }
    }
```

替换 `fill_rect`（约 122-131 行）：

```rust
    /// Fills `r` with `c` at opacity `opa` (0..=255), clipped to `clip` and the buffer area.
    pub fn fill_rect(&mut self, r: Rect, c: Color, opa: u8, clip: Rect) {
        let Some(r) = r.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        if opa >= 255 {
            // Opaque fast path: batch-fill whole rows (no per-pixel bounds check,
            // no per-pixel blending).
            let area_x = self.area.x;
            let area_y = self.area.y;
            let stride = self.stride;
            let w = r.w as usize;
            for y in r.y..r.bottom() {
                let row = ((y - area_y) * stride + (r.x - area_x)) as usize;
                self.pixels[row..row + w].fill(c);
            }
        } else {
            // Translucent: per-pixel blend on the already-clipped region.
            for y in r.y..r.bottom() {
                for x in r.x..r.right() {
                    self.put_fast(x, y, c, opa);
                }
            }
        }
    }
```

- [ ] **Step 4: 运行测试确认全绿**

Run: `cargo test -p qingui --test draw`
Expected: 全部 PASS（包括新增两个 + 现有 `fill_rect_basic`/`fill_rect_clipped`/`fill_rect_opa_blends`）。若 `fill_rect_opa_blends` 失败，核对 blend 期望值并修正测试。

- [ ] **Step 5: 全量验证 + Commit**

Run: `cargo test -p qingui`、`cargo check -p qingui --all-targets`
Expected: 全绿，无新 warning。

```bash
git add qingui/src/draw.rs qingui/tests/draw.rs
git commit -m "perf(draw): batch-fill opaque fill_rect rows and add put_fast helper"
```

---

### Task 2: `draw_line` 平行四边形扫描

**Files:**
- Modify: `qingui/src/draw.rs`

**Interfaces:**
- Consumes: Task 1 的 `put_fast`。
- Produces: 新 `draw_line(p1, p2, width, c, opa, clip)`——逐行扫描线段包围盒，用有向距离 + 4×4 超采样算覆盖率。不再调用 `fill_circle`。

- [ ] **Step 1: 写失败测试（新增性质测试）**

在 `qingui/tests/draw.rs` 末尾追加：

```rust
#[test]
fn draw_line_thick_no_stamp_reuse() {
    // A thick line must be a continuous band with correct width, drawn once per
    // pixel (regression: old code stamped a fill_circle at every Bresenham step).
    let (mut px, area) = buf(16, 16);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 16 };
    d.draw_line(qingui::Point { x: 2, y: 2 }, qingui::Point { x: 13, y: 13 }, 3, Color::WHITE, 255, area);
    // Center of the line at (7,7) must be filled.
    assert_eq!(d.pixels[7 * 16 + 7], Color::WHITE);
    // Band half-width = 1, so (7,4) (3px away from center) must be untouched.
    assert_eq!(d.pixels[4 * 16 + 7], Color::BLACK);
}

#[test]
fn draw_line_width1_single_pixel_path() {
    // width=1 degenerate: the line must cover at least every integer point on the
    // dominant axis (no gaps), endpoints inclusive.
    let (mut px, area) = buf(8, 8);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 8 };
    d.draw_line(qingui::Point { x: 1, y: 1 }, qingui::Point { x: 6, y: 6 }, 1, Color::WHITE, 255, area);
    for i in 1..=6 {
        assert_eq!(d.pixels[i * 8 + i], Color::WHITE, "diagonal (x={i}) must be covered");
    }
}
```

- [ ] **Step 2: 运行测试确认当前失败**

Run: `cargo test -p qingui --test draw draw_line_thick_no_stamp_reuse draw_line_width1_single_pixel_path`
Expected: `draw_line_width1_single_pixel_path` 当前应 PASS（Bresenham width=1 覆盖对角线）；`draw_line_thick_no_stamp_reuse` 当前应 PASS（stamp 也画粗线）。两者均非 RED——同 Task 1，这是重构优化的行为基线测试。

- [ ] **Step 3: 实现新 `draw_line`**

替换 `draw_line`（约 349-377 行）为平行四边形扫描实现：

```rust
    /// Line as a thick segment: for each scanline, the span covered by the line
    /// (width `width`, 4x4-supersampled edge coverage) is computed from the
    /// segment's implicit equation and painted once per pixel. Replaces the old
    /// Bresenham + per-step `fill_circle` stamp (which repainted overlapping
    /// pixels on thick lines).
    pub fn draw_line(&mut self, p1: crate::geometry::Point, p2: crate::geometry::Point, width: i32, c: Color, opa: u8, clip: Rect) {
        let (x0, y0) = (p1.x, p1.y);
        let (x1, y1) = (p2.x, p2.y);
        if width <= 0 {
            return;
        }
        // Segment vector (dx, dy) and its half-width normal radius.
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = dx * dx + dy * dy;
        if len2 == 0 {
            // Degenerate point: single stamped pixel.
            let r = (width / 2).max(0);
            self.put_clipped(x0, y0, c, opa, clip);
            let _ = r;
            return;
        }
        let r = width / 2;
        // Bounding box of the thick segment (with AA margin of 1).
        let (minx, maxx) = (x0.min(x1) - r - 1, x0.max(x1) + r + 1);
        let (miny, maxy) = (y0.min(y1) - r - 1, y0.max(y1) + r + 1);
        for y in miny..=maxy {
            for x in minx..=maxx {
                let cov = line_cov16(x, y, x0, y0, dx, dy, len2, width);
                if cov > 0 {
                    let o = (opa as u32 * cov as u32 / 16) as u8;
                    self.put_clipped(x, y, c, o, clip);
                }
            }
        }
    }
```

同时新增自由函数（`draw.rs` 顶部，`arc_cov16` 之后）：

```rust
/// 4x4 supersampling coverage of a thick line segment: subsample points are
/// considered covered when they lie within `width/2` of the infinite line AND
/// within the segment's extent (rounded caps at both ends).
///
/// All coordinates are in 1/16-pixel fixed-point units (subsample centers are
/// `16*px - 6 + 4*{a,b}`, matching `circle_cov16`). Distances are therefore in
/// (1/16 px)^2; the line half-width `width/2` must be scaled by 16 to compare
/// in the same units: `r16 = width*16/2` and `r16^2` is the squared threshold.
fn line_cov16(px: i32, py: i32, x0: i32, y0: i32, dx: i32, dy: i32, len2: i32, width: i32) -> i32 {
    let r16 = width * 16 / 2;             // half-width in 1/16 px
    let r2 = r16 * r16;                   // squared in (1/16 px)^2
    let cap2 = r2 * 4;                    // round-cap radius^2 (generous: 2x half-width)
    let mut n = 0;
    for a in 0..4 {
        for b in 0..4 {
            let sx = 16 * px - 6 + 4 * a;
            let sy = 16 * py - 6 + 4 * b;
            // Vector from segment start to subsample, in 1/16 px units.
            let (vx, vy) = (sx - 16 * x0, sy - 16 * y0);
            let (ux, uy) = (16 * dx, 16 * dy);
            // Squared distance from the subsample to the infinite line, in 1/16 units.
            // cross = v x u  (i32); to keep precision use i64 for the square.
            let cross = (vx as i64) * (uy as i64) - (vy as i64) * (ux as i64);
            let dist2 = (cross * cross / (len2 as i64).max(1)) as i64;
            // Projection t = (v . u) / |u|^2, in units of 1/256 of the segment length.
            let t_num = (vx as i64) * (ux as i64) + (vy as i64) * (uy as i64);
            let len2_64 = len2 as i64 * 256;
            if t_num < 0 || t_num > len2_64 {
                // Outside the segment extent: round-cap test.
                let (cx, cy) = if t_num < 0 {
                    (vx as i64, vy as i64)
                } else {
                    (vx as i64 - ux as i64, vy as i64 - uy as i64)
                };
                let d2 = cx * cx + cy * cy;
                if d2 > cap2 {
                    continue;
                }
            } else if dist2 > r2 {
                // Within the segment extent but too far from the infinite line.
                continue;
            }
            n += 1;
        }
    }
    n
}
```

> 说明：固定点换算已统一——`r2` 与 `dist2` 都在 (1/16 px)² 尺度；cap 判定用 `cap2 = 4 * r2`（圆帽半径 ≈ 半宽）。若视觉与旧 stamp 有出入，调整 `cap2` 倍率或 `r16` 舍入，**性能与视觉正确优先于逐像素一致**（spec 允许）。

- [ ] **Step 4: 运行测试 + 逐 case 处理现有 ASCII 断言**

Run: `cargo test -p qingui --test draw`
Expected:
- 新增两个性质测试 PASS。
- 现有 `draw_line_*` 六个 ASCII 精确断言：新算法输出可能与旧不同。**逐个查看失败 diff**：
  - 若仅 AA 边缘过渡带差异（如 45° 线边缘出现半透明像素而旧为纯色）→ 重校准该 case 的 ASCII 断言为新输出（用户已同意），保留形状正确性。
  - 若形状/厚度错误（断线、漏像素、宽度错）→ 算法 bug，回 Step 3 修算法，不重校准。
- 重校准后全部 `draw_line_*` PASS。

- [ ] **Step 5: 全量验证 + Commit**

Run: `cargo test -p qingui`、`cargo check -p qingui --all-targets`
Expected: 全绿，无新 warning。

```bash
git add qingui/src/draw.rs qingui/tests/draw.rs
git commit -m "perf(draw): scan-convert draw_line as thick segment instead of Bresenham stamp"
```

---

### Task 3: `fill_rounded` 受益验证 + 性质测试

**Files:**
- Modify: `qingui/tests/draw.rs`（新增性质测试）

**Interfaces:**
- Consumes: Task 1 的新 `fill_rect`。`fill_rounded` 主体 3 次 `fill_rect` 自动提速，四角逻辑不变。

- [ ] **Step 1: 写性质测试**

在 `qingui/tests/draw.rs` 末尾追加：

```rust
#[test]
fn fill_rounded_radius_clamp_and_corners() {
    // Radius larger than half the smaller side clamps; corners are cut, edges filled.
    let (mut px, area) = buf(10, 10);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    d.fill_rounded(Rect::new(0, 0, 10, 10), 99, Color::WHITE, 255, area);
    // Top-left corner pixel must be empty (radius clamps to 5, cutting the corner).
    assert_eq!(d.pixels[0], Color::BLACK, "corner must be cut");
    // Center must be filled.
    assert_eq!(d.pixels[5 * 10 + 5], Color::WHITE);
    // Edge midpoint (top edge, x=5) must be filled.
    assert_eq!(d.pixels[0 * 10 + 5], Color::WHITE);
}
```

- [ ] **Step 2: 运行测试确认通过**

Run: `cargo test -p qingui --test draw fill_rounded_radius_clamp_and_corners`
Expected: PASS（`fill_rounded` 逻辑未变，仅验证半径 clamp 与角裁剪仍正确）。

- [ ] **Step 3: 全量验证 + Commit**

Run: `cargo test -p qingui`、`cargo check -p qingui --all-targets`
Expected: 全绿，无新 warning。

```bash
git add qingui/tests/draw.rs
git commit -m "test(draw): add fill_rounded corner-clamp property test"
```

---

### Task 4: 性能验证 + 视觉对比 + 更新基准记录

**Files:**
- Modify: `docs/BENCHMARK.md`（追加新基线记录）

**Interfaces:**
- Consumes: Task 1-3 的优化后 `fill_rect`/`draw_line`/`fill_rounded`。

- [ ] **Step 1: 跑 host bench 记录优化后数据**

Run: `cargo bench -p qingui --bench time`
Expected: 记录 `fill_rect`/`draw_line`/`fill_rounded` 三行的 min µs。

- [ ] **Step 2: 跑 QEMU release bench 记录优化后数据**

Run: `(cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)`
Expected: 记录三原语 ticks；对比优化前基线（fill_rect 215,729 / draw_line 190,899 / fill_rounded 226,371）。**若新值超过阈值（×2）会触发 assert**——正常，属预期性能变化，Task 5 处理阈值。

- [ ] **Step 3: 人工视觉对比**

Run: 用 host 渲染新旧对比。临时在 `qingui/examples/demo.rs` 或一个小 `examples/` 脚本里，分别调用旧算法（`git stash` 前后）与新算法渲染同场景（粗对角线、水平/垂直/对角、粗圆角矩形），输出 ASCII 或 PPM，肉眼确认：
- 直线连续无断点、端点正确、宽度正确。
- 粗线边缘 AA 过渡自然（与旧 stamp 视觉接近或更好）。
- fill_rounded 角部圆滑。
Expected: 视觉通过。若 draw_line 在某些角度/宽度下明显劣于旧实现，报告回 Task 2。

- [ ] **Step 4: 更新 `docs/BENCHMARK.md`**

在文件末尾追加新一节：

```markdown
## 2026-08-07 — 绘制原语优化后（commit `<新commit>`）

> 优化内容：fill_rect 批量填充、draw_line 平行四边形扫描、fill_rounded 主体受益。
> 对比基线：上一节（commit `d4fe7e5`）。

### Runtime — host（µs，min/median）—— 仅记录三个优化原语

| 原语 | 优化前 min/med | 优化后 min/med |
|---|---|---|
| fill_rect | 51.2 / 51.9 | `<新值>` |
| draw_line | 18.4 / 18.6 | `<新值>` |
| fill_rounded | 52.7 / 54.8 | `<新值>` |

### Runtime — QEMU（ticks，--release）

| 原语 | 优化前 | 优化后 |
|---|---|---|
| fill_rect | 215,729 | `<新值>` |
| draw_line | 190,899 | `<新值>` |
| fill_rounded | 226,371 | `<新值>` |
```

（填入 Step 1/2 实测值。）

- [ ] **Step 5: Commit**

```bash
git add docs/BENCHMARK.md
git commit -m "docs: record post-optimization draw primitive baselines"
```

---

### Task 5: 校准 QEMU 阈值（如优化使 ticks 低于旧阈值不再触发）

**Files:**
- Modify: `tools/qemu-time/src/main.rs`

**Interfaces:**
- Consumes: Task 4 Step 2 的优化后 QEMU ticks。
- 说明：优化**降低** ticks，`assert!(x < LIMIT)` 不受影响（新值 < 旧×2 仍成立）。仅当 Task 4 某原语 **超出** ×2 阈值（意外回退或算法变慢）才需重校准。正常情况本任务仅验证即可。

- [ ] **Step 1: 验证 QEMU 断言全过**

Run: `(cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)`
Expected: 三个优化原语新值 < 阈值 ×2，全部 assert 通过，exit 0（优化使值下降，应无失败）。

- [ ] **Step 2: 若断言意外失败**

若某个原语新值超阈值，读取实测值，按 `新值 × 2 向上取整` 更新对应 `LIMIT_*` 常量（`tools/qemu-time/src/main.rs` 阈值块），注明校准日期与原因（算法重写导致基线变化）。重新运行确认 exit 0。

- [ ] **Step 3: 全量验证 + Commit（如需）**

Run: `cargo test -p qingui`、`cargo test -p qemu-time`、`cargo check -p qingui --all-targets`
Expected: 全绿。若 Step 2 改了阈值，提交：

```bash
git add tools/qemu-time/src/main.rs
git commit -m "bench: recalibrate qemu-time thresholds after draw primitive optimization"
```

---

## Self-Review

**Spec 覆盖：**
- fill_rect 批量填充（opa=255 slice fill、opa<255 put_fast）→ Task 1。
- draw_line 平行四边形扫描替代 Bresenham+stamp、每像素一次 → Task 2。
- fill_rounded 主体自动受益、四角超采样不变 → Task 3（验证）+ Task 1（fill_rect 提速传导）。
- put_fast 共享辅助 → Task 1。
- 测试策略（冻结/重校准三档）→ Task 2 Step 4 逐 case 决策。
- 人工视觉对比 → Task 4 Step 3。
- 性能验证 + 更新 BENCHMARK.md → Task 4。
- QEMU 阈值 → Task 5（降低不触发，异常才重校准）。

**占位符扫描：** 无 TDD/待办。`line_cov16` 的固定点换算已统一到 (1/16 px)² 尺度（`r2`/`dist2` 同单位、`cap2` 独立）——实现时可调 `cap2` 倍率做视觉微调，非占位（spec 允许视觉接近即可）；Task 4 的 `<新值>` 是实测数据占位，由运行结果填充。

**类型一致性：**
- `put_fast(&mut self, x: i32, y: i32, c: Color, opa: u8)` Task 1 定义，Task 1/2 使用，签名一致。
- `line_cov16(px, py, x0, y0, dx, dy, len2, width) -> i32` Task 2 定义并唯一使用。
- 测试辅助 `buf`/`to_ascii`/`ascii_buf` 沿用现有 tests/draw.rs 顶部函数，签名不变。
- `Color::blend(self, over, opa)` 按 geometry.rs:123 签名使用。
- `fill_rounded` 在 Task 1 后主体 `fill_rect` 自动走新路径，无签名变化。

---

## 复审修订（2026-08-07，执行后复审）

> 首轮执行（601af93..6c9e0dc）后发现以下问题，经与用户逐条确认，按以下决议修订：

1. **Task 2 原代码缺陷**：原计划 Step 3 给的是全 AABB 扫描 × 16 子采样，与 Architecture 的
   "逐行求 span" 矛盾，实测 host draw_line 回退 70×（18.4→1285.5µs）、draw_line_many
   QEMU 超 ×2 阈值。**修订**：draw_line 改为真正的逐行 span 扫描——利用胶囊形（线段+圆帽）
   的凸性，每行用整数运算解析求 x 交集（strip ∩ slab，加圆帽半弦），只在 span（+1px AA
   余量）内评估 `line_cov16`。`line_cov16` 语义不变（含 solid-core），像素输出与首轮一致。
2. **验收仲裁原则**（解决验收标准与 Task 5 的矛盾）：任一原语在 host 或 QEMU 回退 >10%
   即视为未完成，**不得**用"新值×2 重校准"掩盖；阈值只向下校准（变快才更新 LIMIT）。
3. **圆帽语义**：`cap2 = r2`（帽半径=半宽，与旧 stamp 行为一致）。原计划 `cap2 = r2*4`
   会让线段两端各延长约 width/2，为计划错误，首轮实现已自行修正，予以追认。
4. **solid-core 快捷路径**（首轮实现新增，计划外）：保留。已知其为近似（像素中心在带内
   不保证全部子采样在带内，细线略粗、AA 略弱），用于修复 1px 对角线半透明问题。
5. **视觉对比形式**：改为输出新旧算法对比图片（PPM），由用户肉眼确认。
6. **调试残留**：首轮遗留的 `line_cov16` 恒 16 stub（未提交，会导致 9 个测试失败）已丢弃。

7. **SDF 覆盖（二次修订，用户确认）**：第一版 span 扫描保留 4×4 子采样 `line_cov16`，
   视觉对比后用户认为粗线观感不如旧 stamp（w=2 边界全实心、偏粗）。改为像素中心
   **SDF 距离场**覆盖（1px 线性 AA 过渡，每线预计算 `inv_len`，热路径无除法无子采样
   循环），w=2 水平线边缘 143 vs 旧 142，观感对齐；粗斜线更实更亮为修复旧 stamp
   重叠混合缺陷，用户已确认。`line_cov16` 移除；最终 host draw_line -35%、QEMU -64%。
