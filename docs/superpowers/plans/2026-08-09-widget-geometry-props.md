# Widget 尺寸/时间属性可配置化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 12 个 widget 写死的尺寸/时间属性提到各自 `Cfg` 中，builder 时链式配置，默认值保持现状。

**Architecture:** 每个 widget 的 `Cfg` 增加普通字段（以现有常量初始化），`WidgetBuilder<Cfg>` 加同名链式 setter；绘制/动画需要的字段存入 State（`pub` 字段），仅在 build 时计算默认尺寸用的字段只留在 Cfg。现有 `pub const` 全部保留为默认值。spec：`docs/superpowers/specs/2026-08-09-widget-geometry-props-design.md`。

**Tech Stack:** Rust（no_std crate，lib 内只能 `alloc::`），cargo test 集成测试在 `qingui/tests/`（可用 std）。

## Global Constraints

- 工作目录命令统一在 `qingui/` 子crate 下执行（`cd qingui && cargo test`）。
- 代码注释、commit message 用英文；commit 遵循 Conventional Commits（`feat:` 等）。
- **只本地提交，绝不 `git push`**；每个 Task 的 commit 步骤执行前需用户确认（AGENTS.md），除非用户已预先批量授权。
- 以下 `pub const` 必须保留且值不变（examples/tests 引用了它们）：`roller::ROW_H=16`、`roller::ROLL_DUR=150`、`list::ROW_H=16`、`list::FX_DUR=200`、`table::CELL_W=60`、`table::CELL_H=16`、`arc::START_DEG=135`、`arc::SWEEP_DEG=270`、`arc::TRACK_W=4`、`scrollview::STEP=20`。
- 不允许修改任何现有测试文件；新测试全部放新文件 `qingui/tests/widget_props.rs`。
- 每个 Task 完成后 `cargo test` 必须全绿，`cargo clippy --all-targets` 不得新增警告。
- 不在本次范围：零件颜色、easing、运行时 setter、spinner 弧形参数。
- State 新增字段一律 `pub`（与现有字段风格一致），集成测试通过 `ui.widget::<XState>(obj)` 读取验证。

---

### Task 1: spinner — `line_width` / `period_ms`

**Files:**
- Modify: `qingui/src/widgets/spinner.rs`
- Test: `qingui/tests/widget_props.rs`（新建）

**Interfaces:**
- Consumes: 现有 `WidgetBuilder`/`WidgetCfg` 模式（`qingui/src/widgets/builder.rs`）。
- Produces: `SpinnerCfg::new().line_width(i32).period_ms(u64)`；`SpinnerState { pub line_width: i32, pub period_ms: u64 }`。

- [ ] **Step 1: Write the failing test**

创建 `qingui/tests/widget_props.rs`：

```rust
use qingui::prelude::*;
use qingui::Ui;
use qingui::widgets::spinner::{SpinnerCfg, SpinnerState};

#[test]
fn spinner_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = SpinnerCfg::new().build(&mut ui, scr);
    let s = ui.widget::<SpinnerState>(a).unwrap();
    assert_eq!((s.line_width, s.period_ms), (3, 1800));
    let b = SpinnerCfg::new().line_width(6).period_ms(1200).build(&mut ui, scr);
    let s = ui.widget::<SpinnerState>(b).unwrap();
    assert_eq!((s.line_width, s.period_ms), (6, 1200));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props`
Expected: FAIL（编译错误：`SpinnerCfg` has no field/method `line_width`）

- [ ] **Step 3: Implement**

`qingui/src/widgets/spinner.rs` 改动：

1. `draw` 改为接收属性，旋转相位由 `period_ms` 推导（默认 1800 时与原 `now/5 % 360` 完全等价）：

```rust
pub(crate) fn draw(line_width: i32, period_ms: u64, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect) {
    let abs = ctx.abs;
    let c = Point { x: abs.x + abs.w / 2, y: abs.y + abs.h / 2 };
    let r = abs.w.min(abs.h) / 2 - 2;
    if r <= 0 {
        return;
    }
    // Continuous rotation start + triangle-wave sweep length (smooth expanding/contracting, no jumps)
    let period = period_ms.max(1);
    let start = ((ctx.now % period) * 360 / period) as i32;
    let phase = (ctx.now / 7) as i32 % 300;
    let tri = if phase < 150 { phase } else { 300 - phase };
    let sweep = 60 + tri;
    d.draw_arc(c, r, line_width, start, start + sweep, Color::rgb(80, 140, 255), ctx.ap(255), clip);
}
```

2. `SpinnerCfg` 从单元结构体改为带字段，并加 setter：

```rust
/// Spinner configuration: arc line width and rotation period.
pub struct SpinnerCfg {
    line_width: i32,
    period_ms: u64,
}

impl SpinnerCfg {
    /// Creates a builder (default 32x32, transparent bg).
    pub fn new() -> WidgetBuilder<SpinnerCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: SpinnerCfg { line_width: 3, period_ms: 1800 } }
    }
}

impl WidgetBuilder<SpinnerCfg> {
    /// Sets the arc line width in pixels (default 3).
    pub fn line_width(mut self, w: i32) -> Self {
        self.cfg.line_width = w;
        self
    }
    /// Sets the rotation period in ms (default 1800).
    pub fn period_ms(mut self, ms: u64) -> Self {
        self.cfg.period_ms = ms;
        self
    }
}
```

3. `SpinnerState` 带字段，`build` 中构造改为：

```rust
/// Spinner state: geometry/timing resolved at build time; the widget only rotates with time.
pub struct SpinnerState {
    pub line_width: i32,
    pub period_ms: u64,
}
```

`build` 内：`alloc::boxed::Box::new(SpinnerState { line_width: self.line_width, period_ms: self.period_ms })`

4. `Widget::draw` 改为：`fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(self.line_width, self.period_ms, ctx, c, clip) }`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "spinner.rs"`（期望 0）

```bash
git add qingui/src/widgets/spinner.rs qingui/tests/widget_props.rs
git commit -m "feat: make spinner line width and rotation period configurable"
```

---

### Task 2: roller — `row_h` / `roll_dur` / `visible_rows`

**Files:**
- Modify: `qingui/src/widgets/roller.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Consumes: Task 1 的测试文件已存在。
- Produces: `RollerCfg::new(items).row_h(i32).roll_dur(u64).visible_rows(usize)`；`RollerState` 新增 `pub row_h: i32, pub roll_dur: u64`。`pub(crate)` 函数签名变为 `sel_f(selected, sel_from, now, dur)`、`fx_active(sel_from, now, dur)`、`draw(items, selected, sel_from, row_h, dur, ctx, d, clip)`、`select(items, selected, sel_from, idx, now, dur)`。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::roller::{RollerCfg, RollerState};`，并追加：

```rust
#[test]
fn roller_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let items = ["a", "b", "c", "d", "e"];
    let a = RollerCfg::new(&items).build(&mut ui, scr);
    assert_eq!(ui.rect(a).h, 3 * qingui::widgets::roller::ROW_H + 8);
    let s = ui.widget::<RollerState>(a).unwrap();
    assert_eq!((s.row_h, s.roll_dur), (16, 150));
    let b = RollerCfg::new(&items).row_h(24).roll_dur(300).visible_rows(5).build(&mut ui, scr);
    assert_eq!(ui.rect(b).h, 5 * 24 + 8);
    let s = ui.widget::<RollerState>(b).unwrap();
    assert_eq!((s.row_h, s.roll_dur), (24, 300));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props roller_props`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/roller.rs` 改动：

1. `sel_f` / `fx_active` 加 `dur: u64` 参数，函数体内 `ROLL_DUR` 替换为 `dur`：

```rust
fn sel_f(selected: usize, sel_from: Option<(f32, u64)>, now: u64, dur: u64) -> f32 {
    match sel_from {
        Some((from, start)) => {
            let t = (now.saturating_sub(start) as f32 / dur as f32).clamp(0.0, 1.0);
            from * (1.0 - t) + selected as f32 * t
        }
        None => selected as f32,
    }
}

pub(crate) fn fx_active(sel_from: Option<(f32, u64)>, now: u64, dur: u64) -> bool {
    sel_from.is_some_and(|(_, s)| now.saturating_sub(s) < dur)
}
```

2. `draw` 签名改为 `pub(crate) fn draw(items: &[String], selected: usize, sel_from: Option<(f32, u64)>, row_h: i32, dur: u64, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`，函数体内：`ROW_H` → `row_h`（高亮 rect、行 y 计算两处），`sel_f(selected, sel_from, ctx.now)` → `sel_f(selected, sel_from, ctx.now, dur)`。

3. `select` 签名改为 `pub(crate) fn select(items: &[String], selected: &mut usize, sel_from: &mut Option<(f32, u64)>, idx: usize, now: u64, dur: u64)`，内部 `sel_f(*selected, *sel_from, now)` → `sel_f(*selected, *sel_from, now, dur)`。

4. `RollerState` 加字段：

```rust
#[derive(Clone)]
pub struct RollerState {
    pub items: Vec<String>,
    pub selected: usize,
    pub sel_from: Option<(f32, u64)>,
    pub row_h: i32,
    pub roll_dur: u64,
}
```

5. `RollerCfg` 加字段 + setter（`visible_rows` 只影响默认高度，不进 State）：

```rust
/// Roller configuration: items, initial selection, and geometry/timing props.
pub struct RollerCfg {
    items: Vec<String>,
    selected: usize,
    row_h: i32,
    roll_dur: u64,
    visible_rows: usize,
}

impl RollerCfg {
    /// Creates a builder with the given items.
    pub fn new(items: &[&str]) -> WidgetBuilder<RollerCfg> {
        WidgetBuilder {
            common: CommonBuilder::default(),
            cfg: RollerCfg {
                items: items.iter().map(|s| (*s).into()).collect(),
                selected: 0,
                row_h: ROW_H,
                roll_dur: ROLL_DUR,
                visible_rows: 3,
            },
        }
    }
}

impl WidgetBuilder<RollerCfg> {
    /// Sets the initially selected index.
    pub fn selected(mut self, idx: usize) -> Self {
        self.cfg.selected = idx;
        self
    }
    /// Sets the row height in pixels (default `ROW_H` = 16).
    pub fn row_h(mut self, h: i32) -> Self {
        self.cfg.row_h = h;
        self
    }
    /// Sets the roll animation duration in ms (default `ROLL_DUR` = 150).
    pub fn roll_dur(mut self, ms: u64) -> Self {
        self.cfg.roll_dur = ms;
        self
    }
    /// Sets the number of visible rows used by the default height (default 3).
    pub fn visible_rows(mut self, n: usize) -> Self {
        self.cfg.visible_rows = n;
        self
    }
}
```

6. `build` 前两行改为：

```rust
let rows = self.items.len().min(self.visible_rows).max(1) as i32;
let (w, h) = common.size.unwrap_or((80, rows * self.row_h + 8));
```

State 构造改为 `RollerState { items: self.items, selected, sel_from: None, row_h: self.row_h, roll_dur: self.roll_dur }`。

7. `Widget` impl 更新三处调用：
   - `draw` → `draw(&self.items, self.selected, self.sel_from, self.row_h, self.roll_dur, ctx, c, clip)`
   - `tick` 内 `fx_active(self.sel_from, now)` → `fx_active(self.sel_from, now, self.roll_dur)`
   - `on_key` 内 `select(&self.items, &mut self.selected, &mut self.sel_from, next as usize, ui.time())` → 末尾追加 `, self.roll_dur`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "roller.rs"`（期望 0）

```bash
git add qingui/src/widgets/roller.rs qingui/tests/widget_props.rs
git commit -m "feat: make roller row height, roll duration and visible rows configurable"
```

---

### Task 3: list — `row_h` / `fx_dur` / `visible_rows`

**Files:**
- Modify: `qingui/src/widgets/list.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Consumes: 无跨任务依赖。
- Produces: `ListCfg::new(items).row_h(i32).fx_dur(u64).visible_rows(usize)`；`ListState` 新增 `pub row_h: i32, pub fx_dur: u64`。`pub(crate)` 函数签名变化：`lerp_t(start, now, dur)`、`ListFx::active(&self, now, dur)`、`ListFx::prune(&mut self, now, dur)`、`draw(items, selected, scroll, fx, row_h, fx_dur, ctx, d, clip)`、`select(items, selected, scroll, fx, idx, vis_h, now, row_h)`、`ensure_visible(selected, item_count, scroll, fx, vis_h, now, row_h)`、`insert(items, fx, idx, text, now, row_h)`、`remove(items, fx, selected, now, row_h)`。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::list::{ListCfg, ListState};`，并追加：

```rust
#[test]
fn list_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let items = ["a", "b", "c", "d", "e", "f", "g"];
    let a = ListCfg::new(&items).build(&mut ui, scr);
    assert_eq!(ui.rect(a).h, 5 * qingui::widgets::list::ROW_H + 2);
    let s = ui.widget::<ListState>(a).unwrap();
    assert_eq!((s.row_h, s.fx_dur), (16, 200));
    let b = ListCfg::new(&items).row_h(24).fx_dur(80).visible_rows(3).build(&mut ui, scr);
    assert_eq!(ui.rect(b).h, 3 * 24 + 2);
    // Row height feeds the insert shift effect offsets
    ui.list_insert(b, 0, "x");
    let s = ui.widget::<ListState>(b).unwrap();
    assert!(s.fx.item_fx.iter().any(|f| f.dy == -24));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props list_props`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/list.rs` 改动（所有替换都是把常量 `ROW_H`/`FX_DUR` 改为参数）：

1. `ListFx::active(&self, now: u64, dur: u64)` 和 `ListFx::prune(&mut self, now: u64, dur: u64)`：签名加 `dur`，内部闭包 `fresh` 中 `FX_DUR` → `dur`。
2. `fn lerp_t(start: u64, now: u64, dur: u64) -> f32`：`FX_DUR` → `dur`。
3. `draw` 签名改为 `pub(crate) fn draw(items: &[String], selected: usize, scroll: i32, fx: &ListFx, row_h: i32, fx_dur: u64, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`：函数体内 `ROW_H` → `row_h`（高亮 rect、`hl` 的 `ROW_H as f32`、行 `ry`/`row`、ghost 行），`lerp_t(start, now)` → `lerp_t(start, now, fx_dur)`（共 4 处调用）。注意 `ctx.resolved.radius.min(ROW_H / 2)` → `ctx.resolved.radius.min(row_h / 2)`。
4. `select` 签名末尾加 `row_h: i32`，内部调用 `ensure_visible(*selected, items.len(), scroll, fx, vis_h, now, row_h)`。
5. `ensure_visible` 签名末尾加 `row_h: i32`，函数体内 `ROW_H` → `row_h`（`vis_rows`、`first`、`scroll` 对齐共 4 处）。
6. `insert` 签名末尾加 `row_h: i32`，`dy: -ROW_H` → `dy: -row_h`。
7. `remove` 签名末尾加 `row_h: i32`，`dy: ROW_H` → `dy: row_h`。
8. `ListState` 加字段 `pub row_h: i32, pub fx_dur: u64`。
9. `ListCfg` 加字段 + setter（仿照 Task 2 roller 的写法，默认值 `ROW_H`/`FX_DUR`/`5`）：

```rust
pub struct ListCfg {
    items: Vec<String>,
    selected: usize,
    row_h: i32,
    fx_dur: u64,
    visible_rows: usize,
}
```

`new()` 初始化 `row_h: ROW_H, fx_dur: FX_DUR, visible_rows: 5`；`impl WidgetBuilder<ListCfg>` 保留 `selected` 并新增 `row_h`/`fx_dur`/`visible_rows` 三个 setter（函数体与 Task 2 roller 的同名 setter 相同）。

10. `build`：`let rows = self.items.len().min(self.visible_rows).max(1) as i32;`、`(120, rows * self.row_h + 2)`；State 构造追加 `row_h: self.row_h, fx_dur: self.fx_dur`。
11. `Widget` impl：
    - `draw` → `draw(&self.items, self.selected, self.scroll, &self.fx, self.row_h, self.fx_dur, ctx, c, clip)`
    - `tick` → `self.fx.active(now, self.fx_dur)`（两处）与 `self.fx.prune(now, self.fx_dur)`
    - `on_key` → `select(&self.items, &mut self.selected, &mut self.scroll, &mut self.fx, idx, vis_h, now, self.row_h)`
12. `UiListExt` 三个方法（都在 `update::<ListState, _>` 闭包内，直接读 `s.row_h`）：
    - `list_select`：`select(&s.items, &mut s.selected, &mut s.scroll, &mut s.fx, idx, vis_h, now, s.row_h)`
    - `list_insert`：`insert(&mut s.items, &mut s.fx, idx, text, now, s.row_h)`
    - `list_remove`：`remove(&mut s.items, &mut s.fx, &mut s.selected, now, s.row_h)` 和 `ensure_visible(s.selected, s.items.len(), &mut s.scroll, &mut s.fx, vis_h, now, s.row_h)`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "list.rs"`（期望 0）

```bash
git add qingui/src/widgets/list.rs qingui/tests/widget_props.rs
git commit -m "feat: make list row height, fx duration and visible rows configurable"
```

---

### Task 4: arc — `track_w` / `start_deg` / `sweep_deg`

**Files:**
- Modify: `qingui/src/widgets/arc.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `ArcCfg::new(min, max).track_w(i32).start_deg(i32).sweep_deg(i32)`；`ArcState` 新增 `pub track_w: i32, pub start_deg: i32, pub sweep_deg: i32`。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::arc::{ArcCfg, ArcState};`，并追加：

```rust
#[test]
fn arc_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ArcCfg::new(0, 100).build(&mut ui, scr);
    let s = ui.widget::<ArcState>(a).unwrap();
    assert_eq!((s.track_w, s.start_deg, s.sweep_deg), (4, 135, 270));
    let b = ArcCfg::new(0, 100).track_w(6).start_deg(0).sweep_deg(180).build(&mut ui, scr);
    let s = ui.widget::<ArcState>(b).unwrap();
    assert_eq!((s.track_w, s.start_deg, s.sweep_deg), (6, 0, 180));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props arc_props`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/arc.rs` 改动：

1. `draw` 签名改为 `pub(crate) fn draw(min: i32, max: i32, value: i32, track_w: i32, start_deg: i32, sweep_deg: i32, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`，函数体内 `TRACK_W` → `track_w`、`START_DEG` → `start_deg`、`SWEEP_DEG` → `sweep_deg`（三个 `pub const` 保留作默认值）。

```rust
    // Background arc (full track)
    d.draw_arc(c, r, track_w, start_deg, start_deg + sweep_deg, Color::rgb(70, 70, 80), ap(255), clip);
    // Indicator arc (turns yellow in edit mode)
    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
    let ind_end = start_deg + (sweep_deg as f32 * frac) as i32;
    if ind_end > start_deg {
        let ic = if ctx.edited { Color::rgb(255, 200, 60) } else { Color::rgb(80, 140, 255) };
        d.draw_arc(c, r, track_w, start_deg, ind_end, ic, ap(255), clip);
    }
```

2. `ArcState` 加字段 `pub track_w: i32, pub start_deg: i32, pub sweep_deg: i32`。
3. `ArcCfg` 加字段 `track_w: i32, start_deg: i32, sweep_deg: i32`，`new()` 初始化 `track_w: TRACK_W, start_deg: START_DEG, sweep_deg: SWEEP_DEG`；`impl WidgetBuilder<ArcCfg>` 保留 `value` 并新增：

```rust
    /// Sets the arc line width in pixels (default `TRACK_W` = 4).
    pub fn track_w(mut self, w: i32) -> Self {
        self.cfg.track_w = w;
        self
    }
    /// Sets the dial start angle in degrees (default `START_DEG` = 135).
    pub fn start_deg(mut self, deg: i32) -> Self {
        self.cfg.start_deg = deg;
        self
    }
    /// Sets the dial sweep range in degrees (default `SWEEP_DEG` = 270).
    pub fn sweep_deg(mut self, deg: i32) -> Self {
        self.cfg.sweep_deg = deg;
        self
    }
```

4. `build` 的 State 构造改为 `ArcState { min: self.min, max: self.max, value: self.value.unwrap_or(self.min), track_w: self.track_w, start_deg: self.start_deg, sweep_deg: self.sweep_deg }`。
5. `Widget::draw` → `draw(self.min, self.max, self.value, self.track_w, self.start_deg, self.sweep_deg, ctx, c, clip)`。

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "arc.rs"`（期望 0）

```bash
git add qingui/src/widgets/arc.rs qingui/tests/widget_props.rs
git commit -m "feat: make arc track width and dial angles configurable"
```

---

### Task 5: slider — `knob_w`

**Files:**
- Modify: `qingui/src/widgets/slider.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `SliderCfg::new(min, max).knob_w(i32)`；`SliderState` 新增 `pub knob_w: i32`；`overflow()` 返回 `self.knob_w / 2`（默认 8 → 4，与现状一致）。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::slider::{SliderCfg, SliderState};`，并追加：

```rust
#[test]
fn slider_knob_w_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = SliderCfg::new(0, 100).build(&mut ui, scr);
    assert_eq!(ui.widget::<SliderState>(a).unwrap().knob_w, 8);
    let b = SliderCfg::new(0, 100).knob_w(14).build(&mut ui, scr);
    assert_eq!(ui.widget::<SliderState>(b).unwrap().knob_w, 14);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props slider_knob_w`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/slider.rs` 改动：

1. `draw` 签名改为 `pub(crate) fn draw(min: i32, max: i32, value: i32, knob_w: i32, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`，knob rect 改为：

```rust
    let kx = abs.x + iw;
    let knob = Rect::new(kx - knob_w / 2, abs.y - 2, knob_w, abs.h + 4);
```

2. `SliderState` 加字段 `pub knob_w: i32`。
3. `SliderCfg` 加字段 `knob_w: i32`，`new()` 初始化 `knob_w: 8`；`impl WidgetBuilder<SliderCfg>` 保留 `value` 并新增：

```rust
    /// Sets the knob width in pixels (default 8).
    pub fn knob_w(mut self, w: i32) -> Self {
        self.cfg.knob_w = w;
        self
    }
```

4. `build` 的 State 构造追加 `knob_w: self.knob_w`。
5. `Widget::draw` → `draw(self.min, self.max, self.value, self.knob_w, ctx, c, clip)`。
6. `overflow` 注释与实现改为：

```rust
    // Slider knob: ±knob_w/2 horizontal, ±2 vertical
    fn overflow(&self) -> i32 { self.knob_w / 2 }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "slider.rs"`（期望 0）

```bash
git add qingui/src/widgets/slider.rs qingui/tests/widget_props.rs
git commit -m "feat: make slider knob width configurable"
```

---

### Task 6: checkbox — `box_size` / `gap`

**Files:**
- Modify: `qingui/src/widgets/checkbox.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `CheckboxCfg::new(text).box_size(i32).gap(i32)`；`CheckboxState` 新增 `pub box_size: i32, pub gap: i32`。私有常量 `BOX: i32 = 12` 保留（作默认值与勾形缩放基准）。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::checkbox::{CheckboxCfg, CheckboxState};`，并追加：

```rust
#[test]
fn checkbox_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = CheckboxCfg::new("ab").build(&mut ui, scr);
    let s = ui.widget::<CheckboxState>(a).unwrap();
    assert_eq!((s.box_size, s.gap), (12, 6));
    let w_default = ui.rect(a).w;
    let b = CheckboxCfg::new("ab").box_size(20).gap(10).build(&mut ui, scr);
    let s = ui.widget::<CheckboxState>(b).unwrap();
    assert_eq!((s.box_size, s.gap), (20, 10));
    // Same text: default width grows by exactly the box/gap delta
    assert_eq!(ui.rect(b).w - w_default, (20 - 12) + (10 - 6));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props checkbox_props`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/checkbox.rs` 改动：

1. `draw` 签名改为 `pub(crate) fn draw(text: &str, checked: bool, box_size: i32, gap: i32, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`，函数体：

```rust
    let by = abs.y + (abs.h - box_size) / 2;
    let brect = Rect::new(abs.x, by, box_size, box_size);
    // Box
    d.draw_border(brect, 1, 2, Color::rgb(150, 150, 160), ap(255), clip);
    if checked {
        // Check mark: two lines, the canonical 12px shape scaled to box_size
        let sc = |v: i32| v * box_size / BOX;
        let p1 = Point { x: abs.x + sc(2), y: by + sc(6) };
        let p2 = Point { x: abs.x + sc(5), y: by + sc(9) };
        let p3 = Point { x: abs.x + sc(10), y: by + sc(3) };
        d.draw_line(p1, p2, 2, Color::rgb(80, 140, 255), ap(255), clip);
        d.draw_line(p2, p3, 2, Color::rgb(80, 140, 255), ap(255), clip);
    }
```

文本 x 坐标 `abs.x + BOX + 6` → `abs.x + box_size + gap`。

2. `CheckboxState` 加字段 `pub box_size: i32, pub gap: i32`。
3. `CheckboxCfg` 加字段 `box_size: i32, gap: i32`，`new()` 初始化 `box_size: BOX, gap: 6`；`impl WidgetBuilder<CheckboxCfg>` 保留 `checked` 并新增：

```rust
    /// Sets the box side length in pixels (default 12).
    pub fn box_size(mut self, v: i32) -> Self {
        self.cfg.box_size = v;
        self
    }
    /// Sets the gap between box and text in pixels (default 6).
    pub fn gap(mut self, v: i32) -> Self {
        self.cfg.gap = v;
        self
    }
```

4. `build` 默认尺寸 `(BOX + 6 + tw, 16)` → `(self.box_size + self.gap + tw, 16)`；State 构造追加 `box_size: self.box_size, gap: self.gap`。
5. `Widget::draw` → `draw(&self.text, self.checked, self.box_size, self.gap, ctx, c, clip)`。

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "checkbox.rs"`（期望 0）

```bash
git add qingui/src/widgets/checkbox.rs qingui/tests/widget_props.rs
git commit -m "feat: make checkbox box size and text gap configurable"
```

---

### Task 7: dropdown — `popup_rows` / `popup_row_h` / `popup_min_w`

**Files:**
- Modify: `qingui/src/widgets/dropdown.rs`
- Modify: `qingui/src/widgets/list.rs`（仅删除失去调用点的 `pub(crate) fn create`）
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Consumes: `super::list::ROW_H` 与 Task 3 的 `ListCfg::row_h`（因此本任务必须在 Task 3 之后执行）。
- Produces: `DropdownCfg::new(items).popup_rows(usize).popup_row_h(i32).popup_min_w(i32)`；`DropdownState` 新增 `pub popup_rows: usize, pub popup_row_h: i32, pub popup_min_w: i32`。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::dropdown::{DropdownCfg, DropdownState};`，并追加：

```rust
#[test]
fn dropdown_popup_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = DropdownCfg::new(&["x", "y"]).build(&mut ui, scr);
    let s = ui.widget::<DropdownState>(a).unwrap();
    assert_eq!((s.popup_rows, s.popup_row_h, s.popup_min_w), (5, 16, 80));
    let b = DropdownCfg::new(&["x", "y"]).popup_rows(3).popup_row_h(20).popup_min_w(120).build(&mut ui, scr);
    let s = ui.widget::<DropdownState>(b).unwrap();
    assert_eq!((s.popup_rows, s.popup_row_h, s.popup_min_w), (3, 20, 120));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props dropdown_popup_props`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/dropdown.rs` 改动：

1. `DropdownState` 加字段：

```rust
#[derive(Clone)]
pub struct DropdownState {
    pub items: Vec<String>,
    pub selected: usize,
    pub popup_rows: usize,
    pub popup_row_h: i32,
    pub popup_min_w: i32,
}
```

2. `open_popup` 中弹层 list 的创建与尺寸改为（弹层行高跟随 `popup_row_h`，消除 `80`/`5`/`16` 三个写死值；`list::create` 因此失去唯一调用点，按第 4 条删除）：

```rust
        let lst = crate::widgets::list::ListCfg::new(&refs).row_h(self.popup_row_h).build(ui, screen);
        ui.move_to_front(lst); // popups draw on top (children order is the stacking order)
        let popup_h = self.items.len().min(self.popup_rows) as i32 * self.popup_row_h + 2;
        ui.set_size(lst, w.max(self.popup_min_w), popup_h);
```

（即删掉原来的 `let lst = crate::widgets::list::create(ui, screen, &refs);`、`ui.move_to_front(lst);` 和 `ui.set_size(lst, w.max(80), (self.items.len().min(5) * 16 + 2) as i32);` 三行，替换为上面三行。`list::create` 因此失去唯一调用点，按第 5 条删除。）

3. `DropdownCfg` 加字段 `popup_rows: usize, popup_row_h: i32, popup_min_w: i32`，`new()` 初始化 `popup_rows: 5, popup_row_h: super::list::ROW_H, popup_min_w: 80`；`impl WidgetBuilder<DropdownCfg>` 保留 `selected` 并新增：

```rust
    /// Sets the popup's maximum visible rows (default 5).
    pub fn popup_rows(mut self, n: usize) -> Self {
        self.cfg.popup_rows = n;
        self
    }
    /// Sets the popup's row height in pixels (default `list::ROW_H` = 16).
    pub fn popup_row_h(mut self, h: i32) -> Self {
        self.cfg.popup_row_h = h;
        self
    }
    /// Sets the popup's minimum width in pixels (default 80).
    pub fn popup_min_w(mut self, w: i32) -> Self {
        self.cfg.popup_min_w = w;
        self
    }
```

4. `build` 的 State 构造追加 `popup_rows: self.popup_rows, popup_row_h: self.popup_row_h, popup_min_w: self.popup_min_w`。
5. 删除 `qingui/src/widgets/list.rs` 里的 `pub(crate) fn create`（dropdown 是其唯一调用点，改用 `ListCfg` 后它成为死代码；若编译器未报 dead_code 警告则保留）。

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "dropdown.rs"`（期望 0）

```bash
git add qingui/src/widgets/dropdown.rs qingui/src/widgets/list.rs qingui/tests/widget_props.rs
git commit -m "feat: make dropdown popup rows, row height and min width configurable"
```

---

### Task 8: table — `cell_w` / `cell_h`

**Files:**
- Modify: `qingui/src/widgets/table.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `TableCfg::new(cols, rows).cell_w(i32).cell_h(i32)`；`TableState` 新增 `pub cell_w: i32, pub cell_h: i32`。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::table::{TableCfg, TableState};`，并追加：

```rust
#[test]
fn table_cell_props_default_and_override() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let a = TableCfg::new(2, 3).build(&mut ui, scr);
    assert_eq!((ui.rect(a).w, ui.rect(a).h), (2 * qingui::widgets::table::CELL_W, 3 * qingui::widgets::table::CELL_H));
    let s = ui.widget::<TableState>(a).unwrap();
    assert_eq!((s.cell_w, s.cell_h), (60, 16));
    let b = TableCfg::new(2, 3).cell_w(40).cell_h(20).build(&mut ui, scr);
    assert_eq!((ui.rect(b).w, ui.rect(b).h), (80, 60));
    let s = ui.widget::<TableState>(b).unwrap();
    assert_eq!((s.cell_w, s.cell_h), (40, 20));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props table_cell_props`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/table.rs` 改动：

1. `draw` 签名改为 `pub(crate) fn draw(cols: u8, rows: u8, cells: &[String], cell_w: i32, cell_h: i32, ctx: &WidgetCtx, d: &mut Canvas, clip: Rect)`，函数体内 `CELL_W` → `cell_w`、`CELL_H` → `cell_h`（各 2 处，常量保留作默认值）。
2. `TableState` 加字段 `pub cell_w: i32, pub cell_h: i32`。
3. `TableCfg` 加字段 `cell_w: i32, cell_h: i32`，`new()` 初始化 `cell_w: CELL_W, cell_h: CELL_H`；`impl WidgetBuilder<TableCfg>` 保留 `cell` 并新增：

```rust
    /// Sets the cell width in pixels (default `CELL_W` = 60).
    pub fn cell_w(mut self, w: i32) -> Self {
        self.cfg.cell_w = w;
        self
    }
    /// Sets the cell height in pixels (default `CELL_H` = 16).
    pub fn cell_h(mut self, h: i32) -> Self {
        self.cfg.cell_h = h;
        self
    }
```

4. `build` 默认尺寸 `(self.cols as i32 * CELL_W, self.rows as i32 * CELL_H)` → `(self.cols as i32 * self.cell_w, self.rows as i32 * self.cell_h)`；State 构造追加 `cell_w: self.cell_w, cell_h: self.cell_h`。
5. `Widget::draw` → `draw(self.cols, self.rows, &self.cells, self.cell_w, self.cell_h, ctx, c, clip)`。

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "table.rs"`（期望 0）

```bash
git add qingui/src/widgets/table.rs qingui/tests/widget_props.rs
git commit -m "feat: make table cell size configurable"
```

---

### Task 9: scrollview — `step`

**Files:**
- Modify: `qingui/src/widgets/scrollview.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `ScrollViewCfg::new().step(i32)`；`ScrollViewState` 新增 `pub step: i32`。`pub const STEP: i32 = 20` 保留作默认值（`tests/scrollview.rs` 引用了它）。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加：

```rust
use qingui::input::Key;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::scrollview::{ScrollViewCfg, ScrollViewState};
```

并追加：

```rust
#[test]
fn scrollview_step_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let sv = ScrollViewCfg::new().size(60, 60).step(8).build(&mut ui, scr);
    assert_eq!(ui.widget::<ScrollViewState>(sv).unwrap().step, 8);
    let content = ui.scrollview_content(sv).unwrap();
    // Content taller than the viewport so there is room to scroll
    let _tall = ObjCfg::new().size(60, 200).build(&mut ui, content);
    ui.group_add(sv);
    ui.group_focus(sv);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.translate(content).y, -8);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props scrollview_step`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/scrollview.rs` 改动：

1. `ScrollViewCfg` 从单元结构体改为带字段：

```rust
/// ScrollView configuration: scroll step per key press.
pub struct ScrollViewCfg {
    step: i32,
}

impl ScrollViewCfg {
    /// Creates an empty builder.
    pub fn new() -> WidgetBuilder<ScrollViewCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ScrollViewCfg { step: STEP } }
    }
}

impl WidgetBuilder<ScrollViewCfg> {
    /// Sets the scroll step per key press in pixels (default `STEP` = 20).
    pub fn step(mut self, v: i32) -> Self {
        self.cfg.step = v;
        self
    }
}
```

2. `ScrollViewState` 加字段 `pub step: i32`。
3. `on_key` 中 `self.scroll + STEP` → `self.scroll + self.step`，`self.scroll - STEP` → `self.scroll - self.step`。
4. `build` 中 `ScrollViewState { content, scroll: 0 }` → `ScrollViewState { content, scroll: 0, step: self.step }`。

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "scrollview.rs"`（期望 0）

```bash
git add qingui/src/widgets/scrollview.rs qingui/tests/widget_props.rs
git commit -m "feat: make scrollview scroll step configurable"
```

---

### Task 10: chart — `line_width`

**Files:**
- Modify: `qingui/src/widgets/chart.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `ChartCfg::new().line_width(i32)`；`ChartState` 新增 `pub line_width: i32`。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::chart::{ChartCfg, ChartState};`，并追加：

```rust
#[test]
fn chart_line_width_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ChartCfg::new().build(&mut ui, scr);
    assert_eq!(ui.widget::<ChartState>(a).unwrap().line_width, 2);
    let b = ChartCfg::new().line_width(4).build(&mut ui, scr);
    assert_eq!(ui.widget::<ChartState>(b).unwrap().line_width, 4);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props chart_line_width`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/chart.rs` 改动：

1. `ChartState` 加字段 `pub line_width: i32`。
2. `draw` 中 `d.draw_line(q, p, 2, ser.color, ctx.ap(255), sc)` 的 `2` → `s.line_width`（单点半径 `fill_circle(p, 1, ...)` 保持不变）。
3. `ChartCfg` 加字段 `line_width: i32`，`new()` 初始化 `line_width: 2`；`impl WidgetBuilder<ChartCfg>` 新增：

```rust
    /// Sets the data line width in pixels (default 2).
    pub fn line_width(mut self, w: i32) -> Self {
        self.cfg.line_width = w;
        self
    }
```

4. `build` 的 `ChartState` 构造追加 `line_width: self.line_width`。

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "chart.rs"`（期望 0）

```bash
git add qingui/src/widgets/chart.rs qingui/tests/widget_props.rs
git commit -m "feat: make chart line width configurable"
```

---

### Task 11: button — `content_pad`

**Files:**
- Modify: `qingui/src/widgets/button.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `ButtonCfg::new(text).content_pad(x: i32, y: i32)`。只影响默认尺寸计算（显式 `.size()` 时无效），不进 State。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::button::ButtonCfg;`，并追加：

```rust
#[test]
fn button_content_pad_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonCfg::new("Go").build(&mut ui, scr);
    let b = ButtonCfg::new("Go").content_pad(40, 20).build(&mut ui, scr);
    let (ra, rb) = (ui.rect(a), ui.rect(b));
    // Same text: the size delta equals the content_pad delta from the default (24, 12)
    assert_eq!((rb.w - ra.w, rb.h - ra.h), (40 - 24, 20 - 12));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props button_content_pad`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/button.rs` 改动：

1. `ButtonCfg` 加字段：

```rust
/// Button configuration: label text and the default content padding.
pub struct ButtonCfg {
    text: alloc::string::String,
    content_pad: (i32, i32),
}

impl ButtonCfg {
    /// Creates a builder with the given label text.
    pub fn new(text: &str) -> WidgetBuilder<ButtonCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ButtonCfg { text: text.into(), content_pad: (24, 12) } }
    }
}

impl WidgetBuilder<ButtonCfg> {
    /// Sets the padding added to the text size for the default widget size (default (24, 12)).
    pub fn content_pad(mut self, x: i32, y: i32) -> Self {
        self.cfg.content_pad = (x, y);
        self
    }
}
```

2. `build` 默认尺寸 `(tw + 24, th + 12)` → `(tw + self.content_pad.0, th + self.content_pad.1)`。

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "button.rs"`（期望 0）

```bash
git add qingui/src/widgets/button.rs qingui/tests/widget_props.rs
git commit -m "feat: make button default content padding configurable"
```

---

### Task 12: msgbox — `size`

**Files:**
- Modify: `qingui/src/widgets/msgbox.rs`
- Test: `qingui/tests/widget_props.rs`（追加）

**Interfaces:**
- Produces: `MsgboxBuilder::new(title, text).size(w: i32, h: i32)`；`pub(crate) fn create(ui, parent, title, text, buttons, size: Option<(i32, i32)>)`。

- [ ] **Step 1: Write the failing test**

`qingui/tests/widget_props.rs` 顶部追加 `use qingui::widgets::msgbox::MsgboxBuilder;`，并追加：

```rust
#[test]
fn msgbox_size_override() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let a = MsgboxBuilder::new("t", "m").buttons(&["OK"]).build(&mut ui, scr);
    assert_eq!((ui.rect(a).w, ui.rect(a).h), (200, 110));
    ui.clear_modal();
    ui.delete(a);
    let b = MsgboxBuilder::new("t", "m").buttons(&["OK"]).size(240, 140).build(&mut ui, scr);
    assert_eq!((ui.rect(b).w, ui.rect(b).h), (240, 140));
    ui.clear_modal();
    ui.delete(b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd qingui && cargo test --test widget_props msgbox_size`
Expected: FAIL（编译错误）

- [ ] **Step 3: Implement**

`qingui/src/widgets/msgbox.rs` 改动：

1. `MsgboxBuilder` 加字段 `size: Option<(i32, i32)>`，`new()` 初始化 `size: None`，并新增 setter：

```rust
    /// Sets the box size in pixels (default 200x110).
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }
```

2. `build` 改为 `create(ui, parent, &self.title, &self.text, &refs, self.size)`。
3. `create` 签名改为 `pub(crate) fn create(ui: &mut Ui, parent: ObjRef, title: &str, text: &str, buttons: &[&str], size: Option<(i32, i32)>) -> ObjRef`，首行改为：

```rust
    let (w, h) = size.unwrap_or((200, 110));
    let root = ui.insert_node(parent, Rect::new(0, 0, w, h), alloc::boxed::Box::new(MsgboxState { selected: -1 }));
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd qingui && cargo test --test widget_props`
Expected: PASS

- [ ] **Step 5: Full suite + commit**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | grep -c "msgbox.rs"`（期望 0）

```bash
git add qingui/src/widgets/msgbox.rs qingui/tests/widget_props.rs
git commit -m "feat: make msgbox size configurable"
```

---

### Task 13: 收尾 — README/AGENTS 一致性检查

**Files:**
- Modify（如需要）: `qingui/README.md`

**Interfaces:**
- Consumes: Task 1-12 的全部 setter。

- [ ] **Step 1: 检查 README 是否宣传了 builder 能力清单**

Run: `cd qingui && grep -n "ROW_H\|FX_DUR\|line_width\|row_h\|roll_dur" README.md`
若 README 的 builder 介绍（"默认尺寸/样式可链式覆盖"附近）需要提及新能力，在对应段落补一句（英文）；若无相关内容则不改。AGENTS.md 无涉及 widget 属性的约定，不需要改。

- [ ] **Step 2: 最终全量验证**

Run: `cd qingui && cargo test && cargo clippy --all-targets 2>&1 | tail -3`
Expected: 全部测试 PASS，clippy 无新增警告（canvas.rs 的既有 precedence 警告除外）。

- [ ] **Step 3: Commit（仅当 README 有改动时）**

```bash
git add qingui/README.md
git commit -m "docs: mention configurable widget geometry/timing props"
```
