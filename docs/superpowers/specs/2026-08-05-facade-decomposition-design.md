# qingui Ui Facade 分解设计：纯子系统（renderer/anim/focus/layout-math）

日期：2026-08-05
状态：已获用户批准（讨论式 brainstorming 后确认）

## 背景与动机

qingui 的 `Ui` 是"结构化数据 + 中央解释器"架构下的唯一可变世界（arena 独占者）。经过前几轮讨论确认：

- **god-object 不是设计失误**，是"安全 Rust + 保留模式 + 拒绝 RefCell"三者的必然结果——`&mut Ui` 逃逸点只有三类：事件回调 `EventCb = Box<dyn FnMut(&mut Ui, ...)>`、动画完成回调 `Box<dyn FnMut(&mut Ui)>`、tick hook / custom on_key。凡派发用户回调的路径必须握着整个 Ui。
- **但"到纯的距离"各处不同**：renderer 函数体已纯（只读 arena/buf/dirty/flush/font/time 六个不相交字段，`kind.draw` 与 `draw_hook` 都不收 `&mut Ui`）；anim 的插值数学纯、驱动器被 on_done 钉死；focus 的索引计算纯、副作用在 Ui；layout 引擎今天就是 `layout_flex(&mut Ui, ...)`（真正绑定 Ui），但其纯数学已是 layout.rs 私有自由函数。

目标：**可测试性**——把纯子系统抽成自由函数，用 fixture arena 直接单测，不建完整 Ui。

## 目标与非目标

**目标**：
- renderer 全拆为 `src/render.rs` 自由函数，脱离 Ui 单测。
- anim 插值数学抽为 `anim::eval` 纯函数，单测。
- focus 索引计算抽为 `focus::step` 纯函数，单测。
- layout 引擎不动，仅给已有纯数学补 `#[cfg(test)]` 单测。
- `ui.rs` 减重 ~150 行，变协调者。

**非目标**：
- 不拆 `send_event`、`tick_widgets`、`step_anims` 驱动器、`call_on_key`、layout 引擎（被 `&mut Ui` 逃逸点钉死）。
- 不引入 ctx struct（方案 B/C 否决：LayoutCtx 会膨胀成第二个 Ui，mini-god-object 风险）。
- 不改变任何公开 API 与可观察行为（纯内部重构，187 集成测试即回归契约）。
- 不改变存储模型（arena 仍是 Ui 唯一字段持有者）。

## 设计

### 1. 模块图

| 模块 | 状态 | 内容 | 脱离 Ui |
|---|---|---|---|
| `src/render.rs` | 新建 | renderer 自由函数 + 共享纯助手 `abs_rect`/`resolved_style` | ✅ |
| `src/anim.rs` | 修改 | `AnimEval` + `eval()` 纯函数 | ✅ 数学 |
| `src/focus.rs` | 新建 | `step()` 纯函数 | ✅ 簿记 |
| `src/layout.rs` | 修改 | 纯数学补 `#[cfg(test)]` 单测；引擎不动 | ⬜ 只测数学 |
| `src/ui.rs` | 修改 | 删 render ~110 行 + step_anims 数学 ~40 行；`render`/`abs_rect`/`resolved_style` 变一行委托；focus 簿记变委托 | 协调者 |

### 2. renderer 子系统（`src/render.rs`）

入口（搬 ui.rs:739-840，`self.xxx` 换显式参数，行为逐字节一致）：

```rust
pub(crate) fn render(
    arena: &mut Arena<Node>,
    buf: &mut [Color],
    dirty: &mut DirtyQueue,
    flush: &mut Option<Box<dyn Flush>>,
    font: &'static MonoFont<'static>,
    time_ms: u64,
)
```

私有逐级函数：

```rust
fn render_area(arena: &mut Arena<Node>, buf: &mut [Color], area: Rect, font: &'static MonoFont<'static>, time_ms: u64)
fn render_chunk(arena: &mut Arena<Node>, buf: &mut [Color], chunk: Rect, flush: &mut Option<Box<dyn Flush>>, font: &'static MonoFont<'static>, time_ms: u64)
fn draw_node(arena: &mut Arena<Node>, buf: &mut [Color], obj: ObjRef, frame: Rect, clip: Rect, len: usize, font: &'static MonoFont<'static>, time_ms: u64)
fn children_z_sorted(arena: &Arena<Node>, obj: ObjRef) -> Vec<ObjRef>
fn node_draw_info(arena: &Arena<Node>, obj: ObjRef, font: &'static MonoFont<'static>) -> Option<(Rect, Flag, u8, ResolvedStyle)>
```

共享纯助手（layout 引擎/浮层也在用，`pub(crate)`，Ui 委托）：

```rust
pub(crate) fn abs_rect(arena: &Arena<Node>, obj: ObjRef) -> Rect
pub(crate) fn resolved_style(arena: &Arena<Node>, obj: ObjRef, font: &'static MonoFont<'static>) -> ResolvedStyle
```

Ui 侧三个方法变一行委托（公开签名不变，split borrow 传不相交字段）：

```rust
pub fn render(&mut self) {
    render::render(&mut self.arena, &mut self.buf, &mut self.dirty, &mut self.flush, self.default_font, self.time_ms)
}
pub fn abs_rect(&self, obj: ObjRef) -> Rect { render::abs_rect(&self.arena, obj) }
pub fn resolved_style(&self, obj: ObjRef) -> ResolvedStyle { render::resolved_style(&self.arena, obj, self.default_font) }
```

实现细节：
- `draw_node` 内 `self.state(obj)` → 私有 `node_state(arena, obj)`；`self.children(obj)` → 私有 `kids(arena, obj)`（renderer 专用，不碰 Ui 公开方法）。
- `draw_hook` 不收 `&mut Ui`（现 ui.rs:808 `hook(&mut d, abs, clip, now)`），故 draw_node 可保持纯——renderer 能全拆的根本前提。
- `time_ms`/`font` 一路传参，不引入任何结构体。

### 3. anim 子系统（`src/anim.rs`）

插值数学抽纯（对应 ui.rs:647-678）：

```rust
pub(crate) enum AnimEval { Delay, Keep(i32), Done(i32) }   // Copy
pub(crate) fn eval(a: &Anim, start_time: u64, now: u64) -> AnimEval
```

`eval` 只算值，`&Anim` 只读，`on_done` 回调不碰。Ui 的 `step_anims` 驱动器保留（被 `on_done(&mut Ui)` 钉死），改为：

```rust
let ev = { let r = &self.anims[i]; anim::eval(&r.anim, r.start_time, now) };
match ev {
    AnimEval::Delay => i += 1,
    AnimEval::Keep(v) => { let prop = self.anims[i].anim.prop; self.apply_anim_value(target, prop, v); i += 1; }
    AnimEval::Done(v) => {
        let r = self.anims.remove(i);
        self.apply_anim_value(r.anim.target, r.anim.prop, v);
        if let Some(mut cb) = r.anim.on_done.take() { cb(self); }
    }
}
```

### 4. focus 子系统（`src/focus.rs`）

索引循环抽纯（对应 ui.rs:960-985）：

```rust
pub(crate) fn step(group: &[ObjRef], focused: Option<usize>, dir: i32, valid: impl Fn(ObjRef) -> bool) -> Option<usize>
```

语义 = 现行逻辑：空组 → `None`；`base = focused.unwrap_or(0)`；`for k in 1..=len { let idx = (base + dir * k).rem_euclid(len); if valid(group[idx]) { return Some(idx) } }` → 全不可选 → `None`。`dir` 为 `+1`/`-1`，由 `rem_euclid` 统一取模。

Ui 侧委托（`focusable` 留 Ui，它要 arena/modal/is_hidden_eff）：

```rust
pub fn group_focus_next(&mut self) {
    if let Some(i) = focus::step(&self.group, self.focused_idx, 1, |o| self.focusable(o)) {
        self.focus_to(i);
    }
}
pub fn group_focus_prev(&mut self) { /* dir = -1，其余同 */ }
```

### 5. layout 子系统（`src/layout.rs`）

引擎 `layout_flex(ui: &mut Ui, ...)`/`layout_grid` 保持不动。给已为纯自由函数的私有函数补 `#[cfg(test)] mod tests`：`axis_basis`、`distribute`、`align_offset`、`track_offset`、`solve_tracks`。全数值断言，无 Ui、无 arena。

### 6. 数据流（重构后 timer_handler）

```rust
pub fn timer_handler(&mut self) -> u32 {
    self.step_anims();                                   // Ui 驱动器（on_done 绑定）
    if self.layout_dirty { self.layout_pass(); self.layout_dirty = false; }  // layout 引擎（不动）
    self.layout_floating(self.screen);                   // 浮层（不动）
    let fx_active = self.tick_widgets();                 // tick_hook 绑定
    render::render(&mut self.arena, &mut self.buf, &mut self.dirty,
                   &mut self.flush, self.default_font, self.time_ms);  // 纯子系统
    if self.anim_running() || fx_active { 0 } else { u32::MAX }
}
```

### 7. 测试策略（新约定）

仓库现有 187 测试全是 `tests/*.rs` 集成测试（建完整 Ui）。纯子系统函数为 `pub(crate)`，集成测试调不到，故引入**模块内 `#[cfg(test)] mod tests` 单元测试**（标准 Rust 做法，不扩大公开 API）：

- **render.rs**：fixture 直接 `Arena::new()` + `Node::new(Some(parent), rect, WidgetKind::Obj(ObjState))` + 手动 `children.push` 建 2-3 层树，配 `FakeFlush`（仿现有 `RecFlush` 收集 `(Rect, Vec<Color>)`）。断言：背景色、Label 文字像素、CLIP_CHILDREN 裁剪、z_index 叠放、HIDDEN 跳过、`abs_rect` 的父子/translate 链。
- **anim.rs**：fixture 直接 `Anim::new(...)`。断言：delay 窗口、easing 端点与中点（linear t=0.5 → 中值）、repeat 次数、playback 奇偶反转、done 值、repeat<0 无限。
- **focus.rs**：纯 `Vec<ObjRef>` + 闭包 predicate。断言：wrap 循环、跳不可选、空组、`focused=None` 起步、全不可选 → None。
- **layout.rs**：五个纯函数数值断言。

## 验收标准

1. `cargo test -p qingui` 全绿（187 集成测试一个不差，行为逐字节不变）。
2. `cargo build -p qingui --target thumbv7em-none-eabihf` 通过（no_std 不破坏）。
3. `cargo check -p qingui --examples` 通过。
4. 四个模块各有 `#[cfg(test)]` 单测，全部不依赖 Ui（源码里 `tests` mod 不出现 `Ui`）。
5. `ui.rs` 减 ~150 行（render ~110 + step_anims 数学 ~40）。
6. 公开 API 零变化（`render`/`abs_rect`/`resolved_style`/`group_focus_next`/`group_focus_prev` 签名与语义不变）。

## 影响面（预估）

- 新建：`qingui/src/render.rs`、`qingui/src/focus.rs`。
- 修改：`qingui/src/anim.rs`（+AnimEval/+eval/+tests）、`qingui/src/layout.rs`（+tests）、`qingui/src/ui.rs`（删 render 块与 step_anims 数学、方法变委托、删对应私有助手）。
- `qingui/src/lib.rs`：新增 `pub mod render; pub mod focus;`（render/focus 内部含 widget 绘制分派，需确认可见性——render 调 `WidgetKind::draw` 与 `WidgetBehavior`，二者 `pub(crate)`，render.rs 与 ui.rs 同 crate，无可见性问题）。

## 风险与对策

- **行为漂移**：搬移必须逐字节等价。对策：187 集成测试是回归契约；renderer 测试从第 1 个 commit 就开始像素级断言。
- **split borrow 不可行处**（如某函数同时要 `&mut arena` 与 Ui 方法）：若实现时发现，退回"impl Ui 分文件"而非 ctx struct（YAGNI）。
- **`draw_hook` 未来若改签名收 `&mut Ui`**：renderer 纯性即破坏；当前签名不收 Ui，维持不变（本次不触及）。
