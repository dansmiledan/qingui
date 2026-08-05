# Memory Benchmark 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 qingui 加一个零依赖的 `cargo bench` 内存评估工具：静态 `size_of` 报告（量化 `WidgetKind` 最大变体税、每节点 4×Style 成本）+ 三档场景的运行时峰值堆，带阈值断言防回归。

**Architecture:** 自定义 bench（`[[bench]] harness = false`，纯 std 零依赖）。计数型 `#[global_allocator]` 放在 bench 二进制内（lib 为 no_std 不设全局分配器，故 qingui 所有 `Vec/String/Box` 分配流经计数器），用 reset-delta 隔离场景段；静态尺寸用 `core::mem::size_of` 打印。阈值先测基线后按 `基线 × 2 向上取整` 校准。

**Tech Stack:** Rust (no_std 库 + std bench 二进制), `cargo bench -p qingui --bench memory`。

## Global Constraints

- **零新增依赖**：只用 `std`（bench 二进制）。`qingui/Cargo.toml` 的 `[dependencies]`/`[dev-dependencies]` 不改。
- **零库代码改动**：只新增 `benches/memory.rs` + `qingui/Cargo.toml` 的 bench 声明。
- **不设 `#[global_allocator]` 于库内**（只在 bench 二进制）。
- **64 位 host**：报告头注释说明相对形状有效、绝对数以 `cargo size` 为准。
- **阈值定值程序**：第一轮（Task 1/2）只打印不 assert，把测到的基线记入 report 文件；最后一轮（Task 3）按 `基线 × 2 向上取整` 填常量并启用断言。
- **`cargo test -p qingui` 必须保持全绿**（bench 默认不进 `cargo test`，不产生影响）。
- **git**：只本地 commit，不 push。
- **验证命令**：`cargo bench -p qingui --bench memory`、`cargo test -p qingui`、`cargo check -p qingui --all-targets`。

---

### Task 1: bench 基建 + 静态 size_of 报告（只打印）

**Files:**
- Create: `qingui/benches/memory.rs`
- Modify: `qingui/Cargo.toml`（加 `autobenches = false` + `[[bench]] memory`）

**Interfaces:**
- Consumes: 无（首个任务）。
- Produces: `Counting` 分配器 + `current()`/`peak()`/`reset()` 辅助 + `report_static_sizes()`（全 `pub` 于模块内、仅 bench 使用）。Task 2 的 `build_scene`/`bench_scene` 复用 `reset()`/`peak()`/`current()`。

- [ ] **Step 1: Cargo.toml 声明 bench**

在 `qingui/Cargo.toml` 的 `[[example]]` 段之后追加：

```toml
# benches/ 下只有一个显式声明的 memory bench（harness = false → 自定义 main）
autobenches = false

[[bench]]
name = "memory"
path = "benches/memory.rs"
harness = false
```

- [ ] **Step 2: 新建 `benches/memory.rs`（分配器 + 静态报告）**

创建 `qingui/benches/memory.rs`，写入：

```rust
//! Memory benchmark: static type sizes + peak heap of representative scenes.
//!
//! NOTE: this runs on the host (64-bit, usize = 8B). The embedded thumbv7
//! target is 32-bit (usize = 4B), so absolute numbers differ. This bench gives
//! the RELATIVE cost shape and a regression gate; absolute embedded sizes come
//! from `cargo size --target thumbv7em-none-eabihf`.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counting allocator: tracks current live bytes and the running peak.
struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let cur = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(cur, Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static G: Counting = Counting;

fn current() -> usize { CURRENT.load(Ordering::Relaxed) }
fn peak() -> usize { PEAK.load(Ordering::Relaxed) }
/// Resets the counters before a measured segment (excludes std runtime noise).
fn reset() {
    CURRENT.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
}

fn report_static_sizes() {
    use core::mem::size_of;
    use qingui::geometry::{Color, Point, Rect};
    use qingui::node::Node;
    use qingui::style::{ResolvedStyle, Style};
    use qingui::widgets::{
        arc, bar, button, chart, checkbox, custom, dropdown, image, itemlist, label, led,
        list, msgbox, obj, roller, scrollview, slider, spinbox, spinner, switch, table,
    };
    use qingui::widgets::WidgetKind;

    println!("== static sizes (host 64-bit) ==");
    println!("Rect          {:>6} B", size_of::<Rect>());
    println!("Point         {:>6} B", size_of::<Point>());
    println!("Color         {:>6} B", size_of::<Color>());
    println!("Style         {:>6} B", size_of::<Style>());
    println!("ResolvedStyle {:>6} B", size_of::<ResolvedStyle>());
    println!("4 x Style     {:>6} B", 4 * size_of::<Style>());
    println!("Node          {:>6} B", size_of::<Node>());
    println!("WidgetKind    {:>6} B", size_of::<WidgetKind>());
    println!("  largest-variant tax = {} B (WidgetKind - ObjState)", size_of::<WidgetKind>() - size_of::<obj::ObjState>());
    macro_rules! row {
        ($name:literal, $t:ty) => { println!("  {:<14} {:>6} B", $name, size_of::<$t>()); };
    }
    row!("Obj", obj::ObjState);
    row!("Label", label::LabelState);
    row!("Button", button::ButtonState);
    row!("Slider", slider::SliderState);
    row!("Switch", switch::SwitchState);
    row!("Bar", bar::BarState);
    row!("List", list::ListState);
    row!("Arc", arc::ArcState);
    row!("Checkbox", checkbox::CheckboxState);
    row!("Chart", chart::ChartState);
    row!("Spinner", spinner::SpinnerState);
    row!("Msgbox", msgbox::MsgboxState);
    row!("Led", led::LedState);
    row!("Table", table::TableState);
    row!("Spinbox", spinbox::SpinboxState);
    row!("Roller", roller::RollerState);
    row!("ScrollView", scrollview::ScrollViewState);
    row!("Dropdown", dropdown::DropdownState);
    row!("Image", image::ImageState);
    row!("ItemList", itemlist::ItemListState);
    row!("Custom", custom::CustomState);
    println!("Ui            {:>6} B", size_of::<qingui::Ui>());
}

fn main() {
    report_static_sizes();
}
```

- [ ] **Step 3: 运行 bench 确认打印**

Run: `cargo bench -p qingui --bench memory`
Expected: 打印静态尺寸表（Rect/Style/Node/WidgetKind + 21 个状态 + Ui），退出码 0。

- [ ] **Step 4: 全量测试不受影响**

Run: `cargo test -p qingui`
Expected: 全绿（bench 不进 test）。

- [ ] **Step 5: 记录基线**

把 Step 3 输出的 `WidgetKind`/`Style`/`4 x Style`/`Node` 四个数记入 report 文件（Task 3 用 `×2` 校准阈值）。

- [ ] **Step 6: Commit**

```bash
git add qingui/Cargo.toml qingui/benches/memory.rs
git commit -m "bench: add zero-dep memory bench with static size report"
```

---

### Task 2: 峰值堆三档场景表（只打印）

**Files:**
- Modify: `qingui/benches/memory.rs`

**Interfaces:**
- Consumes: Task 1 的 `reset()`/`peak()`/`current()`。
- Produces: `enum Tier { Small, Medium, Large }`、`build_scene(tier: Tier) -> qingui::Ui`、`bench_scene(label: &str, tier: Tier)`、`node_count(ui: &qingui::Ui) -> usize`。Task 3 复用 `bench_scene` 加断言。

- [ ] **Step 1: 追加场景代码（RED 不可用，直接实现——新代码，编译即验证）**

在 `benches/memory.rs` 的 `main()` 之前追加：

```rust
enum Tier { Small, Medium, Large }

fn build_scene(tier: Tier) -> qingui::Ui {
    use qingui::prelude::*;
    use qingui::widgets::button::ButtonBuilder;
    use qingui::widgets::chart::ChartBuilder;
    use qingui::widgets::itemlist::ItemListBuilder;
    use qingui::widgets::list::ListBuilder;
    use qingui::widgets::slider::SliderBuilder;
    use qingui::{Color, Ui};

    let (n_items, n_chart_pts) = match tier {
        Tier::Small => (5, 16),
        Tier::Medium => (20, 64),
        Tier::Large => (60, 256),
    };
    let mut ui = Ui::new(320, 240, 24);
    let scr = ui.screen();
    // ListBuilder::new takes &[&str]; build the label strings first (their allocation
    // is counted, which is representative of real use). Same pattern as dropdown.rs.
    let texts: Vec<String> = (0..n_items).map(|i| format!("item{i}")).collect();
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let list = ListBuilder::new(&refs).build(&mut ui, scr);
    for i in 0..n_items {
        ButtonBuilder::new(&format!("btn{i}")).build(&mut ui, scr);
    }
    for _ in 0..n_items / 4 {
        SliderBuilder::new(0, 100).build(&mut ui, scr);
    }
    let chart = ChartBuilder::new().series(Color::RED, n_chart_pts).build(&mut ui, scr);
    let il = ItemListBuilder::new().build(&mut ui, scr);
    for _ in 0..n_items {
        ui.itemlist_add_item(il);
    }
    // Force real allocations through layout / render / animation paths.
    for _ in 0..5 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    ui
}

fn node_count(ui: &qingui::Ui) -> usize {
    let mut n = 0;
    let mut stack = vec![ui.screen()];
    while let Some(o) = stack.pop() {
        n += 1;
        stack.extend(ui.children(o));
    }
    n
}

fn bench_scene(label: &str, tier: Tier) {
    reset();
    let ui = build_scene(tier);
    let nodes = node_count(&ui);
    let peak = peak();
    let live = current();
    drop(ui);
    println!("{label:<8} {nodes:>5} nodes  peak {peak:>9} B  live {live:>9} B");
}

fn main() {
    report_static_sizes();
    bench_scene("small", Tier::Small);
    bench_scene("medium", Tier::Medium);
    bench_scene("large", Tier::Large);
}
```

（`main()` 改为调用三个 `bench_scene`。`list` 变量在场景中故意保留引用以维持树常驻——如编译器报未使用，加 `let _ = &list;`。）

- [ ] **Step 2: 运行 bench 确认三档表**

Run: `cargo bench -p qingui --bench memory`
Expected: 打印静态表 + 三档 `nodes/peak/live` 行，退出码 0。确认 medium/large 的 nodes 数随档位增长（~20/70/200）。

- [ ] **Step 3: 记录基线**

把三档的 peak/live 记入 report 文件（Task 3 用 `×2` 校准）。

- [ ] **Step 4: Commit**

```bash
git add qingui/benches/memory.rs
git commit -m "bench: add peak-heap scene table (small/medium/large)"
```

---

### Task 3: 启用阈值断言（基线 × 2 校准）

**Files:**
- Modify: `qingui/benches/memory.rs`

**Interfaces:**
- Consumes: Task 1/2 记入 report 的基线（`WidgetKind`/`Style`/`Node` 尺寸 + 三档 peak/live）。
- Produces: 最终可交付的 memory bench（报告 + 断言全开）。

- [ ] **Step 1: 读取基线**

读 report 文件（Task 1/2 写入的基线数值）：`WidgetKind`、`Style`、`Node`、三档 `peak`/`live`。对每个数计算 `×2 向上取整`（取整到整字节即可，不必对齐），并确保静态阈值不低于 spec 初值（`WidgetKind < 256`、`Style < 256`、`Node < 1024`）——取 `max(基线 × 2, 初值)`。

- [ ] **Step 2: 加常量与断言**

在 `benches/memory.rs` 顶部（分配器之后）加常量块：

```rust
// Thresholds calibrated <date>: measured baseline x 2 (see spec
// docs/superpowers/specs/2026-08-05-memory-bench-design.md).
const LIMIT_WIDGETKIND: usize = 256;
const LIMIT_STYLE: usize = 256;
const LIMIT_NODE: usize = 1024;
const LIMIT_PEAK_SMALL: usize = 32 * 1024;
const LIMIT_LIVE_SMALL: usize = 16 * 1024;
const LIMIT_PEAK_MEDIUM: usize = 128 * 1024;
const LIMIT_LIVE_MEDIUM: usize = 64 * 1024;
const LIMIT_PEAK_LARGE: usize = 512 * 1024;
const LIMIT_LIVE_LARGE: usize = 256 * 1024;
```

（把 `max(基线 × 2, 初值)` 的结果填入上述常量。）

在 `report_static_sizes()` 末尾加：

```rust
    assert!(size_of::<WidgetKind>() < LIMIT_WIDGETKIND, "WidgetKind {} B exceeds limit", size_of::<WidgetKind>());
    assert!(size_of::<Style>() < LIMIT_STYLE, "Style {} B exceeds limit", size_of::<Style>());
    assert!(size_of::<Node>() < LIMIT_NODE, "Node {} B exceeds limit", size_of::<Node>());
```

在 `bench_scene` 的 `drop(ui)` 之后加：

```rust
    let (peak_limit, live_limit) = match tier {
        Tier::Small => (LIMIT_PEAK_SMALL, LIMIT_LIVE_SMALL),
        Tier::Medium => (LIMIT_PEAK_MEDIUM, LIMIT_LIVE_MEDIUM),
        Tier::Large => (LIMIT_PEAK_LARGE, LIMIT_LIVE_LARGE),
    };
    assert!(peak < peak_limit, "{label}: peak {peak} B exceeds {peak_limit} B");
    assert!(live < live_limit, "{label}: live {live} B exceeds {live_limit} B");
```

- [ ] **Step 3: 运行 bench 确认断言全过**

Run: `cargo bench -p qingui --bench memory`
Expected: 全表打印 + 0 断言失败，退出码 0。

- [ ] **Step 4: 全量验证**

Run: `cargo test -p qingui`
Run: `cargo check -p qingui --all-targets`
Expected: 全绿；`--all-targets` 不含新 warning。

- [ ] **Step 5: 确认唯一源码改动**

Run: `git status --short`
Expected: 只有 `qingui/benches/memory.rs` 被修改（Cargo.toml 已在前任务提交）。

- [ ] **Step 6: Commit**

```bash
git add qingui/benches/memory.rs
git commit -m "bench: enable threshold asserts calibrated to measured baselines"
```

---

## Self-Review

**Spec 覆盖：**
- 零依赖自定义 bench（harness=false）→ Task 1 Step 1。
- 静态 size_of 报告（含最大变体税、4×Style）→ Task 1 Step 2。
- 峰值堆三档场景表（counting allocator + reset-delta）→ Task 2。
- 阈值断言（基线 × 2 校准）→ Task 3。
- 验收：`cargo bench` 跑通 + `cargo test` 绿 + 零依赖 + 唯一新增文件 → 各任务验证步骤。

**占位符扫描：** 无 TBD/TODO。Task 3 的常量值是"取 max(基线×2, 初值)"的确定性程序——由 Task 1/2 报告的实测基线驱动，非拍脑袋。

**类型一致性：**
- `reset()`/`peak()`/`current()` 三处一致（Task 1 定义，Task 2/3 使用）。
- `build_scene(tier) -> qingui::Ui` 与 `bench_scene(label, tier)` 在 Task 2 定义、Task 3 复用签名一致。
- `ListBuilder::new(&refs)` 用 `Vec<String> → Vec<&str>` 转换（dropdown.rs 同款），与 `&[&str]` 签名匹配。
- `Tier` 枚举三变体在 Task 2 定义、Task 3 match 一致。
- 21 个状态类型与 `define_widgets!` 变体一一对应（Obj/ItemList/Label/Button/Slider/Switch/Bar/List/Arc/Checkbox/Chart/Spinner/Msgbox/Led/Table/Spinbox/Roller/ScrollView/Dropdown/Image/Custom），不引入 `canvas`（Canvas 非 WidgetKind 变体）。
