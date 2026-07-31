# qingui 架构重构设计：句柄 API + 状态 struct 化 + 行为归一

日期：2026-07-31
状态：已获用户批准（讨论式 brainstorming 后确认）

## 背景与动机

当前 qingui 是"结构化数据 + 中央解释器"架构：`Arena<Node>` 存全部节点（`ObjRef` 为句柄），`Ui` 独占 arena 并承担布局/动画/fx/渲染/事件全部分派。痛点：

1. 所有公开 API 都是 `Ui` 方法 + `ObjRef` 参数，无法"拿到节点直接操作节点"。
2. "widget" 被切成三处：状态在 `WidgetKind` enum 变体内联字段、绘制在 `widgets/xx.rs` 自由函数、行为散落在 `Ui` 约 8 处 match 分派点（`widgets::draw`、`value_of/set_value_of/overflow_of`、`tick_list_fx`、`keypad_input`、`activate`、`open_dropdown`、各 getter）。widget 文件拆分只是"把函数挪了文件"。
3. 三套逐帧机制并存：属性动画（`AnimProp` + setter 回放）、widget 内嵌 fx（draw 里插值 + `tick_list_fx` 每帧标脏补丁）、Canvas（用户回调，连帧驱动都没有，gallery 用隐藏 Bar + Value 动画手动标脏驱动）。
4. `draw_node` 每帧对每个可见节点 `kind.clone()` 深拷贝（含 List 的 `Vec<String>`）；Canvas 因回调不可 Clone 被迫走旁路注册表，成为特例。
5. `Ui::create_xx` 与 Builder 两套创建路径并存，`ui.rs` 达 1343 行。

## 关键认知（本次讨论的共识）

- **Node 不可能存 `&Ui`**：Node 归 `Ui.arena` 独占，存回引用是自引用结构，安全 Rust 写不出来。"节点自己操作自己"的极限是方法接收 `&mut Ui`。拒绝 interior mutability（`RefCell`）路线：把借用错误推迟到运行时 panic，嵌入式场景不可接受。
- **vtable 不需要继承**：用户想要的"View 父类 + 虚函数"本质是一张行为分派表。当前问题是这张表被打散成 8 处 match 写进了 `Ui`。
- **异构集合只有两种装法**：`dyn Trait` 或 enum。`&impl Widget`（泛型）无法容纳混类型对象树。enum 持有各 widget 状态 struct 即"静态版开放系统"：查询不用 downcast（`as_list() -> Option<&ListState>` 是一个 match）、零 vtable 开销、可 Clone。
- 决策：widget 集对内置封闭，enum 加 `Custom(Box<dyn Widget>)` 变体作为用户自定义 widget 的逃生舱。

## 目标与非目标

**目标**：对外 API 简单（允许提高内部实现复杂度）；结构清晰（行为定义回到各 widget 文件，`Ui` 不含 widget 知识）。

**非目标**：
- 不做 pointer/touch 输入（保持键盘输入）。
- 不做样式继承级联重构。
- fx 与属性动画在语义上保留两套（per-item 状态动画 vs 节点属性插值），只统一驱动机制。

## 设计

### 1. 对外 API：句柄方法 + Builder

`ObjRef` 上直接实现方法（薄封装，转调 `ui` 内部），`&mut Ui` 是显式的"世界"参数：

```rust
let list = ListBuilder::new(&["Settings", "Display"]).build(&mut ui, screen);
list.set_sizing(&mut ui, Some(Sizing::GROW), None);
list.select(&mut ui, 2);                    // widget 专属操作也是方法
let sel = list.selected(&ui);               // 查询返回 Option<usize>
list.on(&mut ui, EventKind::ValueChanged, |ui, obj, _| { ... });

// 通用操作（相当于"View 父类"的方法，约 20 个）：
obj.set_style(&mut ui, s);
obj.set_pos(&mut ui, x, y);
obj.invalidate(&mut ui);
```

- `Ui::create_xx` 全部删除，创建只走 Builder（17 个现有 Builder 保留形态，`build(&mut ui, parent) -> ObjRef`）。
- `Ui` 的公开面缩到：`new / screen / timer_handler / keypad_input / take_dirty` + 焦点组管理（`group_add` 等）。
- 所有 setter 内部自动 invalidate——节点方法接收 `&mut Ui` 正是为此，不存在"忘调 invalidate"的可能。
- `WidgetMut`（`ui.widget(obj)` 链式包装）删除，由句柄方法取代。

### 2. 内部结构：状态 struct 化 + Custom 逃生舱

每个 widget 文件拥有真正的类型与 impl 块：

```rust
// widgets/list.rs
pub struct ListState {
    pub items: Vec<String>,
    pub selected: usize,
    scroll: i32,
    fx: ListFx,
}
impl ListState {
    pub(crate) fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { ... }
    pub(crate) fn tick(&mut self, now: u64) -> bool { ... }   // fx 推进 + 活动检测
    pub(crate) fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, k: Key) -> bool { ... }
}
```

`WidgetKind` 改为持有状态 struct 的 enum，并提供委托方法（新增内置 widget 唯一的分派点）：

```rust
// widgets/mod.rs
pub enum WidgetKind {
    Obj, Label(LabelState), Button(ButtonState), List(ListState), /* …全部 18 种… */
    Custom(Box<dyn Widget>),
}
impl WidgetKind {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) { match self { ... } }
    fn tick(&mut self, now: u64) -> bool { match self { ... } }       // 默认 false
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, k: Key) -> bool { match self { ... } } // 默认 false
    fn value(&self) -> Option<i32> { match self { ... } }             // 属性动画 Value 通道
    fn set_value(&mut self, v: i32) { match self { ... } }
    fn range(&self) -> Option<(i32, i32)> { ... }
    fn overflow(&self) -> i32 { ... }                                 // 原 overflow_of
}
```

用户自定义 widget：

```rust
pub trait Widget {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect);
    fn tick(&mut self, now: u64) -> bool { false }
    fn on_key(&mut self, ui: &mut Ui, obj: ObjRef, key: Key) -> bool { false }
    fn as_any(&self) -> &dyn Any;
}
```

- 创建：`ui.create_custom(parent, Box::new(MyGauge { .. })) -> ObjRef`。
- 查询：`obj.custom::<MyGauge>(&ui) -> Option<&MyGauge>`（经 `as_any` downcast，全系统唯一一处）。
- 内置查询：`kind.as_list() -> Option<&ListState>`（一个 match，零开销）。

`Ui` 不再含 widget 知识：

- `keypad_input` 把键发给焦点节点的 `kind.on_key(...)`，返回 `false` 才走默认行为（移焦/激活）；`activate` 简化为"发 Enter 键"。Spinbox 编辑态按键处理搬回 spinbox.rs。
- `open_dropdown` 逻辑搬回 dropdown.rs。
- `value_of / set_value_of / range_of / overflow_of` 四个 match 收敛为 `WidgetKind` 的方法。

### 3. 帧循环与绘制统一

**tick 统一**：`timer_handler` 帧循环改为"遍历树，调每个节点的 `kind.tick(now)` 及节点级 `tick_hook`，返回 true 则标脏并保持唤醒（返回 0），全部不活动则返回 `u32::MAX` 睡眠"。

- `tick_list_fx` 删除。List fx / Roller `sel_from` / Spinner 自转各自在 `tick` 内实现。
- 节点级 tick 钩子：`obj.on_tick(&mut ui, |ui, obj, now| -> bool { ... })`，签名 `Box<dyn FnMut(&mut Ui, ObjRef, u64) -> bool>`。替代 gallery 中"隐藏 Bar + Value 动画驱动 Canvas"的 hack。存储在 Node 上，调用用 take-调用-放回模式（与事件回调 remove-reinsert 同法）。

**draw 统一**：

- 背景 / 边框 / opa 合成仍由 `Ui::draw_node` 统一处理（内置 widget 免样板）。
- 内容绘制走 `kind.draw(...)`。
- 每个 Node 增加 `draw_hook: Option<Box<dyn FnMut(&mut DrawBuf, Rect, Rect, u64)>>`，在自带内容之后叠加调用。`obj.on_draw(&mut ui, cb)` 对任意节点可用（即 Android 式 draw 覆写，且可叠加在自带内容之上）。
- `WidgetKind::Canvas` 变体与 `canvas_cbs` 旁路注册表删除；保留 `CanvasBuilder` 作为"空节点 + draw_hook"的糖（对外 API 连续性），其内部实现改为创建 `WidgetKind::Obj` 节点并挂 hook。

**消除每帧 `kind.clone()`**：draw 全程只读借用 kind（fx 插值本就只读），用户 hook 用 take-调用-放回，不再为绕借用深拷贝 `Vec<String>`。

### 4. 查询与测试

- `debug_kind` 测试后门删除，测试改用公开的只读查询方法。
- 全部测试（约 26 文件 / 3100 行）迁移到新 API；像素断言（`RecFlush`）方式保留。
- `tests/builders.rs` 中"builder ≡ create_xx"断言随 `create_xx` 一起删除。
- examples（demo / gallery / sim）迁移到新 API。

## 实现影响面（预估）

- `qingui/src/ui.rs`：1343 行 → 预计收缩到约一半，只含树操作、布局、动画推进、渲染管线、焦点/事件分发。
- `qingui/src/widgets/mod.rs`：enum 定义 + 委托 impl + `as_xxx` 查询。
- `qingui/src/widgets/*.rs`（17 个文件）：各自增加 `XxxState` struct 与 impl（draw/tick/on_key），行为代码从 `ui.rs` 搬入。
- `qingui/src/widget.rs`：从 `WidgetMut` 改为 `impl ObjRef` 的句柄方法（或拆为 `handle.rs`）。
- 新增：`qingui/src/widgets/custom.rs`（`Widget` trait + create_custom）。

## 验收标准

1. `cargo test` 全部通过（迁移后），`cargo build --target thumbv7em-none-eabihf` 通过（no_std 不破坏）。
2. `ui.rs` 中不存在任何 `match`/`if let`/`matches!` 对具体 `WidgetKind` 变体的分支（委托方法集中在 `widgets/mod.rs`）。
3. `tick_list_fx`、`canvas_cbs`、`debug_kind`、`Ui::create_xx`、`WidgetMut` 均不存在。
4. draw 路径无 `kind.clone()`。
5. gallery 的 Canvas 动画改用 `on_tick` + `on_draw`，无隐藏 Bar hack。
6. 一个 examples 级演示：用 `create_custom` 实现一个自定义 widget 并参与 draw/tick/按键。
