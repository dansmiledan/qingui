# Widget Trait Object 化重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the macro-generated `WidgetKind` enum with a single `Box<dyn Widget>` trait-object model on `Node`, making user widgets first-class, while slimming `Style` to pure visuals, deleting `z_index` (children order = z-order), and upgrading `DrawBuf` to a public `Canvas` with an `embedded-graphics` `DrawTarget` adapter.

**Architecture:** Dual-track incremental migration (spec: `docs/superpowers/specs/2026-08-08-widget-trait-object-design.md`). Batch 0 builds the skeleton: a unified `Widget` trait with take-out `&mut Ui` dispatch, plus a compatibility shim `impl Widget for WidgetKind` so all 19 legacy widgets keep compiling. Batches 1-4 migrate widgets group by group (each migration = delete one enum variant + standalone `impl Widget`). Batch 5 lands `Canvas` + eg `DrawTarget` and deletes the macro, the enum, and the shim.

**Tech Stack:** Rust 2021, `no_std` + `alloc`, `embedded-graphics 0.8` (already a dependency), `bitflags 2`. Test gates: `cargo test -p qingui`, `cargo check --all-targets -p qingui`, `cargo bench -p qingui --bench memory` / `--bench time`.

## Global Constraints

- `no_std`: only `core` + `alloc`; no `std` anywhere under `qingui/src`.
- Code comments in English; commit messages in English, Conventional Commits (`refactor:`/`feat:`/`test:`/`docs:`).
- Local commits only — never `git push`; commit once per task.
- Behavior preservation: existing tests keep their assertion semantics; adapt only API call sites. Rendering/layout output must stay pixel- and geometry-identical.
- Every task ends green: `cargo test -p qingui` AND `cargo check --all-targets -p qingui` (examples and benches must compile too).
- `Widget` is object-safe: no generic methods on the trait. All type-specific access goes through `as_any`/`as_any_mut` downcast.
- Legacy-state names stay (`SliderState`, `ListState`, ...); the unified accessor is `Ui::widget::<T>(obj)`.

## Deviations from the spec (approved-design amendments, recorded here)

1. `measure` gets a **default implementation** `(0, 0)` instead of being required. Rationale discovered during planning: the current layout pipeline never calls measure — content sizing is precomputed by builders into the node rect, and `layout_flex`/`layout_grid` read `rect.w/h` as content size. Requiring `measure` would force 19 dead implementations. The hook exists for future intrinsic-sizing work; `Label` overrides it as the reference implementation.
2. The spec's batch 0 "临时适配层" is concretely `impl Widget for WidgetKind` (the old enum boxes itself as a trait object). Deleted in Task 22 when the last variant is gone.
3. `KeyOutcome::Deferred` is kept during migration (ScrollView/Dropdown/Msgbox use it) and deleted in Task 22 after those widgets call `&mut Ui` directly.
4. A temporary bridge field `Node.layout: Option<Layout>` holds the container layout config between Task 1 (where `Style.layout` is deleted) and Task 9 (where `FlexLayout`/`GridLayout` kinds replace it).

---

## Batch 0 — 骨架

### Task 1: Node/Style 字段重排（原子编译单元）

**Files:**
- Modify: `qingui/src/style.rs`（Style/ResolvedStyle/resolve/merge/theme_*）
- Modify: `qingui/src/node.rs`（Node 字段 + Node::new）
- Modify: `qingui/src/ui.rs`（LayoutStyle → LayoutProps、setter 群、anim Opa）
- Modify: `qingui/src/layout.rs`（layout_flex/layout_grid 读 Node 字段）
- Modify: `qingui/src/render.rs`（opa 走 resolved_style）
- Modify: `qingui/src/widgets/builder.rs`（CommonBuilder）
- Test: `qingui/tests/style.rs`, `qingui/tests/layout_sizing.rs`, `qingui/tests/layout_transition.rs`, `qingui/tests/flex.rs`, `qingui/tests/grid.rs`（API 适配）

**Interfaces:**
- Consumes: 现状全部代码。
- Produces:
  - `Node { pad: (i32,i32,i32,i32), sizing_w: Option<Sizing>, sizing_h: Option<Sizing>, aspect_ratio: Option<u32>, transition: Option<(u32, Easing)>, item_props: ItemProps, layout: Option<Layout>, .. }`（无 `opa`/`z_index`/`grid_col`/`grid_row`）
  - `pub enum ItemProps { None, Grid { col: (u8,u8), row: (u8,u8) } }`（node.rs）
  - `Style { bg_color, bg_opa, border_color, border_width, radius, text_color, font, opa }`（全部 `Option`，无 pad/sizing/layout/aspect/transition）
  - `Ui::set_pad(obj, (i32,i32,i32,i32))`、`Ui::pad(obj) -> (i32,i32,i32,i32)`、`Ui::set_sizing`（改写 Node 字段）、`Ui::set_aspect`、`Ui::set_transition`、`Ui::set_layout`（改写 `Node.layout`）
  - `Ui::layout_props(obj) -> LayoutProps`（pub(crate)，直接读 Node 字段，无 overlay 解析）
  - `WidgetBuilder::pads(v)` / `WidgetBuilder::pad(l,r,t,b)` / `WidgetBuilder::aspect(ratio)`

- [ ] **Step 1: style.rs — Style 纯视觉化**

删除 `Style` 的 `pad_left/pad_right/pad_top/pad_bottom/layout/sizing_w/sizing_h/aspect_ratio/transition` 字段及其 builder 方法（`pads/pad/sizing/aspect/transition/layout`），删除 `pub enum Layout` 之上的 re-export 依赖（`Layout` enum 本身**保留**，挪到 `layout.rs`：`pub use crate::layout::{Layout}` 中 `Layout { None, Flex(Flex), Grid(Grid) }` 定义从 style.rs 移到 layout.rs，style.rs 不再持有它）。新增字段：

```rust
/// Node opacity multiplier (0..=255), applied to everything the node draws.
pub opa: Option<u8>,
```

`merge()` 同步：删 pad/sizing/layout/aspect/transition 五行，加 `if other.opa.is_some() { self.opa = other.opa; }`。

`ResolvedStyle`：删 `pad_*/layout/sizing_*/aspect_ratio/transition`，加：

```rust
/// Node opacity multiplier (0..=255).
pub opa: u8,
```

`ResolvedStyle::default()`：`opa: 255`，删掉被删字段的初始化。`resolve()`：删对应 pick 行，加 `opa: pick_u8(overlay, |s| s.opa).unwrap_or(d.opa),`。

- [ ] **Step 2: node.rs — Node 新字段**

```rust
use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::layout::{Layout, Sizing};

/// Per-child layout constraints, consumed by the parent's layout widget.
/// Follows the child's lifecycle (no parent-side table to clean up).
pub enum ItemProps {
    /// The parent's layout consumes no constraints (default).
    None,
    /// Grid cell placement: (start, span) per axis.
    Grid { col: (u8, u8), row: (u8, u8) },
}

pub struct Node {
    pub parent: Option<ObjRef>,
    pub children: Vec<ObjRef>,
    pub rect: Rect,
    pub state: State,
    pub flags: Flag,
    pub kind: WidgetKind,               // Task 3 改 Box<dyn Widget>
    pub style: crate::style::Style,
    pub style_pressed: Option<alloc::boxed::Box<crate::style::Style>>,
    pub style_focused: Option<alloc::boxed::Box<crate::style::Style>>,
    pub style_selected: Option<alloc::boxed::Box<crate::style::Style>>,
    pub events: Vec<(crate::event::EventKind, crate::event::EventCb)>,
    pub draw_hook: Option<DrawHook>,
    pub tick_hook: Option<TickHook>,
    /// Padding (l, r, t, b): layout input, content origin offset.
    pub pad: (i32, i32, i32, i32),
    /// Width sizing strategy (None = content size).
    pub sizing_w: Option<Sizing>,
    /// Height sizing strategy (None = content size).
    pub sizing_h: Option<Sizing>,
    /// Aspect ratio (per-mille: 1000 = 1:1).
    pub aspect_ratio: Option<u32>,
    /// Layout transition: (duration ms, easing).
    pub transition: Option<(u32, crate::anim::Easing)>,
    /// Container layout config (bridge field; replaced by layout widget kinds in Task 9).
    pub layout: Option<Layout>,
    /// Per-child layout constraints consumed by the parent.
    pub item_props: ItemProps,
    pub translate: crate::geometry::Point,
    pub floating: Option<(ObjRef, crate::layout::Attach)>,
    /// Stacking order (Task 2 deletes this field together with `Ui::set_z_index`).
    pub z_index: i16,
    pub laid_out: bool,
}
```

`Node::new`：`pad: (0,0,0,0)`, `sizing_w/sizing_h: None`, `aspect_ratio: None`, `transition: None`, `layout: None`, `item_props: ItemProps::None`；删 `opa`/`grid_col`/`grid_row` 初始化（`z_index: 0` 保留到 Task 2）。

- [ ] **Step 3: ui.rs — LayoutStyle 替换为 LayoutProps + setter 群改造**

删除 `LayoutStyle` 与 `layout_style()`，替换为：

```rust
/// The layout-relevant node props, read directly (they live on Node now: no
/// overlay resolution — state overlays can no longer change layout, by design).
#[derive(Clone, Copy, Default)]
pub(crate) struct LayoutProps {
    pub pad: (i32, i32, i32, i32),
    pub sizing_w: Option<crate::layout::Sizing>,
    pub sizing_h: Option<crate::layout::Sizing>,
    pub aspect_ratio: Option<u32>,
    pub transition: Option<(u32, crate::anim::Easing)>,
}

impl Ui {
    pub(crate) fn layout_props(&self, obj: ObjRef) -> LayoutProps {
        let Some(n) = self.arena.get(obj) else { return LayoutProps::default() };
        LayoutProps { pad: n.pad, sizing_w: n.sizing_w, sizing_h: n.sizing_h, aspect_ratio: n.aspect_ratio, transition: n.transition }
    }

    /// Sets padding (l, r, t, b).
    pub fn set_pad(&mut self, obj: ObjRef, pad: (i32, i32, i32, i32)) {
        if let Some(n) = self.arena.get_mut(obj) { n.pad = pad; }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    /// Returns padding (l, r, t, b).
    pub fn pad(&self, obj: ObjRef) -> (i32, i32, i32, i32) {
        self.arena.get(obj).map(|n| n.pad).unwrap_or((0, 0, 0, 0))
    }
}
```

改造现有 setter（函数体换落点，签名不变，全部保留 `self.layout_dirty = true`）：`set_sizing` 写 `n.sizing_w/n.sizing_h`；`set_aspect` 写 `n.aspect_ratio`；`set_transition` 写 `n.transition`；`set_layout` 写 `n.layout = Some(layout)`（参数类型改 `crate::layout::Layout`）。

`grid_cell`/`set_grid_cell` 改读写 `item_props`：

```rust
pub fn grid_cell(&self, obj: ObjRef) -> ((u8, u8), (u8, u8)) {
    match self.arena.get(obj).map(|n| &n.item_props) {
        Some(crate::node::ItemProps::Grid { col, row }) => (*col, *row),
        _ => ((0, 1), (0, 1)),
    }
}
pub fn set_grid_cell(&mut self, obj: ObjRef, col: (u8, u8), row: (u8, u8)) {
    if let Some(n) = self.arena.get_mut(obj) {
        n.item_props = crate::node::ItemProps::Grid { col: (col.0, col.1.max(1)), row: (row.0, row.1.max(1)) };
    }
    self.layout_dirty = true;
}
```

`apply_anim_value` 的 `AnimProp::Opa` 分支改写 style：

```rust
AnimProp::Opa => {
    self.invalidate_obj(target);
    if let Some(n) = self.arena.get_mut(target) {
        n.style.opa = Some(v.clamp(0, 255) as u8);
    }
    self.invalidate_obj(target);
}
```

`layout_move`/`layout_resize` 中 `self.layout_style(obj).transition` 改 `self.layout_props(obj).transition`。

`layout_subtree` 中读 `n.style.layout` 改读 `n.layout`（克隆逻辑不变）。

新增 opa 便捷 API：

```rust
/// Sets the node opacity multiplier (0..=255) via the base style.
pub fn set_opa(&mut self, obj: ObjRef, opa: u8) {
    self.invalidate_obj(obj);
    if let Some(n) = self.arena.get_mut(obj) { n.style.opa = Some(opa); }
    self.invalidate_obj(obj);
}
```

- [ ] **Step 4: layout.rs / render.rs 读路径**

layout.rs：`layout_flex` 中 `let style = ui.layout_style(container);` 改 `let lp = ui.layout_props(container);`，`style.pad_left` 等四值改 `lp.pad.0/.1/.2/.3`；子循环里 `let ls = ui.layout_style(k);` 改 `let ls = ui.layout_props(k);`，`ls.sizing_w` 等同名使用不变，`ls.aspect_ratio` 不变。`layout_grid` 同理（`style.pad_left` → `lp.pad.0`，`ui.layout_style(k)` → `ui.layout_props(k)`）。文件顶部加 `pub enum Layout { None, Flex(Flex), Grid(Grid) }` + `impl Default`（自 style.rs 搬来）。

render.rs：`node_draw_info` 中 `n.opa` 改为先算 `resolved` 再取 `resolved.opa`（调整 tuple 构造顺序：先 `resolved_style(...)`，再从中取 opa）。

- [ ] **Step 5: builder.rs — CommonBuilder 改造**

```rust
#[derive(Default)]
pub(crate) struct CommonBuilder {
    pub size: Option<(i32, i32)>,
    pub style: Option<Style>,
    pub style_pressed: Option<Style>,
    pub style_focused: Option<Style>,
    pub layout: Option<crate::layout::Layout>,
    pub pad: Option<(i32, i32, i32, i32)>,
    pub sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    pub aspect: Option<u32>,
    pub transition: Option<(u32, Easing)>,
    pub events: Vec<(EventKind, EventCb)>,
}

impl CommonBuilder {
    pub fn apply_tail(self, ui: &mut Ui, r: ObjRef) {
        if let Some(l) = self.layout { ui.set_layout(r, l); }
        if let Some(p) = self.pad { ui.set_pad(r, p); }
        if let Some((sw, sh)) = self.sizing { ui.set_sizing(r, sw, sh); }
        if let Some(a) = self.aspect { ui.set_aspect(r, Some(a)); }
        if let Some(t) = self.transition { ui.set_transition(r, Some(t)); }
        for (k, cb) in self.events { ui.add_event_cb(r, k, cb); }
    }
}
```

`WidgetBuilder` 新增：

```rust
/// Sets padding on all four sides.
pub fn pads(mut self, v: i32) -> Self { self.common.pad = Some((v, v, v, v)); self }
/// Sets padding per side: (left, right, top, bottom).
pub fn pad(mut self, l: i32, r: i32, t: i32, b: i32) -> Self { self.common.pad = Some((l, r, t, b)); self }
/// Sets the aspect ratio (per-mille).
pub fn aspect(mut self, ratio: u32) -> Self { self.common.aspect = Some(ratio); self }
```

- [ ] **Step 6: 全仓编译修复（机械替换规则）**

对以下调用点按规则逐一修改（用 grep 找全）：

- `rg 'layout_style' qingui/src qingui/tests` → 只剩 ui.rs 定义处（已删）与 layout.rs（已改）。
- `rg '\.pads?\(|\.sizing\(|\.aspect\(|\.transition\(|\.layout\(' qingui/examples qingui/tests qingui/benches`：`Style` 上的这些方法调用全部移除，等价语义改用 builder 方法（`.pads(8)` 直接挂在 builder 上）或 build 后 `ui.set_pad(...)`。
- `rg 'resolved\.(pad_|layout|sizing_|aspect|transition)' qingui/src` → 逐一改走 `ui.layout_props` 或 `ui.pad`。
- `rg 'theme_' qingui/src/style.rs`：各 `theme_*` 函数若引用了被删的 Style 字段（如 `Style::new().layout(...)`），改为只保留视觉字段；调用方需要的布局默认值改由 builder 落点表达。
- `rg '\.opa' qingui/src qingui/tests`：`n.opa` 残留全改 `resolved.opa` / `style.opa`。
- `rg 'z_index|set_z_index' qingui/src qingui/tests qingui/examples`：本任务先**保留** `set_z_index`（Node.z_index 字段也保留），Task 2 才删。

- [ ] **Step 7: 测试适配并跑绿**

`qingui/tests/style.rs` 中断言 Style/ResolvedStyle 字段的用例按新字段集更新；`layout_sizing.rs`/`layout_transition.rs`/`flex.rs`/`grid.rs` 中经由 Style 设置布局参数的调用改为 builder/ui setter。

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor: move layout props from Style to Node, opa into Style"
```

---

### Task 2: z_index 删除 / children 即 z 序

**Files:**
- Modify: `qingui/src/node.rs`（删字段）
- Modify: `qingui/src/render.rs`（删 children_z_sorted，按索引直遍历）
- Modify: `qingui/src/ui.rs`（删 set_z_index，加 move_to_front/back）
- Modify: `qingui/src/widgets/msgbox.rs`, `qingui/src/widgets/dropdown.rs`（z_index 调用点）
- Test: `qingui/tests/render.rs`（叠放顺序用例）

**Interfaces:**
- Consumes: Task 1 的 Node。
- Produces: `Ui::move_to_front(obj)`、`Ui::move_to_back(obj)`；render 按 `children` 顺序直接遍历（靠后在上层）。

- [ ] **Step 1: 先写失败测试**

在 `qingui/tests/render.rs` 加：

```rust
#[test]
fn move_to_front_raises_stacking() {
    // 两个重叠兄弟：后创建的 B 覆盖 A；move_to_front(A) 后 A 覆盖 B
    let mut ui = Ui::new(20, 20, 20);
    let scr = ui.screen();
    let a = ObjCfg::new().size(10, 10).build(&mut ui, scr);
    let b = ObjCfg::new().size(10, 10).build(&mut ui, scr);
    ui.set_style(a, { let mut s = Style::default(); s.bg_color = Some(Color::rgb(255,0,0)); s.bg_opa = Some(255); s });
    ui.set_style(b, { let mut s = Style::default(); s.bg_color = Some(Color::rgb(0,0,255)); s.bg_opa = Some(255); s });
    // 用 take_dirty + render 后像素的判定可走 ui.render() + 自定义 Flush（参照本文件既有 fixture）
    // 初始：B 在上 → (5,5) 为蓝
    ui.move_to_front(a);
    // 之后：A 在上 → (5,5) 为红
}
```

（像素断言复用该文件已有的 Flush 录制 fixture；两处断言分别放在 move_to_front 前后各 render 一次。）

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p qingui --test render move_to_front_raises_stacking`
Expected: FAIL（`move_to_front` 不存在，编译错误）。

- [ ] **Step 3: 实现**

ui.rs：删 `set_z_index`，新增：

```rust
/// Moves `obj` to the end of its parent's children (drawn last = on top).
pub fn move_to_front(&mut self, obj: ObjRef) {
    let Some(parent) = self.arena.get(obj).and_then(|n| n.parent) else { return };
    if let Some(p) = self.arena.get_mut(parent) {
        if let Some(pos) = p.children.iter().position(|&c| c == obj) {
            let c = p.children.remove(pos);
            p.children.push(c);
        }
    }
    self.invalidate_obj(obj);
}

/// Moves `obj` to the start of its parent's children (drawn first = bottom).
pub fn move_to_back(&mut self, obj: ObjRef) {
    let Some(parent) = self.arena.get(obj).and_then(|n| n.parent) else { return };
    if let Some(p) = self.arena.get_mut(parent) {
        if let Some(pos) = p.children.iter().position(|&c| c == obj) {
            let c = p.children.remove(pos);
            p.children.insert(0, c);
        }
    }
    self.invalidate_obj(obj);
}
```

node.rs：删 `z_index` 字段与初始化。render.rs：删 `children_z_sorted` 与 `kids`；两处遍历改为按索引直遍历（零分配，同 `layout_subtree` 模式）：

```rust
let nkids = arena.get(obj).map(|n| n.children.len()).unwrap_or(0);
for i in 0..nkids {
    let Some(c) = arena.get(obj).and_then(|n| n.children.get(i).copied()) else { break };
    draw_node(arena, buf, c, frame, child_clip, len, font, time_ms);
}
```

（`render_chunk` 顶层的 `roots` 同理按索引遍历 screen 的 children。）

msgbox.rs / dropdown.rs：`rg 'set_z_index|z_index' qingui/src/widgets` 的调用点改为创建后 `ui.move_to_front(obj)`（语义等价：它们本就后创建排末尾，move_to_front 幂等保险）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: drop z_index, children order is the stacking order"
```

---

### Task 3: Widget trait + take-out 通道 + WidgetKind 适配层

**Files:**
- Modify: `qingui/src/widgets/mod.rs`（Widget trait、NoopWidget、MeasureCtx、impl Widget for WidgetKind）
- Modify: `qingui/src/node.rs`（kind: Box<dyn Widget>）
- Modify: `qingui/src/ui.rs`（insert_node、update、widget::<T>、call_on_key、tick_widgets、create_widget）
- Modify: 全部 widget 文件（build 中 `insert_node(..., WidgetKind::X(s))` → `Box::new(WidgetKind::X(s))`）
- Test: `qingui/tests/custom_widget.rs`（新增 take-out 用例）

**Interfaces:**
- Consumes: Task 1-2 的 Node/Ui。
- Produces:
  - `pub trait Widget`（下方完整定义）+ `pub struct NoopWidget`
  - `pub struct MeasureCtx { pub font: &'static MonoFont<'static>, pub cur: (i32, i32) }`
  - `Ui::insert_node(parent, rect, kind: Box<dyn Widget>) -> ObjRef`（pub(crate)）
  - `Ui::create_widget(parent, w, h, widget: Box<dyn Widget>) -> ObjRef`（pub，用户控件入口，替代 create_custom）
  - `Ui::widget::<T: 'static>(obj) -> Option<&T>`（统一只读 downcast）
  - `Ui::update<T, R>(obj, f) -> Option<R>`（双跳 downcast，兼容 legacy enum）

- [ ] **Step 1: widgets/mod.rs — trait 定义**

在 `define_widgets!` 宏之前新增（`WidgetCtx`/`TickOut`/`KeyCtx`/`KeyOutcome` 保留不动）：

```rust
// Temporary alias: Task 21 renames DrawBuf to Canvas and switches this to a re-export.
use crate::draw::DrawBuf as Canvas;

/// Measure context: read-only inputs for intrinsic content sizing.
pub struct MeasureCtx {
    /// Resolved font (node style font or the Ui default).
    pub font: &'static embedded_graphics::mono_font::MonoFont<'static>,
    /// The node's current size (layout treats it as content size today).
    pub cur: (i32, i32),
}

/// The single widget behavior interface. Node owns common data; the trait object
/// owns behavior and widget-specific data (reached via `as_any` downcast).
///
/// `draw`/`measure` take `&self` and never leave the arena. `layout`/`tick`/`on_key`
/// take `&mut self` and are called via take-out (the node temporarily holds a
/// `NoopWidget` placeholder), so they receive `&mut Ui` and may operate on any
/// other node; rules while taken out:
/// - mutate your own state directly on `self`;
/// - `ui.update(self_obj, ...)` is a silent no-op (your kind is not in the arena);
/// - deleting your own node is allowed (Ui treats the outcome as consumed).
pub trait Widget {
    /// Content drawing (background/border/opa are handled uniformly by Ui). Default: draws nothing.
    fn draw(&self, _ctx: &WidgetCtx, _c: &mut Canvas, _clip: Rect) {}
    /// Intrinsic content size; `(0, 0)` means "no intrinsic size" (layout uses the current rect).
    fn measure(&self, _ctx: &MeasureCtx) -> (i32, i32) { (0, 0) }
    /// Lays out direct children. Default: manual positioning (children keep their rects).
    fn layout(&mut self, _ui: &mut Ui, _obj: ObjRef) {}
    /// Per-frame progress. Default: idle.
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, _now: u64) -> TickOut { TickOut::IDLE }
    /// Key handling. Default: not consumed (falls through to focus move / Clicked).
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, _key: Key) -> KeyOutcome { KeyOutcome::Pass }
    /// Property-animation Value channel.
    fn value(&self) -> i32 { 0 }
    fn set_value(&mut self, _v: i32) -> bool { false }
    fn set_range(&mut self, _min: i32, _max: i32) {}
    /// Draw overflow beyond the node rect (knobs, etc.), for dirty-area expansion.
    fn overflow(&self) -> i32 { 0 }
    fn as_any(&self) -> &dyn core::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any;
}

/// Zero-sized placeholder swapped in during take-out (Box of a ZST does not allocate).
pub struct NoopWidget;

impl Widget for NoopWidget {
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

`KeyOutcome` 提升为 `pub`（trait 签名需要；`Deferred` 变体暂留，Task 22 删）。`WidgetBehavior`/`clamp_val`/`select_clamp` 保留。

- [ ] **Step 2: 适配层 impl Widget for WidgetKind**

在 `define_widgets!` 调用之后新增：

```rust
/// Compatibility shim: the legacy enum boxes itself as a trait object while
/// widgets are migrated one by one. Deleted together with the enum (Task 22).
impl Widget for WidgetKind {
    fn draw(&self, ctx: &WidgetCtx, c: &mut Canvas, clip: Rect) {
        WidgetKind::draw(self, ctx, c, clip);
    }
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, now: u64) -> TickOut {
        WidgetKind::tick(self, now)
    }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> KeyOutcome {
        let ctx = KeyCtx {
            edited: ui.state(obj).contains(crate::node::State::EDITED),
            vis_h: ui.rect(obj).h,
            now: ui.time(),
        };
        // Legacy Custom variant: user state already received `&mut Ui` — keep that path.
        if let Some(w) = self.as_custom_mut() {
            return if w.on_key(ui, obj, key) { KeyOutcome::Consumed } else { KeyOutcome::Pass };
        }
        WidgetKind::on_key(self, key, ctx)
    }
    fn value(&self) -> i32 { WidgetKind::value(self) }
    fn set_value(&mut self, v: i32) -> bool { WidgetKind::set_value(self, v) }
    fn set_range(&mut self, min: i32, max: i32) { WidgetKind::set_range(self, min, max) }
    fn overflow(&self) -> i32 { WidgetKind::overflow(self) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

- [ ] **Step 3: Node.kind Box 化 + insert_node + create_widget**

node.rs：`pub kind: alloc::boxed::Box<dyn crate::widgets::Widget>`，`Node::new(parent, rect, kind: alloc::boxed::Box<dyn crate::widgets::Widget>)`。删 `pub use crate::widgets::WidgetKind;`（调用点改直接引用 `crate::widgets::WidgetKind`）。

ui.rs：

```rust
pub(crate) fn insert_node(&mut self, parent: ObjRef, rect: Rect, kind: alloc::boxed::Box<dyn crate::widgets::Widget>) -> ObjRef {
    let r = self.arena.insert(Node::new(Some(parent), rect, kind));
    if let Some(p) = self.arena.get_mut(parent) { p.children.push(r); }
    self.invalidate_obj(r);
    self.layout_dirty = true;
    r
}

/// Mounts a user-defined widget (implementing `widgets::Widget`). This is the
/// same insertion path built-in widgets use: user widgets are first-class.
pub fn create_widget(&mut self, parent: ObjRef, w: i32, h: i32, widget: alloc::boxed::Box<dyn crate::widgets::Widget>) -> ObjRef {
    self.insert_node(parent, Rect::new(0, 0, w, h), widget)
}
```

`Ui::new` 中 screen 创建：`Box::new(WidgetKind::Obj(...))`。

- [ ] **Step 4: 统一 downcast（widget::<T> / update 双跳）**

先在宏生成的 `impl WidgetKind` 内补一个与 `downcast_mut` 同构的只读版本（同一 match 结构，`wref!` 替代 `wmut!`）：

```rust
/// Read-only counterpart of `downcast_mut` (used by `Ui::widget` during migration).
pub(crate) fn downcast_ref<T: 'static>(&self) -> Option<&T> {
    $(
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<$state>() {
            if let WidgetKind::$variant(s) = self {
                return (wref!($store, s) as &dyn core::any::Any).downcast_ref::<T>();
            }
        }
    )+
    None
}
```

然后 ui.rs：

```rust
/// Read-only access to widget state by type (returns `None` on type mismatch).
/// During migration this also reaches into the legacy `WidgetKind` enum.
pub fn widget<T: 'static>(&self, obj: ObjRef) -> Option<&T> {
    let kind = self.arena.get(obj).map(|n| &n.kind)?;
    if let Some(t) = kind.as_any().downcast_ref::<T>() {
        return Some(t);
    }
    kind.as_any()
        .downcast_ref::<crate::widgets::WidgetKind>()
        .and_then(|legacy| legacy.downcast_ref::<T>())
}
```

`update` 同理双跳：

```rust
pub fn update<T: 'static, R>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> R) -> Option<R> {
    let r = match self.arena.get_mut(obj) {
        Some(n) => {
            if let Some(t) = n.kind.as_any_mut().downcast_mut::<T>() {
                Some(f(t))
            } else if let Some(legacy) = n.kind.as_any_mut().downcast_mut::<crate::widgets::WidgetKind>() {
                legacy.downcast_mut::<T>().map(f)
            } else {
                None
            }
        }
        None => None,
    };
    if r.is_some() { self.invalidate_obj(obj); }
    r
}
```

`as_list`/`as_roller` 改一行委托：`self.widget::<crate::widgets::list::ListState>(obj)`（旧实现 `n.kind.as_list()` 已随 Box 化失效）。`kind()`/`kind_mut()` 返回类型改 `Option<&Box<dyn Widget>>`（pub(crate) 调用点同步）。`custom::<T>`/`custom_mut::<T>` 暂保留（内部走 as_custom 分支），Task 20 删。

- [ ] **Step 5: take-out 通道（call_on_key / tick_widgets）**

`call_on_key` 整体替换为：

```rust
/// Widget key handling: takes the kind out, calls its `on_key` with `&mut Ui`,
/// puts it back, then runs the common side effects.
fn call_on_key(&mut self, obj: ObjRef, key: crate::input::Key) -> bool {
    let mut kind = match self.arena.get_mut(obj) {
        Some(n) => core::mem::replace(&mut n.kind, alloc::boxed::Box::new(crate::widgets::NoopWidget)),
        None => return false,
    };
    let out = kind.on_key(self, obj, key);
    if let Some(n) = self.arena.get_mut(obj) {
        n.kind = kind;
    } else {
        return true; // the node was deleted during handling: treat as consumed
    }
    self.apply_key_outcome(obj, out)
}
```

`tick_widgets` 中 `n.kind.tick(now)` 的调用改为同样的 take-out：

```rust
let mut taken = match self.arena.get_mut(r) {
    Some(n) => core::mem::replace(&mut n.kind, alloc::boxed::Box::new(crate::widgets::NoopWidget)),
    None => continue,
};
let out = taken.tick(self, r, now);
let children = match self.arena.get_mut(r) {
    Some(n) => {
        let c = n.children.clone();
        n.kind = taken;
        c
    }
    None => continue, // node deleted during tick
};
let has_hook = self.arena.get(r).map(|n| n.tick_hook.is_some()).unwrap_or(false);
```

（`hidden` 预查与 tick_hook 段保持不变；`tick_hook` 的 take-call-put-back 已是现成模式。）

- [ ] **Step 6: 全部 builder 调用点 Box 化 + 新增 take-out 测试**

`rg 'insert_node\(' qingui/src`：每个 `insert_node(parent, rect, WidgetKind::X(s))` 改为 `insert_node(parent, rect, alloc::boxed::Box::new(WidgetKind::X(s)))`；`Ui::new`、scrollview 的 `kind_mut` 替换处、canvas.rs、`create_custom` 同步。`create_custom` 内部改调 `create_widget` 的 legacy 包装（`Box::new(WidgetKind::Custom(CustomState(widget)))`）。

在 `qingui/tests/custom_widget.rs` 新增用例（复用该文件已有的测试 widget 骨架）：

```rust
#[test]
fn on_key_receives_mut_ui_via_takeout() {
    // A widget whose on_key creates a sibling label through &mut Ui, then returns Consumed.
    // Assert: the sibling exists after keypad_input, and the widget's own state was mutated on self.
}
```

（测试 widget：`on_key` 内 `self.hit = true; let scr = ui.screen(); LabelCfg::new("x").build(ui, scr); KeyOutcome::Consumed`，经 `create_widget` 挂载、group_add、keypad_input(Enter) 后断言 `hit` 与 label 存在。）

- [ ] **Step 7: 跑测试**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿（含新用例）。

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor: box widget kind behind a single Widget trait with take-out dispatch"
```

---

### Task 4: layout trait 化桥接

**Files:**
- Modify: `qingui/src/ui.rs`（layout_subtree 走 take-out kind.layout）
- Modify: `qingui/src/widgets/mod.rs`（适配层 layout() 分发）

**Interfaces:**
- Consumes: Task 3 的 take-out 通道、`Node.layout`（Task 1）。
- Produces: `layout_subtree` 只调 `kind.layout(ui, obj)`；`impl Widget for WidgetKind::layout` 读 `Node.layout` 分发 `layout_flex`/`layout_grid`（Task 9 被 FlexLayout/GridLayout 取代后删除）。

- [ ] **Step 1: 适配层 layout()**

`impl Widget for WidgetKind` 中加：

```rust
fn layout(&mut self, ui: &mut Ui, obj: ObjRef) {
    // Bridge: read the container layout config from the node (Task 1 bridge field).
    let layout = ui.arena.get(obj).and_then(|n| match &n.layout {
        Some(crate::layout::Layout::Flex(f)) => Some(crate::layout::Layout::Flex(*f)),
        other => other.cloned(),
    });
    match layout {
        Some(crate::layout::Layout::Flex(f)) => crate::layout::layout_flex(ui, obj, &f),
        Some(crate::layout::Layout::Grid(g)) => crate::layout::layout_grid(ui, obj, &g),
        _ => {}
    }
}
```

（`ui.arena` 是 pub(crate)，widgets/mod.rs 同属 crate，可访问；`Layout::Grid` 需要 `Clone`——layout.rs 的 `Layout` derive 加 `Clone`。）

- [ ] **Step 2: layout_subtree 改 take-out**

```rust
fn layout_subtree(&mut self, obj: ObjRef) {
    let mut kind = match self.arena.get_mut(obj) {
        Some(n) => core::mem::replace(&mut n.kind, alloc::boxed::Box::new(crate::widgets::NoopWidget)),
        None => return,
    };
    kind.layout(self, obj);
    if let Some(n) = self.arena.get_mut(obj) {
        n.kind = kind;
    } else {
        return; // node deleted during layout
    }
    let nkids = self.arena.get(obj).map(|n| n.children.len()).unwrap_or(0);
    for i in 0..nkids {
        let Some(c) = self.arena.get(obj).and_then(|n| n.children.get(i).copied()) else { break };
        self.layout_subtree(c);
    }
}
```

- [ ] **Step 3: 跑测试**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿（flex/grid/scrollview/layout_transition 用例覆盖此路径）。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: dispatch container layout through the Widget trait (bridge)"
```

---

## Batch 1 — 最简链路

### Task 5: Manual（Obj）迁移

**Files:**
- Modify: `qingui/src/widgets/obj.rs`
- Modify: `qingui/src/widgets/mod.rs`（enum 删 Obj 变体、take-out 占位换 NoopWidget）
- Modify: `qingui/src/ui.rs`（Ui::new、call_on_key 旧占位残留清理）
- Modify: `qingui/src/render.rs`（测试 fixture 的 WidgetKind::Obj 引用）
- Test: `qingui/tests/tree.rs`, `qingui/tests/render.rs`

**Interfaces:**
- Consumes: Task 3/4 的 Widget trait。
- Produces: `pub struct Manual;`（obj.rs，`impl Widget`，全默认）；`ObjCfg` build 产出 `Box::new(Manual)`。

- [ ] **Step 1: obj.rs 改写**

```rust
use crate::arena::ObjRef;
use crate::geometry::Rect;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::{MeasureCtx, Widget};

/// Manual-positioning container: hosts children and (via Task 9's bridge) a layout
/// config; draws nothing itself. Replaces the old unit `ObjState`.
pub struct Manual;

impl Widget for Manual {
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

`ObjCfg::build` 中 `ui.insert_node(parent, Rect::new(0, 0, w, h), Box::new(Manual))`。`ObjState` 删除。

- [ ] **Step 2: enum 删 Obj 变体 + 引用点修复**

`widgets/mod.rs`：`define_widgets!` 删 `Obj(...)` 行。`rg 'ObjState|WidgetKind::Obj' qingui/src qingui/tests qingui/benches`：Ui::new 的 screen 改 `Box::new(Manual)`；scrollview.rs/canvas.rs 占位改 `Box::new(Manual)`；render.rs 测试 fixture 改 `Box::new(Manual)`；`impl Widget for WidgetKind` 的 Custom 分支不动。

- [ ] **Step 3: 跑测试**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: migrate Obj to the Manual widget, drop the Obj enum variant"
```

---

### Task 6: Label 迁移

**Files:**
- Modify: `qingui/src/widgets/label.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Label 变体）
- Test: `qingui/tests/label.rs`, `qingui/tests/font.rs`

**Interfaces:**
- Consumes: `Widget` trait、`MeasureCtx`、`crate::font::text_size`。
- Produces: `LabelState`（名字不变）独立 `impl Widget`；`Ui::widget::<LabelState>(obj)` 可用。

- [ ] **Step 1: label.rs 迁移**

`impl super::WidgetBehavior for LabelState` 替换为：

```rust
impl super::Widget for LabelState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(&self.text, ctx, c, clip) }
    fn measure(&self, ctx: &MeasureCtx) -> (i32, i32) {
        if self.text.is_empty() { return ctx.cur; }
        crate::font::text_size(ctx.font, &self.text)
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

`set_text`/`text` 两个自由函数中 `if let WidgetKind::Label(s) = &mut n.kind` 改为经 `ui.update::<LabelState, _>(obj, |s| { s.text = text.into(); })` + rect 写回（保持原逻辑：先量尺寸、写 text、写 rect.w/h、layout_dirty）。`build` 中 `WidgetKind::Label(...)` 改 `Box::new(LabelState { text: self.text })`。widgets/mod.rs 删 `Label(...)` 行。

- [ ] **Step 2: 跑测试**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: migrate Label to the Widget trait"
```

---

### Task 7: Button 迁移

**Files:**
- Modify: `qingui/src/widgets/button.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Button 变体）
- Test: `qingui/tests/p0_widgets.rs`, `qingui/tests/fluent_api.rs`

**Interfaces:**
- Consumes: 同 Task 6。
- Produces: `ButtonState` 独立 `impl Widget`。

- [ ] **Step 1: button.rs 迁移**

`impl super::WidgetBehavior for ButtonState` 替换为：

```rust
impl super::Widget for ButtonState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(&self.text, ctx, c, clip) }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

`build` 中 `WidgetKind::Button(...)` 改 `Box::new(ButtonState { text: self.text })`。widgets/mod.rs 删 `Button(...)` 行。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Button to the Widget trait"
```

---

### Task 8: Batch 1 benchmark 对比

**Files:**
- Modify: `docs/BENCHMARK.md`（追加对比记录）

- [ ] **Step 1: 跑 memory bench**

Run: `cargo bench -p qingui --bench memory`
记录：Node 静态尺寸新旧对比（旧值见 BENCHMARK.md 既有表）、三档场景 peak/live 对比基线。

- [ ] **Step 2: 跑 time bench**

Run: `cargo bench -p qingui --bench time`
Expected: layout/render 路径不出现数量级回退（±10% 以内可接受，超出则在提交信息中记录原因）。

- [ ] **Step 3: 红线判定 + 记录**

若三档场景峰值堆涨幅 >15%：停下来向用户汇报，评估 small-kind 内联预案（不擅自实施）。否则把数据追加到 `docs/BENCHMARK.md`（新一节 "trait-object migration, batch 1"）。

```bash
git add docs/BENCHMARK.md
git commit -m "docs: record batch-1 trait-object benchmark comparison"
```

---

## Batch 2 — 布局

### Task 9: FlexLayout + GridLayout（删 Node.layout 桥接）

**Files:**
- Create: `qingui/src/widgets/flexbox.rs`（FlexLayout）
- Create: `qingui/src/widgets/gridbox.rs`（GridLayout）
- Modify: `qingui/src/widgets/mod.rs`（mod 声明、适配层 layout() 删除）
- Modify: `qingui/src/widgets/obj.rs`（ObjCfg.layout() 桥）
- Modify: `qingui/src/widgets/scrollview.rs`（set_layout 调用点）
- Modify: `qingui/src/node.rs`（删 layout 字段）、`qingui/src/ui.rs`（删 set_layout）
- Modify: `qingui/src/layout.rs`（删 Layout enum）
- Test: `qingui/tests/flex.rs`, `qingui/tests/grid.rs`, `qingui/tests/scrollview.rs`, `qingui/examples/*.rs`

**Interfaces:**
- Consumes: `layout_flex(ui, obj, &Flex)`、`layout_grid(ui, obj, &Grid)`（签名不变）。
- Produces:
  - `pub struct FlexLayout { pub flex: Flex }`（flexbox.rs，`impl Widget`：layout 调 `layout_flex`）
  - `pub struct GridLayout { pub grid: Grid }`（gridbox.rs，`impl Widget`：layout 调 `layout_grid`）
  - `Ui::set_flex(obj, Flex)`、`Ui::set_grid(obj, Grid)`（替换 kind，供运行时已建节点改布局——保留旧 set_layout 的唯一真实用途）

- [ ] **Step 1: 两个 layout widget**

flexbox.rs：

```rust
use crate::arena::ObjRef;
use crate::layout::Flex;
use crate::ui::Ui;
use super::Widget;

/// Flex container layout widget: arranges children per `flex` each layout pass.
pub struct FlexLayout {
    pub flex: Flex,
}

impl Widget for FlexLayout {
    fn layout(&mut self, ui: &mut Ui, obj: ObjRef) {
        crate::layout::layout_flex(ui, obj, &self.flex);
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

gridbox.rs 同构（`Grid` 字段、`layout_grid`）。

- [ ] **Step 2: ObjCfg 桥 + set_flex/set_grid + 删桥接**

obj.rs `ObjCfg::build`：`common.layout` 不再走 `apply_tail`——在 insert 时直接决定 kind：

```rust
let kind: alloc::boxed::Box<dyn super::Widget> = match common.layout.take() {
    Some(crate::layout::Layout::Flex(f)) => alloc::boxed::Box::new(super::flexbox::FlexLayout { flex: f }),
    Some(crate::layout::Layout::Grid(g)) => alloc::boxed::Box::new(super::gridbox::GridLayout { grid: g }),
    _ => alloc::boxed::Box::new(Manual),
};
let r = ui.insert_node(parent, Rect::new(0, 0, w, h), kind);
```

（`CommonBuilder.apply_tail` 删 layout 分支；`WidgetBuilder::layout()` 保留为 ObjCfg 专用——其他控件的 build 若收到 `common.layout`，忽略并在文档注明：布局是 kind，非通用属性。scrollview.rs 内部对 content 的 `set_layout` 改为直接 insert `FlexLayout` kind。）

ui.rs：删 `set_layout`，新增：

```rust
/// Replaces the node's widget kind with a flex layout (runtime layout change).
pub fn set_flex(&mut self, obj: ObjRef, flex: crate::layout::Flex) {
    if let Some(n) = self.arena.get_mut(obj) {
        n.kind = alloc::boxed::Box::new(crate::widgets::flexbox::FlexLayout { flex });
    }
    self.layout_dirty = true;
}
/// Replaces the node's widget kind with a grid layout.
pub fn set_grid(&mut self, obj: ObjRef, grid: crate::layout::Grid) {
    if let Some(n) = self.arena.get_mut(obj) {
        n.kind = alloc::boxed::Box::new(crate::widgets::gridbox::GridLayout { grid });
    }
    self.layout_dirty = true;
}
```

node.rs 删 `layout` 字段；layout.rs 删 `Layout` enum（调用点 `crate::layout::Layout` 随 set_layout 一并消失）；widgets/mod.rs 适配层 `layout()` 方法删除。

- [ ] **Step 3: 调用点修复**

`rg 'set_layout|Layout::' qingui/src qingui/tests qingui/examples qingui/benches`：builder `.layout(Layout::Flex(f))` 调用保持不动（ObjCfg 桥处理）；直接 `ui.set_layout(...)` 的改 `ui.set_flex/set_grid`；`use crate::style::Layout` 改 `crate::layout::Layout`。

- [ ] **Step 4: 跑测试**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: introduce FlexLayout/GridLayout widget kinds, drop the layout bridge"
```

---

### Task 10: ScrollView 迁移

**Files:**
- Modify: `qingui/src/widgets/scrollview.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 ScrollView 变体）
- Test: `qingui/tests/scrollview.rs`

**Interfaces:**
- Consumes: Task 9 的 FlexLayout；take-out `on_key(&mut self, ui, obj, key)`。
- Produces: `ScrollViewState` 独立 `impl Widget`；`KeyOutcome::Deferred` 少一个用户。

- [ ] **Step 1: scrollview.rs 迁移**

`ScrollViewState::on_key` 改直调（去 Deferred，删 `scroll_by_exec`）：

```rust
impl super::Widget for ScrollViewState {
    // Container: content is drawn by child nodes (CLIP_CHILDREN handled by the pipeline).
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        match key {
            Key::Up => { ui.scrollview_scroll_by(obj, -STEP); super::KeyOutcome::Consumed }
            Key::Down => { ui.scrollview_scroll_by(obj, STEP); super::KeyOutcome::Consumed }
            _ => super::KeyOutcome::Pass,
        }
    }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

注意 reentrancy：`ui.scrollview_scroll_by(obj, ...)` 内部走 `kind()/kind_mut()` + `as_scrollview()`——此刻 kind 已 take-out，返回 None 会静默失败。因此 `UiScrollViewExt` 的三个方法改为优先经 `ui.widget::<ScrollViewState>(sv)` 读取、经 `ui.update::<ScrollViewState, _>` 写入；而 `scroll` 字段在 take-out 期间写不到——解决方案：`on_key` 内**直接改 self.scroll** 并复用现有 clamp 逻辑，把 `scroll_to` 的核心抽成自由函数：

```rust
/// Core of scroll_to: clamps `y`, writes `state.scroll`, applies the translate.
/// Callable both from the ext trait (kind in arena) and from `on_key` (kind taken out).
pub(crate) fn apply_scroll(ui: &mut Ui, sv: ObjRef, state: &mut ScrollViewState, y: i32) {
    if ui.layout_dirty { ui.layout_pass(); ui.layout_dirty = false; }
    let content_h = ui.children(state.content).iter().map(|&c| ui.rect(c).y + ui.rect(c).h).max().unwrap_or(0);
    let view_h = ui.rect(sv).h;
    let ny = y.clamp(-(content_h - view_h).max(0), 0);
    if state.scroll == ny { return; }
    state.scroll = ny;
    let content = state.content;
    ui.set_translate(content, 0, ny);
}
```

`on_key` 调 `apply_scroll(ui, obj, self, self.scroll ± STEP)`；ext 的 `scrollview_scroll_to` 改为 `ui.update::<ScrollViewState, _>(sv, |s| apply_scroll(ui_in_closure?))`——闭包无法拿 ui，所以 ext 侧用 take-out 之外的写法：`scrollview_scroll_to` 先读 `content`（`ui.widget::<ScrollViewState>(sv).map(|s| s.content)`），算出 ny 后经 `ui.update::<ScrollViewState, _>(sv, |s| { let changed = s.scroll != ny; s.scroll = ny; changed })`，再 `ui.set_translate(content, 0, ny)`（仅在 changed 时，保持原 early-return 语义）。

`build` 中占位 `WidgetKind::Obj(...)` 已在 Task 5 改 `Box::new(Manual)`；`kind_mut` 替换处改 `n.kind = Box::new(ScrollViewState { content, scroll: 0 })`；content 节点 kind 按 Task 9 改 `Box::new(FlexLayout { ... })`，viewport 的 `set_layout` 调用删除（viewport kind 是 ScrollViewState——它也需要排 content 这一个子节点：给 `ScrollViewState` 实现 `layout` 调 `layout_flex`，flex 参数内嵌为常量 Column。即 `ScrollViewState` 增加 `fn layout(&mut self, ui, obj) { crate::layout::layout_flex(ui, obj, &SCROLL_FLEX) }`，`const SCROLL_FLEX: Flex = Flex { dir: FlexDir::Column, wrap: false, main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0 };`）。

widgets/mod.rs 删 `ScrollView(...)` 行与 `as_scrollview` 相关调用（ext 已改 `widget::<ScrollViewState>`）。

- [ ] **Step 2: 跑测试**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿（scrollview.rs 测试覆盖滚动/按键/裁剪）。

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: migrate ScrollView to the Widget trait with direct &mut Ui scrolling"
```

---

## Batch 3 — 交互控件

> 本批每个任务的迁移模式固定（此处完整给出一次，各任务引用时仍给出每控件的完整 impl 块，不省略）：
>
> 1. `impl super::WidgetBehavior for XxxState` 整段替换为 `impl super::Widget for XxxState`；
> 2. `draw` 签名改 `fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect)`，body 原样（`d` 形参改名 `c` 或保留 `d`，仅类型路径变化）；
> 3. `on_key` 旧签名 `(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome` 改 `(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome`；body 开头用 `let edited = ui.state(obj).contains(crate::node::State::EDITED);` 替代 `ctx.edited`，`ctx.vis_h` 改 `ui.rect(obj).h`，`ctx.now` 改 `ui.time()`；此后逻辑原样；
> 4. `tick` 旧签名 `(&mut self, now: u64)` 改 `(&mut self, _ui: &mut Ui, _obj: ObjRef, now: u64)`，body 原样；
> 5. `value/set_value/set_range/overflow` 签名不变，body 原样；
> 6. 补 `as_any/as_any_mut`（每个 widget 的两行样板同上）；
> 7. 文件内 `if let WidgetKind::X(s) = &mut n.kind` 直改 kind 的辅助函数改走 `ui.update::<XxxState, _>` / `ui.widget::<XxxState>`；
> 8. `build` 中 `Box::new(WidgetKind::X(...))` 改 `Box::new(XxxState { ... })`；widgets/mod.rs 删对应 `define_widgets!` 行；
> 9. 该控件专属的 Ui ext trait（`UiXxxExt`）内部实现改走 `widget::<XxxState>`/`update::<XxxState, _>`，pub 签名不变。

### Task 11: Slider / Bar / Arc 迁移（value 通道组）

**Files:**
- Modify: `qingui/src/widgets/slider.rs`, `qingui/src/widgets/bar.rs`, `qingui/src/widgets/arc.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Slider/Bar/Arc 三行）
- Test: `qingui/tests/p0_widgets.rs`, `qingui/tests/anim.rs`

**Interfaces:**
- Consumes: 迁移模式（Batch 3 引言）、`clamp_val`。
- Produces: 三个 State 独立 `impl Widget`；`AnimProp::Value` 通道对三者不变。

- [ ] **Step 1: slider.rs 迁移**

```rust
impl super::Widget for SliderState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { draw(self.min, self.max, self.value, ctx, c, clip) }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        use super::KeyOutcome::*;
        let edited = ui.state(obj).contains(crate::node::State::EDITED);
        if edited {
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
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn set_range(&mut self, min: i32, max: i32) { self.min = min; self.max = max; self.value = self.value.clamp(min, max); }
    fn overflow(&self) -> i32 { 4 }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

（原 `impl SliderState { fn on_key(...) }`  inherent 方法删除，逻辑并入 trait 方法。）

- [ ] **Step 2: bar.rs / arc.rs 迁移**

bar.rs（无 on_key，纯 value 通道 + draw）：

```rust
impl super::Widget for BarState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { /* 原 draw body 原样 */ }
    fn value(&self) -> i32 { self.value }
    fn set_value(&mut self, v: i32) -> bool { super::clamp_val(self.min, self.max, &mut self.value, v) }
    fn set_range(&mut self, min: i32, max: i32) { /* 原 body 原样 */ }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

arc.rs 同构（保留其 `overflow` 与 `on_key`——若有——按模式第 3 条改签名，body 原样）。

- [ ] **Step 3: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Slider/Bar/Arc to the Widget trait"
```

---

### Task 12: Switch / Checkbox / Led 迁移

**Files:**
- Modify: `qingui/src/widgets/switch.rs`, `qingui/src/widgets/checkbox.rs`, `qingui/src/widgets/led.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Switch/Checkbox/Led 三行）
- Test: `qingui/tests/p0_widgets.rs`, `qingui/tests/p1_widgets.rs`

**Interfaces:**
- Consumes: 迁移模式（Batch 3 引言）。
- Produces: 三个 State 独立 `impl Widget`。

- [ ] **Step 1: 三控件按模式迁移**

switch.rs：

```rust
impl super::Widget for SwitchState {
    fn draw(&self, ctx: &WidgetCtx, c: &mut super::Canvas, clip: Rect) { /* 原 draw body 原样 */ }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> super::KeyOutcome {
        // 原 on_key body：Enter 翻转 on、返回 ValueChanged/Consumed；仅签名按模式第 3 条调整
    }
    fn value(&self) -> i32 { /* 原 body */ }
    fn set_value(&mut self, v: i32) -> bool { /* 原 body */ }
    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
```

checkbox.rs / led.rs 同构（Checkbox 的 text 存取辅助函数按模式第 7 条改 `ui.update::<CheckboxState, _>`；Led 无按键，draw + value 通道原样）。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Switch/Checkbox/Led to the Widget trait"
```

---

### Task 13: Spinbox 迁移

**Files:**
- Modify: `qingui/src/widgets/spinbox.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Spinbox 行）
- Test: `qingui/tests/p1_widgets.rs`, `qingui/tests/input.rs`

**Interfaces:**
- Consumes: 迁移模式（Batch 3 引言）。
- Produces: `SpinboxState` 独立 `impl Widget`（编辑态按键逻辑原样，仅签名调整）。

- [ ] **Step 1: spinbox.rs 按模式迁移**

`on_key` 中编辑态 cursor/digits 逻辑 body 原样，签名按模式第 3 条；`value/set_value/set_range` 原样；`as_any/as_any_mut` 样板。删 `define_widgets!` 的 `Spinbox(...)` 行。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Spinbox to the Widget trait"
```

---

## Batch 4 — 复合控件

### Task 14: Spinner 迁移

**Files:**
- Modify: `qingui/src/widgets/spinner.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Spinner 行）
- Test: `qingui/tests/tick.rs`

**Interfaces:**
- Consumes: 迁移模式第 4 条（tick 签名）。
- Produces: `SpinnerState` 独立 `impl Widget`（tick 自转逻辑原样）。

- [ ] **Step 1: spinner.rs 按模式迁移**

`tick` 签名改 `(&mut self, _ui: &mut Ui, _obj: ObjRef, now: u64) -> TickOut`，自转 body 原样；`draw` 签名调整；`as_any/as_any_mut` 样板；删 enum 行。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Spinner to the Widget trait"
```

---

### Task 15: Chart / Table / Image 迁移（纯绘制组）

**Files:**
- Modify: `qingui/src/widgets/chart.rs`, `qingui/src/widgets/table.rs`, `qingui/src/widgets/image.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Chart/Table/Image 三行）
- Test: `qingui/tests/chart.rs`, `qingui/tests/image.rs`, `qingui/tests/p1_widgets.rs`

**Interfaces:**
- Consumes: 迁移模式；`UiChartExt`/`UiTableExt`/`UiImageExt`（若有）pub 签名不变。
- Produces: 三个 State 独立 `impl Widget`。

- [ ] **Step 1: 三控件按模式迁移**

三者均为 draw-only（Chart 的 series 数据存取、Table 的 cell 存取、Image 的位图存取辅助函数按模式第 7 条改 `ui.update::<XxxState, _>` / `ui.widget::<XxxState>`；ext trait pub 签名不变）。`blit565` 等 draw 调用 body 原样，仅 `DrawBuf` → `Canvas` 路径名。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Chart/Table/Image to the Widget trait"
```

---

### Task 16: List / ItemList 迁移

**Files:**
- Modify: `qingui/src/widgets/list.rs`, `qingui/src/widgets/itemlist.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 List/ItemList 两行）
- Test: `qingui/tests/list_nav.rs`, `qingui/tests/list_fx.rs`, `qingui/tests/itemlist.rs`, `qingui/tests/selected.rs`

**Interfaces:**
- Consumes: 迁移模式第 3/4 条；`Ui::as_list` 已委托 `widget::<ListState>`（Task 3）。
- Produces: `ListState`/`ItemListState` 独立 `impl Widget`；fx/ensure_visible 行为不变。

- [ ] **Step 1: list.rs 迁移**

`tick`（List fx 推进）签名按模式第 4 条，body 原样；`on_key`（项导航/滚动/选中）签名按模式第 3 条，body 原样（`ctx.vis_h` → `ui.rect(obj).h`）；`value/set_value/set_range`（选中项通道）原样；`select_clamp` 继续复用。List 的 Ui ext（add_item/clear/ensure_visible 等）内部 `as_list`/`as_list_mut` 调用改 `ui.widget::<ListState>`/`ui.update::<ListState, _>`，pub 签名不变。

- [ ] **Step 2: itemlist.rs 迁移**

同构（ItemList 的 ensure_visible 里若直接读 kind，按模式第 7 条改造；`layout_dirty` 时先 flush layout 的既有逻辑保留）。

- [ ] **Step 3: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate List/ItemList to the Widget trait"
```

---

### Task 17: Roller 迁移

**Files:**
- Modify: `qingui/src/widgets/roller.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Roller 行）
- Test: `qingui/tests/roller_ghost.rs`, `qingui/tests/p1_widgets.rs`

**Interfaces:**
- Consumes: 迁移模式第 3/4 条。
- Produces: `RollerState` 独立 `impl Widget`（sel_from 动画 tick 原样）。

- [ ] **Step 1: roller.rs 按模式迁移并验证**

`tick`（sel_from 插值）与 `on_key`（上下滚动选中）签名调整，body 原样；删 enum 行。`Ui::as_roller` 已在 Task 3 委托。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿（roller_ghost 用例覆盖动画末态）。

```bash
git add -A
git commit -m "refactor: migrate Roller to the Widget trait"
```

---

### Task 18: Dropdown 迁移

**Files:**
- Modify: `qingui/src/widgets/dropdown.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Dropdown 行）
- Test: `qingui/tests/p1_widgets.rs`, `qingui/tests/fluent_api.rs`

**Interfaces:**
- Consumes: take-out `&mut Ui`（Task 3）、`move_to_front`（Task 2）、floating 机制（不变）。
- Produces: `DropdownState` 独立 `impl Widget`；弹窗开启逻辑从 ext/Deferred 收进 `on_key` 直调。

- [ ] **Step 1: dropdown.rs 迁移**

`on_key`：Enter 开弹窗的分支从"返回 Deferred/外部 open"改为直接执行现有 `open` 逻辑（创建 List 弹窗、`set_floating`、`set_modal`、`move_to_front`——替代原 z_index 置顶，已在 Task 2 改好），返回 `KeyOutcome::Consumed`。弹窗 List 的选中回写（`sel_exec` 类 Deferred）同样改直调：写 `self.selected`（take-out 期间 self 可写）+ `ui.clear_modal()` + 删除弹窗节点 + `KeyOutcome::ValueChanged`。

迁移时注意：开弹窗分支里 `self` 是 DropdownState，弹窗是新建子节点——均在 take-out 允许范围内（操作其他节点不受限）。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Dropdown to the Widget trait, open popup via direct &mut Ui"
```

---

### Task 19: Msgbox 迁移

**Files:**
- Modify: `qingui/src/widgets/msgbox.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Msgbox 行）
- Test: `qingui/tests/p1_widgets.rs`

**Interfaces:**
- Consumes: 同 Task 18。
- Produces: `MsgboxState` 独立 `impl Widget`；`clear_modal`/删除自身走直调。

- [ ] **Step 1: msgbox.rs 迁移**

`on_key` 中按钮选择/确认逻辑原样（签名按模式第 3 条）；确认后"clear_modal + delete(msgbox 根)"从 Deferred 改直调（`ui.clear_modal(); ui.delete(root);`，root 从 self 或父链取得——保持现有取得方式）。删 enum 行。

- [ ] **Step 2: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: migrate Msgbox to the Widget trait, close via direct &mut Ui"
```

---

### Task 20: Custom 删除 + 用户控件一等公民化

**Files:**
- Delete: `qingui/src/widgets/custom.rs`
- Modify: `qingui/src/widgets/mod.rs`（删 Custom 变体、as_custom/as_custom_mut、适配层 Custom 分支）
- Modify: `qingui/src/ui.rs`（删 create_custom/custom/custom_mut）
- Test: `qingui/tests/custom_widget.rs`（改写）

**Interfaces:**
- Consumes: `Ui::create_widget`（Task 3）、`Ui::widget::<T>`/`Ui::update`。
- Produces: 用户控件唯一入口 `create_widget` + `widget::<T>`/`update::<T, _>`；`custom::Widget` trait 不复存在（`widgets::Widget` 即扩展点）。

- [ ] **Step 1: tests/custom_widget.rs 改写**

把现有测试 widget 从 `impl custom::Widget` 改为 `impl widgets::Widget`（加 `as_any/as_any_mut`；`on_key(&mut self, ui, obj, key) -> KeyOutcome` 返回 `KeyOutcome::Consumed` 替代原 `true`），挂载从 `create_custom` 改 `create_widget`，查询从 `ui.custom::<T>(obj)` 改 `ui.widget::<T>(obj)`、从 `custom_mut` 改 `ui.update::<T, _>`。

- [ ] **Step 2: 删除旧通道**

删 custom.rs；mod.rs 删 `Custom(...)` 行、`as_custom`/`as_custom_mut`、适配层 `on_key` 的 Custom 分支；ui.rs 删 `create_custom`/`custom`/`custom_mut`。`rg 'custom' qingui/src qingui/examples qingui/benches` 清残留。

- [ ] **Step 3: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: drop the Custom widget channel, create_widget is the extension point"
```

---

## Batch 5 — Canvas + 收尾

### Task 21: DrawBuf → Canvas + 公开 eg DrawTarget

**Files:**
- Create: `qingui/src/canvas.rs`（自 draw.rs 的 DrawBuf 段迁入 + DrawTarget 实现）
- Modify: `qingui/src/draw.rs`（保留私有 rasterize 助手；DrawBuf 移出）
- Modify: `qingui/src/lib.rs`（`pub mod canvas;` + re-export）
- Modify: `qingui/src/widgets/mod.rs`（`Canvas` 改指 `crate::canvas::Canvas`，删 Task 3 的临时 alias）
- Modify: `qingui/src/render.rs`（构造点）
- Test: `qingui/tests/draw.rs`, `qingui/tests/canvas.rs`（适配 + 新增 eg 用例）

**Interfaces:**
- Consumes: `draw.rs` 现有 DrawBuf 实现（775 行中的结构与方法）、font.rs 的 eg Text 适配段。
- Produces:
  - `pub struct Canvas<'a> { pub pixels: &'a mut [Color], pub area: Rect, pub stride: i32 }`（字段/方法签名与 DrawBuf 完全一致，仅改名）
  - `impl embedded_graphics::draw_target::DrawTarget for Canvas<'_>`（`type Color = Rgb888; type Error = Infallible`）
  - `impl From<Color> for embedded_graphics::pixelcolor::rgb::Rgb888`（geometry.rs）

- [ ] **Step 1: 先写失败测试（eg 兼容层）**

`qingui/tests/canvas.rs` 新增：

```rust
#[test]
fn eg_draw_target_fill_rect_via_primitives() {
    // 用 eg 的 Rectangle primitive 画一个实心矩形到 Canvas，断言像素
    let mut buf = [Color::BLACK; 100];
    let area = Rect::new(0, 0, 10, 10);
    {
        let mut c = Canvas { pixels: &mut buf, area, stride: 10 };
        use embedded_graphics::prelude::*;
        let r = embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::new(2, 2),
            embedded_graphics::geometry::Size::new(4, 4),
        );
        r.into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            embedded_graphics::pixelcolor::rgb::Rgb888::new(255, 0, 0),
        ))
        .draw(&mut c)
        .unwrap();
    }
    assert_eq!(buf[2 * 10 + 2], Color::rgb(255, 0, 0));
    assert_eq!(buf[0], Color::BLACK);
}
```

Run: `cargo test -p qingui --test canvas eg_draw_target_fill_rect_via_primitives`
Expected: FAIL（Canvas/DrawTarget 不存在，编译错误）。

- [ ] **Step 2: canvas.rs — DrawBuf 迁移 + DrawTarget**

把 draw.rs 的 `pub struct DrawBuf` 与其 `impl` 块整体移入新文件 `canvas.rs` 并改名 `Canvas`（draw.rs 保留 `circle_cov16`/`arc_cov16`/`SIN90`/`dir_vec` 等私有助手，canvas.rs `use crate::draw::*` 私有项需放宽为 `pub(crate)`）。追加：

```rust
impl embedded_graphics::draw_target::DrawTarget for Canvas<'_> {
    type Color = embedded_graphics::pixelcolor::rgb::Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        // Per-pixel path: ecosystem compatibility, no performance promise.
        for embedded_graphics::Pixel(p, color) in pixels {
            self.put(p.x, p.y, color.into(), 255);
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &embedded_graphics::primitives::Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // Fast path: route through the batch row fill (eg's default would fall back to draw_iter).
        let clip = self.area;
        self.fill_rect(from_eg_rect(*area), color.into(), 255, clip);
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        Canvas::clear(self, color.into());
        Ok(())
    }

    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        crate::draw::eg_rect(self.area)
    }
}
```

（实现要点：`draw_iter` 逐 `Pixel(point, color)` 调 `self.put(point.x, point.y, color.into(), 255)`；`fill_solid` 走 `fill_rect` 快路径——eg 的 `fill_contiguous`/`fill_solid` 默认实现会退化到 `draw_iter`，必须覆盖。`from_eg_rect` 为 eg Rectangle → crate Rect 的换算助手，与 draw.rs 现有 `eg_rect` 互逆。）

geometry.rs 追加：

```rust
impl From<Color> for embedded_graphics::pixelcolor::rgb::Rgb888 {
    fn from(c: Color) -> Self { Self::new(c.r, c.g, c.b) }
}
impl From<embedded_graphics::pixelcolor::rgb::Rgb888> for Color {
    fn from(c: embedded_graphics::pixelcolor::rgb::Rgb888) -> Self { Color::rgb(c.r(), c.g(), c.b()) }
}
```

lib.rs：`pub mod canvas;`。widgets/mod.rs 删 `use crate::draw::DrawBuf as Canvas;` 改 `pub use crate::canvas::Canvas;`。render.rs/draw.rs 内构造点 `crate::draw::DrawBuf { .. }` 改 `crate::canvas::Canvas { .. }`；draw_text_opa 内的私有 `EgTarget` 保留（BinaryColor 文字渲染不动）。

- [ ] **Step 3: 全仓改名收尾**

`rg 'DrawBuf' qingui/src qingui/tests qingui/examples qingui/benches qingui-codegen`：类型引用全改 `Canvas`（`crate::draw::DrawBuf` → `crate::canvas::Canvas`）。

- [ ] **Step 4: 跑测试**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿（含新 eg 用例）。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: promote DrawBuf to public Canvas with an eg DrawTarget adapter"
```

---

### Task 22: 删除宏/enum/Deferred/canvas 控件

**Files:**
- Modify: `qingui/src/widgets/mod.rs`（删 define_widgets! 宏、WidgetKind、WidgetBehavior、KeyCtx、KeyOutcome::Deferred、wtype/wref/wmut）
- Delete: `qingui/src/widgets/canvas.rs`
- Modify: `qingui/src/ui.rs`（update/widget 的 legacy 双跳分支、apply_key_outcome 的 Deferred 分支）
- Modify: `qingui/src/widgets/dropdown.rs`, `qingui/src/widgets/msgbox.rs`, `qingui/src/widgets/scrollview.rs`（确认无 Deferred 残留）
- Test: `qingui/tests/registry.rs`（删除或改写）、`qingui/tests/canvas.rs`（draw_hook 用例保留，CanvasCfg 用例改写）

**Interfaces:**
- Consumes: 全部控件已迁移（Task 5-20）。
- Produces: 终态——无宏、无 enum、无适配层；`KeyOutcome { Pass, Consumed, ValueChanged, EnterEdit, ExitEdit }`。

- [ ] **Step 1: 确认 enum 已空并删除**

此时 `define_widgets!` 调用应已无剩余变体（19 个全部迁出）。删宏定义、调用、`WidgetBehavior` trait、`wtype/wref/wmut` 宏、`KeyCtx`（已无消费者）；`WidgetKind` 与 `impl Widget for WidgetKind` 适配层整体删除。`clamp_val`/`select_clamp` 保留（迁移后控件在用）。

- [ ] **Step 2: Deferred 删除**

`rg 'Deferred' qingui/src`：确认 Task 10/18/19 后无生产者；删 `KeyOutcome::Deferred` 变体与 `apply_key_outcome` 对应分支。

- [ ] **Step 3: 双跳清理 + canvas 控件删除**

ui.rs：`widget::<T>`/`update` 中 `downcast::<WidgetKind>()` 的 legacy 分支删除（单跳即可）。删 widgets/canvas.rs（CanvasCfg）；tests/canvas.rs 中 CanvasCfg 用例改为 `ObjCfg + ui.set_draw_hook`（draw_hook 用例不动）。tests/registry.rs 整体删除（其断言对象是宏注册表）。

- [ ] **Step 4: 跑测试并提交**

Run: `cargo test -p qingui && cargo check --all-targets -p qingui`
Expected: 全绿。

```bash
git add -A
git commit -m "refactor: remove the widget registry macro and the legacy WidgetKind enum"
```

---

### Task 23: benchmark 终测 + 文档更新

**Files:**
- Modify: `docs/BENCHMARK.md`
- Modify: `README.md`（特性/快速开始中涉及 Custom/Canvas/z_index/Style 布局字段的描述）
- Modify: `qingui/README.md`（同步）

- [ ] **Step 1: benchmark 终测**

Run: `cargo bench -p qingui --bench memory && cargo bench -p qingui --bench time`
红线复核：三档场景峰值堆涨幅 ≤15%（超出则停下汇报，不擅自做 small-kind 优化）。数据追加到 `docs/BENCHMARK.md`（新一节 "trait-object migration, final"），更新既有"最大变体税"相关描述。

- [ ] **Step 2: 文档更新**

README：控件列表删 Canvas/Custom 提及，加"用户控件：impl Widget + create_widget"；快速开始示例中涉及被删 API 的片段同步；`docs/superpowers/specs/2026-08-08-widget-trait-object-design.md` 顶部状态改"已落地"。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "docs: record final trait-object baselines and update README"
```

---

## 完成判定

- `rg 'WidgetKind|WidgetBehavior|define_widgets|DrawBuf|custom::Widget|z_index|KeyCtx|Deferred' qingui/src` 无残留。
- `cargo test -p qingui`、`cargo check --all-targets -p qingui`、两个 bench 全绿且红线达标。
- 用户控件经 `create_widget` 挂载、与内置控件同权（tests/custom_widget.rs 为证）。
