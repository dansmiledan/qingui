# qingui 架构重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 qingui 从"Ui 中央解释器 + 内联 enum 状态"重构为"ObjRef 句柄 API + 每 widget 状态 struct + 行为归一（tick/draw/on_key 委托）+ Custom 逃生舱"。

**Architecture:** 详见 `docs/superpowers/specs/2026-07-31-qingui-arch-redesign-design.md`。核心：`WidgetKind` enum 持有各 widget 的状态 struct；行为（draw/tick/on_key/value）收敛为 `WidgetKind` 委托方法，各 widget 文件拥有自己的 `XxxState` impl；Node 增加 `draw_hook`/`tick_hook` 通用钩子取代 Canvas 特例；对外 API 从 `ui.set_xxx(obj, ...)` 迁移为 `obj.set_xxx(&mut ui, ...)`。

**Tech Stack:** Rust, `#![no_std]` + `alloc`，仅依赖 `bitflags`/`font8x8`；测试为 host 端集成测试（`qingui/tests/`，RecFlush 像素断言）。

## Global Constraints

- **no_std**：`qingui/src/` 内禁止 `std`，只用 `core`/`alloc`；不新增任何依赖。
- **注释用中文**，风格与现有代码一致（简短、陈述事实、必要时对齐 LVGL 术语）。
- **Git 规则（AGENTS.md）**：每个 commit 步骤执行前，先向用户展示待提交内容并获得确认；**永不 push**。
- 所有命令在 workspace 根目录 `/Users/yintan/Documents/workspace/project/rust-lvgl` 执行；测试命令统一为 `cargo test -p qingui`。
- 每个 Task 结束时 `cargo test -p qingui` 必须全绿才可 commit。
- 用户已批准破坏性 API 变更；但本计划的 Task 顺序保证**每个 commit 都编译且测试通过**（先加后删：Task 7 加新 API，Task 8 迁移，Task 9 删旧 API）。
- `WidgetKind` 保持 `#[derive(Clone)]` 直到 Task 6 引入 `Custom` 变体时移除（届时 draw 路径的 `kind.clone()` 已在 Task 5 消除）。

## 关键机制约定（后续各 Task 引用）

**take-调用-放回**：当回调/行为需要 `&mut Ui` 而数据又在 `Ui.arena` 内时，先把数据从节点取出（`mem::replace`/`Option::take`），调用，再放回（节点可能已被回调删除，放回前检查 `is_valid`）。现有 `send_event`（`qingui/src/ui.rs:954-988`）已是此模式。

**KeyOutcome 模式**：内置 widget 的 `on_key` 不接收 `&mut Ui`（避免 kind 拆出期间 Ui 操作打到占位 kind），只改自身状态并返回 `KeyOutcome`，由 `Ui::apply_key_outcome` 执行通用副作用（标脏/发事件/EDITED 态/开下拉）。`Custom` widget 例外：用户状态在拆出的 `Box<dyn Widget>` 里，其 `on_key` 可安全接收 `&mut Ui`（约定：改自身状态用 `self`，不要对自身的 kind 做 Ui 级操作）。

---

### Task 1: WidgetKind 状态 struct 化

纯内部重构：enum 变体从内联字段改为持有各 widget 文件定义的 `XxxState`。公开 API 与行为完全不变。

**Files:**
- Modify: `qingui/src/widgets/mod.rs:25-45`（enum 定义）
- Modify: `qingui/src/widgets/*.rs`（全部 16 个 widget 文件：各加 `XxxState` + 构造函数更新）
- Modify: `qingui/src/ui.rs`（所有对 kind 变体的 match 解构点）
- Modify: `qingui/tests/list_fx.rs:4-9,17,46,93,100,116`、`qingui/tests/p1_widgets.rs:127`、`qingui/tests/list_nav.rs:51`（匹配语法更新）

**Interfaces:**
- Consumes: 现有 `WidgetKind` 内联变体。
- Produces: 各 widget 文件的 `pub struct XxxState`（字段全 `pub`）；新 `WidgetKind` 变体签名（后续所有 Task 依赖）：

```rust
// qingui/src/widgets/mod.rs
#[derive(Clone)]
pub enum WidgetKind {
    Obj,
    Label(label::LabelState),
    Button(button::ButtonState),
    Slider(slider::SliderState),
    Switch(switch::SwitchState),
    Bar(bar::BarState),
    List(list::ListState),
    /// 自定义绘制控件：cb 为 Ui 回调注册表中的索引（Task 5 删除）
    Canvas { cb: usize },
    Arc(arc::ArcState),
    Checkbox(checkbox::CheckboxState),
    Spinner,
    Msgbox(msgbox::MsgboxState),
    Led(led::LedState),
    Table(table::TableState),
    Spinbox(spinbox::SpinboxState),
    Roller(roller::RollerState),
    Dropdown(dropdown::DropdownState),
}
```

各状态 struct（放在各自 widget 文件顶部，均 `#[derive(Clone)]`，字段全 `pub`）：

```rust
// label.rs        pub struct LabelState { pub text: String }
// button.rs       pub struct ButtonState { pub text: String }
// slider.rs       pub struct SliderState { pub min: i32, pub max: i32, pub value: i32 }
// switch.rs       pub struct SwitchState { pub on: bool }
// bar.rs          pub struct BarState { pub min: i32, pub max: i32, pub value: i32 }
// list.rs         pub struct ListState { pub items: Vec<String>, pub selected: usize, pub scroll: i32, pub fx: ListFx }
// arc.rs          pub struct ArcState { pub min: i32, pub max: i32, pub value: i32 }
// checkbox.rs     pub struct CheckboxState { pub text: String, pub checked: bool }
// msgbox.rs       pub struct MsgboxState { pub selected: i32 }
// led.rs          pub struct LedState { pub color: Color, pub bright: u8 }
// table.rs        pub struct TableState { pub cols: u8, pub rows: u8, pub cells: Vec<String> }
// spinbox.rs      pub struct SpinboxState { pub min: i32, pub max: i32, pub value: i32, pub digits: u8, pub cursor: u8 }
// roller.rs       pub struct RollerState { pub items: Vec<String>, pub selected: usize, pub sel_from: Option<(f32, u64)> }
// dropdown.rs     pub struct DropdownState { pub items: Vec<String>, pub selected: usize }
```

- [ ] **Step 1: 基线**

Run: `cargo test -p qingui`
Expected: 全部通过（记录通过数，后续每 Task 对照）。

- [ ] **Step 2: 在各 widget 文件定义状态 struct**

按上方清单，在每个 `qingui/src/widgets/xx.rs` 顶部（`use` 之后）添加对应的 `XxxState` 定义。`led.rs` 的 `Color` 来自 `crate::geometry::Color`。

- [ ] **Step 3: 替换 WidgetKind 定义**

用上方"Produces"中的新 enum 替换 `qingui/src/widgets/mod.rs:25-45`。`Canvas { cb: usize }` 与 `Obj`、`Spinner` 保持原样。

- [ ] **Step 4: 更新 mod.rs 内 5 个自由函数的解构**

`draw/overflow_of/value_of/set_value_of/set_range_of`（`mod.rs:64-158`）的 match 臂改为从新变体解构。注意：**原来合并的 `Slider|Bar|Arc` 臂必须拆开**（三种 struct 是不同类型，不能共享绑定）。示例：

```rust
// value_of 中：
WidgetKind::Slider(s) => s.value,
WidgetKind::Bar(s) => s.value,
WidgetKind::Arc(s) => s.value,
WidgetKind::Switch(s) => s.on as i32,
WidgetKind::Checkbox(s) => s.checked as i32,
WidgetKind::Spinbox(s) => s.value,
WidgetKind::Led(s) => s.bright as i32,
WidgetKind::Roller(s) => s.selected as i32,
WidgetKind::Dropdown(s) => s.selected as i32,
_ => 0,
```

```rust
// draw 中（两个例子，其余类推）：
WidgetKind::Label(s) => label::draw(&s.text, ctx, d, clip),
WidgetKind::List(s) => list::draw(&s.items, s.selected, s.scroll, &s.fx, ctx, d, clip),
```

`set_value_of` 原逻辑逐臂保留（注意：现状 `Switch` 不在 set_value_of 里，落入 `_ => false`，**保持这个行为**）。`set_range_of` 拆成 Slider/Bar 两臂。

- [ ] **Step 5: 更新各 widget 文件的构造点**

所有 `WidgetKind::Xxx { ... }` 构造改为 `WidgetKind::Xxx(XxxState { ... })`。位置：各 Builder 的 `build` 及 `msgbox::create`。例（`list.rs:284-288`）：

```rust
let r = ui.insert_node(
    parent,
    Rect::new(0, 0, w, h),
    WidgetKind::List(ListState { items: self.items, selected, scroll: 0, fx: ListFx::default() }),
);
```

- [ ] **Step 6: 更新 ui.rs 的所有 kind 解构点**

逐一编译驱动修复（`cargo check -p qingui 2>&1 | grep WidgetKind`）：`msgbox_selected`（800）、`table_set_cell`（817）、`list_select/list_insert/list_remove/list_len/list_selected`（879-938, 1222）、`roller_selected`（837）、`keypad_input` 的 is_spinbox/is_list/is_roller 与 spinbox 编辑块（1127-1174, 1183-1184）、`activate`（1236-1254）、`roller_step`（1256-1266）、`open_dropdown`（1269-1312）、`toggle_checkbox/toggle_switch`（1314-1334）、`tick_list_fx`（309-349）。`matches!(..., Some(WidgetKind::Spinbox { .. }))` 改为 `matches!(..., Some(WidgetKind::Spinbox(_)))`；解构 `WidgetKind::List { items, selected, .. }` 改为 `WidgetKind::List(s)` 后用 `s.items` 等。

- [ ] **Step 7: 更新 3 个测试文件的匹配语法**

`tests/list_fx.rs` 的 helper 改为：

```rust
fn list_fx(ui: &Ui, l: qingui::ObjRef) -> qingui::widgets::list::ListFx {
    match ui.debug_kind(l) {
        WidgetKind::List(s) => s.fx.clone(),
        _ => panic!("not a list"),
    }
}
```

文件内其余 `WidgetKind::List { items, fx, .. }` 匹配同理改为 `WidgetKind::List(s)` + `s.items`/`s.fx`。`tests/p1_widgets.rs:127`、`tests/list_nav.rs:51` 同样处理。

- [ ] **Step 8: 验证**

Run: `cargo test -p qingui`
Expected: 全绿，通过数与 Step 1 基线一致。

- [ ] **Step 9: Commit（先获用户确认）**

```bash
git add qingui/src qingui/tests
git commit -m "refactor: WidgetKind variants hold per-widget state structs"
```

---

### Task 2: WidgetKind 委托方法（draw/value/set_value/range/overflow + as_xxx 访问器）

把 `mod.rs` 的 5 个自由函数变为 `impl WidgetKind` 方法，并新增 `as_xxx`/`as_xxx_mut` 访问器；`ui.rs` 调用点全部改写，ui.rs 中的变体 match 替换为访问器。

**Files:**
- Modify: `qingui/src/widgets/mod.rs`（删 5 个自由函数，加 impl 块）
- Modify: `qingui/src/ui.rs`（调用点改写）

**Interfaces:**
- Consumes: Task 1 的新 `WidgetKind`。
- Produces（Task 3/4/5/6/7 依赖这些方法名）：
  - `WidgetKind::draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect)`
  - `WidgetKind::overflow(&self) -> i32`
  - `WidgetKind::value(&self) -> i32`
  - `WidgetKind::set_value(&mut self, v: i32) -> bool`
  - `WidgetKind::set_range(&mut self, min: i32, max: i32)`
  - `WidgetKind::as_xxx(&self) -> Option<&XxxState>` / `as_xxx_mut(&mut self) -> Option<&mut XxxState>`，覆盖：list, roller, dropdown, table, checkbox, switch, msgbox, spinbox, label, button, led, slider, bar, arc（均 `pub`）

- [ ] **Step 1: 5 个自由函数改为方法**

把 `draw/overflow_of/value_of/set_value_of/set_range_of` 的函数体移入 `impl WidgetKind`，签名按"Produces"。`set_value` 内可提一个局部闭包减少重复：

```rust
pub(crate) fn set_value(&mut self, v: i32) -> bool {
    fn clamp_val(min: i32, max: i32, value: &mut i32, v: i32) -> bool {
        let nv = v.clamp(min, max);
        let changed = nv != *value;
        *value = nv;
        changed
    }
    fn select_clamp(len: usize, selected: &mut usize, v: i32) -> bool {
        if len == 0 { return false; }
        let nv = (v.max(0) as usize).min(len - 1);
        let changed = nv != *selected;
        *selected = nv;
        changed
    }
    match self {
        WidgetKind::Slider(s) => clamp_val(s.min, s.max, &mut s.value, v),
        WidgetKind::Bar(s) => clamp_val(s.min, s.max, &mut s.value, v),
        WidgetKind::Arc(s) => clamp_val(s.min, s.max, &mut s.value, v),
        WidgetKind::Spinbox(s) => clamp_val(s.min, s.max, &mut s.value, v),
        WidgetKind::Checkbox(s) => {
            let nv = v != 0;
            let c = nv != s.checked;
            s.checked = nv;
            c
        }
        WidgetKind::Led(s) => {
            let nv = v.clamp(0, 255) as u8;
            let c = nv != s.bright;
            s.bright = nv;
            c
        }
        WidgetKind::Roller(s) => select_clamp(s.items.len(), &mut s.selected, v),
        WidgetKind::Dropdown(s) => select_clamp(s.items.len(), &mut s.selected, v),
        _ => false,
    }
}
```

注意 `draw`/`overflow`/`value`/`set_range` 同理逐臂搬运，逻辑一字不改。

- [ ] **Step 2: 新增 as_xxx 访问器**

在 `impl WidgetKind` 中添加。模式（写全 14 组，不要宏）：

```rust
pub fn as_list(&self) -> Option<&list::ListState> {
    match self { WidgetKind::List(s) => Some(s), _ => None }
}
pub fn as_list_mut(&mut self) -> Option<&mut list::ListState> {
    match self { WidgetKind::List(s) => Some(s), _ => None }
}
```

对照表（xxx → 类型 → 变体）：list→`list::ListState`→`List`，roller→`roller::RollerState`→`Roller`，dropdown→`dropdown::DropdownState`→`Dropdown`，table→`table::TableState`→`Table`，checkbox→`checkbox::CheckboxState`→`Checkbox`，switch→`switch::SwitchState`→`Switch`，msgbox→`msgbox::MsgboxState`→`Msgbox`，spinbox→`spinbox::SpinboxState`→`Spinbox`，label→`label::LabelState`→`Label`，button→`button::ButtonState`→`Button`，led→`led::LedState`→`Led`，slider→`slider::SliderState`→`Slider`，bar→`bar::BarState`→`Bar`，arc→`arc::ArcState`→`Arc`。

- [ ] **Step 3: ui.rs 调用点改写**

- `crate::widgets::overflow_of(&n.kind)` → `n.kind.overflow()`（`ui.rs:153, 202`）
- `crate::widgets::draw(&kind_snap, &ctx, &mut d, clip)` → `kind_snap.draw(&ctx, &mut d, clip)`（`ui.rs:720`）
- `crate::widgets::set_value_of(&mut n.kind, v)` → `n.kind.set_value(v)`（`ui.rs:853`）
- `crate::widgets::value_of(&n.kind)` → `n.kind.value()`（`ui.rs:868`）
- `crate::widgets::set_range_of(&mut n.kind, min, max)` → `n.kind.set_range(min, max)`（`ui.rs:874`）
- `msgbox_selected`（800-807）：`if let Some(s) = self.arena.get(obj).and_then(|n| n.kind.as_msgbox()) { return s.selected; } -1`
- `table_set_cell`（817-827）：kind 匹配改为 `if let Some(s) = n.kind.as_table_mut() { if row < s.rows && col < s.cols { s.cells[row as usize * s.cols as usize + col as usize] = text.into(); } }`
- `roller_selected`（837-844）、`list_selected/list_len`（879-886, 1222-1229）：改用 `as_roller()`/`as_list()`。
- `list_select/list_insert/list_remove`（888-938）：改用 `as_list_mut()`，如：

```rust
pub fn list_select(&mut self, obj: ObjRef, idx: usize) {
    self.invalidate_obj(obj);
    let now = self.time_ms;
    if let Some(n) = self.arena.get_mut(obj) {
        let vis_h = n.rect.h;
        if let Some(s) = n.kind.as_list_mut() {
            crate::widgets::list::select(&s.items, &mut s.selected, &mut s.scroll, &mut s.fx, idx, vis_h, now);
        }
    }
    self.invalidate_obj(obj);
}
```

- `toggle_checkbox/toggle_switch`（1314-1334）：改用 `as_checkbox_mut()`/`as_switch_mut()`。
- `tick_list_fx` 内的变体 match（316-335）：本 Task 不动（Task 3 整体替换）。
- `keypad_input`/`activate`/`roller_step`/`open_dropdown` 内的变体 match：本 Task 不动（Task 4 处理）。

- [ ] **Step 4: 删除 mod.rs 旧自由函数并验证**

Run: `cargo test -p qingui`
Expected: 全绿（无行为变化）。

- [ ] **Step 5: Commit（先获用户确认）**

```bash
git add qingui/src
git commit -m "refactor: WidgetKind delegation methods and as_xxx accessors"
```

---

### Task 3: tick 统一（TickOut + WidgetKind::tick，删除 tick_list_fx）

**Files:**
- Modify: `qingui/src/widgets/mod.rs`（`TickOut`、`WidgetKind::tick`）
- Modify: `qingui/src/widgets/list.rs`（`ListState::tick`）
- Modify: `qingui/src/widgets/roller.rs`（`RollerState::tick`）
- Modify: `qingui/src/ui.rs:294-349`（`tick_widgets` 替换 `tick_list_fx`）
- Test: `qingui/tests/tick.rs`（新建）

**Interfaces:**
- Consumes: `WidgetKind` 委托方法（Task 2）、`ListFx::active/prune`（`list.rs:41-72`）、`roller::fx_active`（`roller.rs:26`）。
- Produces:
  - `pub struct TickOut { pub redraw: bool, pub active: bool }`，常量 `TickOut::IDLE`、`TickOut::ACTIVE`（Task 5/6 依赖，Custom trait 的 tick 返回它）
  - `WidgetKind::tick(&mut self, now: u64) -> TickOut`（Task 5 的 tick_widgets 依赖）
  - `ListState::tick(&mut self, now: u64) -> TickOut`、`RollerState::tick(&mut self, now: u64) -> TickOut`

- [ ] **Step 1: 写失败测试**

新建 `qingui/tests/tick.rs`：

```rust
use qingui::Ui;

#[test]
fn spinner_keeps_timer_awake() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    ui.create_spinner(s);
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // 自转控件保持唤醒
}

#[test]
fn static_ui_sleeps_after_first_frame() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    ui.create_label(s, "hi");
    ui.tick_inc(16);
    ui.timer_handler(); // 首帧（渲染建屏脏区）
    assert_eq!(ui.timer_handler(), u32::MAX); // 无动画无效果 → 睡眠
}

#[test]
fn list_fx_expires_and_sleeps() {
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.screen();
    let l = ui.create_list(s, &["a", "b", "c"]);
    ui.list_select(l, 2); // 触发高亮滑动 fx（FX_DUR=200ms）
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // fx 活动
    ui.tick_inc(300); // 超过 FX_DUR
    ui.timer_handler();
    assert_eq!(ui.timer_handler(), u32::MAX); // fx 已过期 → 睡眠
}
```

Run: `cargo test -p qingui --test tick`
Expected: 编译失败（`tick.rs` 能编译但行为应已通过——spinner/静态用例现状即如此）；若三个用例现状全过，直接保留作为防回归测试并跳到 Step 2（本 Task 是行为保持的重构，测试作用是锁定行为）。

- [ ] **Step 2: 定义 TickOut 与 WidgetKind::tick**

`qingui/src/widgets/mod.rs` 中添加：

```rust
/// 每帧效果推进结果：redraw = 本帧需重绘；active = 效果仍活动（保持唤醒）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TickOut {
    pub redraw: bool,
    pub active: bool,
}

impl TickOut {
    pub const IDLE: Self = Self { redraw: false, active: false };
    pub const ACTIVE: Self = Self { redraw: true, active: true };
}

impl WidgetKind {
    /// 每帧效果推进（fx/自转）。默认无逐帧行为。
    pub(crate) fn tick(&mut self, now: u64) -> TickOut {
        match self {
            WidgetKind::List(s) => s.tick(now),
            WidgetKind::Roller(s) => s.tick(now),
            // Spinner 永远自转
            WidgetKind::Spinner => TickOut::ACTIVE,
            _ => TickOut::IDLE,
        }
    }
}
```

- [ ] **Step 3: ListState::tick / RollerState::tick**

`list.rs` 的 `impl ListState`（新建 impl 块，紧接 struct 定义之后）：

```rust
impl ListState {
    pub(crate) fn tick(&mut self, now: u64) -> super::TickOut {
        let was_active = self.fx.active(now);
        let removed = self.fx.prune(now);
        // 活动中逐帧重绘；清理掉效果的这一帧也补一次重绘（清掉 ghost 残影）
        super::TickOut { redraw: was_active || removed, active: self.fx.active(now) }
    }
}
```

`roller.rs` 的 `impl RollerState`：

```rust
impl RollerState {
    pub(crate) fn tick(&mut self, now: u64) -> super::TickOut {
        let had_fx = self.sel_from.is_some();
        let active = fx_active(self.sel_from, now);
        if !active {
            self.sel_from = None;
        }
        // 有 fx（含本帧过期）就重绘：完成帧必须补最后一定格
        super::TickOut { redraw: had_fx, active }
    }
}
```

- [ ] **Step 4: Ui::tick_widgets 替换 tick_list_fx**

删除 `ui.rs:307-349` 的 `tick_list_fx`，替换为：

```rust
/// 遍历对象树推进每帧效果（fx/Spinner），活动节点标脏。
/// 返回是否仍有活动效果（决定 timer_handler 是否持续唤醒）。
fn tick_widgets(&mut self) -> bool {
    let now = self.time_ms;
    let mut any = false;
    let mut stack = alloc::vec![self.screen];
    while let Some(r) = stack.pop() {
        let (out, children) = match self.arena.get_mut(r) {
            Some(n) => (n.kind.tick(now), n.children.clone()),
            None => continue,
        };
        if out.redraw {
            self.invalidate_obj(r);
        }
        if out.active {
            any = true;
        }
        stack.extend_from_slice(&children);
    }
    any
}
```

`timer_handler`（`ui.rs:294-305`）中 `let list_fx_active = self.tick_list_fx();` 改为 `let fx_active = self.tick_widgets();`，返回值判断变量同步改名。

- [ ] **Step 5: 验证**

Run: `cargo test -p qingui`
Expected: 全绿（含新 `tick.rs` 与既有 `list_fx.rs`/`roller_ghost.rs`）。

- [ ] **Step 6: Commit（先获用户确认）**

```bash
git add qingui/src qingui/tests/tick.rs
git commit -m "refactor: unify per-frame tick via WidgetKind::tick, drop tick_list_fx"
```

---

### Task 4: 按键行为下沉（KeyOutcome + on_key + keypad_input 瘦身）

**Files:**
- Modify: `qingui/src/widgets/mod.rs`（`KeyCtx`/`KeyOutcome`/`WidgetKind::on_key`）
- Modify: `qingui/src/widgets/{slider,spinbox,switch,checkbox,list,roller,dropdown}.rs`（各 `on_key`）
- Modify: `qingui/src/widgets/dropdown.rs`（新增 `pub(crate) fn open`）
- Modify: `qingui/src/ui.rs:1118-1334`（`keypad_input` 重写；删 `activate`/`roller_step`/`open_dropdown`）

**Interfaces:**
- Consumes: `WidgetKind::as_xxx`（Task 2）、`spinbox::move_cursor/step_digit`、`list::select`、`roller::select`。
- Produces:
  - `pub(crate) struct KeyCtx { pub edited: bool, pub vis_h: i32, pub now: u64 }`
  - `pub(crate) enum KeyOutcome { Pass, Consumed, ValueChanged, EnterEdit, ExitEdit, OpenDropdown }`
  - `WidgetKind::on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome`
  - `dropdown::open(ui: &mut Ui, obj: ObjRef)`（Task 9 后 dropdown 的对外打开路径仍走按键）
  - `Ui::call_on_key(&mut self, obj: ObjRef, key: Key) -> bool`（Task 6 将在此加 Custom 分支）

- [ ] **Step 1: 定义 KeyCtx/KeyOutcome/WidgetKind::on_key**

`qingui/src/widgets/mod.rs`：

```rust
use crate::input::Key;

/// 按键处理上下文（由 Ui 从节点/自身状态收集后传入）
pub(crate) struct KeyCtx {
    pub edited: bool, // 节点处于 EDITED 态
    pub vis_h: i32,   // 节点可视高度（滚动控件用）
    pub now: u64,
}

/// 按键处理结果：Ui 据此执行通用副作用（标脏/事件/EDITED 态/开下拉）
pub(crate) enum KeyOutcome {
    Pass,          // 未消费 → 走默认（移焦/Clicked）
    Consumed,      // 已消费，标脏
    ValueChanged,  // 已消费，标脏并发 ValueChanged 事件
    EnterEdit,     // 进入 EDITED 态
    ExitEdit,      // 退出 EDITED 态并标脏
    OpenDropdown,  // 打开下拉浮层
}

impl WidgetKind {
    /// 按键处理（无 &mut Ui：只改自身状态，副作用由 Ui 按 KeyOutcome 执行）
    pub(crate) fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
        match self {
            WidgetKind::Slider(s) => s.on_key(key, ctx),
            WidgetKind::Spinbox(s) => s.on_key(key, ctx),
            WidgetKind::Switch(s) => s.on_key(key, ctx),
            WidgetKind::Checkbox(s) => s.on_key(key, ctx),
            WidgetKind::List(s) => s.on_key(key, ctx),
            WidgetKind::Roller(s) => s.on_key(key, ctx),
            WidgetKind::Dropdown(s) => s.on_key(key, ctx),
            _ => KeyOutcome::Pass,
        }
    }
}
```

- [ ] **Step 2: 各 widget 的 on_key 实现**

`slider.rs`（在 `impl SliderState` 块中添加；Task 3 只为 list/roller 建了 impl 块，其余文件本 Task 新建）：

```rust
pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
    use super::KeyOutcome::*;
    if ctx.edited {
        return match key {
            Key::Left | Key::Right => {
                let d = if key == Key::Left { -1 } else { 1 };
                let nv = (self.value + d).clamp(self.min, self.max);
                if nv != self.value { self.value = nv; ValueChanged } else { Consumed }
            }
            Key::Enter | Key::Esc => ExitEdit,
            _ => Consumed,
        };
    }
    if key == Key::Enter { EnterEdit } else { Pass }
}
```

`spinbox.rs`：

```rust
pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
    use super::KeyOutcome::*;
    if !ctx.edited {
        return if key == Key::Enter { EnterEdit } else { Pass };
    }
    match key {
        Key::Left => { move_cursor(self.digits, &mut self.cursor, -1); Consumed }
        Key::Right => { move_cursor(self.digits, &mut self.cursor, 1); Consumed }
        Key::Up | Key::Down => {
            let d = if key == Key::Up { 1 } else { -1 };
            let mut nv = self.value;
            step_digit(self.min, self.max, &mut nv, self.digits, self.cursor, d);
            if nv != self.value { self.value = nv; ValueChanged } else { Consumed }
        }
        Key::Enter | Key::Esc => ExitEdit,
        _ => Consumed,
    }
}
```

`switch.rs`：

```rust
pub(crate) fn on_key(&mut self, key: Key, _ctx: super::KeyCtx) -> super::KeyOutcome {
    if key == Key::Enter { self.on = !self.on; super::KeyOutcome::ValueChanged } else { super::KeyOutcome::Pass }
}
```

`checkbox.rs`：同上，`self.checked = !self.checked`。

`list.rs`（注意：Up/Down 即使列表为空也消费——保持现状行为）：

```rust
pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
    let n = self.items.len();
    match key {
        Key::Up | Key::Down => {
            if n > 0 {
                let idx = if key == Key::Up { (self.selected + n - 1) % n } else { (self.selected + 1) % n };
                select(&self.items, &mut self.selected, &mut self.scroll, &mut self.fx, idx, ctx.vis_h, ctx.now);
            }
            super::KeyOutcome::Consumed
        }
        _ => super::KeyOutcome::Pass,
    }
}
```

`roller.rs`：

```rust
pub(crate) fn on_key(&mut self, key: Key, ctx: super::KeyCtx) -> super::KeyOutcome {
    match key {
        Key::Up | Key::Down => {
            let dir = if key == Key::Up { -1 } else { 1 };
            let next = (self.selected as i32 + dir).clamp(0, self.items.len().saturating_sub(1) as i32);
            select(&self.items, &mut self.selected, &mut self.sel_from, next as usize, ctx.now);
            super::KeyOutcome::Consumed
        }
        _ => super::KeyOutcome::Pass,
    }
}
```

`dropdown.rs`：

```rust
impl DropdownState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: super::KeyCtx) -> super::KeyOutcome {
        if key == Key::Enter { super::KeyOutcome::OpenDropdown } else { super::KeyOutcome::Pass }
    }
}
```

- [ ] **Step 3: dropdown::open 搬迁**

把 `ui.rs:1269-1312` 的 `open_dropdown` 整体搬到 `dropdown.rs`，改为自由函数，函数体逐字保留，仅调整两点：kind 解构改用新变体、`ui.` 前缀不变（参数已是 `ui: &mut Ui`）：

```rust
/// 打开 Dropdown 的浮层列表（Attach::Bottom 锚定，模态锁定）
pub(crate) fn open(ui: &mut Ui, obj: ObjRef) {
    let Some((items, sel, w)) = ui.arena.get(obj).map(|n| match &n.kind {
        WidgetKind::Dropdown(s) => (s.items.clone(), s.selected, n.rect.w),
        _ => (Vec::new(), 0, 0),
    }) else { return };
    if items.is_empty() {
        return;
    }
    // …以下与 ui.rs:1277-1311 完全一致（create_list/set_size/list_select/set_floating/
    // group_add/set_modal/两个 add_event_cb 回调；回调内 kind 匹配改为 as_dropdown_mut）…
}
```

回调内写回选中值处（原 `if let WidgetKind::Dropdown { selected, .. } = &mut n.kind`）改为 `if let Some(s) = n.kind.as_dropdown_mut() { s.selected = idx; }`。

- [ ] **Step 4: keypad_input 重写 + call_on_key + apply_key_outcome**

删除 `ui.rs` 的 `activate`（1236-1254）、`roller_step`（1256-1266）、`open_dropdown`（1269-1312）。`keypad_input`（1118-1220）整体替换为：

```rust
pub fn keypad_input(&mut self, key: crate::input::Key) {
    use crate::input::Key;
    let Some(f) = self.focused() else { return };
    if !self.is_valid(f) {
        return;
    }
    self.send_event(f, crate::event::EventKind::Key(key));
    if !self.is_valid(f) {
        return; // Key 回调可能删除了焦点对象
    }
    if self.call_on_key(f, key) {
        return;
    }
    // 默认：未被控件消费的按键走焦点导航 / Clicked
    match key {
        Key::Next | Key::Right | Key::Down => self.group_focus_next(),
        Key::Prev | Key::Left | Key::Up => self.group_focus_prev(),
        Key::Enter => self.send_event(f, crate::event::EventKind::Clicked),
        Key::Esc => {}
    }
}

/// 控件的按键处理：kind 拆出后调用其 on_key，放回再执行通用副作用。
/// （拆出期间节点 kind 为占位 Obj，故内置控件的 on_key 不接收 &mut Ui）
fn call_on_key(&mut self, obj: ObjRef, key: crate::input::Key) -> bool {
    use crate::widgets::{KeyCtx, KeyOutcome};
    let edited = self.state(obj).contains(State::EDITED);
    let vis_h = self.rect(obj).h;
    let now = self.time_ms;
    let mut kind = match self.arena.get_mut(obj) {
        Some(n) => core::mem::replace(&mut n.kind, WidgetKind::Obj),
        None => return false,
    };
    let out = kind.on_key(key, KeyCtx { edited, vis_h, now });
    if let Some(n) = self.arena.get_mut(obj) {
        n.kind = kind;
    } else {
        return true; // 节点已在处理过程中被删除：视为已消费
    }
    self.apply_key_outcome(obj, out)
}

fn apply_key_outcome(&mut self, obj: ObjRef, out: crate::widgets::KeyOutcome) -> bool {
    use crate::widgets::KeyOutcome;
    match out {
        KeyOutcome::Pass => false,
        KeyOutcome::Consumed => {
            self.invalidate_obj(obj);
            true
        }
        KeyOutcome::ValueChanged => {
            self.invalidate_obj(obj);
            self.send_event(obj, crate::event::EventKind::ValueChanged);
            true
        }
        KeyOutcome::EnterEdit => {
            self.set_state(obj, State::EDITED, true);
            true
        }
        KeyOutcome::ExitEdit => {
            self.set_state(obj, State::EDITED, false);
            self.invalidate_obj(obj);
            true
        }
        KeyOutcome::OpenDropdown => {
            crate::widgets::dropdown::open(self, obj);
            true
        }
    }
}
```

注意 `KeyOutcome` 需要 `pub(crate)` 可见性匹配（mod.rs 中定义处已标 `pub(crate)`；`KeyCtx` 字段同理）。

- [ ] **Step 5: 验证**

Run: `cargo test -p qingui`
Expected: 全绿（重点：`input.rs`、`list_nav.rs`、`p0_widgets.rs`、`p1_widgets.rs`、`roller_ghost.rs` 全部按键行为不变）。

- [ ] **Step 6: Commit（先获用户确认）**

```bash
git add qingui/src
git commit -m "refactor: move key handling into widget states via KeyOutcome"
```

---

### Task 5: Node 钩子（draw_hook/tick_hook）+ Canvas 特例消除 + draw_node 去 clone

**Files:**
- Modify: `qingui/src/node.rs`（`DrawHook`/`TickHook` 类型 + Node 两个字段）
- Modify: `qingui/src/ui.rs`（`set_draw_hook`/`set_tick_hook`；`draw_node` 重写；`tick_widgets` 加 hook；删 `canvas_cbs`/`register_canvas_cb`）
- Modify: `qingui/src/widgets/mod.rs`（删 `Canvas` 变体及其 draw 臂）
- Modify: `qingui/src/widgets/canvas.rs`（Builder 改为"Obj 节点 + draw_hook"的糖）
- Modify: `qingui/examples/gallery.rs`（删除隐藏 Bar hack，改用 tick_hook）
- Test: `qingui/tests/hooks.rs`（新建）

**Interfaces:**
- Consumes: `WidgetKind::draw/tick`（Task 2/3）、`tick_widgets`（Task 3）。
- Produces:
  - `pub type DrawHook = Box<dyn FnMut(&mut DrawBuf, Rect, Rect, u64)>`（`node.rs`）
  - `pub type TickHook = Box<dyn FnMut(&mut Ui, ObjRef, u64) -> bool>`（`node.rs`）
  - `Node { pub draw_hook: Option<DrawHook>, pub tick_hook: Option<TickHook>, .. }`
  - `Ui::set_draw_hook(&mut self, obj: ObjRef, hook: Option<DrawHook>)`、`Ui::set_tick_hook(&mut self, obj: ObjRef, hook: Option<TickHook>)`（Task 7 包装为 `obj.on_draw/on_tick`）

- [ ] **Step 1: 写失败测试**

新建 `qingui/tests/hooks.rs`：

```rust
use qingui::display::Flush;
use qingui::{Color, Rect, Ui};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}
fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Color {
    let chunks = &rec.borrow().chunks;
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

#[test]
fn draw_hook_overlays_builtin_widget() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let btn = ui.create_button(ui.screen(), "ok");
    ui.set_pos(btn, 10, 10);
    ui.set_draw_hook(btn, Some(Box::new(|d, abs, clip, _now| {
        d.fill_rect(Rect::new(abs.x, abs.y, 3, 3), Color::RED, 255, clip);
    })));
    ui.render();
    // 钩子叠加在按钮自带内容之上（左上角 3x3 被覆盖为红色）
    assert_eq!(px(&rec, 10, 10), Color::RED);
    assert_eq!(px(&rec, 11, 11), Color::RED);
}

#[test]
fn tick_hook_drives_wakeup_and_redraw() {
    let mut ui = Ui::new(64, 64, 16);
    let o = ui.create_obj(ui.screen());
    let hits = Rc::new(Cell::new(0u32));
    let h = hits.clone();
    ui.set_tick_hook(o, Some(Box::new(move |_ui, _obj, _now| {
        h.set(h.get() + 1);
        true
    })));
    ui.tick_inc(16);
    ui.timer_handler(); // 首帧（含建屏全屏脏）
    assert!(hits.get() >= 1);
    assert_eq!(ui.timer_handler(), 0); // 活动 hook 保持唤醒
    // 换成不活动的 hook → 睡眠
    ui.set_tick_hook(o, Some(Box::new(|_, _, _| false)));
    assert_eq!(ui.timer_handler(), u32::MAX);
}
```

Run: `cargo test -p qingui --test hooks`
Expected: 编译失败（`set_draw_hook`/`set_tick_hook` 不存在）。

- [ ] **Step 2: node.rs 增加钩子类型与字段**

`qingui/src/node.rs` 顶部（`use` 之后）：

```rust
/// 叠加绘制钩子：在控件自带内容之后调用，参数为 (画板, 控件绝对矩形, 裁剪矩形, 当前时间 ms)
pub type DrawHook = alloc::boxed::Box<dyn FnMut(&mut crate::draw::DrawBuf, Rect, Rect, u64)>;
/// 每帧钩子：返回 true 表示仍活动（标脏并保持 timer_handler 唤醒）
pub type TickHook = alloc::boxed::Box<dyn FnMut(&mut crate::ui::Ui, ObjRef, u64) -> bool>;
```

`Node` 增加两个字段（放在 `events` 之后），`Node::new` 初始化为 `None`：

```rust
pub draw_hook: Option<DrawHook>,
pub tick_hook: Option<TickHook>,
```

- [ ] **Step 3: Ui 增删方法 + draw_node 重写**

`Ui` 结构体删除 `canvas_cbs` 字段（`ui.rs:22`）及 `register_canvas_cb`（36-39）。`Ui::new` 初始化列表同步删除 `canvas_cbs: Vec::new()`。新增：

```rust
/// 设置叠加绘制钩子（None 清除）。在控件自带内容之上追加绘制
pub fn set_draw_hook(&mut self, obj: ObjRef, hook: Option<crate::node::DrawHook>) {
    if let Some(n) = self.arena.get_mut(obj) {
        n.draw_hook = hook;
    }
    self.invalidate_obj(obj);
}

/// 设置每帧钩子（None 清除）。返回 true 的帧：标脏该对象并保持唤醒
pub fn set_tick_hook(&mut self, obj: ObjRef, hook: Option<crate::node::TickHook>) {
    if let Some(n) = self.arena.get_mut(obj) {
        n.tick_hook = hook;
    }
}
```

`draw_node`（698-735）整体替换为（不再 `kind.clone()`；`self.buf` 与 `self.arena` 是不同字段，可同时可变借用）：

```rust
fn draw_node(&mut self, obj: ObjRef, clip: Rect, len: usize) {
    let Some((abs, flags, node_opa, resolved)) = self.node_draw_info(obj) else {
        return;
    };
    if flags.contains(Flag::HIDDEN) {
        return;
    }
    if abs.intersect(&clip).is_some() {
        let edited = self.state(obj).contains(State::EDITED);
        let now = self.time_ms;
        // 节点 opa 作为乘数作用于本对象的所有绘制
        let ap = |base: u8| (base as u32 * node_opa as u32 / 255) as u8;
        let mut d = crate::draw::DrawBuf {
            pixels: &mut self.buf[..len],
            area: clip,
            stride: clip.w,
        };
        let Some(n) = self.arena.get_mut(obj) else { return };
        if resolved.bg_opa > 0 && ap(resolved.bg_opa) > 0 {
            d.fill_rounded(abs, resolved.radius, resolved.bg_color, ap(resolved.bg_opa), clip);
        }
        let ctx = crate::widgets::WidgetCtx { abs, resolved: &resolved, edited, opa: node_opa, now };
        n.kind.draw(&ctx, &mut d, clip);
        // 叠加绘制钩子（原 Canvas 机制的通用化）
        if let Some(hook) = n.draw_hook.as_mut() {
            hook(&mut d, abs, clip, now);
        }
        // 边框最后画（对齐 LVGL：border 在内容之上），避免被控件内容覆盖
        if resolved.border_width > 0 {
            d.draw_border(abs, resolved.border_width, resolved.radius, resolved.border_color, ap(255), clip);
        }
    }
    for c in self.children_z_sorted(obj) {
        self.draw_node(c, clip, len);
    }
}
```

- [ ] **Step 4: tick_widgets 接入 tick_hook**

`tick_widgets`（Task 3 版本）替换为：

```rust
/// 遍历对象树推进每帧效果（fx/Spinner/tick_hook），活动节点标脏。
/// 返回是否仍有活动效果（决定 timer_handler 是否持续唤醒）。
fn tick_widgets(&mut self) -> bool {
    let now = self.time_ms;
    let mut any = false;
    let mut stack = alloc::vec![self.screen];
    while let Some(r) = stack.pop() {
        let (out, children, has_hook) = match self.arena.get_mut(r) {
            Some(n) => (n.kind.tick(now), n.children.clone(), n.tick_hook.is_some()),
            None => continue,
        };
        if out.redraw {
            self.invalidate_obj(r);
        }
        if out.active {
            any = true;
        }
        if has_hook {
            // take-调用-放回：hook 签名含 &mut Ui
            let mut hook = self.arena.get_mut(r).and_then(|n| n.tick_hook.take());
            if let Some(h) = hook.as_mut() {
                if h(self, r, now) {
                    any = true;
                    self.invalidate_obj(r);
                }
            }
            if let Some(n) = self.arena.get_mut(r) {
                n.tick_hook = hook;
            }
        }
        stack.extend_from_slice(&children);
    }
    any
}
```

- [ ] **Step 5: 删除 Canvas 变体，canvas.rs 改为糖**

`widgets/mod.rs`：删除 `Canvas { cb: usize }` 变体及 `draw` 中的 `Canvas` 臂与注释。`widgets/canvas.rs` 整体替换：

```rust
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::node::DrawHook;
use crate::style::Style;
use crate::ui::Ui;
use super::WidgetKind;

/// Canvas 构建器：空节点 + 叠加绘制钩子的糖；size 必填（无默认），默认透明背景
pub struct CanvasBuilder {
    cb: DrawHook,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<crate::layout::Sizing>, Option<crate::layout::Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl CanvasBuilder {
    pub fn new(cb: DrawHook) -> Self {
        Self { cb, size: None, style: None, sizing: None, transition: None, events: Vec::new() }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    pub fn sizing(mut self, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((32, 32));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj);
        let mut s = self.style.unwrap_or_default();
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0); // 默认透明背景：画布只承载自定义绘制
        }
        ui.set_style(r, s);
        ui.set_draw_hook(r, Some(self.cb));
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}

pub(crate) fn create(ui: &mut Ui, parent: ObjRef, w: i32, h: i32, cb: DrawHook) -> ObjRef {
    CanvasBuilder::new(cb).size(w, h).build(ui, parent)
}
```

`Ui::create_canvas` 保留（Task 9 才删），签名中 `CanvasCb` 改为 `crate::node::DrawHook`；删除 `canvas.rs` 的 `CanvasCb` 类型定义。`draw_node` 里的 Canvas 特殊判断（旧 722-726）已随 Step 3 删除。

- [ ] **Step 6: gallery 去 hack**

读 `qingui/examples/gallery.rs` 约 160-174 行：现状用"隐藏 Bar + 无限 Value 动画 + ValueChanged 回调里 invalidate"驱动 Canvas 逐帧。删除该 hack（隐藏 bar 节点、其动画与回调），改为在创建 canvas 后：

```rust
ui.set_tick_hook(cv, Some(Box::new(|ui, cv, _now| {
    ui.invalidate_obj(cv);
    true // 每帧重绘
})));
```

Run: `cargo check -p qingui --examples`
Expected: 编译通过。

- [ ] **Step 7: 验证**

Run: `cargo test -p qingui`
Expected: 全绿（`hooks.rs` 新测试通过；`canvas.rs` 既有测试不经改动通过——`Ui::create_canvas` 签名中 `CanvasCb` 与 `DrawHook` 同为 `Box<dyn FnMut(&mut DrawBuf, Rect, Rect, u64)>`，调用处无需改）。
再跑：`grep -rn "kind.clone()" qingui/src` → 无输出；`grep -n "canvas_cbs" qingui/src/ui.rs` → 无输出。

- [ ] **Step 8: Commit（先获用户确认）**

```bash
git add qingui/src qingui/tests/hooks.rs qingui/examples/gallery.rs
git commit -m "feat: per-node draw/tick hooks; Canvas becomes plain node + hook"
```

---

### Task 6: Custom widget 逃生舱

**Files:**
- Create: `qingui/src/widgets/custom.rs`
- Modify: `qingui/src/widgets/mod.rs`（`pub mod custom;` + `Custom` 变体 + 各委托臂 + 移除 `#[derive(Clone)]`）
- Modify: `qingui/src/ui.rs`（`create_custom`/`custom`/`custom_mut`；`call_on_key` 加 Custom 分支）
- Modify: `qingui/src/lib.rs`（re-export）
- Test: `qingui/tests/custom_widget.rs`（新建）

**Interfaces:**
- Consumes: `TickOut`（Task 3）、`call_on_key`（Task 4）、draw/tick 钩子（Task 5）。
- Produces:
  - `pub trait Widget`（`widgets::custom`）：`draw(&self, &WidgetCtx, &mut DrawBuf, Rect)` 必需；`tick(&mut self, u64) -> TickOut`、`on_key(&mut self, &mut Ui, ObjRef, Key) -> bool` 有默认；`as_any`/`as_any_mut` 必需
  - `WidgetKind::Custom(Box<dyn Widget>)`
  - `Ui::create_custom(&mut self, parent: ObjRef, w: i32, h: i32, widget: Box<dyn Widget>) -> ObjRef`
  - `Ui::custom<T: 'static>(&self, obj: ObjRef) -> Option<&T>`
  - `Ui::custom_mut<T: 'static, R>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> R) -> Option<R>`
  - `WidgetKind::as_custom(&self) -> Option<&dyn Widget>` / `as_custom_mut(&mut self) -> Option<&mut dyn Widget>`（pub(crate)）

- [ ] **Step 1: 写失败测试**

新建 `qingui/tests/custom_widget.rs`：

```rust
use core::any::Any;
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::custom::Widget;
use qingui::widgets::WidgetCtx;
use qingui::{Color, ObjRef, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

struct Gauge {
    v: i32,
}
impl Widget for Gauge {
    fn draw(&self, ctx: &WidgetCtx, d: &mut qingui::draw::DrawBuf, clip: Rect) {
        d.fill_rect(ctx.abs, Color::RED, 255, clip);
    }
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, key: Key) -> bool {
        if key == Key::Up {
            self.v += 1;
            true
        } else {
            false
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}
fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Color {
    let chunks = &rec.borrow().chunks;
    for (area, buf) in chunks.iter().rev() {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

#[test]
fn custom_widget_draws_and_handles_keys() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let g = ui.create_custom(ui.screen(), 20, 20, Box::new(Gauge { v: 0 }));
    ui.set_pos(g, 5, 5);
    ui.render();
    assert_eq!(px(&rec, 6, 6), Color::RED); // draw 被调用

    assert_eq!(ui.custom::<Gauge>(g).unwrap().v, 0);
    ui.group_add(g);
    ui.keypad_input(Key::Up); // 焦点对象收到键 → on_key 消费
    assert_eq!(ui.custom::<Gauge>(g).unwrap().v, 1);

    ui.custom_mut::<Gauge, _>(g, |g| g.v = 42);
    assert_eq!(ui.custom::<Gauge>(g).unwrap().v, 42);
    assert!(ui.custom::<String>(g).is_none()); // 类型不匹配 → None
}
```

Run: `cargo test -p qingui --test custom_widget`
Expected: 编译失败（`create_custom` 等不存在）。

- [ ] **Step 2: custom.rs**

新建 `qingui/src/widgets/custom.rs`：

```rust
use core::any::Any;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::input::Key;
use crate::ui::Ui;

use super::{TickOut, WidgetCtx};

/// 用户自定义 widget：经 Ui::create_custom 挂载为 WidgetKind::Custom，
/// 与内置控件一样参与绘制/逐帧/按键。
///
/// 注意：on_key 调用期间本节点的 kind 处于"拆出"状态（节点内是占位 Obj），
/// 修改自身状态请直接改 self；对其他节点的操作不受限。
pub trait Widget {
    /// 内容绘制（背景/边框/opa 由 Ui 统一处理）
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect);
    /// 每帧推进：返回活动状态（默认无逐帧行为）
    fn tick(&mut self, _now: u64) -> TickOut {
        TickOut::IDLE
    }
    /// 按键处理：返回 true 表示消费（默认不消费，走默认移焦/Clicked）
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, _key: Key) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

- [ ] **Step 3: WidgetKind 接入 Custom**

`widgets/mod.rs`：`pub mod custom;` 加入模块列表；enum 加变体：

```rust
    /// 用户自定义 widget（逃生舱；不可 Clone，故 WidgetKind 不再 derive Clone）
    Custom(alloc::boxed::Box<dyn custom::Widget>),
```

删除 enum 上的 `#[derive(Clone)]`。各委托方法加臂：

```rust
// draw:    WidgetKind::Custom(w) => w.draw(ctx, d, clip),
// tick:    WidgetKind::Custom(w) => w.tick(now),
// value/set_value/set_range/overflow: 落入已有的 _ 臂（0/false/无操作），无需新臂
```

新增访问器（供 Ui::custom 使用，避免 ui.rs 出现变体 match）：

```rust
pub(crate) fn as_custom(&self) -> Option<&dyn custom::Widget> {
    match self { WidgetKind::Custom(w) => Some(w.as_ref()), _ => None }
}
pub(crate) fn as_custom_mut(&mut self) -> Option<&mut dyn custom::Widget> {
    match self { WidgetKind::Custom(w) => Some(w.as_mut()), _ => None }
}
```

- [ ] **Step 4: Ui 三方法 + call_on_key Custom 分支**

`ui.rs` 新增：

```rust
/// 挂载用户自定义 widget（实现 widgets::custom::Widget）
pub fn create_custom(&mut self, parent: ObjRef, w: i32, h: i32, widget: alloc::boxed::Box<dyn crate::widgets::custom::Widget>) -> ObjRef {
    self.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Custom(widget))
}

/// 只读查询自定义 widget 状态（类型不匹配或对象非 Custom 返回 None）
pub fn custom<T: 'static>(&self, obj: ObjRef) -> Option<&T> {
    self.arena.get(obj)?.kind.as_custom()?.as_any().downcast_ref::<T>()
}

/// 可变更新自定义 widget 状态（前后自动标脏）
pub fn custom_mut<T: 'static, R>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> R) -> Option<R> {
    self.invalidate_obj(obj);
    let r = self
        .arena
        .get_mut(obj)?
        .kind
        .as_custom_mut()?
        .as_any_mut()
        .downcast_mut::<T>()
        .map(f);
    self.invalidate_obj(obj);
    r
}
```

`call_on_key`（Task 4 版本）中，`let out = kind.on_key(...)` 之前插入 Custom 分支（Custom 的 on_key 需要 `&mut Ui`，此时 kind 已拆出、Ui 自由）：

```rust
// Custom：用户状态在拆出的 Box 里，on_key 可安全接收 &mut Ui
if let Some(w) = kind.as_custom_mut() {
    let consumed = w.on_key(self, obj, key);
    if let Some(n) = self.arena.get_mut(obj) {
        n.kind = kind;
    }
    if consumed {
        self.invalidate_obj(obj);
    }
    return consumed;
}
let out = kind.on_key(key, KeyCtx { edited, vis_h, now });
```

- [ ] **Step 5: lib.rs re-export + 验证 Clone 移除无残留**

`qingui/src/lib.rs` 添加：

```rust
pub use widgets::custom::Widget;
pub use widgets::TickOut;
```

Run: `grep -rn "\.kind\.clone()\|kind_snap" qingui/src` → 应无输出（Task 5 已清除）；`grep -rn "clone" qingui/src/widgets/mod.rs` → 确认无 `derive(Clone)`。
Run: `cargo test -p qingui`
Expected: 全绿（含 `custom_widget.rs`）。

- [ ] **Step 6: Commit（先获用户确认）**

```bash
git add qingui/src qingui/tests/custom_widget.rs
git commit -m "feat: custom widget escape hatch via Widget trait and WidgetKind::Custom"
```

---

### Task 7: 句柄 API 新增（additive，不动旧 API）

**Files:**
- Create: `qingui/src/handle.rs`（`impl ObjRef` 全部句柄方法）
- Create: `qingui/src/widgets/obj.rs`（`ObjBuilder`）
- Modify: `qingui/src/lib.rs`（`mod handle;`）
- Modify: `qingui/src/widgets/mod.rs`（`pub mod obj;`）
- Test: `qingui/tests/handle_api.rs`（新建）

**Interfaces:**
- Consumes: 现有 `Ui` 公开方法（Task 9 才把它们的可见性降为 `pub(crate)`）。
- Produces: `impl ObjRef` 方法全集（Task 8 迁移、Task 9 删旧 API 的映射依据）：

| 句柄方法 | 委托的 Ui 方法 |
|---|---|
| `set_pos(ui, x, y)` / `set_size(ui, w, h)` / `rect(ui) -> Rect` / `abs_rect(ui) -> Rect` | 同名 |
| `set_translate(ui, x, y)` / `translate(ui) -> Point` | 同名 |
| `set_hidden(ui, bool)` / `is_hidden(ui) -> bool` | 同名 |
| `set_style(ui, Style)` / `set_style_pressed(ui, Style)` / `set_style_focused(ui, Style)` | 同名 |
| `set_state(ui, State, bool)` / `state(ui) -> State` | 同名 |
| `set_sizing(ui, Option<Sizing>, Option<Sizing>)` / `set_aspect(ui, Option<u32>)` / `set_transition(ui, Option<(u32, Easing)>)` | 同名 |
| `set_layout(ui, Layout)` / `set_grid_cell(ui, (u8,u8), (u8,u8))` / `grid_cell(ui) -> ((u8,u8),(u8,u8))` | 同名 |
| `set_z_index(ui, i16)` / `set_floating(ui, ObjRef, Attach)` / `clear_floating(ui)` / `set_ignore_layout(ui, bool)` / `is_ignore_layout(ui) -> bool` / `move_child_to_index(ui, usize)` | 同名 |
| `set_value(ui, i32)` / `value(ui) -> i32` / `set_range(ui, i32, i32)` | 同名 |
| `invalidate(ui)` | `invalidate_obj` |
| `delete(ui)` / `children(ui) -> Vec<ObjRef>` | 同名 |
| `on(ui, EventKind, EventCb)` | `add_event_cb` |
| `send_event(ui, EventKind)` | 同名 |
| `group_add(ui)` / `group_remove(ui)` | 同名 |
| `set_text(ui, &str)` / `text(ui) -> String` | 同名 |
| `list_select(ui, usize)` / `list_selected(ui) -> usize` / `list_insert(ui, usize, &str)` / `list_remove(ui) -> bool` / `list_len(ui) -> usize` | 同名 |
| `roller_selected(ui) -> usize` / `msgbox_selected(ui) -> i32` / `table_set_cell(ui, u8, u8, &str)` | 同名 |
| `toggle_checkbox(ui)` / `toggle_switch(ui)` | 同名 |
| `on_draw(ui, DrawHook)` / `clear_draw_hook(ui)` | `set_draw_hook`（Some/None） |
| `on_tick(ui, TickHook)` / `clear_tick_hook(ui)` | `set_tick_hook`（Some/None） |
| `as_list(ui) -> Option<&ListState>` / `as_roller(ui) -> Option<&RollerState>` | 新增（经 `kind.as_list()` 等） |
| `custom::<T>(ui) -> Option<&T>` / `custom_mut::<T,R>(ui, f) -> Option<R>` | 同名 |

`ObjBuilder`（`widgets/obj.rs`）：`new()` + `size/style/style_with/sizing/transition/layout/on` 链式 + `build(ui, parent) -> ObjRef`（`WidgetKind::Obj` 节点；`layout` 调 `ui.set_layout`）。

- [ ] **Step 1: 写失败测试**

新建 `qingui/tests/handle_api.rs`：

```rust
use qingui::widgets::label::LabelBuilder;
use qingui::widgets::obj::ObjBuilder;
use qingui::{EventKind, Ui};

#[test]
fn handle_methods_roundtrip() {
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.screen();
    let l = LabelBuilder::new("hi").build(&mut ui, s);
    l.set_pos(&mut ui, 5, 7);
    assert_eq!(l.rect(&ui).x, 5);
    assert_eq!(l.rect(&ui).y, 7);
    l.set_text(&mut ui, "hello");
    assert_eq!(l.text(&ui), "hello");
    l.set_hidden(&mut ui, true);
    assert!(l.is_hidden(&ui));

    let c = ObjBuilder::new().size(50, 20).build(&mut ui, s);
    assert_eq!(c.rect(&ui).w, 50);
    let child = LabelBuilder::new("kid").build(&mut ui, c);
    assert_eq!(c.children(&ui), vec![child]);
}

#[test]
fn handle_event_and_value() {
    use std::cell::Cell;
    use std::rc::Rc;
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.screen();
    let sl = qingui::widgets::slider::SliderBuilder::new(0, 100).build(&mut ui, s);
    let hits = Rc::new(Cell::new(0));
    let h = hits.clone();
    sl.on(&mut ui, EventKind::ValueChanged, Box::new(move |_, _, _| h.set(h.get() + 1)));
    sl.set_value(&mut ui, 30);
    assert_eq!(sl.value(&ui), 30);
    assert_eq!(hits.get(), 1);
}
```

Run: `cargo test -p qingui --test handle_api`
Expected: 编译失败（句柄方法不存在）。

- [ ] **Step 2: handle.rs**

新建 `qingui/src/handle.rs`。文件级文档 + 完整 `impl ObjRef`（按上表逐行委托，每个方法一行函数体）。骨架：

```rust
//! ObjRef 句柄方法：节点操作的对外主 API。
//! 每个方法都是对 Ui 内部实现的薄封装；`ui` 参数是显式的"世界"借用，
//! 使节点操作天然带无效化与布局标记。
use alloc::string::String;
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Point, Rect};
use crate::layout::{Attach, Sizing};
use crate::node::{DrawHook, State, TickHook};
use crate::style::{Layout, Style};
use crate::ui::Ui;

impl ObjRef {
    /// 设置对象位置（本地坐标；布局管理的对象位置归布局所有）
    pub fn set_pos(self, ui: &mut Ui, x: i32, y: i32) {
        ui.set_pos(self, x, y);
    }
    // …按上表写全…
    /// 只读访问 List 状态（非 List 返回 None）
    pub fn as_list(self, ui: &Ui) -> Option<&crate::widgets::list::ListState> {
        ui.arena.get(self).and_then(|n| n.kind.as_list())
    }
    /// 只读访问 Roller 状态（非 Roller 返回 None）
    pub fn as_roller(self, ui: &Ui) -> Option<&crate::widgets::roller::RollerState> {
        ui.arena.get(self).and_then(|n| n.kind.as_roller())
    }
}
```

`on_draw`/`on_tick` 包装：

```rust
    /// 叠加绘制钩子：在控件自带内容之后调用
    pub fn on_draw(self, ui: &mut Ui, hook: DrawHook) {
        ui.set_draw_hook(self, Some(hook));
    }
    /// 清除叠加绘制钩子
    pub fn clear_draw_hook(self, ui: &mut Ui) {
        ui.set_draw_hook(self, None);
    }
    /// 每帧钩子：返回 true 的帧标脏并保持唤醒
    pub fn on_tick(self, ui: &mut Ui, hook: TickHook) {
        ui.set_tick_hook(self, Some(hook));
    }
    /// 清除每帧钩子
    pub fn clear_tick_hook(self, ui: &mut Ui) {
        ui.set_tick_hook(self, None);
    }
```

注意 `as_list`/`as_roller` 需要访问 `ui.arena`（已是 `pub(crate)`），以及 `kind.as_list()`（Task 2 已是 `pub`）。

- [ ] **Step 3: widgets/obj.rs**

新建 `qingui/src/widgets/obj.rs`：

```rust
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::style::Style;
use crate::ui::Ui;
use super::WidgetKind;

/// 通用容器 Obj 的构建器（无自带绘制内容，承载布局与子对象）
#[derive(Default)]
pub struct ObjBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<crate::layout::Sizing>, Option<crate::layout::Sizing>)>,
    transition: Option<(u32, crate::anim::Easing)>,
    layout: Option<crate::style::Layout>,
    events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
}

impl ObjBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.style = Some(f(self.style.unwrap_or_default())); self
    }
    pub fn sizing(mut self, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    pub fn transition(mut self, dur: u32, easing: crate::anim::Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    pub fn layout(mut self, layout: crate::style::Layout) -> Self { self.layout = Some(layout); self }
    pub fn on(mut self, kind: crate::event::EventKind, cb: crate::event::EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((0, 0));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj);
        if let Some(s) = self.style {
            ui.set_style(r, s);
        }
        if let Some(l) = self.layout {
            ui.set_layout(r, l);
        }
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}
```

`widgets/mod.rs` 模块列表加 `pub mod obj;`（字母序在 msgbox 前）。

- [ ] **Step 4: lib.rs 挂载 + 验证**

`qingui/src/lib.rs` 添加 `mod handle;`（放在 `pub mod geometry;` 后，私有模块：inherent impl 不受模块可见性限制）。

Run: `cargo test -p qingui`
Expected: 全绿（新 `handle_api.rs` 通过；旧测试零改动）。

- [ ] **Step 5: Commit（先获用户确认）**

```bash
git add qingui/src qingui/tests/handle_api.rs
git commit -m "feat: ObjRef handle methods and ObjBuilder (additive)"
```

---

### Task 8: 测试 / 示例 / README 迁移到句柄 API

旧 API 仍在（Task 9 才删），本 Task 逐个文件迁移，每次迁移后 `cargo test -p qingui` 保持绿。

**Files:**
- Modify: `qingui/tests/*.rs`（25 个文件）
- Modify: `qingui/examples/demo.rs`、`qingui/examples/gallery.rs`、`qingui/examples/sim/mod.rs`
- Modify: `qingui/README.md`（若含 `ui.create_xx`/`ui.set_xxx` 示例）

**Interfaces:**
- Consumes: Task 7 句柄方法全集 + `ObjBuilder`。
- Produces: 无新接口；迁移后代码即 Task 9 删旧 API 的最终形态。

- [ ] **Step 1: 迁移测试文件（机械替换模式）**

对 `qingui/tests/` 每个文件应用以下模式（逐文件替换 → `cargo test -p qingui --test <name>` 验证 → 下一个）：

| 旧 | 新 |
|---|---|
| `ui.set_xxx(a, ...)`（a 为 ObjRef） | `a.set_xxx(&mut ui, ...)` |
| `let x = ui.rect(a)` 等查询 | `let x = a.rect(&ui)` |
| `ui.create_label(p, t)` | `LabelBuilder::new(t).build(&mut ui, p)`（use `qingui::widgets::label::LabelBuilder`） |
| `ui.create_button(p, t)` | `ButtonBuilder::new(t).build(&mut ui, p)` |
| `ui.create_slider(p, mn, mx)` / `create_bar` / `create_arc` | `SliderBuilder::new(mn, mx)` / `BarBuilder::new(mn, mx)` / `ArcBuilder::new(mn, mx)` `.build(&mut ui, p)` |
| `ui.create_switch(p)` / `create_spinner(p)` | `SwitchBuilder::new()` / `SpinnerBuilder::new()` `.build(&mut ui, p)` |
| `ui.create_checkbox(p, t)` | `CheckboxBuilder::new(t).build(&mut ui, p)` |
| `ui.create_list(p, items)` | `ListBuilder::new(items).build(&mut ui, p)` |
| `ui.create_msgbox(p, t, txt, btns)` | `MsgboxBuilder::new(t, txt).buttons(btns).build(&mut ui, p)` |
| `ui.create_led(p, c)` | `LedBuilder::new(c).build(&mut ui, p)` |
| `ui.create_table(p, c, r)` | `TableBuilder::new(c, r).build(&mut ui, p)` |
| `ui.create_spinbox(p, mn, mx, d)` | `SpinboxBuilder::new(mn, mx, d).build(&mut ui, p)` |
| `ui.create_roller(p, items)` / `create_dropdown(p, items)` | `RollerBuilder::new(items)` / `DropdownBuilder::new(items)` `.build(&mut ui, p)` |
| `ui.create_canvas(p, w, h, cb)` | `CanvasBuilder::new(cb).size(w, h).build(&mut ui, p)` |
| `ui.create_obj(p)` | `ObjBuilder::new().build(&mut ui, p)` |
| `ui.add_event_cb(a, k, cb)` | `a.on(&mut ui, k, cb)` |
| `ui.send_event(a, k)` | `a.send_event(&mut ui, k)` |
| `ui.invalidate_obj(a)` | `a.invalidate(&mut ui)` |
| `ui.delete(a)` / `ui.children(a)` | `a.delete(&mut ui)` / `a.children(&ui)` |
| `ui.list_*(a, ...)` / `ui.set_text(a, ...)` / `ui.text(a)` / `ui.toggle_*(a)` / `ui.table_set_cell(a, ...)` / `ui.roller_selected(a)` / `ui.msgbox_selected(a)` | `a.list_*(&mut ui, ...)` 等 |
| `ui.debug_kind(l)` 匹配取状态 | `l.as_list(&ui).unwrap()` / `l.as_roller(&ui).unwrap()`（`tests/list_fx.rs` 的 `list_fx` helper 改为 `l.as_list(ui).unwrap().fx.clone()`；`p1_widgets.rs:127`、`list_nav.rs:51` 同理） |
| `tests/fluent_api.rs` 中 `ui.widget(a).pos(...)...` 链 | 改为句柄方法逐句调用（WidgetMut 将被删除；链式语义不再保留，测试意图改为验证句柄方法） |

Builder 已在用的文件（`builders.rs`、`demo.rs` 部分）只改 setter/event 部分。`group_add/group_remove`：`ui.group_add(a)` → `a.group_add(&mut ui)`。`ui.set_flush/tick_inc/timer_handler/keypad_input/anim_start/...` 保持不变（仍是 Ui API）。

- [ ] **Step 2: 迁移示例**

`examples/demo.rs`、`examples/gallery.rs`、`examples/sim/mod.rs` 应用同一套模式。gallery 中 `ui.set_tick_hook(cv, ...)`（Task 5 加的）改为 `cv.on_tick(&mut ui, ...)`。

Run: `cargo check -p qingui --examples`
Expected: 编译通过。

- [ ] **Step 3: 迁移 README**

检查 `qingui/README.md` 与根 `README.md` 中的代码示例，按同一模式更新（若存在旧 API 示例）。

- [ ] **Step 4: 全量验证**

Run: `cargo test -p qingui`
Expected: 全绿。

- [ ] **Step 5: Commit（先获用户确认）**

```bash
git add qingui/tests qingui/examples qingui/README.md README.md
git commit -m "refactor: migrate tests, examples and docs to handle API"
```

---

### Task 9: 删除旧 API + 验收

**Files:**
- Modify: `qingui/src/ui.rs`（删 create_xx 系、Ui::widget、debug_kind；其余 obj 首参方法降 `pub(crate)`）
- Delete: `qingui/src/widget.rs`（WidgetMut）
- Modify: `qingui/src/lib.rs`（删 `pub mod widget;`）

**Interfaces:**
- Consumes: Task 8 迁移完成的调用方。
- Produces: 最终公开面——`Ui`: `new/screen/timer_handler/keypad_input/tick_inc/time/set_flush/render/take_dirty/dirty_is_empty/anim_start/anim_stop/anim_running/group_focus/group_focus_next/group_focus_prev/focused/set_modal/clear_modal/is_valid/invalidate_area/create_custom`；`ObjRef` 句柄方法；各 Builder。

- [ ] **Step 1: 删除**

- `ui.rs`：删除 `create_obj/create_label/create_button/create_slider/create_switch/create_bar/create_list/create_arc/create_checkbox/create_spinner/create_msgbox/create_led/create_table/create_spinbox/create_roller/create_dropdown/create_canvas`、`widget()`、`debug_kind`。
- `ui.rs` 中以下方法 `pub` → `pub(crate)`：`set_pos/set_size/rect/abs_rect/set_translate/translate/set_hidden/is_hidden/set_style/set_style_pressed/set_style_focused/set_state/state/set_sizing/set_aspect/set_transition/set_layout/set_grid_cell/grid_cell/set_z_index/set_floating/clear_floating/set_ignore_layout/is_ignore_layout/move_child_to_index/set_value/value/set_range/invalidate_obj/set_text/text/list_select/list_selected/list_insert/list_remove/list_len/roller_selected/msgbox_selected/table_set_cell/toggle_checkbox/toggle_switch/add_event_cb/send_event/group_add/group_remove/set_draw_hook/set_tick_hook/custom/custom_mut/delete/children`。
- 删除 `qingui/src/widget.rs`；`lib.rs` 删 `pub mod widget;`。
- `msgbox.rs`/`canvas.rs` 等各 widget 文件底部的 `pub(crate) fn create(...)` 若已无人调用（`Ui::create_xx` 已删），一并删除（`cargo check` 会以 dead_code 警告指出）。

- [ ] **Step 2: 验收检查（对照 spec 验收标准）**

逐条执行并确认：

```bash
# 1. 全测试绿 + 示例编译
cargo test -p qingui && cargo check -p qingui --examples
# 2. ui.rs 无 WidgetKind 变体 match（只允许 Obj 构造与 Custom 构造出现）
grep -n "WidgetKind::" qingui/src/ui.rs
#    预期仅剩：Ui::new 的 WidgetKind::Obj、call_on_key 占位 WidgetKind::Obj、create_custom 的 WidgetKind::Custom(...)
# 3. 已删除物不存在
grep -rn "tick_list_fx\|canvas_cbs\|debug_kind\|WidgetMut\|register_canvas_cb" qingui/src qingui/tests
grep -n "pub fn create_" qingui/src/ui.rs   # 预期仅 create_custom
grep -rn "kind.clone()" qingui/src          # 预期无输出
# 4. no_std 目标构建（若已安装该 target）
cargo build -p qingui --target thumbv7em-none-eabihf
# 5. ui.rs 行数显著下降（重构前 1343 行）
wc -l qingui/src/ui.rs
```

Expected: 1 全绿；2/3 输出如上预期；4 编译通过（若无该 target 则记录跳过原因）；5 行数明显下降。

- [ ] **Step 3: Commit（先获用户确认）**

```bash
git add -A qingui
git commit -m "refactor!: remove Ui-centric API; handle methods are the public API"
```

---

## Self-Review 记录

- **Spec 覆盖**：句柄 API（Task 7/8/9）、状态 struct 化（Task 1）、行为委托（Task 2/3/4）、tick 统一（Task 3/5）、draw_hook 与 Canvas 归一（Task 5）、kind.clone 消除（Task 5）、Custom 逃生舱（Task 6）、debug_kind 删除（Task 8 迁移 + Task 9 删除）、gallery hack 移除（Task 5）、Builder 唯一创建路径（Task 7 ObjBuilder + Task 9 删 create_xx）。非目标（pointer/样式级联/fx 与属性动画语义合并）未引入。
- **行为保持点**：Switch 不在 set_value（Task 1/2 保留）；空 List 的 Up/Down 仍消费按键（Task 4）；Roller 首尾停止（Task 4）；spinbox 值不变时不发 ValueChanged（Task 4）；tick 的 prune 帧补绘（Task 3）。
- **类型一致性**：`TickOut`/`KeyOutcome`/`KeyCtx`/`DrawHook`/`TickHook`/`Widget` 的签名在定义 Task 与消费 Task 间已核对；`call_on_key` 在 Task 4 定义、Task 6 加 Custom 分支，两处代码均完整给出。
