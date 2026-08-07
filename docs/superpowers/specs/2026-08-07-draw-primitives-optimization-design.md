# 绘制原语优化设计

日期：2026-08-07
状态：已获用户批准（逐项 brainstorming 讨论后确认）

## 背景与动机

`docs/BENCHMARK.md`（commit `d4fe7e5` 基线）显示基础绘制原语是渲染成本的核心，最贵的前五（QEMU ticks / host µs）：

| 原语 | QEMU ticks | host min µs |
|---|---|---|
| fill_rounded | 226,371 | 52.7 |
| fill_rect | 215,729 | 51.2 |
| draw_line | 190,899 | 18.4 |
| draw_arc | 173,846 | 330.3 |
| fill_circle | 166,656 | 16.6 |

成本模式：**全包围盒逐像素扫描 + 每像素 4×4（16 子采样）超采样覆盖率计算**，像素写入率低（大量 cov=0 被跳过）；`draw_line` 更是每步 Bresenham 都调一次 `fill_circle`，重叠像素反复重画。

本轮聚焦 **Top 3**：`fill_rect`、`draw_line`、`fill_rounded`。

## 目标与非目标

**目标**：
- 替换 `fill_rect` 为批量填充（slice `fill`），消除逐像素冗余边界检查。
- 替换 `draw_line` 的 Bresenham+stamp 为平行四边形逐行 span 扫描，每像素恰好画一次。
- `fill_rounded` 主体自动受益于新 `fill_rect`；角超采样保持复用 `circle_cov16`。
- 保留 4×4 超采样 AA，视觉接近或更好。
- 验收：性能"不慢即可"（期望显著提升）；所有测试绿；人工视觉对比通过。

**非目标**：
- 不改 AA 采样方案（保留 4×4、保留 `circle_cov16`/`arc_cov16` 的采样点）。
- 不优化 draw_arc / draw_circle / fill_circle / draw_text / blit565（后续轮次）。
- 不做真硬件测量（延续现有双端 bench 定位）。

## 设计

### 1. `fill_rect` — 批量填充

现状（draw.rs:122-131）：先 `intersect(clip)` + `intersect(area)` 得安全子矩形，再嵌套循环逐像素 `put()`。`put()` 每次做 `area.contains` 检查 + 索引计算（draw.rs:101-113）。

新实现：
- 保留 intersect 前置（保证子矩形在 buffer 内，无需越界检查）。
- **`opa == 255`**：按行 slice 批量填充——`pixels[start..end].fill(c)`，每行一次 `fill`（memcpy 级）而非逐像素调用。
- **`opa < 255`**：逐像素 blend，但走新辅助 `put_fast`（无 `contains` 检查，调用方已保证在界内）。

伪代码：
```rust
pub fn fill_rect(&mut self, r: Rect, c: Color, opa: u8, clip: Rect) {
    let Some(r) = r.intersect(&clip).and_then(|r| r.intersect(&self.area)) else { return };
    if opa >= 255 {
        for y in r.y..r.bottom() {
            let start = ((y - self.area.y) * self.stride + (r.x - self.area.x)) as usize;
            let end = start + r.w as usize;
            self.pixels[start..end].fill(c);
        }
    } else {
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.put_fast(x, y, c, opa);
            }
        }
    }
}
```

### 2. `draw_line` — 平行四边形扫描

现状（draw.rs:349-377）：Bresenham 步进，每步 `fill_circle(center, width/2, ...)`。长线 = 大量重叠 stamp 重画。

新实现：把线段视为宽度 `width` 的粗线段（沿方向向量法向 ±半宽），逐行扫描 y 范围 `[min(y0,y1)-r, max(y0,y1)+r]`：
- 用线段隐式方程（有向距离）计算当前行 y 与线段的相交 span `[x_lo, x_hi]`。
- span 内像素用 4×4 超采样计算覆盖率（沿用 `circle_cov16` 同款覆盖语义：对像素中心距离线段 ≤ width/2 的部分采样判定），`put_clipped` 写入。
- 覆盖 span 边界的像素获得 AA 过渡；span 内部像素 opa 全量。

要点：
- 统一处理水平/垂直/对角/任意角度。
- `width == 0` 或无跨距时退化为 1px 线（单点 put_clipped）。
- 避免 Bresenham+stamp 的重叠重画；每像素至多一次写入。
- 边缘 AA 与现有圆 stamp 视觉接近（同样 4×4 覆盖），但实现为纯扫描，无中间 `fill_circle` 调用。

### 3. `fill_rounded` — 拆分 + 复用

现状（draw.rs:155-185）：中心带 + 左右侧带 3 次 `fill_rect` + 4 角 `circle_cov16` 超采样。

新实现：
- 主体 3 次 `fill_rect` 自动受益于新批量填充。
- 四角超采样逻辑**不变**（复用 `circle_cov16`），保持 AA 质量与测试语义。
- 无新增算法；仅因 `fill_rect` 变快而整体提速。

### 4. 共享辅助 `put_fast`

新增私有方法：
```rust
/// Writes a pixel without bounds check; caller must ensure (x, y) is inside
/// the buffer area. Used by paths that already clipped via intersect.
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
- 保留 `put`（含检查）与 `put_clipped`（clip 检查）供外部路径（clip 未预裁、坐标可能越界）继续使用。
- 新增 `fill_rect` 的 opa<255 分支与 `draw_line` 使用 `put_fast`（内部 span 已 intersect 到 buffer 内）；边缘 AA 像素仍走 `put_clipped`。

### 5. 测试策略

- **冻结现有精确 ASCII 测试为旧算法快照**：`tests/draw.rs` 的 `assert_eq!(to_ascii(&d), ...)` 断言保留原文作为回归对照。**决策程序**：换算法后逐个运行现有测试——
  - 若新输出与断言**逐位一致**（如 `fill_rect` 硬边矩形本就不受算法影响，或 `fill_rounded` 角超采样未变）→ 断言保留不改。
  - 若新输出**仅在 AA 边缘过渡带有差异**（如 `draw_line` 的圆 stamp → 平行四边形扫描）→ 用新输出**重校准**该 case 的 ASCII 断言（用户已同意视觉接近即可），并保留 `partial > 0` 抗锯齿性质断言。
  - 若差异超出 AA 过渡带（形状/厚度错误）→ 判定算法 bug，修复算法而非重校准测试。
- **新增性质测试**（`tests/draw.rs`）：
  - `fill_rect` 全量填充精确覆盖矩形区域（随机矩形 + opa=255 全量、opa<255 blend）。
  - `draw_line` 端点厚度：水平/垂直/对角线的长度、宽度、端点位置正确；不越界（无 panic）。
  - 边缘保留 AA：新 `draw_line` 边缘像素含半透明过渡（`partial > 0` 同款断言）。
  - 回归：新实现与旧实现的像素差在"仅边缘 AA 过渡带"内（若有差异）。
- **人工视觉对比**：host 渲染新旧对比（同场景同参数输出 ASCII/PPM），肉眼确认视觉接近或更好。

### 6. 验证与文档

- `cargo test -p qingui` 全绿（含新增性质测试）。
- host bench `cargo bench -p qingui --bench time`：Top3 原语对比优化前后（min µs）。
- QEMU `(cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)`：Top3 原语 ticks 对比。
- 更新 `docs/BENCHMARK.md`：追加新基线记录（新 commit id、日期、前后数据）。

## 验收标准

1. `cargo test -p qingui` 全绿。
2. `draw_line` 不再调用 `fill_circle`（代码层面确认）；`fill_rect` 的 opa=255 路径用 slice `fill`。
3. host bench + QEMU release bench：三个原语不慢于基线（期望显著提升，量化记录到 BENCHMARK.md）。
4. 人工视觉对比通过：draw_line 直线（水平/垂直/对角/粗线）视觉正确，边缘 AA 保留。
5. `cargo check -p qingui --all-targets` 无新 warning。

## 影响面

- `qingui/src/draw.rs`：`fill_rect` 重写、`draw_line` 重写、新增 `put_fast`、`fill_rounded` 主体自动受益。
- `qingui/tests/draw.rs`：新增性质测试；现有精确断言按 case 冻结或重校准。
- `docs/BENCHMARK.md`：追加优化后基线。
- 其余文件零改动。

## 风险与对策

- **精确 ASCII 测试因换算法而失败**：逐个核实差异是否仅限 AA 边缘过渡带；若是则重校准该 case（用户已同意），否则修复算法。测试策略第 5 节已覆盖。
- **draw_line 平行四边形覆盖在极端角度/大宽度下与圆 stamp 视觉不同**：保留 4×4 AA 覆盖，视觉接近；人工对比把关；若不可接受回退该 case。
- **put_fast 越界风险**：仅内部已 intersect 路径使用；`fill_rect` opa<255 分支在安全子矩形内调用，draw_line 内部 span 已 clip 到 buffer；边缘像素仍走 put_clipped。
