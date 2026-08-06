# 统一泛型 WidgetBuilder 设计

日期：2026-08-06
状态：已获用户批准（分节 brainstorming 确认）

## 背景与动机

21 个 `XxxBuilder` 结构体把公共字段（`size`/`style`/`style_pressed`/`style_focused`/`sizing`/`transition`/`events`）+ 公共 setter（`size`/`style`/`style_with`/`style_pressed`/`style_focused`/`sizing`/`transition`/`on`）+ build 尾部的 4 段 `if let` 原样复制粘贴。宏（`define_builder!`）能消除重复，但生成代码可读性/调试性差；trait 无法共享"返回 `Self` 的链式 setter"（Rust 无 self-type）。

当前版本自 builder 引入后尚未发布过版本，调用点语法允许破坏性修改。用户选定**方案 B：统一泛型 `WidgetBuilder<Cfg>`**。

## 目标与非目标

**目标**：
- 一个 `WidgetBuilder<Cfg>` + `CommonBuilder`，公共字段/setter/build 尾部只写一遍。
- 每个控件只保留：`XxxCfg`（专属字段 + `new()` 返回 builder）+ `impl WidgetCfg`（默认尺寸/样式/post-insert 设置）。
- 调用点形态：`XxxBuilder::new(...).setters.build(...)` → `XxxCfg::new(...).setters.build(...)`。
- 零行为回归：现有测试（builders/fluent_api 等）全绿即等价性证明。

**非目标**：
- 不做宏生成（本方案就是要替代宏路径）。
- 不改 `define_widgets!`（WidgetKind 枚举/派发机制不动）。
- 不统一 Msgbox（无公共字段可去重，保持独立）。
- 不做性能优化（零运行时开销，纯编译期结构）。

## 设计

### 1. 核心类型（widgets/mod.rs 或 widgets/builder.rs）

```rust
pub struct WidgetBuilder<Cfg> {
    pub(crate) common: CommonBuilder,
    pub(crate) cfg: Cfg,
}

pub(crate) struct CommonBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    style_pressed: Option<Style>,
    style_focused: Option<Style>,
    layout: Option<Layout>,          // 原 Obj 专属，推广为通用
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

pub(crate) trait WidgetCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, common: CommonBuilder) -> ObjRef;
    fn default_style() -> Style { Style::default() }
}
```

`impl<Cfg: WidgetCfg> WidgetBuilder<Cfg>` 提供公共 setter（`size`/`style`/`style_pressed`/`style_focused`/`layout`/`sizing`/`transition`/`on`/`style_with`）与 `build`。

`CommonBuilder::apply_tail(ui, r)` 应用 `layout`/`sizing`/`transition`/`events`（样式默认值因控件而异，留在各 `Cfg::build` 内）。

### 2. 每控件形态（Button 示例）

```rust
pub type ButtonBuilder = WidgetBuilder<ButtonCfg>;
pub struct ButtonCfg { text: String }
impl ButtonCfg {
    pub fn new(text: &str) -> WidgetBuilder<ButtonCfg> {
        WidgetBuilder { common: CommonBuilder::default(), cfg: ButtonCfg { text: text.into() } }
    }
}
impl WidgetCfg for ButtonCfg {
    fn build(self, ui: &mut Ui, parent: ObjRef, common: CommonBuilder) -> ObjRef {
        let (w, h) = common.size.unwrap_or_else(|| /* text 测默认尺寸 */);
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Button(ButtonState { text: self.text }));
        ui.set_style(r, common.style.unwrap_or_else(theme_button));
        ui.set_style_pressed(r, common.style_pressed.unwrap_or_else(theme_button_pressed));
        ui.set_style_focused(r, common.style_focused.unwrap_or_else(theme_button_focused));
        if let Some(n) = ui.arena.get_mut(r) { n.flags |= Flag::CLICKABLE; }
        common.apply_tail(ui, r);
        r
    }
    fn default_style() -> Style { theme_button() }
}
```

控件专属 setter 写在 `impl WidgetBuilder<XxxCfg>` 里：`impl WidgetBuilder<SliderCfg> { pub fn value(self, v: i32) -> Self { self.cfg.value = Some(v); self } }`。专属 setter 不得与公共 setter 同名（见第 4 节注）。

### 3. 各类控件差异的落位

| 控件族 | 处理 |
|---|---|
| 常规 | `insert_node` + 默认样式 + `apply_tail` |
| boxed 状态（List/Roller/ItemList） | `WidgetKind::List(Box::new(State))`，不变 |
| 两段式插入（ScrollView/ItemList） | 占位 Obj → 建 content 子节点 → 覆盖 kind，全部在 `Cfg::build` 内 |
| Canvas（非真实 widget） | 建 Obj + `set_draw_hook` |
| Msgbox | 不转换 |

默认尺寸四种家族（固定常量 / 文本测量 / 内容派生 / 无）：各 `Cfg::build` 内按现状保留，统一读 `common.size.unwrap_or_else(|| <控件默认>)`。

### 4. API 超集变化（均为不破坏现有用法的扩展）

1. Label 获得 `.size()`：显式设置则尊重，否则按文本测。
2. Switch 的开关 setter 改为 `.checked(bool)`（与 Checkbox 一致），事件注册统一用公共 `.on(kind, cb)`；取消原 `on_event`。
   > 注：曾设想用 inherent 方法特化恢复 Switch 的 `.on(bool)`，但 Rust 不允许——泛型 `impl<Cfg>` 与 `impl<WidgetBuilder<SwitchCfg>>` 同名方法直接 `E0592 duplicate definitions`。实测验证失败，故采用 `.checked(bool)`。
3. 全部控件获得 `.style_pressed()`/`.style_focused()`/`.layout()`。
4. `.style_with()` 全量可用，默认样式来自 `Cfg::default_style()`。

### 5. 迁移范围

- `src/widgets/` 20 个文件 → `XxxCfg` + `impl WidgetCfg`；`mod.rs` 加核心类型；`custom.rs` 不动。
- 内部 `create()`（button/label/list）改用 `XxxCfg::new`。
- 全部调用点（examples demo/gallery、benches/memory.rs、tools/qemu-mem、全部 tests，约 267 处）`XxxBuilder::new` → `XxxCfg::new`，import 同步。
- 每个模块导出 `pub type XxxBuilder = WidgetBuilder<XxxCfg>` 供类型标注。

## 验收标准

1. `cargo test -p qingui` 全绿（50 个测试，含 builders.rs/fluent_api.rs 链式调用覆盖）。
2. `cargo check -p qingui --all-targets` 无新 warning。
3. `cargo bench -p qingui --bench memory` 输出数值不变。
4. `cargo test -p qemu-mem` 全绿（scenes.rs 同步改）。
5. 调用点形态：库内无 `XxxBuilder::new(` 残留（除非 XxxBuilder 仅作类型标注）。

## 影响面

- `src/widgets/mod.rs`：+`WidgetBuilder`/`CommonBuilder`/`WidgetCfg`。
- `src/widgets/` 20 个文件：`XxxBuilder` → `XxxCfg` + `impl WidgetCfg`。
- 调用点文件：机械替换构造名与 import。
- 其余零改动。

## 风险与对策

- **两段式插入与 Canvas 改写风险**：改造顺序先简单控件后三个特殊件；现有测试兜底。
- **267 处调用点机械替换**：脚本批量 + 人工复核 import；`cargo test` 作为最终门禁。
- **控件专属 setter 与公共 setter 同名**：已验证 inherent 特化不可行（E0592）；规则是控件专属 setter 一律不与公共 setter 同名（Switch 用 `checked` 而非 `on`）。
