# 统一泛型 WidgetBuilder 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 21 个复制粘贴的 `XxxBuilder` 收敛为单一 `WidgetBuilder<Cfg>`（`CommonBuilder` 承载公共字段），20 个控件迁移，Msgbox 保持独立。

**Architecture:** `WidgetBuilder<Cfg>`（公共 setter，`impl<Cfg: WidgetCfg>`）+ `CommonBuilder`（公共字段 + `apply_tail`）+ `pub(crate) trait WidgetCfg`（每控件 `build`/`default_style`）。每个控件只留 `XxxCfg`（专属字段 + `new()` 返回 builder）+ `impl WidgetCfg` + `impl WidgetBuilder<XxxCfg>`（专属 setter）。调用点 `XxxBuilder::new(` → `XxxCfg::new(`。

**Tech Stack:** Rust (no_std + alloc), 零新依赖, 零运行时开销。

## Global Constraints

- **调用点形态**：`XxxCfg::new(...).setters.build(ui|&mut ui, parent)`；库内不得残留 `XxxBuilder::new(`。
- **每控件** 导出 `pub type XxxBuilder = WidgetBuilder<XxxCfg>`（仅类型标注用）。
- **WidgetCfg / CommonBuilder 为 `pub(crate)`**；`WidgetBuilder<Cfg>` 与 `XxxCfg` 为 `pub`。
- **专属 setter 不与公共 setter 同名**（inherent 特化不可行，E0592）；Switch 开关 setter 改名 `.checked(bool)`，事件注册统一 `.on(kind, cb)`。
- **不统一 Msgbox**、**不改 `define_widgets!`**、**不新增依赖**。
- 代码注释英文；commit message 英文 Conventional Commits，仅本地不 push。
- **验证命令**：`cargo test -p qingui`、`cargo check -p qingui --all-targets`、`cargo bench -p qingui --bench memory`、`cargo test -p qemu-mem`、`cargo check --workspace`。

---

### Task 1: 核心基础设施（WidgetBuilder / CommonBuilder / WidgetCfg）

**Files:**
- Create: `qingui/src/widgets/builder.rs`
- Modify: `qingui/src/widgets/mod.rs`（`pub mod builder;` 或 `pub use`）

**Interfaces:**
- Consumes: 无。
- Produces: `WidgetBuilder<Cfg>`（`common`/`cfg` 字段 `pub(crate)`）、`CommonBuilder`（字段 + `Default` + `apply_tail(ui, r)`）、`trait WidgetCfg { fn build(self, ui, parent, common) -> ObjRef; fn default_style() -> Style }`。公共 setter：`size`/`style`/`style_pressed`/`style_focused`/`layout`/`sizing`/`transition`/`on`/`style_with`/`build`。后续 Task 全部依赖。

- [ ] **Step 1: 新建 `builder.rs`**

写入完整代码（先读一遍 `qingui/src/widgets/mod.rs` 确认 `Ui`/`Style`/`EventKind`/`EventCb`/`ObjRef`/`Sizing`/`Layout`/`Easing` 的 import 路径，与下列代码对齐）：

```rust
//! Shared builder scaffolding: common config + the generic WidgetBuilder.
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::event::{EventCb, EventKind};
use crate::layout::Sizing;
use crate::style::{Layout, Style};
use crate::ui::Ui;

/// Common fields shared by every widget builder.
#[derive(Default)]
pub(crate) struct CommonBuilder {
    pub size: Option<(i32, i32)>,
    pub style: Option<Style>,
    pub style_pressed: Option<Style>,
    pub style_focused: Option<Style>,
    pub layout: Option<Layout>,
    pub sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    pub transition: Option<(u32, Easing)>,
    pub events: Vec<(EventKind, EventCb)>,
}

impl CommonBuilder {
    /// Applies the layout/sizing/transition/events tail to an inserted node.
    /// Style defaults are widget-specific and stay in each `WidgetCfg::build`.
    pub fn apply_tail(self, ui: &mut Ui, r: ObjRef) {
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
    }
}

/// Widget-specific build logic: default size/style and post-insert setup.
pub(crate) trait WidgetCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, common: CommonBuilder) -> ObjRef;
    fn default_style() -> Style {
        Style::default()
    }
}

/// A fluent builder for any widget. Common setters live here once.
pub struct WidgetBuilder<Cfg> {
    pub(crate) common: CommonBuilder,
    pub(crate) cfg: Cfg,
}

impl<Cfg: WidgetCfg> WidgetBuilder<Cfg> {
    pub fn size(mut self, w: i32, h: i32) -> Self { self.common.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.common.style = Some(s); self }
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self {
        self.common.style = Some(f(Cfg::default_style()));
        self
    }
    pub fn style_pressed(mut self, s: Style) -> Self { self.common.style_pressed = Some(s); self }
    pub fn style_focused(mut self, s: Style) -> Self { self.common.style_focused = Some(s); self }
    pub fn layout(mut self, l: Layout) -> Self { self.common.layout = Some(l); self }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.common.sizing = Some((w, h));
        self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.common.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.common.events.push((kind, cb));
        self
    }
    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        Cfg::build(self.cfg, ui, parent, self.common)
    }
}
```

- [ ] **Step 2: mod.rs 挂载**

`qingui/src/widgets/mod.rs` 顶部加：
```rust
pub(crate) mod builder;
pub use builder::WidgetBuilder; // 公开返回类型（XxxCfg::new 返回它）
```
`CommonBuilder`/`WidgetCfg` 保持 crate 内可见（`WidgetCfg` 是 `pub(crate)` trait，`CommonBuilder` 只在 `WidgetBuilder` 私有字段与 `pub(crate)` 签名中出现，均不泄漏到公共 API）。各控件文件统一 `use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};`。

- [ ] **Step 3: 编译验证**

Run: `cargo check -p qingui`
Expected: 通过，无新 warning（新增未用代码若报 dead_code，加 `#[allow(dead_code)]` 到 `WidgetCfg::default_style` 默认实现或该 task 内先不引入引用——用 `pub(crate) use` 暴露即可）。

- [ ] **Step 4: Commit**

```bash
git add qingui/src/widgets/builder.rs qingui/src/widgets/mod.rs
git commit -m "refactor(widgets): add WidgetBuilder/CommonBuilder/WidgetCfg scaffolding"
```

---

### Task 2: 转换 Obj（参考实现，确立"转换配方"）

**Files:**
- Modify: `qingui/src/widgets/obj.rs`（全部重写）
- 调用点（`ObjBuilder::new(` → `ObjCfg::new(` + import）：
  `tests/grid.rs, tests/layout_sizing.rs, tests/anim.rs, tests/tree.rs, tests/input.rs, tests/dirty.rs, tests/style.rs, tests/focus_visual.rs, tests/flex.rs, tests/clip.rs, tests/render.rs, tests/scrollview.rs, tests/transition_ghost.rs, tests/layout_transition.rs, tests/fluent_api.rs, tests/floating.rs, tests/hooks.rs, tests/tick.rs, tests/selected.rs, examples/demo.rs, examples/gallery.rs`

**Interfaces:**
- Consumes: Task 1 的 `WidgetBuilder`/`CommonBuilder`/`WidgetCfg`。
- Produces: 转换配方（Task 3-7 复用）：(a) 删公共字段只留专属字段；(b) `new()` 返回 `WidgetBuilder { common: Default::default(), cfg }`；(c) `WidgetCfg::build` 里 `self.X` → `self`（专属）或 `common.X`（公共）；(d) 专属 setter 移入 `impl WidgetBuilder<XxxCfg>`；导出 `pub type XxxBuilder = WidgetBuilder<XxxCfg>`。

- [ ] **Step 1: 重写 obj.rs**

```rust
use alloc::vec::Vec;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::style::Layout;
use crate::ui::Ui;
use super::builder::{CommonBuilder, WidgetBuilder, WidgetCfg};
use super::WidgetKind;

/// Builder for the generic container Obj (hosts layout and child objects).
pub type ObjBuilder = WidgetBuilder<ObjCfg>;

pub struct ObjCfg;

impl ObjCfg {
    pub fn new() -> WidgetBuilder<ObjCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ObjCfg }
    }
}

impl WidgetCfg for ObjCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or((0, 0));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(ObjState));
        if let Some(s) = common.style {
            ui.set_style(r, s);
        }
        common.apply_tail(ui, r);
        r
    }
}

/// Placeholder state: Obj carries no data.
pub struct ObjState;

impl super::WidgetBehavior for ObjState {
    fn draw(&self, _ctx: &super::WidgetCtx, _d: &mut DrawBuf, _clip: Rect) {}
}
```

> 注意：Obj 的 `layout` 公共字段现在由 `common.apply_tail` 处理（`set_layout`）。原 `style_with` 默认样式为 `Style::default()`，与 `WidgetCfg::default_style()` 默认实现一致，无需覆写。

- [ ] **Step 2: 批量替换调用点**

Run（zsh，全仓含 tools）：
```bash
perl -pi -e 's/\bObjBuilder::new\(/ObjCfg::new(/g' $(rg -l "ObjBuilder::new" qingui tools)
```
然后修 import：上一步列出的文件里 `use qingui::widgets::obj::ObjBuilder;` 改为 `use qingui::widgets::obj::ObjCfg;`（编译器会报 `cannot find ObjCfg`/`unused import ObjBuilder`，按报错逐个改）。`examples/demo.rs`/`gallery.rs` 同理。

- [ ] **Step 3: 全量测试**

Run: `cargo test -p qingui`
Expected: 全绿。有 `cannot find`/`unused` 报错 → 修 import 重跑。

- [ ] **Step 4: Commit**

```bash
git add -u && git add qingui/src/widgets/obj.rs
git commit -m "refactor(widgets): migrate ObjBuilder to WidgetBuilder<ObjCfg>"
```

---

### Task 3: 文本控件（Button / Label / Checkbox）

**Files:**
- Modify: `qingui/src/widgets/button.rs`, `label.rs`, `checkbox.rs`
- 调用点：
  - Button：`tests/input.rs, tests/builders.rs, tests/focus_visual.rs, tests/hooks.rs, tests/font.rs, tests/fluent_api.rs, tests/list_nav.rs, tests/p1_widgets.rs, tests/widgets.rs, examples/demo.rs, examples/gallery.rs, benches/memory.rs, tools/qemu-mem/src/scenes.rs, tools/qemu-mem/tests/alloc_host.rs`（内部 `button.rs:117 create`）
  - Label：`examples/demo.rs, examples/gallery.rs, tests/font.rs, tests/label.rs, tests/itemlist.rs, tests/transition_ghost.rs, tests/tick.rs, benches/memory.rs, tools/qemu-mem/src/scenes.rs, tools/qemu-mem/tests/alloc_host.rs`（内部 `label.rs:93 create`）
  - Checkbox：`examples/demo.rs, examples/gallery.rs, tests/p0_widgets.rs`

**Interfaces:**
- Consumes: Task 2 配方。
- Produces: Button/Label/Checkbox 的 `XxxCfg` 与 `impl WidgetCfg`，含 `default_style()` 覆写。

- [ ] **Step 1: 转换 button.rs**

按 Task 2 配方移植现有 `ButtonBuilder`（`qingui/src/widgets/button.rs:29-115`）。关键点：
- `Cfg` 字段：`text: String`。
- `new(text: &str) -> WidgetBuilder<ButtonCfg>`。
- `WidgetCfg::build`：`common.size.unwrap_or_else(|| { font = measure_font(common.style.as_ref(), ui); text_size(font,&self.text)+(24,12) })`；插 `WidgetKind::Button(ButtonState { text })`；`set_style(common.style.unwrap_or_else(theme_button))`、`set_style_pressed(common.style_pressed.unwrap_or_else(theme_button_pressed))`、`set_style_focused(common.style_focused.unwrap_or_else(theme_button_focused))`；保留 CLICKABLE flag；`common.apply_tail`。
- `fn default_style() -> Style { theme_button() }`。
- `create(ui, parent, text)` 改为 `ButtonCfg::new(text).build(ui, parent)`。

- [ ] **Step 2: 转换 label.rs**

移植现有 `LabelBuilder`（`label.rs:32-91`）。关键点：
- `Cfg` 字段：`text: String`。
- 现在 Label 尊重 `common.size`（若显式设置则用它，否则 `text_size` 测默认）——这是 spec 的 API 超集点。
- `default_style() -> theme_label()`。
- 内部 `create` 同步改 `LabelCfg::new`。

- [ ] **Step 3: 转换 checkbox.rs**

移植现有 `CheckboxBuilder`（`checkbox.rs:55-160`）。关键点：
- `Cfg` 字段：`text: String`, `checked: bool`。
- 专属 setter：`checked(bool)` → `impl WidgetBuilder<CheckboxCfg> { pub fn checked(...) -> Self { self.cfg.checked = ...; self } }`。
- 默认样式内联构造（base `{ bg_opa:0, text_color:WHITE }`，focused = base + 白边框）→ `default_style()` 返回 base；build 里 focused 默认在 base 上叠加。
- `style_with` 语义由通用实现 + `default_style()` 覆盖。

- [ ] **Step 4: 批量替换调用点 + 修 import**

分别对 Button/Label/Checkbox 跑：
```bash
perl -pi -e 's/\bButtonBuilder::new\(/ButtonCfg::new(/g' $(rg -l "ButtonBuilder::new" qingui tools)
perl -pi -e 's/\bLabelBuilder::new\(/LabelCfg::new(/g' $(rg -l "LabelBuilder::new" qingui tools)
perl -pi -e 's/\bCheckboxBuilder::new\(/CheckboxCfg::new(/g' $(rg -l "CheckboxBuilder::new" qingui tools)
```
按编译报错修各处 import（`use qingui::widgets::button::{ButtonBuilder → ButtonCfg}` 等）。

- [ ] **Step 5: 全量验证**

Run: `cargo test -p qingui`（含 `tests/builders.rs`、`tests/fluent_api.rs`）
Run: `cargo test -p qemu-mem`（scenes.rs/alloc_host.rs 用了 Button/Label）
Expected: 全绿。

- [ ] **Step 6: Commit**

```bash
git add -u
git commit -m "refactor(widgets): migrate Button/Label/Checkbox builders to WidgetCfg"
```

---

### Task 4: 数值控件（Slider / Bar / Arc / Spinbox / Switch）

**Files:**
- Modify: `qingui/src/widgets/slider.rs`, `bar.rs`, `arc.rs`, `spinbox.rs`, `switch.rs`
- 调用点：
  - Slider：`tests/builders.rs, tests/focus_visual.rs, tests/input.rs, tests/anim.rs, tests/widgets.rs, examples/gallery.rs, examples/demo.rs, benches/memory.rs, tools/qemu-mem/src/scenes.rs, tools/qemu-mem/tests/alloc_host.rs`
  - Bar：`tests/widgets.rs, tests/dirty.rs, tests/registry.rs, examples/demo.rs, examples/gallery.rs`
  - Arc：`tests/p0_widgets.rs, examples/demo.rs, examples/gallery.rs`
  - Spinbox：`tests/p1_widgets.rs, examples/demo.rs, examples/gallery.rs`
  - Switch：`tests/input.rs, tests/focus_visual.rs, tests/widgets.rs, examples/demo.rs, examples/gallery.rs`

**Interfaces:**
- Consumes: Task 2 配方。
- Produces: 五个 `XxxCfg`；Switch 的开关 setter 更名 `.checked(bool)`（spec 第 4 节）。

- [ ] **Step 1: 转换 slider/bar/arc/spinbox**

四个控件的共同形态（`min`/`max` + `value: Option<i32>`，spinbox 多 `digits: u8`）：
- `Cfg` 字段：`min: i32, max: i32, value: Option<i32>`（spinbox 加 `digits: u8`）。
- `new(min, max[, digits]) -> WidgetBuilder<XxxCfg>`。
- 专属 setter：`impl WidgetBuilder<XxxCfg> { pub fn value(self, v: i32) -> Self }`。
- `build`：`common.size.unwrap_or(<各自默认>)`（Slider (100,12)、Bar (100,8)、Arc (60,60)、Spinbox `(digits*advance+12, line_height+8)`）；插对应 state（`value.unwrap_or(min)`）；默认样式按现状（Slider/Bar 用 theme_*，Arc 透明 bg，Spinbox 内联 base + focused 白边框）。
- Arc/Spinbox 的 `default_style()`：Arc 返回透明 bg 默认，Spinbox 返回内联 base。

- [ ] **Step 2: 转换 switch.rs + 开关 setter 更名**

- `Cfg` 字段：`on: bool`。
- `new() -> WidgetBuilder<SwitchCfg>`。
- **专属 setter 改名**：`impl WidgetBuilder<SwitchCfg> { pub fn checked(self, on: bool) -> Self { self.cfg.on = on; self } }`（不再叫 `on`，取消 `on_event`；事件注册统一走通用 `.on(kind, cb)`）。
- `build`：默认 (40,20)；插 `SwitchState { on }`；`theme_switch`/`theme_switch_focused`。

- [ ] **Step 3: 批量替换调用点 + 修 import**

```bash
perl -pi -e 's/\bSliderBuilder::new\(/SliderCfg::new(/g' $(rg -l "SliderBuilder::new" qingui tools)
perl -pi -e 's/\bBarBuilder::new\(/BarCfg::new(/g' $(rg -l "BarBuilder::new" qingui tools)
perl -pi -e 's/\bArcBuilder::new\(/ArcCfg::new(/g' $(rg -l "ArcBuilder::new" qingui tools)
perl -pi -e 's/\bSpinboxBuilder::new\(/SpinboxCfg::new(/g' $(rg -l "SpinboxBuilder::new" qingui tools)
perl -pi -e 's/\bSwitchBuilder::new\(/SwitchCfg::new(/g' $(rg -l "SwitchBuilder::new" qingui tools)
```
按编译报错修 import。Switch 调用点当前均无 `.on(bool)`/`.on_event(`（已核实），无需其他改动。

- [ ] **Step 4: 全量验证**

Run: `cargo test -p qingui`；`cargo test -p qemu-mem`
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add -u
git commit -m "refactor(widgets): migrate value widgets (Slider/Bar/Arc/Spinbox/Switch) to WidgetCfg"
```

---

### Task 5: 选择控件（List / Roller / Dropdown）

**Files:**
- Modify: `qingui/src/widgets/list.rs`, `roller.rs`, `dropdown.rs`
- 调用点：
  - List：`tests/list_fx.rs, tests/list_nav.rs, tests/focus_visual.rs, tests/transition_ghost.rs, tests/tick.rs, tests/widgets.rs, tests/builders.rs, examples/demo.rs, examples/gallery.rs, benches/memory.rs, tools/qemu-mem/src/scenes.rs, tools/qemu-mem/tests/alloc_host.rs`（内部 `list.rs:354 create`）
  - Roller：`tests/roller_ghost.rs, tests/builders.rs, tests/p1_widgets.rs, examples/demo.rs, examples/gallery.rs`
  - Dropdown：`tests/builders.rs, tests/p1_widgets.rs, examples/demo.rs, examples/gallery.rs`

**Interfaces:**
- Consumes: Task 2 配方。
- Produces: 三个 `XxxCfg`；boxed 状态插入路径。

- [ ] **Step 1: 转换 list.rs / roller.rs**

- `Cfg` 字段：`items: Vec<String>, selected: usize`。
- `new(items: &[&str]) -> WidgetBuilder<XxxCfg>`（转 `Vec<String>`，selected 默认 0）。
- 专属 setter：`selected(usize)`。
- `build`：默认尺寸 List `(120, items.clamp(1,5)*16+2)`、Roller `(80, items.clamp(1,3)*16+8)`；**boxed 插入**：`WidgetKind::List(Box::new(ListState { items, selected: clamp, scroll: 0, fx: ListFx::default() }))`（Roller 同理 `Box::new(RollerState { ..., sel_from: None })`，字段名以现有源码为准）；默认样式按现状（List theme_*，Roller 内联 base + focused 白边框）。
- 内部 `create`（list.rs:354）改 `ListCfg::new(...).build(...)`。

- [ ] **Step 2: 转换 dropdown.rs**

- `Cfg` 字段：`items: Vec<String>, selected: usize`。
- 同 List 形态，默认 (100,20)，内联 base + focused 白边框。注意 DropdownState 构造（`DropdownState { items, selected }`）——参照现有 `dropdown.rs:96-200`。

- [ ] **Step 3: 批量替换调用点 + 修 import**

```bash
perl -pi -e 's/\bListBuilder::new\(/ListCfg::new(/g' $(rg -l "ListBuilder::new" qingui tools)
perl -pi -e 's/\bRollerBuilder::new\(/RollerCfg::new(/g' $(rg -l "RollerBuilder::new" qingui tools)
perl -pi -e 's/\bDropdownBuilder::new\(/DropdownCfg::new(/g' $(rg -l "DropdownBuilder::new" qingui tools)
```

- [ ] **Step 4: 全量验证**

Run: `cargo test -p qingui`（重点 `tests/list_fx.rs`、`tests/roller_ghost.rs`、`tests/builders.rs`）；`cargo test -p qemu-mem`
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add -u
git commit -m "refactor(widgets): migrate List/Roller/Dropdown builders to WidgetCfg"
```

---

### Task 6: 简单固定控件（Spinner / Led / Table / Image）

**Files:**
- Modify: `qingui/src/widgets/spinner.rs`, `led.rs`, `table.rs`, `image.rs`
- 调用点：
  - Spinner：`tests/tick.rs, tests/p0_widgets.rs, examples/demo.rs, examples/gallery.rs`
  - Led：`tests/p1_widgets.rs, examples/demo.rs, examples/gallery.rs`
  - Table：`tests/p1_widgets.rs, examples/demo.rs, examples/gallery.rs`
  - Image：`tests/image.rs, examples/demo.rs, examples/gallery.rs`

**Interfaces:**
- Consumes: Task 2 配方。
- Produces: 四个 `XxxCfg`。

- [ ] **Step 1: 转换 spinner.rs / led.rs / table.rs / image.rs**

- Spinner：无专属字段，`new()`；默认 (32,32)，透明 bg。
- Led：`color: Color, bright: Option<u8>`，`new(color)`，专属 setter `bright(u8)`；默认 (16,16)，透明 bg。
- Table：`cols: u8, rows: u8, cells: Vec<String>`，`new(cols, rows)`，专属 setter `cell(row, col, &str)`；默认 `(cols*60, rows*16)`，透明 bg + 白字。
- Image：`data: &'static ImageData`，`new(data)`；默认 `(fw, fh)`（首帧），透明 bg。
- 各 build 按现有源码移植（`common.size.unwrap_or(默认)`、插 state、透明 bg 处理、`common.apply_tail`）。

- [ ] **Step 2: 批量替换调用点 + 修 import**

```bash
perl -pi -e 's/\bSpinnerBuilder::new\(/SpinnerCfg::new(/g' $(rg -l "SpinnerBuilder::new" qingui tools)
perl -pi -e 's/\bLedBuilder::new\(/LedCfg::new(/g' $(rg -l "LedBuilder::new" qingui tools)
perl -pi -e 's/\bTableBuilder::new\(/TableCfg::new(/g' $(rg -l "TableBuilder::new" qingui tools)
perl -pi -e 's/\bImageBuilder::new\(/ImageCfg::new(/g' $(rg -l "ImageBuilder::new" qingui tools)
```

- [ ] **Step 3: 全量验证**

Run: `cargo test -p qingui`
Expected: 全绿。

- [ ] **Step 4: Commit**

```bash
git add -u
git commit -m "refactor(widgets): migrate Spinner/Led/Table/Image builders to WidgetCfg"
```

---

### Task 7: 特殊控件（Chart / ScrollView / ItemList / Canvas）

**Files:**
- Modify: `qingui/src/widgets/chart.rs`, `scrollview.rs`, `itemlist.rs`, `canvas.rs`
- 调用点：
  - Chart：`tests/chart.rs, tests/registry.rs, examples/demo.rs, examples/gallery.rs, benches/memory.rs, tools/qemu-mem/src/scenes.rs, tools/qemu-mem/tests/alloc_host.rs`
  - ScrollView：`tests/scrollview.rs, examples/demo.rs`
  - ItemList：`tests/itemlist.rs, examples/demo.rs, examples/gallery.rs, benches/memory.rs, tools/qemu-mem/src/scenes.rs, tools/qemu-mem/tests/alloc_host.rs`
  - Canvas：`tests/canvas.rs, examples/gallery.rs`

**Interfaces:**
- Consumes: Task 2 配方。
- Produces: 四个 `XxxCfg`（两段式插入、draw_hook、可追加 series 等特殊逻辑全部收进 `WidgetCfg::build`）。

- [ ] **Step 1: 转换 chart.rs**

- `Cfg` 字段：`min: i32, max: i32, series: Vec<(Color, usize)>`。
- `new()`（min 0 max 100）。
- 专属 setter（`impl WidgetBuilder<ChartCfg>`）：`range(i32, i32)`、`series(Color, usize)`（append）。
- `build`：`common.size.unwrap_or((120,60))`；`ChartState { min, max, series: series.into_iter().map(Series::new).collect() }`；`set_style(common.style.unwrap_or_default())`；`apply_tail`。

- [ ] **Step 2: 转换 scrollview.rs（两段式插入）**

- 无专属字段，`new()`。
- `WidgetCfg::build` 原样移植 `scrollview.rs:80-96` 的两段式插入：插占位 `WidgetKind::Obj(ObjState)` → `set_clip_children` → 建 content 子节点 → `*ui.kind_mut(r) = WidgetKind::ScrollView(ScrollViewState { content, scroll: 0 })`。样式/布局（透明 bg、column flex、`theme_list_focused`）照旧；`apply_tail`。

- [ ] **Step 3: 转换 itemlist.rs（两段式 + boxed）**

- `Cfg` 字段：`style_selected: Option<Style>`。
- `new()`，专属 setter `style_selected(Style)`。
- `build` 原样移植 `itemlist.rs:103-115`：占位 Obj → content 子节点 → `n.kind = WidgetKind::ItemList(Box::new(ItemListState { selected: 0, content, sel_style }))`。sel_style 默认 `default_sel_style` 保留。

- [ ] **Step 4: 转换 canvas.rs**

- `Cfg` 字段：`cb: DrawHook`。
- `new(cb) -> WidgetBuilder<CanvasCfg>`。
- `build`：`common.size.unwrap_or((32,32))`；插 `WidgetKind::Obj(ObjState)`；透明 bg；`ui.set_draw_hook(r, Some(self.cb))`；`apply_tail`。

- [ ] **Step 5: 批量替换调用点 + 修 import**

```bash
perl -pi -e 's/\bChartBuilder::new\(/ChartCfg::new(/g' $(rg -l "ChartBuilder::new" qingui tools)
perl -pi -e 's/\bScrollViewBuilder::new\(/ScrollViewCfg::new(/g' $(rg -l "ScrollViewBuilder::new" qingui tools)
perl -pi -e 's/\bItemListBuilder::new\(/ItemListCfg::new(/g' $(rg -l "ItemListBuilder::new" qingui tools)
perl -pi -e 's/\bCanvasBuilder::new\(/CanvasCfg::new(/g' $(rg -l "CanvasBuilder::new" qingui tools)
```

- [ ] **Step 6: 全量验证**

Run: `cargo test -p qingui`（重点 `tests/chart.rs`、`tests/scrollview.rs`、`tests/itemlist.rs`、`tests/canvas.rs`）；`cargo test -p qemu-mem`
Expected: 全绿。

- [ ] **Step 7: Commit**

```bash
git add -u
git commit -m "refactor(widgets): migrate Chart/ScrollView/ItemList/Canvas builders to WidgetCfg"
```

---

### Task 8: 收尾扫描与全量验证

**Files:**
- 全仓（若 Task 2-7 有遗漏）

**Interfaces:**
- Consumes: 全部转换完成的 20 个控件。
- Produces: 可交付状态。

- [ ] **Step 1: 扫描残留**

Run:
```bash
rg -n "Builder::new" qingui tools
rg -n "on_event" qingui tools
```
Expected: 无输出（`msgbox` 的 `MsgboxBuilder::new` 保留属正常——Msgbox 不转换；确认输出里只有 `MsgboxBuilder` 无其他 `XxxBuilder::new`）。

- [ ] **Step 2: 全量验证**

Run: `cargo test -p qingui`、`cargo test -p qemu-mem`、`cargo check -p qingui --all-targets`、`cargo check --workspace`、`cargo bench -p qingui --bench memory`
Expected: 全部通过；memory bench 输出与重构前数值一致（静态表 + 三档 peak/live）；`--all-targets` 无新增 warning（`roller_ghost.rs` 2 个既有 warning 除外）。

- [ ] **Step 3: 确认公共 API 形态**

Run:
```bash
rg -n "pub type (Obj|Button|Label|Slider|Switch|Bar|List|Arc|Checkbox|Chart|Spinner|Led|Table|Spinbox|Roller|ScrollView|Dropdown|Image|ItemList|Canvas)Builder" qingui/src/widgets
```
Expected: 20 行 `pub type XxxBuilder = WidgetBuilder<XxxCfg>;`（obj/button/label/checkbox 4 个可选，其余 16 个必须）。

- [ ] **Step 4: Commit**

```bash
git add -u
git commit -m "refactor(widgets): verify generic builder migration, no leftovers"
```

---

## Self-Review

**Spec 覆盖：**
- WidgetBuilder/CommonBuilder/WidgetCfg + 公共 setter → Task 1。
- `Cfg::new` 返回 builder、`impl WidgetBuilder<XxxCfg>` 专属 setter → Task 2 配方 + Task 3-7。
- boxed（List/Roller/ItemList）、两段式（ScrollView/ItemList）、Canvas、Msgbox 独立 → Task 5/7。
- Label `.size()` 超集 → Task 3 Step 2。
- Switch `.checked(bool)`（inherent 特化不可行的实测结论）→ Task 4 Step 2。
- 通用 `style_pressed`/`style_focused`/`layout`/`style_with` → Task 1 公共 setter + 各 Task `default_style()`。
- 267 处调用点迁移 + import → 各 Task 的 perl 替换 + 编译报错驱动修 import。
- 验收：test/check/bench/qemu-mem 全绿、无残留 → Task 8。

**占位符扫描：** 无 TBD/TODO；每 Task 给出 perl 命令与具体行为移植点，字段名以现有源码为准（已在任务内标注"以现有源码为准"避免臆造）。

**类型一致性：**
- `WidgetBuilder<Cfg>` 字段 `common`/`cfg`、`CommonBuilder::apply_tail`、`WidgetCfg::build(self, ui, parent, common)`、`default_style()` 在 Task 1 定义，Task 2-8 一致引用。
- `pub type XxxBuilder = WidgetBuilder<XxxCfg>` 在 Task 2 确立，Task 8 Step 3 核验 20 个。
- perl 替换的目标名（`ObjCfg::new` 等）与各 Task 定义的 `XxxCfg::new` 签名一致。
