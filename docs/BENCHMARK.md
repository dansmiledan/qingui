# qingui 性能基线记录（Benchmark Baselines）

本文件记录 qingui 双端（host / QEMU）memory 与 runtime 基准的**基线快照**，作为后续优化的对照记录。每次重新校准阈值或做优化时追加一条记录。

## 约定

- **host**：64 位 macOS（aarch64-apple-darwin），`cargo bench -p qingui --bench memory|time`。
- **QEMU**：`qemu-system-arm -machine mps2-an386 -icount shift=3`，thumbv7em-none-eabihf（Cortex-M4F，32 位）。
  - memory：`(cd tools/qemu-mem && cargo run --target thumbv7em-none-eabihf)`。
  - time：`(cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)`。**必须 `--release`**（dev profile 指令计数失真，见 [spec](superpowers/specs/2026-08-07-runtime-bench-design.md)）。
- **单位**：memory 为字节（B）；time 为 host µs（min/median）/ QEMU SysTick ticks（确定性，1 tick = 8 虚拟 ns）。
- **QEMU 语义边界**：ticks 是 QEMU 确定性指令级计数，非真实硬件周期；绝对数以真硬件 DWT 为准，本记录用于相对成本形状与回归对照。
- **环境绑定**：QEMU ticks 的确定性只在同一环境（rustc / QEMU 版本）内成立；跨工具链会因 codegen 差异漂移约 ±3%（本文件已观察到一次）。×2 阈值用于吸收该漂移；记录新基线时请注明环境。

---

## 2026-08-07 — 初始基线（commit `d4fe7e5`）

> commit: `d4fe7e54361130d33ec0bdfeb4c143e15890e8f9`（fix(tools): recalibrate qemu-time thresholds to release profile baselines）
> 日期：2026-08-07。当前最新基线；阈值断言均已按此校准（×2）。
>
> 更正（见下一条记录）：本节 QEMU-time 数字实际是在 macOS aarch64 +
> rustc 1.95.0-nightly + QEMU 11.0.0 环境下测的，与 d4fe7e5 提交当时写在
> `qemu-time/src/main.rs` 里的基线（layout 8784 / render full Minimal 72619 /
> blit565 9178 等）并不相同——两者代码一致，差异来自测量环境。QEMU-time 的
> memory 阈值当时复制的是 host 64 位基线，并非 QEMU 实测。

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

### 优化热点提示（初始基线）

- **QEMU 端**：`fill_rounded`(226k)、`fill_rect`(215k)、`draw_line`(190k)、`draw_arc`(173k)、`fill_circle`(166k) 是前五贵的原语——4×4 超采样绘制与像素填充是主要成本。
- **host 端**：`draw_arc`(330µs) 明显最贵，`fill_rounded`/`fill_rect`(~52µs) 次之。
- **完整帧**：host Large frame ≈ 307µs（约 3.2k FPS 上限），QEMU Large frame ≈ 1.95M ticks。
- **内存**：大场景 live 堆 host 160KB / QEMU 129KB；每节点 Node 376B（64 位）/ 280B（32 位）。

---

## 2026-08-07 — bench 修复后复测（基于 `e443601` 的工作树）

> 环境：macOS aarch64，rustc 1.95.0-nightly (18d13b533 2026-02-09)，QEMU 11.0.0。
> QEMU 端同环境两次运行逐 tick 一致。
>
> 本次变更（不影响场景构成，memory 数字不变）：
> - 场景构建代码收敛为单一来源 `tools/qemu-mem/src/scene.rs`（host memory bench、
>   qemu-mem、alloc_host 测试、qemu-time 共用）。
> - `blit565` 的源图片分配移出计时循环（此前每 draw 都计入一次 alloc+清零）。
> - 两个 QEMU 工具的 free-list allocator 修复对齐 padding / 过小 tail 的 arena 泄漏；
>   分配路径略有变化，layout/render 数字随之轻微移动。
> - qemu-mem 阈值改为 QEMU 实测基线 ×2（此前错用 host 64 位基线）；qemu-time 阈值
>   按本表重新校准；host memory bench 去掉会下溢的计数器 reset。

### Runtime — QEMU（SysTick ticks，--release，当前基线）

**layout**

| 场景 | ticks |
|---|---|
| flex 40 children, 320×240 | 8,938 |

**render / frame**

| 档位 | full | partial | frame |
|---|---|---|---|
| Minimal | 74,154 | 25,258 | 74,779 |
| Small | 341,923 | 156,325 | 343,756 |
| Medium | 777,980 | 535,579 | 780,209 |
| Large | 1,945,377 | 1,539,405 | 1,950,700 |

**基础绘制原语（DrawBuf 320×240，50 draws each，ticks）**

| 原语 | ticks | 原语 | ticks |
|---|---|---|---|
| fill_rect | 215,729 | fill_rounded | 226,371 |
| draw_line | 190,899 | draw_border | 60,274 |
| draw_line_many | 36,081 | draw_arc | 173,846 |
| draw_circle | 161,012 | draw_text | 9,735 |
| fill_circle | 166,656 | blit565 | 8,418 |

### Runtime — host（µs，100 采样 min/median，当前基线）

layout flex 40 children：min 2.2 / median 2.2。

| 档位 | full min/med | partial min/med | frame min/med |
|---|---|---|---|
| Minimal | 14.3 / 14.4 | 3.8 / 3.9 | 14.5 / 15.2 |
| Small | 77.7 / 78.0 | 22.8 / 22.9 | 78.0 / 78.3 |
| Medium | 138.2 / 138.6 | 74.2 / 74.4 | 138.9 / 139.3 |
| Large | 300.2 / 300.7 | 210.5 / 210.9 | 302.1 / 303.5 |

| 原语 | min | median | 原语 | min | median |
|---|---|---|---|---|---|
| fill_rect | 50.3 | 50.7 | fill_rounded | 51.6 | 51.8 |
| draw_line | 18.1 | 18.1 | draw_border | 7.5 | 7.5 |
| draw_line_many | 6.6 | 6.6 | draw_arc | 322.2 | 323.7 |
| draw_circle | 45.9 | 46.3 | draw_text | 2.4 | 2.5 |
| fill_circle | 16.3 | 16.5 | blit565 | 2.1 | 2.1 |

### Memory（host / QEMU）

与初始基线完全一致（host peak 5,871 / 35,069 / 70,800 / 209,576 B；QEMU peak
5,431 / 32,205 / 59,476 / 165,516 B），静态尺寸不变。
