# qingui 内存评估 bench 设计

日期：2026-08-05
状态：已获用户批准（讨论式 brainstorming 后确认）

## 背景与动机

qingui 是 no_std + alloc 的嵌入式 GUI 库，内存是核心关注点。此前讨论中反复触及几个"想量化却没数"的问题：

- **最大变体税**：`WidgetKind` 枚举内嵌各控件状态，每个节点都付最大变体的字节（ImageState ~100B / 64 位），纯 Obj/Label 节点也在背。
- **每节点 4 份 Style**：`Node` 持有 base/pressed/focused/selected 四份完整 `Style`（node.rs:44-47），是每节点内存的大头。
- **运行时峰值堆**：`List` 的 `Vec<String>`、`Chart` 的 `Vec<Series>`+`VecDeque`、ItemList 的子节点等真实界面会吃多少堆，目前无任何测量。

需要一个**零依赖的本地回归工具**，把这三件事变成可打印、可断言、能防回归的数。

## 目标与非目标

**目标**：
- 零依赖自定义 bench（`[[bench]] harness = false`），`cargo bench -p qingui --bench memory` 一键跑。
- 静态 `size_of` 报告：`Node`/`WidgetKind`/全部 `XxxState`/`Style`/`Ui`，直接量化最大变体税与 4×Style 成本。
- 运行时峰值堆：计数型 `GlobalAlloc` + 三档（small/medium/large）场景表，报告 peak/live/nodes 增长曲线。
- 阈值断言防回归（基线 × 2 余量）。

**非目标**：
- 不做 Criterion / 性能（时间）bench。
- 不做目标板测量脚本（`cargo size` 仅作报告头说明，不落地）。
- 不接 CI（仓库无 CI 基建，本工具作为本地回归）。
- 不改任何库代码（纯新增测量文件）。

## 设计

### 1. bench 基建与文件结构

`qingui/Cargo.toml` 追加（沿用 `autoexamples = false` 的显式声明风格）：

```toml
# benches/ 下只有一个显式声明的 memory bench（harness = false → 自定义 main）
autobenches = false

[[bench]]
name = "memory"
path = "benches/memory.rs"
harness = false
```

唯一新增文件：`qingui/benches/memory.rs`。结构：

```rust
//! Memory benchmark: static type sizes + peak heap of representative scenes.
//!
//! NOTE: host is 64-bit (usize = 8B); thumbv7 is 32-bit (usize = 4B).
//! This bench gives the RELATIVE cost shape and a regression gate; absolute
//! embedded sizes come from `cargo size --target thumbv7em-none-eabihf`.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    // alloc: 记 CURRENT += size，PEAK = max(PEAK, CURRENT)
    // dealloc: CURRENT -= size（System 转发）
}
#[global_allocator]
static G: Counting = Counting;

fn reset() {
    CURRENT.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
}

fn main() {
    report_static_sizes();
    bench_scene("small", Tier::Small);
    bench_scene("medium", Tier::Medium);
    bench_scene("large", Tier::Large);
}
```

**关键机制**：lib 为 no_std + alloc、不设全局分配器，故 `#[global_allocator]` 放 bench 二进制内，qingui 所有 `Vec/String/Box` 分配都流经计数器。`reset-delta` 把 std 运行时背景分配排除在场景段外。

### 2. 静态 size_of 报告

`report_static_sizes()` 用 `core::mem::size_of` 打印（无运行时）：

| 分组 | 类型 | 回答的问题 |
|---|---|---|
| 基础 | `Rect`/`Point`/`Color` | 坐标/颜色开销 |
| 每节点 | `Node`、`WidgetKind`、全部 21 个 `XxxState` | 每节点占地 + 最大变体税 |
| 样式 | `Style`、`ResolvedStyle`、`4 × Style` | 每节点 4 份样式成本 |
| 世界 | `Ui` | 系统固定开销 |

派生指标（打印时计算）：
- `WidgetKind 最大变体税 = size_of::<WidgetKind>() − size_of::<ObjState>()`
- `每节点样式成本 = 4 × size_of::<Style>()`

阈值断言：

```rust
const LIMIT_WIDGETKIND: usize = 256;
const LIMIT_STYLE: usize = 256;
const LIMIT_NODE: usize = 1024;
```

**阈值定值程序**（可重复，非拍脑袋）：实现时第一轮**只打印不 assert**，记录真实基线进本 spec；随后按 `基线 × 2 向上取整` 校准并启用断言。

### 3. 峰值堆场景表

```rust
enum Tier { Small, Medium, Large }

fn bench_scene(label: &str, tier: Tier) {
    reset();
    let ui = build_scene(tier);
    let peak = PEAK.load(Ordering::Relaxed);   // 构造期峰值（含临时分配）
    let live = CURRENT.load(Ordering::Relaxed); // 场景常驻堆（树本身）
    drop(ui);
    // 打印 label / nodes / peak KB / live KB；断言 peak、live 各自 < 对应上限
}
```

`build_scene(tier)`（三档共用同一函数，参数化）：

```rust
fn build_scene(tier: Tier) -> Ui {
    let (n_items, n_chart_pts) = match tier {
        Tier::Small  => (5, 16),
        Tier::Medium => (20, 64),
        Tier::Large  => (60, 256),
    };
    let mut ui = Ui::new(320, 240, 24);
    let scr = ui.screen();
    // ListBuilder::new takes &[&str]; build the label strings first (their allocation
    // is counted, which is representative of real use). Same pattern as dropdown.rs.
    let texts: Vec<String> = (0..n_items).map(|i| format!("item{i}")).collect();
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let list = ListBuilder::new(&refs).build(&mut ui, scr); // Vec<String>
    for i in 0..n_items { ButtonBuilder::new(&format!("btn{i}")).build(&mut ui, scr); }
    for _ in 0..n_items / 4 { SliderBuilder::new(0, 100).build(&mut ui, scr); }
    let chart = ChartBuilder::new().series(Color::RED, n_chart_pts).build(&mut ui, scr);
    let il = ItemListBuilder::new().build(&mut ui, scr);
    for _ in 0..n_items { ui.itemlist_add_item(il); }
    for _ in 0..5 { ui.tick_inc(16); ui.timer_handler(); } // 强制布局/渲染/动画分配
    ui
}
```

场景堆源对应真实 UI 用法：`Vec<String>`（List）、label 文本（Button/Slider）、`Vec<Series>`+`VecDeque`（Chart）、每 item 子节点（ItemList）、`children.clone()` 临时分配（布局）。

### 4. 阈值与验收

峰值阈值（初值估计，按"基线 × 2 向上取整"校准）：

| 档 | 节点数（约） | peak 上限 | live 上限 |
|---|---|---|---|
| small | ~20 | 32 KB | 16 KB |
| medium | ~70 | 128 KB | 64 KB |
| large | ~200 | 512 KB | 256 KB |

常量如 `LIMIT_PEAK_SMALL`，集中于一个 `const` 块。`bench_scene` 内 `assert!(peak < LIMIT_*, "peak {} bytes", peak)`。

## 验收标准

1. `cargo bench -p qingui --bench memory` 运行成功，打印完整报告（size_of 表 + 三档峰值表），全部断言通过。
2. `cargo test -p qingui` 全绿（bench 不影响测试）。
3. 零新增依赖。
4. `benches/memory.rs` 是唯一新增源码文件，`target/` 不入库。

## 影响面

- `qingui/Cargo.toml`：+`autobenches = false` + `[[bench]] memory`。
- 新增 `qingui/benches/memory.rs`。
- 其余文件零改动。

## 风险与对策

- **host 64 位 ≠ 目标 32 位**：绝对值不同。对策：报告头注释说明本工具价值在相对形状与回归护栏，绝对数以 `cargo size` 为准（设计决策，不落地）。
- **std 运行时背景分配污染峰值**：`reset-delta` 隔离场景段。
- **阈值拍脑袋**：第一轮只打印测基线，按 `×2` 校准（定值程序写入第 2/4 节）。
- **`format!` 在场景内分配**：这符合真实用法（label 文本本就该被计数），不回避。
