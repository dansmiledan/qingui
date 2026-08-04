# KeyOutcome Deferred 重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `KeyOutcome` 中三个 widget 特异副作用变体（`OpenDropdown`/`NavSelect`/`ScrollBy`）替换为一个通用的 `Deferred(fn(&mut Ui, ObjRef, i32), i32)`，让 Ui 不再拥有任何 widget 语义知识。

**Architecture:** `on_key` 仍保持"纯决策 + 结果枚举"形态，但 widget 特异的执行代码从 `Ui::apply_key_outcome` 搬回各 widget 文件，成为零捕获的静态函数（fn 指针）。Ui 的 `apply_key_outcome` 对 `Deferred(f, p)` 只需放回 kind 后调用 `f(self, obj, p)`——干净窗口，无占位风险，无堆分配。通用副作用（标脏/事件/EDITED）仍作为枚举变体由 Ui 执行，避免每个 widget 重写样板。

**Tech Stack:** Rust (no_std + alloc), 现有测试框架 `cargo test`（integration tests 于 `qingui/tests/`）。

## Global Constraints

- **行为保持重构**：不改变任何对外 API 与可观察行为。现有 171 个测试即回归契约——本计划各任务的测试步骤运行"契约测试文件"，确认重构前后全绿。
- **零新增分配**：fn 指针 8 字节、零捕获、`Copy`——替换原 `Box<dyn FnOnce>` 提案后无堆分配（这正是采用 fn 指针而非闭包的原因）。
- **零新增公开 API**：`Deferred` 变体与三个执行函数均 `pub(crate)`。`KeyOutcome` 与 `KeyCtx` 保持现状。
- **Ui 不得新增对具体 widget 变体的 `match`/`if let`/`matches!`**：本次重构后 `apply_key_outcome` 只剩通用变体 + `Deferred(f, p)`。
- **执行窗口纪律**：执行函数由 Ui 在 `call_on_key` 把 kind 放回 arena **之后**调用，此时 widget 自己的 kind 已还原，可安全经 `ui` 访问自身节点（这就是"干净窗口"）。
- **git 规则**：只本地 commit，不 push（见 AGENTS.md）。每个 commit 只暂存本任务改动的文件。
- **验证命令**（仓库惯例）：
  - `cargo test -p qingui`
  - `cargo build -p qingui --target thumbv7em-none-eabihf`（no_std 不破坏；若未装 target 先 `rustup target add thumbv7em-none-eabihf`）
  - `cargo check -p qingui --examples`

---

### Task 1: 引入 `Deferred` 变体并迁移 ItemList

**Files:**
- Modify: `qingui/src/widgets/mod.rs:1-4`（imports）、`qingui/src/widgets/mod.rs:66-77`（KeyOutcome 枚举）
- Modify: `qingui/src/widgets/itemlist.rs:20-29`（on_key）、`qingui/src/widgets/itemlist.rs:29` 后新增执行函数
- Modify: `qingui/src/ui.rs:1108-1149`（apply_key_outcome）
- Test: `qingui/tests/itemlist.rs`（契约测试，全部现有用例）

**Interfaces:**
- Consumes: 现有 `KeyCtx`、`Ui::keypad_input` → `call_on_key` → `apply_key_outcome` 链路（ui.rs:1053-1149）。
- Produces:
  - `KeyOutcome::Deferred(fn(&mut Ui, ObjRef, i32), i32)`（pub(crate)）——widget 特异副作用的通用结果变体。
  - `itemlist::nav_select_exec(ui: &mut Ui, il: ObjRef, d: i32)`（pub(crate)）——ItemList 导航执行函数，Task 2/3 照此模式新增各自执行函数。
  - `apply_key_outcome` 新增 `Deferred` 分支：调用 `f(self, obj, p)` 并返回 `true`（已消费，空列表也消费，对齐旧 `NavSelect` 语义）。

- [ ] **Step 1: 运行 ItemList 契约测试确认基线全绿**

Run: `cargo test -p qingui --test itemlist`
Expected: 全部 PASS（基线）。

- [ ] **Step 2: widgets/mod.rs 增加 imports**

在 `qingui/src/widgets/mod.rs` 顶部（现有 `use crate::style::ResolvedStyle;` 之后）追加：

```rust
use crate::arena::ObjRef;
use crate::ui::Ui;
```

- [ ] **Step 3: KeyOutcome 增加 `Deferred` 变体、删除 `NavSelect`**

把 `qingui/src/widgets/mod.rs:66-77` 的枚举整体替换为：

```rust
pub(crate) enum KeyOutcome {
    Pass,          // 未消费 → 走默认（移焦/Clicked）
    Consumed,      // 已消费，标脏
    ValueChanged,  // 已消费，标脏并发 ValueChanged 事件
    EnterEdit,     // 进入 EDITED 态
    ExitEdit,      // 退出 EDITED 态并标脏
    OpenDropdown,  // 打开下拉浮层
    /// 特异副作用延迟执行：widget 文件提供的静态执行函数 + i32 载荷。
    /// Ui 在把 kind 放回 arena 后调用 f(self, obj, p)（干净窗口，无占位），视为已消费。
    Deferred(fn(&mut Ui, ObjRef, i32), i32),
    /// 滚动容器滚动(步进 ±px),由 Ui 执行(clamp + translate)
    ScrollBy(i32),
}
```

（本任务只删 `NavSelect` 与它的 doc 注释行；`OpenDropdown`/`ScrollBy` 分别留到 Task 2/3 删除。）

- [ ] **Step 4: ItemList on_key 改用 `Deferred`**

把 `qingui/src/widgets/itemlist.rs:21-28` 的 `on_key` 替换为：

```rust
    pub(crate) fn on_key(&mut self, key: Key, _ctx: KeyCtx) -> KeyOutcome {
        match key {
            // 导航细节需要 Ui（子节点/滚动/事件），由 Deferred 执行函数在 kind 放回后执行
            Key::Up => KeyOutcome::Deferred(nav_select_exec, -1),
            Key::Down => KeyOutcome::Deferred(nav_select_exec, 1),
            _ => KeyOutcome::Pass,
        }
    }
```

- [ ] **Step 5: 新增 `nav_select_exec` 执行函数**

在 `qingui/src/widgets/itemlist.rs` 的 `impl ItemListState` 块结束（第 29 行 `}`）之后插入：

```rust

/// NavSelect 的执行函数：Ui 在 kind 放回后调用（obj 的 kind 已还原，可安全经 ui 访问自身）。
/// 语义与旧 apply_key_outcome 的 NavSelect 分支完全一致：空列表也消费。
pub(crate) fn nav_select_exec(ui: &mut Ui, il: ObjRef, d: i32) {
    let n = ui.itemlist_len(il);
    if n > 0 {
        let cur = ui.itemlist_selected(il);
        let next = (cur as i32 + d).rem_euclid(n as i32) as usize;
        ui.itemlist_select(il, next);
    }
}
```

- [ ] **Step 6: apply_key_outcome 删除 NavSelect 分支、新增 Deferred 分支**

把 `qingui/src/ui.rs:1134-1143` 的 NavSelect 分支替换为：

```rust
            KeyOutcome::Deferred(f, p) => {
                f(self, obj, p);
                true
            }
```

（`OpenDropdown` 与 `ScrollBy` 分支保持原样，`dropdown::open` 此刻仍为 2 参签名。）

- [ ] **Step 7: 运行 ItemList 契约测试确认行为保持**

Run: `cargo test -p qingui --test itemlist`
Expected: 全部 PASS。重点确认 `keyboard_nav_wraps_and_consumes`、`empty_list_key_does_not_panic_and_consumes`、`enter_fires_clicked_but_nav_key_does_not` 仍绿。

- [ ] **Step 8: 全量编译检查无残留引用**

Run: `cargo check -p qingui`
Expected: 无错误。`rg -n "NavSelect" qingui/src` 无任何匹配。

- [ ] **Step 9: Commit**

```bash
git add qingui/src/widgets/mod.rs qingui/src/widgets/itemlist.rs qingui/src/ui.rs
git commit -m "refactor(widgets): KeyOutcome 引入 Deferred(fn,i32),ItemList 导航迁移为 Deferred 执行"
```

---

### Task 2: ScrollView 迁移为 `Deferred`

**Files:**
- Modify: `qingui/src/widgets/scrollview.rs:23-31`（on_key）、`qingui/src/widgets/scrollview.rs:31` 后新增执行函数
- Modify: `qingui/src/widgets/mod.rs:75-76`（删除 `ScrollBy` 变体与 doc）
- Modify: `qingui/src/ui.rs:1144-1147`（删除 `ScrollBy` 分支）
- Test: `qingui/tests/scrollview.rs`（契约测试，全部现有用例）

**Interfaces:**
- Consumes: Task 1 的 `KeyOutcome::Deferred(fn(&mut Ui, ObjRef, i32), i32)` 与 `apply_key_outcome` 的 Deferred 分支。
- Produces: `scrollview::scroll_by_exec(ui: &mut Ui, sv: ObjRef, delta: i32)`（pub(crate)）——滚动执行函数，封装 `UiScrollViewExt::scrollview_scroll_by`（scrollview.rs:144）。

- [ ] **Step 1: 运行 ScrollView 契约测试确认基线全绿**

Run: `cargo test -p qingui --test scrollview`
Expected: 全部 PASS（基线）。

- [ ] **Step 2: ScrollView on_key 改用 `Deferred`**

把 `qingui/src/widgets/scrollview.rs:23-31` 的 `on_key` 替换为：

```rust
    pub(crate) fn on_key(&mut self, key: Key, _ctx: KeyCtx) -> KeyOutcome {
        match key {
            Key::Up => KeyOutcome::Deferred(scroll_by_exec, -STEP),
            Key::Down => KeyOutcome::Deferred(scroll_by_exec, STEP),
            _ => KeyOutcome::Pass,
        }
    }
```

- [ ] **Step 3: 新增 `scroll_by_exec` 执行函数**

在 `qingui/src/widgets/scrollview.rs` 的 `impl ScrollViewState` 块结束（第 31 行 `}`）之后插入：

```rust

/// ScrollBy 的执行函数：Ui 在 kind 放回后调用。
pub(crate) fn scroll_by_exec(ui: &mut Ui, sv: ObjRef, delta: i32) {
    ui.scrollview_scroll_by(sv, delta);
}
```

- [ ] **Step 4: widgets/mod.rs 删除 `ScrollBy` 变体**

把 `qingui/src/widgets/mod.rs:75-76` 的两行：

```rust
    /// 滚动容器滚动(步进 ±px),由 Ui 执行(clamp + translate)
    ScrollBy(i32),
```

替换为（即删除，只保留前面的 `Deferred` 行与收尾 `}`）：

```rust
}
```

即 `KeyOutcome` 枚举的最后一个变体现在是 `Deferred(...)`，其后紧跟 `}`。

- [ ] **Step 5: apply_key_outcome 删除 `ScrollBy` 分支**

把 `qingui/src/ui.rs:1144-1147` 的分支：

```rust
            KeyOutcome::ScrollBy(d) => {
                self.scrollview_scroll_by(obj, d);
                true
            }
```

替换为（即删除）：

```rust
```

- [ ] **Step 6: 运行 ScrollView 契约测试确认行为保持**

Run: `cargo test -p qingui --test scrollview`
Expected: 全部 PASS。重点确认 `focused_up_down_scrolls_and_clamps`、`short_content_never_scrolls` 仍绿。

- [ ] **Step 7: 全量编译检查无残留引用**

Run: `cargo check -p qingui`
Expected: 无错误。`rg -n "ScrollBy" qingui/src` 无任何匹配。

- [ ] **Step 8: Commit**

```bash
git add qingui/src/widgets/scrollview.rs qingui/src/widgets/mod.rs qingui/src/ui.rs
git commit -m "refactor(scrollview): 滚动副作用迁移为 Deferred 执行,删除 ScrollBy 变体"
```

---

### Task 3: Dropdown 迁移为 `Deferred`

**Files:**
- Modify: `qingui/src/widgets/dropdown.rs:21-25`（on_key）、`qingui/src/widgets/dropdown.rs:28`（`open` 签名加 `_payload: i32`）
- Modify: `qingui/src/widgets/mod.rs:72`（删除 `OpenDropdown` 变体）
- Modify: `qingui/src/ui.rs:1130-1133`（删除 `OpenDropdown` 分支）
- Test: `qingui/tests/p1_widgets.rs`（契约测试，含 `dropdown_open_select_close`）

**Interfaces:**
- Consumes: Task 1 的 `Deferred` 变体。`dropdown::open` 现在是唯一的 pub(crate) 消费者入口。
- Produces: `dropdown::open(ui: &mut Ui, obj: ObjRef, _payload: i32)`（pub(crate)，签名从 2 参改为 3 参以匹配 `fn(&mut Ui, ObjRef, i32)`）——直接作为 `Deferred` 的 fn 指针使用，无需包装函数。

- [ ] **Step 1: 运行 Dropdown 契约测试确认基线全绿**

Run: `cargo test -p qingui --test p1_widgets`
Expected: 全部 PASS（基线）。重点 `dropdown_open_select_close`。

- [ ] **Step 2: Dropdown on_key 改用 `Deferred(open, 0)`**

把 `qingui/src/widgets/dropdown.rs:22-24` 的 `on_key` 替换为：

```rust
    pub(crate) fn on_key(&mut self, key: Key, _ctx: super::KeyCtx) -> super::KeyOutcome {
        if key == Key::Enter { super::KeyOutcome::Deferred(open, 0) } else { super::KeyOutcome::Pass }
    }
```

- [ ] **Step 3: `open` 签名增加 `_payload: i32`**

把 `qingui/src/widgets/dropdown.rs:28` 的：

```rust
pub(crate) fn open(ui: &mut Ui, obj: ObjRef) {
```

替换为：

```rust
pub(crate) fn open(ui: &mut Ui, obj: ObjRef, _payload: i32) {
```

（函数体与两个事件回调不变。`_payload` 前缀避免未使用告警。）

- [ ] **Step 4: widgets/mod.rs 删除 `OpenDropdown` 变体**

把 `qingui/src/widgets/mod.rs:72` 的：

```rust
    OpenDropdown,  // 打开下拉浮层
```

替换为（即删除该行）。

- [ ] **Step 5: apply_key_outcome 删除 `OpenDropdown` 分支**

把 `qingui/src/ui.rs:1130-1133` 的分支：

```rust
            KeyOutcome::OpenDropdown => {
                crate::widgets::dropdown::open(self, obj);
                true
            }
```

替换为（即删除）。

- [ ] **Step 6: 运行 Dropdown 契约测试确认行为保持**

Run: `cargo test -p qingui --test p1_widgets`
Expected: 全部 PASS。`dropdown_open_select_close` 覆盖：Enter 打开浮层（模态）、Down+Enter 选中写回并发 ValueChanged、焦点还原、Esc 关闭不改值——全部仍绿。

- [ ] **Step 7: 全量编译检查无残留引用**

Run: `cargo check -p qingui`
Expected: 无错误。`rg -n "OpenDropdown" qingui/src` 无任何匹配。

- [ ] **Step 8: Commit**

```bash
git add qingui/src/widgets/dropdown.rs qingui/src/widgets/mod.rs qingui/src/ui.rs
git commit -m "refactor(dropdown): 打开浮层迁移为 Deferred 执行,删除 OpenDropdown 变体"
```

---

### Task 4: 全量验证与收尾

**Files:**
- Test: 全仓库验证（无需改代码）

**Interfaces:**
- Consumes: 前三任务的成果。此任务只做验收，不改行为。

- [ ] **Step 1: 全量测试**

Run: `cargo test -p qingui`
Expected: 全部 PASS（171 个测试，一个不差）。

- [ ] **Step 2: no_std 目标编译**

Run: `cargo build -p qingui --target thumbv7em-none-eabihf`
Expected: 编译成功。（若报 target 未安装：`rustup target add thumbv7em-none-eabihf` 后重试。）

- [ ] **Step 3: examples 编译**

Run: `cargo check -p qingui --examples`
Expected: 无错误（demo/gallery 正常）。

- [ ] **Step 4: 确认 Ui 不再持有 widget 语义**

Run: `rg -n "OpenDropdown|NavSelect|ScrollBy|Deferred" qingui/src`
Expected: 匹配仅出现在 `qingui/src/widgets/`（枚举定义、各 widget on_key、执行函数）——`qingui/src/ui.rs` 中不再有任何 `KeyOutcome::` 之外的 widget 语义引用。`apply_key_outcome` 的 `Deferred(f, p) => { f(self, obj, p); true }` 是 Ui 对 widget 特异的唯一接触点。

- [ ] **Step 5: 最终提交（如需）**

Run: `git status --short`
Expected: 工作区干净（前三任务已提交，无遗留改动）。

---

## Self-Review

**Spec 覆盖：**
- `Deferred(fn(&mut Ui, ObjRef, i32), i32)` 替换三个特异变体 → Task 1/2/3 各迁移一个，Task 4 验证无残留。
- 通用副作用（标脏/事件/EDITED）留在 Ui → 未动 `Consumed`/`ValueChanged`/`EnterEdit`/`ExitEdit`/`Pass`。
- 零分配、fn 指针静态执行 → 贯穿各任务，无 `Box`。
- Ui 无 widget 语义 → Task 4 Step 4 用 grep 验收。
- 执行窗口在 kind 放回后 → `call_on_key` 未改动，`apply_key_outcome` 在放回后调用 Deferred，天然满足。

**占位符扫描：** 无 TBD/TODO；每个代码步骤给出完整可粘贴代码。

**类型一致性：**
- `Deferred` 签名三处一致：`mod.rs` 枚举、`apply_key_outcome` 的 `f(self, obj, p)`、三个执行函数 `(ui, obj, payload: i32)`。
- `nav_select_exec`/`scroll_by_exec` 为新增包装函数；dropdown 直接复用改签名的 `open`，无第二套命名。
- 载荷语义一致：nav ±1、scroll ±STEP、dropdown 0（弃用）。
- Task 2/3 中 `OpenDropdown`/`ScrollBy` 分支在各自迁移前保持 2 参 `open(self, obj)` 与 `scrollview_scroll_by` 调用，跨任务无签名漂移。
