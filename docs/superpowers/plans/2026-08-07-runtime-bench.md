# Runtime Benchmark 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 qingui 加一个双端运行时间评估工具：host 端零依赖墙钟 bench（`cargo bench -p qingui --bench time`）+ QEMU bare-metal 确定性周期计数 tool（`tools/qemu-time`，SysTick + `-icount shift=3`），覆盖 layout / render 全屏 / render 局部 / 完整帧 / 基础绘制原语 5 项指标，QEMU 端带阈值断言防回归。

**Architecture:** 场景与测量逻辑集中到 `tools/qemu-time/src/scenes.rs`（单一来源，no_std + alloc，时钟以 `&mut dyn FnMut() -> u64` 注入）；QEMU 端 `main.rs` 直接 `mod scenes;`，host 端 bench 用 `#[path = "../../tools/qemu-time/src/scenes.rs"] mod scenes;` 引入同一文件（`tools/qemu-mem/tests/alloc_host.rs` 已用同款 `#[path]` 手法）。QEMU 端 SysTick 计时（DWT 已在 QEMU 中验证未实现，见 spec §2）。唯一库改动：`ui.rs` 加 `#[doc(hidden)] pub fn layout()` 封装 `layout_pass`。

**Tech Stack:** Rust（no_std 库 + std bench 二进制 + thumbv7em bare-metal tool），`cargo bench -p qingui --bench time`、`qemu-system-arm -machine mps2-an386 -icount shift=3`。

## Global Constraints

- **零新增第三方依赖**：host bench 只用 `std`；QEMU tool 复用 workspace 已有 `cortex-m-rt`/`cortex-m-semihosting`（`qingui` 的传递依赖模式，见 qemu-mem）。
- **唯一库代码改动**：`qingui/src/ui.rs` 加 `#[doc(hidden)] pub fn layout(&mut self) { self.layout_pass(); }`（纯封装，1 行）。
- **场景单一来源**：`tools/qemu-time/src/scenes.rs` 只能 `core`/`alloc` + `qingui`，不得用 `std`、不得直接引用 `cortex-m`/semihosting；时钟参数为 `now: &mut dyn FnMut() -> u64`（host 传 ns、QEMU 传 ticks）。
- **QEMU 计时**：`-icount shift=3`；SysTick `RVR=0xFFFFFF`，wrap-aware 累计（24 位回绕安全）。
- **阈值程序**：Task 3 只打印测基线；Task 5 按 `基线 × 2` 填入常量并启用断言。
- **`cargo test -p qingui`、`cargo test -p qemu-time` 必须全绿**。
- **git**：只本地 commit，不 push；Commit message 英文（Conventional Commits）。
- **验证命令**：`cargo test -p qingui`、`cargo test -p qemu-time`、`cargo bench -p qingui --bench time`、`cargo run -p qemu-time --target thumbv7em-none-eabihf`。

---

### Task 1: 库加 `layout()` 方法（TDD）

**Files:**
- Modify: `qingui/src/ui.rs`（在 `layout_pass` 定义后加方法；`#[cfg(test)]` 模块末尾加测试）

**Interfaces:**
- Produces: `Ui::layout(&mut self)` —— `#[doc(hidden)] pub`，强制跑一遍完整布局（不检查 `layout_dirty`）。Task 3 的 `time_layout` 与 Task 4 的 host bench 依赖它。

- [ ] **Step 1: 写失败测试**

在 `qingui/src/ui.rs` 的 `#[cfg(test)] mod tests` 模块内追加：

```rust
#[test]
fn layout_runs_flex_pass() {
    use crate::layout::{Align, Flex, FlexDir};
    use crate::style::Layout;
    use crate::widgets::label::LabelCfg;
    use crate::widgets::obj::ObjCfg;
    let mut ui = Ui::new(320, 240, 24);
    let scr = ui.screen();
    let container = ObjCfg::new()
        .size(320, 240)
        .layout(Layout::Flex(Flex {
            dir: FlexDir::Column, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
        }))
        .build(&mut ui, scr);
    let a = LabelCfg::new("A").size(10, 10).build(&mut ui, container);
    let b = LabelCfg::new("B").size(10, 10).build(&mut ui, container);
    ui.layout();
    let (ra, rb) = (ui.rect(a), ui.rect(b));
    assert!(rb.y > ra.y, "B should be below A in a column flex (a.y={} b.y={})", ra.y, rb.y);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p qingui ui::tests::layout_runs_flex_pass`
Expected: FAIL —— `Ui::layout` 不存在（`no method named `layout``）。

- [ ] **Step 3: 最小实现**

在 `qingui/src/ui.rs` 的 `layout_pass`（约 443 行）之后追加：

```rust
/// Forces a full layout pass, bypassing the `layout_dirty` flag.
///
/// Intended as a benchmark hook so tools can time the layout phase in
/// isolation (see `docs/superpowers/specs/2026-08-07-runtime-bench-design.md`).
#[doc(hidden)]
pub fn layout(&mut self) {
    self.layout_pass();
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p qingui ui::tests::layout_runs_flex_pass`
Expected: PASS。

- [ ] **Step 5: 全量测试 + Commit**

Run: `cargo test -p qingui`
Expected: 全绿。

```bash
git add qingui/src/ui.rs
git commit -m "feat(ui): add doc(hidden) layout() hook for benchmark timing"
```

---

### Task 2: QEMU tool 基建（scaffold）

**Files:**
- Create: `tools/qemu-time/Cargo.toml`
- Create: `tools/qemu-time/build.rs`
- Create: `tools/qemu-time/memory.x`
- Create: `tools/qemu-time/.cargo/config.toml`
- Create: `tools/qemu-time/src/allocator.rs`（从 qemu-mem 复制）
- Create: `tools/qemu-time/src/timer.rs`
- Create: `tools/qemu-time/src/main.rs`（stub + `mod` 声明）
- Modify: `Cargo.toml`（workspace members 加 `"tools/qemu-time"`）

**Interfaces:**
- Produces:
  - `tools/qemu-time/src/timer.rs`：`pub fn init()`、`pub fn elapsed() -> u64`（SysTick wrap-aware，QEMU ticks）。Task 3/4/5 用 `timer::elapsed()` 作 `now`。
  - `tools/qemu-time/src/allocator.rs`：`pub struct Counting` + `#[global_allocator]`（裸机可用的 arena 分配器，qingui 的 `Vec/String/Box` 依赖它）。Task 3 的 scenes 在 QEMU 端分配依赖它。

- [ ] **Step 1: 写 `Cargo.toml`**

```toml
[package]
name = "qemu-time"
version = "0.1.0"
edition = "2021"
description = "no_std runtime bench for qingui, run on QEMU mps2-an386 (thumbv7em-none-eabihf)"
publish = false

[dependencies]
qingui = { path = "../../qingui" }

# Cortex-M runtime + semihosting are only needed for the bare-metal target;
# host builds (cargo test / cargo build --workspace) get a stub main instead.
[target.'cfg(target_arch = "arm")'.dependencies]
cortex-m-rt = "0.7"
cortex-m-semihosting = "0.5"

[profile.release]
panic = "abort"
opt-level = 2
```

- [ ] **Step 2: 写 `build.rs` 与 `memory.x`**

`build.rs`（与 qemu-mem 完全相同）：

```rust
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
}
```

`memory.x`（与 qemu-mem 完全相同）：

```
MEMORY
{
  FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 16M
  RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 4M
}

_stack_size = 64K;
```

- [ ] **Step 3: 写 `.cargo/config.toml`**

关键区别：runner 加 `-icount shift=3`（SysTick 时钟前进的必要条件，spec §2 已验证）。

```toml
[target.thumbv7em-none-eabihf]
runner = "qemu-system-arm -machine mps2-an386 -icount shift=3 -nographic -semihosting-config enable=on,target=native -kernel"
rustflags = ["-C", "link-arg=-Tlink.x"]
```

- [ ] **Step 4: 复制 allocator**

```bash
cp tools/qemu-mem/src/allocator.rs tools/qemu-time/src/allocator.rs
```

- [ ] **Step 5: 写 `src/timer.rs`**

```rust
//! SysTick-based deterministic cycle counter for QEMU timing.
//!
//! QEMU's Cortex-M model does NOT implement the DWT cycle counter (source:
//! `hw/intc/armv7m_nvic.c` has `TODO: Implement debug registers`). SysTick
//! advances deterministically only when QEMU runs with `-icount shift=3`
//! (verified: 4 identical runs, 10k->8003 / 1M->800002 ticks, exactly 100x
//! linear). Each tick = 2^shift = 8 virtual ns; absolute hardware cycles come
//! from real-hardware DWT — this gives a deterministic relative shape.

use core::sync::atomic::{AtomicBool, Ordering};

const SYST_CSR: usize = 0xE000_E010;
const SYST_RVR: usize = 0xE000_E014;
const SYST_CVR: usize = 0xE000_E018;
const RELOAD: u32 = 0x00FF_FFFF;

static INIT: AtomicBool = AtomicBool::new(false);
// Bare metal, single thread: static mut is fine here.
static mut PREV: u32 = RELOAD;
static mut ACC: u64 = 0;

#[inline(never)]
fn mmio_write(addr: usize, val: u32) {
    unsafe { core::ptr::write_volatile(addr as *mut u32, val) };
}
#[inline(never)]
fn mmio_read(addr: usize) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

/// Enables SysTick: max reload, core clock source, count down to zero.
pub fn init() {
    mmio_write(SYST_RVR, RELOAD);
    mmio_write(SYST_CVR, 0);
    mmio_write(SYST_CSR, 0x5); // CLKSOURCE=1 (core clock), ENABLE=1
    unsafe { PREV = mmio_read(SYST_CVR) & 0xFF_FFFF; }
    INIT.store(true, Ordering::Relaxed);
}

/// Monotonic elapsed ticks since `init()` (or first call), wrap-aware:
/// the 24-bit SysTick wraps back to RELOAD; we add the wrapped distance.
pub fn elapsed() -> u64 {
    if !INIT.load(Ordering::Relaxed) {
        init();
    }
    let cur = mmio_read(SYST_CVR) & 0xFF_FFFF;
    unsafe {
        let prev = PREV;
        PREV = cur;
        if cur <= prev {
            ACC += (prev - cur) as u64;
        } else {
            ACC += prev as u64 + (RELOAD - cur) as u64;
        }
        ACC
    }
}
```

- [ ] **Step 6: 写 `src/main.rs` stub**

```rust
#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

//! 32-bit runtime bench for qingui, run on QEMU (`-machine mps2-an386`,
//! Cortex-M4F, thumbv7em-none-eabihf, `-icount shift=3`).
//!
//! Prints layout / render (full + partial dirty) / full-frame / primitive
//! timing as deterministic SysTick ticks. Semihosting carries the output; the
//! exit code reflects whether all asserts passed.
//!
//! Build/run for the target:
//!
//! ```text
//! cargo run -p qemu-time --target thumbv7em-none-eabihf
//! ```
//!
//! On host builds (workspace `cargo test`/`cargo build`) this compiles to a
//! stub so the crate stays a clean workspace member.

#[cfg(target_arch = "arm")]
extern crate alloc;

#[cfg(target_arch = "arm")]
mod allocator;
#[cfg(target_arch = "arm")]
mod timer;

#[cfg(target_arch = "arm")]
use cortex_m_rt::entry;
#[cfg(target_arch = "arm")]
use cortex_m_semihosting::debug::{exit, EXIT_SUCCESS};
#[cfg(target_arch = "arm")]
use cortex_m_semihosting::hprintln;

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = hprintln!("PANIC: {}", info);
    exit(cortex_m_semihosting::debug::EXIT_FAILURE);
    loop {}
}

#[cfg(target_arch = "arm")]
#[entry]
fn main() -> ! {
    hprintln!("qemu-time: scene modules wired in Task 3");
    exit(EXIT_SUCCESS);
    loop {}
}

#[cfg(not(target_arch = "arm"))]
fn main() {
    println!("qemu-time targets the bare-metal Cortex-M4F; build and run it for the embedded target:");
    println!("  cargo run -p qemu-time --target thumbv7em-none-eabihf");
}
```

- [ ] **Step 7: 注册 workspace + 验证构建**

修改根 `Cargo.toml`：

```toml
members = ["qingui", "qingui-codegen", "tools/qemu-mem", "tools/qemu-time"]
```

Run:
- `cargo build -p qemu-time`
- `cargo build -p qemu-time --target thumbv7em-none-eabihf`
- `cargo run -p qemu-time --target thumbv7em-none-eabihf`
- `cargo test -p qemu-time`

Expected: host build 编译通过；QEMU 端输出 `qemu-time: scene modules wired in Task 3` 后退出 0；`cargo test -p qemu-time` 全绿（stub main）。

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml tools/qemu-time/
git commit -m "feat(tools): scaffold qemu-time crate with SysTick timer and bare-metal runtime"
```

---

### Task 3: 共享场景 + QEMU 报告（只打印）

**Files:**
- Create: `tools/qemu-time/src/scenes.rs`
- Modify: `tools/qemu-time/src/main.rs`（`mod scenes;` + 报告函数 + 测量调用）

**Interfaces:**
- Produces（scenes.rs，Task 4 host bench 和 Task 5 依赖）：
  - `pub enum Tier { Minimal, Small, Medium, Large }`（`#[derive(Clone, Copy, Debug)]`）
  - `pub struct RenderScene { pub ui: Ui, pub leaf: ObjRef }`
  - `pub fn build_render_scene(tier: Tier) -> RenderScene`
  - `pub fn build_layout_scene(children: usize) -> Ui`
  - `pub fn time_layout(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64`
  - `pub fn time_render_full(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64`
  - `pub fn time_render_partial(ui: &mut Ui, leaf: ObjRef, now: &mut dyn FnMut() -> u64) -> u64`
  - `pub fn time_frame(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64`
  - `pub struct PrimResults { pub fill_rect: u64, pub draw_line: u64, pub draw_line_many: u64, pub draw_circle: u64, pub fill_circle: u64, pub fill_rounded: u64, pub draw_border: u64, pub draw_arc: u64, pub draw_text: u64, pub blit565: u64 }`（全部 u64 = 单次平均 ticks/ns）
  - `pub fn run_primitives(now: &mut dyn FnMut() -> u64) -> PrimResults`
  - `pub const PRIM_ITERS: u32`（= 50，供 host 端报告时按需引用）
  - `pub fn scene_label(scene: &RenderScene) -> (usize, usize)`（nodes, width×height，供报告头）

- [ ] **Step 1: 写 `src/scenes.rs`（完整代码）**

```rust
//! Shared benchmark scenes + measurement helpers. Single source of truth for
//! the host bench (`qingui/benches/time.rs` includes this via `#[path]`) and
//! the QEMU tool (`tools/qemu-time` declares `mod scenes;`).
//!
//! no_std + alloc only; the clock is injected as `now: &mut dyn FnMut() -> u64`
//! (host: ns since a base Instant; QEMU: `timer::elapsed()` SysTick ticks).
//! Scene building follows `tools/qemu-mem/src/scenes.rs` so memory + runtime
//! numbers stay comparable.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use qingui::geometry::{Color, Point, Rect};
use qingui::prelude::*;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::chart::ChartCfg;
use qingui::widgets::itemlist::ItemListCfg;
use qingui::widgets::label::LabelCfg;
use qingui::widgets::list::ListCfg;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::{ObjRef, Ui};

/// How many times each primitive is drawn inside one `run_primitives` call
/// (the reported value is the per-draw average).
pub const PRIM_ITERS: u32 = 50;

#[derive(Clone, Copy, Debug)]
pub enum Tier {
    Minimal,
    Small,
    Medium,
    Large,
}

/// A built render scene plus a leaf widget handle (for partial-dirty timing).
pub struct RenderScene {
    pub ui: Ui,
    pub leaf: ObjRef,
}

/// Scene for layout timing: a 320x240 flex container with `children` leaf
/// widgets. `layout()` is idempotent — running a pass repeatedly is
/// representative of a real relayout.
pub fn build_layout_scene(children: usize) -> Ui {
    use qingui::layout::{Align, Flex, FlexDir};
    use qingui::style::Layout;

    let mut ui = Ui::new(320, 240, 24);
    let scr = ui.screen();
    let container = ObjCfg::new()
        .size(320, 240)
        .layout(Layout::Flex(Flex {
            dir: FlexDir::Column,
            wrap: false,
            main: Align::Start,
            cross: Align::Start,
            track: Align::Start,
            gap: 8,
        }))
        .build(&mut ui, scr);
    for i in 0..children {
        LabelCfg::new(&format!("item{i}")).size(60, 20).build(&mut ui, container);
    }
    ui.layout();
    ui
}

/// Render scene per tier (mirrors `tools/qemu-mem/src/scenes.rs`).
pub fn build_render_scene(tier: Tier) -> RenderScene {
    let (n_items, n_chart_pts) = match tier {
        Tier::Minimal => {
            let mut ui = Ui::new(160, 120, 8);
            let scr = ui.screen();
            LabelCfg::new("hello").build(&mut ui, scr);
            let leaf = ButtonCfg::new("OK").build(&mut ui, scr);
            ui.tick_inc(16);
            ui.timer_handler();
            return RenderScene { ui, leaf };
        }
        Tier::Small => (5, 16),
        Tier::Medium => (20, 64),
        Tier::Large => (60, 256),
    };
    let mut ui = Ui::new(320, 240, 24);
    let scr = ui.screen();
    // ListCfg takes &[&str]; build the label strings first (their allocation
    // is counted, which is representative of real use).
    let texts: Vec<String> = (0..n_items).map(|i| format!("item{i}")).collect();
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let _list = ListCfg::new(&refs).build(&mut ui, scr);
    let mut leaf = None;
    for i in 0..n_items {
        let b = ButtonCfg::new(&format!("btn{i}")).build(&mut ui, scr);
        if leaf.is_none() {
            leaf = Some(b);
        }
    }
    for _ in 0..n_items / 4 {
        SliderCfg::new(0, 100).build(&mut ui, scr);
    }
    let _chart = ChartCfg::new().series(Color::RED, n_chart_pts).build(&mut ui, scr);
    let _il = ItemListCfg::new().build(&mut ui, scr);
    for _ in 0..n_items {
        ui.itemlist_add_item(_il);
    }
    // Force real allocations / layout / animation paths (same as memory bench).
    for _ in 0..5 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    RenderScene { ui, leaf: leaf.unwrap() }
}

/// Counts nodes and returns (nodes, width*height) for report headers.
pub fn scene_label(scene: &RenderScene) -> (usize, usize) {
    let mut n = 0;
    let mut stack = vec![scene.ui.screen()];
    while let Some(o) = stack.pop() {
        n += 1;
        stack.extend(scene.ui.children(o));
    }
    let r = scene.ui.rect(scene.ui.screen());
    (n, (r.w * r.h) as usize)
}

fn now_delta(now: &mut dyn FnMut() -> u64, f: &mut dyn FnMut()) -> u64 {
    let t0 = now();
    f();
    now() - t0
}

/// One full layout pass over the whole tree.
pub fn time_layout(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64 {
    now_delta(now, &mut || ui.layout())
}

/// Render after dirtying the full screen (worst case).
pub fn time_render_full(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64 {
    ui.invalidate_area(ui.rect(ui.screen()));
    now_delta(now, &mut || ui.render())
}

/// Render after dirtying a single leaf widget (typical interaction).
pub fn time_render_partial(ui: &mut Ui, leaf: ObjRef, now: &mut dyn FnMut() -> u64) -> u64 {
    ui.invalidate_obj(leaf);
    now_delta(now, &mut || ui.render())
}

/// End-to-end frame: anims + layout (if dirty) + floating + tick + render.
pub fn time_frame(ui: &mut Ui, now: &mut dyn FnMut() -> u64) -> u64 {
    ui.invalidate_area(ui.rect(ui.screen()));
    now_delta(now, &mut || {
        ui.timer_handler();
    })
}

/// Per-draw average timing for each basic primitive on a full 320x240 buffer.
pub fn run_primitives(now: &mut dyn FnMut() -> u64) -> PrimResults {
    let full = Rect::new(0, 0, 320, 240);
    let mut pixels = vec![Color::BLACK; 320 * 240];
    let mut d = qingui::draw::DrawBuf { pixels: &mut pixels, area: full, stride: 320 };
    let clip = full;
    let iters = PRIM_ITERS;

    fn bench(now: &mut dyn FnMut() -> u64, iters: u32, f: &mut dyn FnMut()) -> u64 {
        let t0 = now();
        for _ in 0..iters {
            f();
        }
        (now() - t0) / iters as u64
    }

    PrimResults {
        fill_rect: bench(now, iters, &mut || d.fill_rect(full, Color::RED, 255, clip)),
        draw_line: bench(now, iters, &mut || {
            d.draw_line(Point::new(0, 0), Point::new(319, 239), 2, Color::WHITE, 255, clip)
        }),
        draw_line_many: bench(now, iters, &mut || {
            for k in 0..10 {
                d.draw_line(
                    Point::new(k * 32, 0),
                    Point::new(k * 32 + 16, 239),
                    1,
                    Color::WHITE,
                    255,
                    clip,
                );
            }
        }),
        draw_circle: bench(now, iters, &mut || {
            d.draw_circle(Point::new(160, 120), 60, 2, Color::WHITE, 255, clip)
        }),
        fill_circle: bench(now, iters, &mut || {
            d.fill_circle(Point::new(160, 120), 40, Color::WHITE, 255, clip)
        }),
        fill_rounded: bench(now, iters, &mut || d.fill_rounded(full, 8, Color::WHITE, 255, clip)),
        draw_border: bench(now, iters, &mut || d.draw_border(full, 4, 8, Color::WHITE, 255, clip)),
        draw_arc: bench(now, iters, &mut || {
            d.draw_arc(Point::new(160, 120), 80, 4, 0, 270, Color::WHITE, 255, clip)
        }),
        draw_text: bench(now, iters, &mut || {
            d.draw_text(Point::new(10, 10), qingui::font::DEFAULT_FONT, "qingui bench", Color::WHITE, clip)
        }),
        blit565: bench(now, iters, &mut || {
            let img = vec![0u8; 32 * 24 * 2];
            d.blit565(10, 10, 32, 24, &img, 255, clip)
        }),
    }
}

pub struct PrimResults {
    pub fill_rect: u64,
    pub draw_line: u64,
    pub draw_line_many: u64,
    pub draw_circle: u64,
    pub fill_circle: u64,
    pub fill_rounded: u64,
    pub draw_border: u64,
    pub draw_arc: u64,
    pub draw_text: u64,
    pub blit565: u64,
}
```

- [ ] **Step 2: QEMU 报告（改 `src/main.rs`）**

在 arm 分支加入场景测量与打印。把 `main()` 替换为：

```rust
#[cfg(target_arch = "arm")]
fn report_layout() {
    hprintln!("== layout (flex column, 40 children, 320x240) ==");
    let mut ui = scenes::build_layout_scene(40);
    let mut now = || timer::elapsed();
    let t = scenes::time_layout(&mut ui, &mut now);
    hprintln!("  layout           {:>10} ticks", t);
}

#[cfg(target_arch = "arm")]
fn report_render() {
    use scenes::Tier;
    for tier in [Tier::Minimal, Tier::Small, Tier::Medium, Tier::Large] {
        let mut sc = scenes::build_render_scene(tier);
        let (nodes, area) = scenes::scene_label(&sc);
        hprintln!("== render ({:?}, {} nodes, {} px) ==", tier, nodes, area);
        let mut now = || timer::elapsed();
        let full = scenes::time_render_full(&mut sc.ui, &mut now);
        let partial = scenes::time_render_partial(&mut sc.ui, sc.leaf, &mut now);
        let frame = scenes::time_frame(&mut sc.ui, &mut now);
        hprintln!("  render full      {:>10} ticks", full);
        hprintln!("  render partial   {:>10} ticks", partial);
        hprintln!("  full frame       {:>10} ticks", frame);
    }
}

#[cfg(target_arch = "arm")]
fn report_primitives() {
    hprintln!("== primitives (DrawBuf 320x240, {} draws each) ==", scenes::PRIM_ITERS);
    let mut now = || timer::elapsed();
    let p = scenes::run_primitives(&mut now);
    hprintln!("  fill_rect        {:>10} ticks", p.fill_rect);
    hprintln!("  draw_line        {:>10} ticks", p.draw_line);
    hprintln!("  draw_line_many   {:>10} ticks", p.draw_line_many);
    hprintln!("  draw_circle      {:>10} ticks", p.draw_circle);
    hprintln!("  fill_circle      {:>10} ticks", p.fill_circle);
    hprintln!("  fill_rounded     {:>10} ticks", p.fill_rounded);
    hprintln!("  draw_border      {:>10} ticks", p.draw_border);
    hprintln!("  draw_arc         {:>10} ticks", p.draw_arc);
    hprintln!("  draw_text        {:>10} ticks", p.draw_text);
    hprintln!("  blit565          {:>10} ticks", p.blit565);
}

#[cfg(target_arch = "arm")]
#[entry]
fn main() -> ! {
    timer::init();
    report_layout();
    report_render();
    report_primitives();
    exit(EXIT_SUCCESS);
    loop {}
}
```

同时在 arm 分支顶部加 `mod scenes;`（在 `mod timer;` 之后）。

- [ ] **Step 3: 运行 QEMU bench 记录基线**

Run: `cargo run -p qemu-time --target thumbv7em-none-eabihf`
Expected: 打印 layout / 4 档 render（full/partial/frame）/ 10 个原语，全部为 `ticks`，退出码 0。

把以下数值**记入本计划的 Task 5 Step 1 使用**（作为基线）：
- layout ticks
- 每档 render full / partial / frame ticks
- 每个原语 ticks

**记录到 `tools/qemu-time/src/main.rs` 顶部注释或临时文件**，例如：
```
// BASELINE 2026-08-07 (icount shift=3):
//   layout medium            = <L>
//   render full  minimal     = <R1>   partial = <R2>   frame = <R3>
//   ... (all tiers)
//   fill_rect = <P1> ... blit565 = <P10>
```

- [ ] **Step 4: 验证 host 侧测试不受影响**

Run: `cargo test -p qemu-time`、`cargo test -p qingui`
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add tools/qemu-time/src/scenes.rs tools/qemu-time/src/main.rs
git commit -m "feat(tools): add shared runtime-bench scenes and QEMU tick report"
```

---

### Task 4: host 端墙钟 bench（只报告）

**Files:**
- Create: `qingui/benches/time.rs`
- Modify: `qingui/Cargo.toml`（加 `[[bench]] time`）

**Interfaces:**
- Consumes: Task 3 的 `scenes` 模块（`#[path]` include）、`Ui::layout()`。
- Produces: `cargo bench -p qingui --bench time`，warmup + 100 次迭代取 min/median（µs），只报告不断言。

- [ ] **Step 1: 写 `benches/time.rs`（完整代码）**

```rust
//! Runtime benchmark (host, wall-clock): layout / render / frame / primitives.
//!
//! Uses the SAME scene + measurement code as the QEMU tool via `#[path]`
//! (single source of truth, see
//! `tools/qemu-time/src/scenes.rs` and the spec
//! `docs/superpowers/specs/2026-08-07-runtime-bench-design.md`).
//!
//! Wall-clock is noisy; we warm up then take N samples and report min/median.
//! No assertions here — regression gates live in the deterministic QEMU tool.

extern crate alloc;

#[path = "../../tools/qemu-time/src/scenes.rs"]
mod scenes;

use std::time::Instant;

const SAMPLES: usize = 100;
const WARMUP: usize = 5;

fn now_from(base: &Instant) -> impl FnMut() -> u64 + '_ {
    move || base.elapsed().as_nanos() as u64
}

/// Runs `measure` SAMPLES times after WARMUP, returns (min_us, median_us).
fn bench<F: FnMut(&mut dyn FnMut() -> u64) -> u64>(mut measure: F) -> (f64, f64) {
    let base = Instant::now();
    let mut now = now_from(&base);
    for _ in 0..WARMUP {
        measure(&mut now);
    }
    let mut v: Vec<u64> = (0..SAMPLES).map(|_| measure(&mut now)).collect();
    v.sort_unstable();
    let min = *v.first().unwrap();
    let median = v[v.len() / 2];
    (min as f64 / 1000.0, median as f64 / 1000.0) // ns -> us
}

fn main() {
    println!("== runtime bench (host wall-clock, {} samples, min/median us) ==", SAMPLES);

    // ---- layout ----
    let (min, med) = bench(|now| {
        let mut ui = scenes::build_layout_scene(40);
        scenes::time_layout(&mut ui, now)
    });
    println!("layout (flex, 40 children)   min {min:>8.1} us  median {med:>8.1} us");

    // ---- render / frame ----
    for tier in [scenes::Tier::Minimal, scenes::Tier::Small, scenes::Tier::Medium, scenes::Tier::Large] {
        let (nodes, area) = {
            let sc = scenes::build_render_scene(tier);
            scenes::scene_label(&sc)
        };
        let (fmin, fmed) = bench(|now| {
            let mut sc = scenes::build_render_scene(tier);
            scenes::time_render_full(&mut sc.ui, now)
        });
        let (pmin, pmed) = bench(|now| {
            let mut sc = scenes::build_render_scene(tier);
            scenes::time_render_partial(&mut sc.ui, sc.leaf, now)
        });
        let (frmin, frmed) = bench(|now| {
            let mut sc = scenes::build_render_scene(tier);
            scenes::time_frame(&mut sc.ui, now)
        });
        println!("render {:?} ({} nodes, {} px)", tier, nodes, area);
        println!("  full     min {fmin:>8.1} us  median {fmed:>8.1} us");
        println!("  partial  min {pmin:>8.1} us  median {pmed:>8.1} us");
        println!("  frame    min {frmin:>8.1} us  median {frmed:>8.1} us");
    }

    // ---- primitives ----
    println!("== primitives (DrawBuf 320x240, {} draws each) ==", scenes::PRIM_ITERS);
    let mut report = |name: &str, f: &mut dyn FnMut(&mut dyn FnMut() -> u64) -> u64| {
        let (min, med) = bench(f);
        println!("  {name:<16} min {min:>8.1} us  median {med:>8.1} us");
    };
    report("fill_rect", &mut |now| scenes::run_primitives(now).fill_rect);
    report("draw_line", &mut |now| scenes::run_primitives(now).draw_line);
    report("draw_line_many", &mut |now| scenes::run_primitives(now).draw_line_many);
    report("draw_circle", &mut |now| scenes::run_primitives(now).draw_circle);
    report("fill_circle", &mut |now| scenes::run_primitives(now).fill_circle);
    report("fill_rounded", &mut |now| scenes::run_primitives(now).fill_rounded);
    report("draw_border", &mut |now| scenes::run_primitives(now).draw_border);
    report("draw_arc", &mut |now| scenes::run_primitives(now).draw_arc);
    report("draw_text", &mut |now| scenes::run_primitives(now).draw_text);
    report("blit565", &mut |now| scenes::run_primitives(now).blit565);
}
```

> 说明：`run_primitives` 内部固定 50 次绘制取平均（PRIM_ITERS），host 端再叠 100 次采样取 min/median，每次采样重建整个 primitive 结果再取一个字段——开销小且逻辑一致。

- [ ] **Step 2: Cargo.toml 声明 bench**

在 `qingui/Cargo.toml` 的 `[[bench]] memory` 之后追加：

```toml
[[bench]]
name = "time"
path = "benches/time.rs"
harness = false
```

- [ ] **Step 3: 运行 host bench 确认报告**

Run: `cargo bench -p qingui --bench time`
Expected: 打印 layout / 4 档 render（full/partial/frame）/ 10 个原语，全为 `us`，退出码 0。数据量合理（min 应显著小于 median，符合 min 为最佳情况预期）。

- [ ] **Step 4: 全量验证**

Run: `cargo test -p qingui`、`cargo test -p qemu-time`
Expected: 全绿（bench 不进 test）。

- [ ] **Step 5: Commit**

```bash
git add qingui/Cargo.toml qingui/benches/time.rs
git commit -m "bench: add host wall-clock runtime bench with shared scenes"
```

---

### Task 5: QEMU 阈值断言（基线 × 2 校准）

**Files:**
- Modify: `tools/qemu-time/src/main.rs`

**Interfaces:**
- Consumes: Task 3 记录的各基线 ticks（layout / 各档 render full·partial·frame / 各原语）。

- [ ] **Step 1: 读取基线并计算阈值**

读取 Task 3 Step 3 记录的每个基线，计算 `× 2 向上取整`（整数 tick），填入下方常量。阈值形式：

| 常量 | 对应 | 值（示例占位，用实测替换） |
|---|---|---|
| `LIMIT_LAYOUT` | layout | `2 * <L>` |
| `LIMIT_RENDER_FULL_MINIMAL` | render full minimal | `2 * <R1>` |
| `LIMIT_RENDER_PARTIAL_MINIMAL` | render partial minimal | `2 * <R2>` |
| `LIMIT_FRAME_MINIMAL` | frame minimal | `2 * <R3>` |
| ...（Small/Medium/Large 同理，共 12 个 render 常量） | | |
| `LIMIT_FILL_RECT` … `LIMIT_BLIT565` | 各原语（10 个） | `2 * <Pk>` |

**阈值集中在一个 `const` 块**，标注校准日期。

- [ ] **Step 2: 加常量与断言**

在 `src/main.rs` arm 分支顶部加常量块（示例结构，数值按 Step 1 替换）：

```rust
// Thresholds calibrated 2026-08-07: QEMU baseline x 2 (see spec
// docs/superpowers/specs/2026-08-07-runtime-bench-design.md).
const LIMIT_LAYOUT: u64 = 0; // <-- 2 * baseline
const LIMIT_RENDER_FULL_MINIMAL: u64 = 0; // <-- etc.
// ... (fill in all 23 limits)
```

在每个报告函数末尾加断言（以 `report_layout` 为例，其余同理）：

```rust
#[cfg(target_arch = "arm")]
fn report_layout() {
    hprintln!("== layout (flex column, 40 children, 320x240) ==");
    let mut ui = scenes::build_layout_scene(40);
    let mut now = || timer::elapsed();
    let t = scenes::time_layout(&mut ui, &mut now);
    hprintln!("  layout           {:>10} ticks", t);
    assert!(t < LIMIT_LAYOUT, "layout {} ticks exceeds {}", t, LIMIT_LAYOUT);
}
```

`report_render` 内部按 tier 断言：

```rust
let (f, p, fr) = (full, partial, frame);
let (fl, pl, frl) = match tier {
    Tier::Minimal => (LIMIT_RENDER_FULL_MINIMAL, LIMIT_RENDER_PARTIAL_MINIMAL, LIMIT_FRAME_MINIMAL),
    Tier::Small => (LIMIT_RENDER_FULL_SMALL, LIMIT_RENDER_PARTIAL_SMALL, LIMIT_FRAME_SMALL),
    Tier::Medium => (LIMIT_RENDER_FULL_MEDIUM, LIMIT_RENDER_PARTIAL_MEDIUM, LIMIT_FRAME_MEDIUM),
    Tier::Large => (LIMIT_RENDER_FULL_LARGE, LIMIT_RENDER_PARTIAL_LARGE, LIMIT_FRAME_LARGE),
};
assert!(f < fl, "render full {tier:?}: {f} exceeds {fl}");
assert!(p < pl, "render partial {tier:?}: {p} exceeds {pl}");
assert!(fr < frl, "frame {tier:?}: {fr} exceeds {frl}");
```

`report_primitives` 末尾按字段断言：

```rust
assert!(p.fill_rect < LIMIT_FILL_RECT, "fill_rect {} exceeds {}", p.fill_rect, LIMIT_FILL_RECT);
assert!(p.draw_line < LIMIT_DRAW_LINE, "draw_line {} exceeds {}", p.draw_line, LIMIT_DRAW_LINE);
// ... (all 10 primitives)
```

- [ ] **Step 3: 运行确认断言全过**

Run: `cargo run -p qemu-time --target thumbv7em-none-eabihf`
Expected: 全表打印 + 0 断言失败，退出码 0。

- [ ] **Step 4: 全量验证**

Run: `cargo test -p qingui`、`cargo test -p qemu-time`、`cargo check -p qingui --all-targets`
Expected: 全绿；`--all-targets` 不含新 warning。

- [ ] **Step 5: Commit**

```bash
git add tools/qemu-time/src/main.rs
git commit -m "bench: enable QEMU threshold asserts calibrated to measured baselines"
```

---

## Self-Review

**Spec 覆盖：**
- host 端零依赖 bench（`[[bench]] time`，harness=false）→ Task 4。
- QEMU 端 `tools/qemu-time` + SysTick `-icount shift=3` → Task 2（timer.rs + config）+ Task 3（报告）。
- 5 项指标：layout（`Ui::layout()`）→ Task 1/3；render 全屏 / 局部 / 完整帧 → Task 3 场景 B；原语 → Task 3 场景 C（`run_primitives`）。
- 3 类场景：layout-heavy（flex 40 子控件）、render-heavy（Tier 四档）、primitive（320×240 单 buffer 单次绘制）→ Task 3。
- 阈值：QEMU 断言（基线 ×2）→ Task 5；host 只报告（warmup + min/median）→ Task 4。
- 唯一库改动 `#[doc(hidden)] pub fn layout()` → Task 1。
- 场景单一来源（`#[path]` 复用 `tools/qemu-mem/tests/alloc_host.rs` 手法）→ Task 4 Step 1。
- 验收：`cargo bench`/QEMU 运行/`cargo test -p qingui`/`cargo test -p qemu-time` 全绿 → 各任务验证步骤。

**占位符扫描：** Task 3/5 的数值是"实测基线 × 2"的确定性程序——由运行输出驱动，非拍脑袋。Task 5 Step 1 的常量示例标 `0` 并用 `<--` 注释标明替换来源；Task 4 中一处 `fill_rect` 的占位调用已在文档内修正说明删除。

**类型一致性：**
- `now: &mut dyn FnMut() -> u64` 在 Task 3 所有 `time_*` / `run_primitives` 中一致；host 传 ns、QEMU 传 ticks。
- `RenderScene { ui, leaf }` / `build_render_scene(tier) -> RenderScene` / `scene_label(&RenderScene) -> (usize, usize)` 三处签名一致（Task 3 定义，Task 3/4 使用）。
- `PrimResults` 10 字段名在 Task 3 定义、Task 3/4/5 引用一致（fill_rect/draw_line/draw_line_many/draw_circle/fill_circle/fill_rounded/draw_border/draw_arc/draw_text/blit565）。
- `Tier` 四变体 Task 3 定义，Task 3/4/5 match 一致。
- `Ui::layout()` Task 1 定义，Task 3 `time_layout` 使用。
- `qingui::draw::DrawBuf` 字段 `pixels`/`area`/`stride` 均 `pub`（draw.rs:86-92），`draw_*` 方法均 `pub`（draw.rs:122/135/155/190/227/232/248/266/305/349），Task 3 直接构造调用无需额外库改动。
- `qingui::font::DEFAULT_FONT`（font.rs:9）供 `draw_text`。
- 场景 B 用 `itemlist_add_item`（`prelude::*` 引入 `UiItemListExt`，itemlist.rs:143），与 memory bench 一致。
