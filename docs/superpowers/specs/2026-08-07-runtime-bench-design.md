# qingui 运行时间 bench 设计

日期：2026-08-07
状态：已获用户批准（逐项 brainstorming 讨论后确认）

## 背景与动机

qingui 是 no_std + alloc 的嵌入式 GUI 库。memory bench（`cargo bench -p qingui --bench memory` + `tools/qemu-mem`）已量化内存成本；但运行时间是另一个核心关注维度，且从未被量化：

- **每帧耗时**：layout + render 各占多少？全屏刷新 vs 局部刷新差多少？无任何数据。
- **布局压力**：flex/grid 布局在大量子控件下的成本（`children.clone()` + 排序 + 坐标计算）从未测量。
- **基础绘制原语**：`draw_line`/`fill_circle`/`draw_arc` 等是后续要优化的对象，但没有基线无法量化优化收益。

需要一个**双端运行时间评估工具**：host 端零依赖墙钟 bench（参考数据）+ QEMU bare-metal 确定性周期计数（回归护栏），让每帧/每阶段的耗时变成可打印、可断言、能防回归的数。

## 目标与非目标

**目标**：
- host 端自定义 bench（`[[bench]] harness = false`）：`cargo bench -p qingui --bench time` 一键跑。
- QEMU bare-metal 计时 tool（`tools/qemu-time`）：SysTick + `-icount shift=3` 确定性周期计数，输出 `ticks`。
- 5 项指标：layout、render 全屏 dirty、render 局部 dirty、完整帧、基础绘制原语。
- 3 类场景：layout-heavy（flex/grid + N 子控件）、render-heavy（复用 memory 场景）、primitive（全屏 DrawBuf 单次绘制）。
- QEMU 端阈值断言防回归（基线 × 2）；host 端只报告（warmup + N 次取 min/median）。

**非目标**：
- 不做 Criterion（延续 memory bench 零依赖原则）。
- 不做真硬件测量脚本（绝对周期数以真硬件 DWT 为准，bench 给相对形状 + 回归门）。
- 不接 CI（仓库无 CI 基建）。
- 不改 widget/render/layout 业务代码（仅 `ui.rs` 加一个 `#[doc(hidden)] pub fn layout()` 封装）。

## 设计

### 1. 架构与文件结构

```
qingui/benches/time.rs    ← host 端：零依赖，harness=false，std::time::Instant 墙钟（µs）
tools/qemu-time/          ← QEMU 端：SysTick + -icount shift=3，确定性周期计数（ticks）
  ├── Cargo.toml          ← 复用 qemu-mem 的 target 依赖模式（cortex-m-rt + cortex-m-semihosting）
  ├── build.rs            ← memory.x 生成（同 qemu-mem）
  ├── memory.x            ← 同 qemu-mem（FLASH 16M / RAM 4M，_stack_size 64K）
  └── src/
      ├── main.rs         ← 入口 + 报告（host 构建为 stub，同 qemu-mem）
      ├── timer.rs        ← SysTick 初始化 + wrap-aware elapsed 读取
      └── scenes.rs       ← 双端共享场景（Tier + build_scene + primitive bench）
```

**关键机制**：
- **host 端**：`std::time::Instant` 测墙钟。`layout_pass` 是 `pub(crate)`，故 `ui.rs` 新增 `#[doc(hidden)] pub fn layout()`（封装 `layout_pass`），host bench 和 QEMU tool 都能调用。
- **QEMU 端**：SysTick 计数。已实测验证（mps2-an386 + `-icount shift=3`）：4 次运行结果完全一致（10k→8003 / 1M→800002 ticks，精确 100 倍线性），是确定性指令级计数。

### 2. QEMU 计时源验证结论

在 mps2-an386 上实测 bare-metal 探针，结论：

| 计时源 | 结果 |
|---|---|
| DWT CYCCNT | **QEMU 未实现**（QEMU 源码 `TODO: Implement debug registers`；DEMCR/DWT_CTRL 写入被忽略，读回 0） |
| SYS_CLOCK / SYS_ELAPSED semihosting | 不支持（返回 -1 / 0） |
| SysTick（无 `-icount`） | 不走（虚拟时钟只在宿主墙钟到点才跳变） |
| **SysTick + `-icount shift=3`** | **可用**：确定性、随指令数线性推进 |

**运行方式**（写入 `.cargo/config.toml` runner；工具必须用 `--release` 构建运行）：
```
qemu-system-arm -machine mps2-an386 -icount shift=3 -nographic -semihosting-config enable=on,target=native -kernel
```

**profile 要求（重要）**：QEMU 工具必须用 `--release` 构建。dev profile（opt-level 0 + debug_assertions 全开）会让 SysTick 指令计数严重失真——实测 dev 下 `draw_border`/`draw_circle` 比 `draw_arc` 贵、`fill_rounded` 比 `fill_rect` 便宜（后者数学上不可能，fill_rounded 做严格更多工作），而 release 下两者反转且与 host 排序一致（arc > circle > border）。阈值断言按 release 基线 ×2 校准，dev 运行会（正确地）断言失败。

**语义边界**：`ticks` 是 QEMU 确定性虚拟周期（1 tick = 2^shift = 8 虚拟 ns），非真实硬件周期。绝对数以真硬件 DWT 为准；本工具价值在相对成本形状 + 回归护栏——与 memory bench 完全一致的定位（README 已记录同样措辞）。

### 3. 五项指标与测量实现

| # | 指标 | 实现 | 说明 |
|---|---|---|---|
| 1 | layout | `ui.layout()` | `#[doc(hidden)] pub fn layout(&mut self)` 封装 `layout_pass`（ui.rs:443） |
| 2 | render 全屏 | 全屏 `invalidate_area` → `ui.render()` | 最坏情况渲染 |
| 3 | render 局部 | 单控件 `invalidate_obj` → `ui.render()` | 典型交互（value 变化） |
| 4 | 完整帧 | `ui.timer_handler()` | 端到端一帧 |
| 5 | 基础原语 | 构造全屏 `DrawBuf` 直接调 `draw_*` | 单次绘制不分块 |

**为什么原语层无需库改动**：`DrawBuf` 字段（`pixels`/`area`/`stride`）和全部 `draw_*` 方法都是 `pub`（draw.rs），bench 可直接构造 `DrawBuf { pixels: &mut buf, area: full, stride: w }` 调用。这是原语层比 widget 层更干净的原因。

### 4. 场景表

**场景 A：layout-heavy**（测指标 1）
```
320×240 容器，设 Flex/Grid，内放 N 个子控件（N 分档）
每次测量前改容器尺寸强制 layout_dirty，再测 ui.layout()
```
N 分档：Small 10 / Medium 40 / Large 160。子控件含混合类型（Label/Button/Slider）。

**场景 B：render-heavy**（测指标 2/3/4，复用 memory bench 场景）
- Minimal：160×120，Label + Button
- Small/Medium/Large：320×240，List + Buttons + Sliders + Chart + ItemList
- 全屏 dirty：`invalidate_area(全屏)` → `render()`
- 局部 dirty：`invalidate_obj(单控件)` → `render()`
- 完整帧：`timer_handler()`

**场景 C：primitive**（测指标 5，单次绘制，PFB 不分块）
320×240 全屏单 buffer，每个原语固定参数固定调用次数：

| 原语 | 参数 |
|---|---|
| `fill_rect` | 全屏矩形 |
| `draw_line` 单条 | 对角线长线 |
| `draw_line` 多条 | 100 条短线 |
| `draw_circle` | r=60 空心圆 |
| `fill_circle` | r=40 实心圆（4×4 超采样，最重） |
| `fill_rounded` | 全屏 r=8 圆角矩形 |
| `draw_border` | 全屏边框 w=4 |
| `draw_arc` | r=80 弧 0°..270° |
| `draw_text` | FONT_6X10 短字符串 |
| `blit565` | 合成 32×24 RGB565 图块 blit |

每次调用计时，报单次耗时。**参数固定才可比**——原语耗时强依赖 buffer/形状参数。

### 5. 计时方法与报告

**host 端**：
- warmup 若干次 → N 次迭代（如 100）取 min/median
- 只报告不断言（墙钟受宿主负载影响，min 代表最佳情况）
- 单位：µs

**QEMU 端**：
- 确定性，单次测量即可
- 阈值断言（基线 × 2，同 memory bench 校准程序：第一轮只打印测基线，按 ×2 校准后启用）
- 单位：ticks

**报告格式**：
```
== layout (medium, 40 children) ==
  host min 12.3 µs  median 13.1 µs  (100 iters)
  qemu       8123 ticks                < 16246 (baseline*2)

== primitives (DrawBuf 320x240) ==
  fill_rect    host min 0.2 µs   qemu  18 ticks
  draw_line    host min 1.1 µs   qemu  92 ticks
  ...
```

## 验收标准

1. `cargo bench -p qingui --bench time` 运行成功，打印完整报告（layout/render 全屏/局部/帧/原语），host 端退出码 0。
2. QEMU 端 `cargo run --release --target thumbv7em-none-eabihf`（tools/qemu-time，**必须 `--release`**）运行成功，打印同结构报告，断言全过，退出码 0。
3. `cargo test -p qingui` 全绿（bench 不影响测试）。
4. `cargo test -p qemu-time` 全绿（host stub + timer 逻辑测试）。
5. 零新增第三方依赖（复用 workspace 已有依赖）。
6. 唯一库代码改动：`ui.rs` 加 `#[doc(hidden)] pub fn layout()`（纯封装）。

## 影响面

- `qingui/Cargo.toml`：+`[[bench]] time`。
- `qingui/src/ui.rs`：+`#[doc(hidden)] pub fn layout()`（封装 `layout_pass`，1 行）。
- `qingui/benches/time.rs`：新增。
- `tools/qemu-time/`：新增 tool crate（含 tests 或 host stub）。
- 其余文件零改动。

## 风险与对策

- **QEMU ticks ≠ 真实硬件周期**：报告头注释说明相对形状 + 回归护栏定位，绝对数以真硬件 DWT 为准（同 memory bench）。
- **原语耗时依赖参数**：参数固定于 spec 第 4 节场景表，改动需走 spec。
- **SysTick 24 位回绕**（16.7M ticks 上限）：timer.rs 的 wrap-aware 累计读取（CVR 回绕时 +RVR）正确覆盖单次读取间至多一次回绕；当前最大单次测量（render full Large，~15.8M ticks）约为一个 reload 的 94%，在双回绕少计（>~33.5M ticks）之前约有 2.1× 余量。超出需把 timer 改为多回绕计数（或拆段测量），当前阈值内无风险。
- **host 墙钟噪声**：warmup + 取 min + 只报告不断言，噪声不触发误报。
- **双端场景漂移**：场景定义集中一处为源，host bench 与 QEMU tool 各自引入。方案：场景源码放 `tools/qemu-time/src/scenes.rs`（QEMU 端直接 `mod scenes`）；host bench 用 `#[path = "../../tools/qemu-time/src/scenes.rs"] mod scenes;` 引入同一文件（`tools/qemu-mem/tests/alloc_host.rs` 已用同款 `#[path]` 手法，先例可循）。单一来源，避免漂移。
