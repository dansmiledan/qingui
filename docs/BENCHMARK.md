# qingui 性能基线记录（Benchmark Baselines）

本文件记录 qingui 双端（host / QEMU）memory 与 runtime 基准的**基线快照**，作为后续优化的对照记录。每次重新校准阈值或做优化时追加一条记录。

## 约定

- **host**：64 位 macOS（aarch64-apple-darwin），`cargo bench -p qingui --bench memory|time`。
- **QEMU**：`qemu-system-arm -machine mps2-an386 -icount shift=3`，thumbv7em-none-eabihf（Cortex-M4F，32 位）。
  - memory：`(cd tools/qemu-mem && cargo run --target thumbv7em-none-eabihf)`。
  - time：`(cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)`。**必须 `--release`**（dev profile 指令计数失真，见 [spec](superpowers/specs/2026-08-07-runtime-bench-design.md)）。
- **单位**：memory 为字节（B）；time 为 host µs（min/median）/ QEMU SysTick ticks（确定性，1 tick = 8 虚拟 ns）。
- **QEMU 语义边界**：ticks 是 QEMU 确定性指令级计数，非真实硬件周期；绝对数以真硬件 DWT 为准，本记录用于相对成本形状与回归对照。

---

## 2026-08-07 — 初始基线（commit `d4fe7e5`）

> commit: `d4fe7e54361130d33ec0bdfeb4c143e15890e8f9`（fix(tools): recalibrate qemu-time thresholds to release profile baselines）
> 日期：2026-08-07。当前最新基线；阈值断言均已按此校准（×2）。

### Memory — host（64 位）

**静态尺寸（字节）**

| 类型 | 大小 | 类型 | 大小 |
|---|---|---|---|
| Rect | 16 | Node | 376 |
| Point | 8 | WidgetKind | 40 |
| Color | 3 | Ui | 248 |
| Style | 168 | ResolvedStyle | 144 |
| 4 × Style（旧内联成本） | 672 | WidgetKind 判别开销 | 8 |

**场景峰值堆**

| 档位 | 节点 | peak | live |
|---|---|---|---|
| minimal | 3 | 5,871 B | 5,751 B |
| small | 16 | 35,069 B | 33,045 B |
| medium | 50 | 70,800 B | 60,736 B |
| large | 140 | 209,576 B | 159,496 B |

### Memory — QEMU（32 位，thumbv7em）

**静态尺寸（字节）**

| 类型 | 大小 | 类型 | 大小 |
|---|---|---|---|
| Rect | 16 | Node | 280 |
| Point | 8 | WidgetKind | 24 |
| Color | 3 | Ui | 152 |
| Style | 140 | ResolvedStyle | 112 |
| 4 × Style（旧内联成本） | 560 | WidgetKind 判别开销 | 4 |

**场景峰值堆**

| 档位 | 节点 | peak | live |
|---|---|---|---|
| minimal | 3 | 5,431 B | 5,311 B |
| small | 16 | 32,205 B | 30,849 B |
| medium | 50 | 59,476 B | 52,380 B |
| large | 140 | 165,516 B | 128,588 B |

### Runtime — host（µs，100 采样 min/median）

**layout**

| 场景 | min | median |
|---|---|---|
| flex 40 children, 320×240 | 2.3 | 2.4 |

**render / frame**

| 档位 | 节点 / px | full min/med | partial min/med | frame min/med |
|---|---|---|---|---|
| Minimal | 3 / 19,200 | 14.8 / 14.9 | 4.0 / 4.2 | 14.8 / 15.5 |
| Small | 16 / 76,800 | 79.0 / 80.5 | 23.3 / 23.5 | 79.5 / 79.9 |
| Medium | 50 / 76,800 | 140.6 / 143.0 | 75.5 / 76.8 | 141.2 / 143.8 |
| Large | 140 / 76,800 | 300.9 / 308.8 | 214.3 / 217.0 | 306.8 / 310.0 |

**基础绘制原语（DrawBuf 320×240，50 draws each，min/median µs）**

| 原语 | min | median |
|---|---|---|
| fill_rect | 51.2 | 51.9 |
| draw_line | 18.4 | 18.6 |
| draw_line_many | 6.8 | 6.8 |
| draw_circle | 47.0 | 47.7 |
| fill_circle | 16.6 | 17.2 |
| fill_rounded | 52.7 | 54.8 |
| draw_border | 7.6 | 7.8 |
| draw_arc | 330.3 | 334.4 |
| draw_text | 2.5 | 2.6 |
| blit565 | 2.3 | 2.3 |

### Runtime — QEMU（SysTick ticks，确定性，--release）

**layout**

| 场景 | ticks |
|---|---|
| flex 40 children, 320×240 | 8,786 |

**render / frame**

| 档位 | full | partial | frame |
|---|---|---|---|
| Minimal | 74,516 | 25,283 | 75,122 |
| Small | 341,206 | 156,226 | 342,729 |
| Medium | 777,778 | 535,558 | 779,942 |
| Large | 1,945,162 | 1,539,378 | 1,950,455 |

**基础绘制原语（DrawBuf 320×240，50 draws each，ticks）**

| 原语 | ticks |
|---|---|
| fill_rect | 215,729 |
| draw_line | 190,899 |
| draw_line_many | 36,081 |
| draw_circle | 161,012 |
| fill_circle | 166,656 |
| fill_rounded | 226,371 |
| draw_border | 60,274 |
| draw_arc | 173,846 |
| draw_text | 9,733 |
| blit565 | 9,108 |

## 2026-08-07 — 绘制原语优化后（commit `858f54f`）

> 优化内容：fill_rect 批量填充、draw_line 平行四边形扫描、fill_rounded 主体受益。
> 对比基线：上一节（commit `d4fe7e5`）。
> 数值为优化合并点 `858f54f`（fill_rect 批量填充 `601af93` + draw_line 扫描转换 `b1a2193` + fill_rounded 角钳制测试 `858f54f`）实测。

### Runtime — host（µs，min/median）—— 仅记录三个优化原语

| 原语 | 优化前 min/med | 优化后 min/med |
|---|---|---|
| fill_rect | 51.2 / 51.9 | 24.5 / 24.7 |
| draw_line | 18.4 / 18.6 | 1285.5 / 1296.6 |
| fill_rounded | 52.7 / 54.8 | 25.8 / 26.3 |

### Runtime — QEMU（ticks，--release）

| 原语 | 优化前 | 优化后 |
|---|---|---|
| fill_rect | 215,729 | 77,583 |
| draw_line | 190,899 | 165,793 |
| fill_rounded | 226,371 | 88,860 |

### 回归提示

- **host draw_line 大幅变慢**（18.4 → 1285.5µs）：新实现扫描线段 AABB（O(L²·16)），对 bench 的全屏对角线（320px）为最坏情形；QEMU 端反而小幅变快（190,899 → 165,793），因旧实现每步 stamp 一个 fill_circle，在无 FPU 内核上开销更大。真实代码中应避免极长对角线单次绘制。
- **draw_line_many 同步回归**（host 6.8 → 805.8µs；QEMU 36,081 → 165,604 ticks）：10 条短线各扫描一个 16×240 AABB。QEMU 阈值断言（×2 = 72,162）因此触发，Task 5 需同步重新校准该阈值。

### 优化热点提示（初始基线）

- **QEMU 端**：`fill_rounded`(226k)、`fill_rect`(215k)、`draw_line`(190k)、`draw_arc`(173k)、`fill_circle`(166k) 是前五贵的原语——4×4 超采样绘制与像素填充是主要成本。
- **host 端**：`draw_arc`(330µs) 明显最贵，`fill_rounded`/`fill_rect`(~52µs) 次之。
- **完整帧**：host Large frame ≈ 307µs（约 3.2k FPS 上限），QEMU Large frame ≈ 1.95M ticks。
- **内存**：大场景 live 堆 host 160KB / QEMU 129KB；每节点 Node 376B（64 位）/ 280B（32 位）。
