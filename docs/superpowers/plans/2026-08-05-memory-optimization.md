# Memory Optimization（Style 覆盖独立分配 + WidgetKind Box 化）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把每节点内存成本降一半以上：`Node` 1000 → ~520 B（三个状态覆盖样式从内联 3×168 B 改为 3×`Option<Box<Style>>`）、`WidgetKind` 184 → ~40 B（`List`/`ItemList`/`Roller` 三个 >40 B 变体 Box 化），并给 bench 加 `Minimal` 档 + 32 位说明。

**Architecture:** 纯内部结构重构，公开 API 与行为逐字节不变。`AsRef`/`AsMut`（`impl<T> AsRef<T> for T` + `Box<T>: AsRef<T>`）使宏对 `&T` 与 `&Box<T>` 统一用 `s.as_ref()`/`s.as_mut()`，`define_widgets!` 只需一个 `wtype!` 类型选择器 + 注册表第 4 参（inline/boxed）。

**Tech Stack:** Rust (no_std + alloc), `cargo test` + `cargo bench -p qingui --bench memory`。

## Global Constraints

- **行为保持重构**：公开 API 零变化（`set_style_pressed/focused/selected(obj, Style)`、`as_list() -> Option<&ListState>` 签名不变）；211+ 测试即回归契约。
- **no_std + alloc**：`Box` 一律 `alloc::boxed::Box`。
- **阈值定值程序**：bench 阈值 = 优化后测量基线 × 2（spec 初值不再适用，直接 ×2）。
- **行号漂移**：所有"第 N 行"按**内容**定位。
- **commit message 用英文**（AGENTS.md 规则，Conventional Commits）。
- **git**：只本地 commit，不 push。
- **验证命令**：`cargo test -p qingui`；`cargo build -p qingui --target thumbv7em-none-eabihf`；`cargo bench -p qingui --bench memory`；`cargo check -p qingui --all-targets`。

---

### Task 1: Style 覆盖独立分配（Node/render/ui）

**Files:**
- Modify: `qingui/src/node.rs`（Node 字段 + `new()`）
- Modify: `qingui/src/render.rs`（`resolved_style` + 单测）
- Modify: `qingui/src/ui.rs`（`set_style_pressed/focused/selected`）
- Test: `qingui/tests/selected.rs`、`qingui/tests/style.rs`、`qingui/tests/focus_visual.rs`（契约）+ 全量

**Interfaces:**
- Consumes: 现有 `crate::style::Style`、`resolved_style`、`resolve`。
- Produces: `Node.style_pressed/focused/selected: Option<alloc::boxed::Box<Style>>`（None = 未设置覆盖）。Task 2 依赖 `Node` 新布局（本任务后 Node 变小，但 Task 2 不直接引用这些字段）。

- [ ] **Step 1: node.rs 字段类型**

把 `qingui/src/node.rs` 的 Node 三个覆盖字段（按内容定位：`pub style_pressed: crate::style::Style,` 等三行）替换为：

```rust
    pub style_pressed: Option<alloc::boxed::Box<crate::style::Style>>,
    pub style_focused: Option<alloc::boxed::Box<crate::style::Style>>,
    pub style_selected: Option<alloc::boxed::Box<crate::style::Style>>,
```

`Node::new` 里对应的三行（`style_pressed: crate::style::Style::default(),` 等）替换为：

```rust
            style_pressed: None,
            style_focused: None,
            style_selected: None,
```

- [ ] **Step 2: render.rs `resolved_style` overlay 解引用**

把 `qingui/src/render.rs` 的 `resolved_style` 里 overlay 选择块（按内容定位：`Some(&n.style_pressed)` 等三处）替换为：

```rust
        let overlay = if n.state.contains(State::PRESSED) {
            n.style_pressed.as_deref()
        } else if n.state.contains(State::FOCUSED) {
            n.style_focused.as_deref()
        } else if n.state.contains(State::SELECTED) {
            n.style_selected.as_deref()
        } else {
            None
        };
```

- [ ] **Step 3: render.rs 单测改为构造 Box**

把 `qingui/src/render.rs` 测试模块里 `resolved_style_state_precedence` 的三行直接字段赋值（`n.style_selected.bg_color = Some(...)` 等）替换为：

```rust
        n.style_selected = Some(Box::new(style(Color::rgb(1, 0, 0))));
        n.style_focused = Some(Box::new(style(Color::rgb(2, 0, 0))));
        n.style_pressed = Some(Box::new(style(Color::rgb(3, 0, 0))));
```

（`Box` 已在该测试模块导入，`style(bg)` 是已有辅助。其余断言不变。）

- [ ] **Step 4: ui.rs 三个 setter 包 Box**

把 `qingui/src/ui.rs` 的 `set_style_pressed`/`set_style_focused`/`set_style_selected` 三处 `n.style_pressed = style;` 等赋值替换为：

```rust
            n.style_pressed = Some(alloc::boxed::Box::new(style));
```

```rust
            n.style_focused = Some(alloc::boxed::Box::new(style));
```

```rust
            n.style_selected = Some(alloc::boxed::Box::new(style));
```

（每处对应自己的字段；函数签名与 `invalidate_obj` 调用不变。）

- [ ] **Step 5: 契约测试 + 全量**

Run: `cargo test -p qingui`
Expected: 全绿。重点 `tests/selected.rs`、`tests/style.rs`、`tests/focus_visual.rs`（覆盖优先级像素断言）。

- [ ] **Step 6: no_std 编译**

Run: `cargo build -p qingui --target thumbv7em-none-eabihf`
Expected: 编译成功。

- [ ] **Step 7: Commit**

```bash
git add qingui/src/node.rs qingui/src/render.rs qingui/src/ui.rs
git commit -m "refactor(node): store state overlay styles as Option<Box<Style>> to slim Node"
```

---

### Task 2: WidgetKind 大变体 Box 化（宏 + 构造点）

**Files:**
- Modify: `qingui/src/widgets/mod.rs`（`define_widgets!` 宏 + `wtype!` + 注册表第 4 参）
- Modify: `qingui/src/widgets/list.rs`（构造点 + `Box::new`）
- Modify: `qingui/src/widgets/itemlist.rs`（构造点 + `Box::new`）
- Modify: `qingui/src/widgets/roller.rs`（构造点 + `Box::new`）
- Test: `qingui/tests/list_nav.rs`、`qingui/tests/itemlist.rs`、`qingui/tests/roller_ghost.rs`、`qingui/tests/p1_widgets.rs`（契约）+ 全量

**Interfaces:**
- Consumes: Task 1 的 `Node` 新布局（本任务不引用，仅同 crate 编译）。
- Produces: `WidgetKind::{List, ItemList, Roller}` 变为 `Box<ListState>` 等；`as_list/as_itemlist/as_roller` 及 `downcast_mut` 仍返回 `&ListState`/`&mut ListState`（公开签名不变）。Task 3 依赖新的 `size_of::<WidgetKind>()` 与 `Node` 尺寸。

- [ ] **Step 1: `wtype!` 选择器 + 宏改 `as_ref()/as_mut()`**

把 `qingui/src/widgets/mod.rs` 的 `define_widgets!` 宏整体替换（按内容定位 `pub enum WidgetKind {` 所在宏）：

```rust
/// Variant storage: inline = inlined state, boxed = heap-allocated (large states to
/// avoid the "largest-variant tax").
macro_rules! wtype {
    (inline, $state:ty) => { $state };
    (boxed,  $state:ty) => { alloc::boxed::Box<$state> };
}

macro_rules! define_widgets {
    ($($variant:ident($state:ty, $as:ident, $as_mut:ident, $store:ident)),+ $(,)?) => {
        pub enum WidgetKind {
            $( $variant(wtype!($store, $state)), )+
        }

        impl WidgetKind {
            pub(crate) fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::draw(s.as_ref(), ctx, d, clip), )+ }
            }
            pub(crate) fn overflow(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::overflow(s.as_ref()), )+ }
            }
            pub(crate) fn value(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::value(s.as_ref()), )+ }
            }
            pub(crate) fn set_value(&mut self, v: i32) -> bool {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_value(s.as_mut(), v), )+ }
            }
            pub(crate) fn set_range(&mut self, min: i32, max: i32) {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_range(s.as_mut(), min, max), )+ }
            }
            pub(crate) fn tick(&mut self, now: u64) -> TickOut {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::tick(s.as_mut(), now), )+ }
            }
            pub(crate) fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::on_key(s.as_mut(), key, ctx), )+ }
            }
            $(
                pub fn $as(&self) -> Option<&$state> {
                    match self { WidgetKind::$variant(s) => Some(s.as_ref()), _ => None }
                }
                pub fn $as_mut(&mut self) -> Option<&mut $state> {
                    match self { WidgetKind::$variant(s) => Some(s.as_mut()), _ => None }
                }
            )+
            /// Dispatches a &mut state by type (used by Ui::update); TypeId compare + Any downcast.
            pub(crate) fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
                $(
                    if core::any::TypeId::of::<T>() == core::any::TypeId::of::<$state>() {
                        if let WidgetKind::$variant(s) = self {
                            return (s.as_mut() as &mut dyn core::any::Any).downcast_mut::<T>();
                        }
                    }
                )+
                None
            }
        }
    };
}
```

- [ ] **Step 2: 注册表加第 4 参**

把 `define_widgets! { ... }` 调用块整体替换——所有 21 个变体加 `, inline`，其中 `List`/`ItemList`/`Roller` 加 `, boxed`：

```rust
define_widgets! {
    Obj(obj::ObjState, as_obj, as_obj_mut, inline),
    ItemList(itemlist::ItemListState, as_itemlist, as_itemlist_mut, boxed),
    Label(label::LabelState, as_label, as_label_mut, inline),
    Button(button::ButtonState, as_button, as_button_mut, inline),
    Slider(slider::SliderState, as_slider, as_slider_mut, inline),
    Switch(switch::SwitchState, as_switch, as_switch_mut, inline),
    Bar(bar::BarState, as_bar, as_bar_mut, inline),
    List(list::ListState, as_list, as_list_mut, boxed),
    Arc(arc::ArcState, as_arc, as_arc_mut, inline),
    Checkbox(checkbox::CheckboxState, as_checkbox, as_checkbox_mut, inline),
    Chart(chart::ChartState, as_chart, as_chart_mut, inline),
    Spinner(spinner::SpinnerState, as_spinner, as_spinner_mut, inline),
    Msgbox(msgbox::MsgboxState, as_msgbox, as_msgbox_mut, inline),
    Led(led::LedState, as_led, as_led_mut, inline),
    Table(table::TableState, as_table, as_table_mut, inline),
    Spinbox(spinbox::SpinboxState, as_spinbox, as_spinbox_mut, inline),
    Roller(roller::RollerState, as_roller, as_roller_mut, boxed),
    ScrollView(scrollview::ScrollViewState, as_scrollview, as_scrollview_mut, inline),
    Dropdown(dropdown::DropdownState, as_dropdown, as_dropdown_mut, inline),
    Image(image::ImageState, as_image, as_image_mut, inline),
    Custom(custom::CustomState, as_custom_state, as_custom_state_mut, inline),
}
```

- [ ] **Step 3: 三个构造点加 `Box::new`**

`qingui/src/widgets/list.rs`（按内容定位 `WidgetKind::List(ListState {`）：

```rust
            WidgetKind::List(Box::new(ListState { items: self.items, selected, scroll: 0, fx: ListFx::default() })),
```

`qingui/src/widgets/itemlist.rs`（`WidgetKind::ItemList(ItemListState {`）：

```rust
            n.kind = WidgetKind::ItemList(Box::new(ItemListState { selected: 0, content, sel_style }));
```

`qingui/src/widgets/roller.rs`（`WidgetKind::Roller(RollerState {`）：

```rust
            WidgetKind::Roller(Box::new(RollerState { items: self.items, selected, sel_from: None })),
```

- [ ] **Step 4: 全量编译 + 契约测试**

Run: `cargo test -p qingui`
Expected: 全绿（重点 `list_nav.rs`、`itemlist.rs`、`roller_ghost.rs`、`p1_widgets.rs`——List/ItemList/Roller 的行为经 `as_list`/`as_itemlist`/`as_roller` + `downcast_mut` 全路径覆盖）。

Run: `cargo check -p qingui --all-targets`
Expected: 无新 warning。

- [ ] **Step 5: no_std 编译**

Run: `cargo build -p qingui --target thumbv7em-none-eabihf`
Expected: 编译成功。

- [ ] **Step 6: Commit**

```bash
git add qingui/src/widgets/mod.rs qingui/src/widgets/list.rs qingui/src/widgets/itemlist.rs qingui/src/widgets/roller.rs
git commit -m "refactor(widgets): box large WidgetKind variants (List/ItemList/Roller) to cut size tax"
```

---

### Task 3: bench Minimal 档 + 32 位说明 + 阈值重校

**Files:**
- Modify: `qingui/benches/memory.rs`

**Interfaces:**
- Consumes: Task 1/2 后的新尺寸（Node ~520 B、WidgetKind ~40 B、三档峰值变小）。
- Produces: 最终可交付 bench（四档 + 32 位说明 + 新阈值）。

- [ ] **Step 1: 加 `Tier::Minimal` 与场景**

`qingui/benches/memory.rs`：

- `enum Tier { Minimal, Small, Medium, Large }`
- `build_scene` 开头加 Minimal 分支（现有 match 变四臂）：

```rust
        Tier::Minimal => {
            use qingui::widgets::button::ButtonBuilder;
            use qingui::widgets::label::LabelBuilder;
            let mut ui = qingui::Ui::new(160, 120, 8);
            let scr = ui.screen();
            LabelBuilder::new("hello").build(&mut ui, scr);
            ButtonBuilder::new("OK").build(&mut ui, scr);
            ui.tick_inc(16);
            ui.timer_handler();
            ui
        }
```

- `bench_scene` 的 `match tier` 加一臂：`Tier::Minimal => (LIMIT_PEAK_MINIMAL, LIMIT_LIVE_MINIMAL),`
- `main()` 在 small 前加 `bench_scene("minimal", Tier::Minimal);`

- [ ] **Step 2: 加 32 位说明到头部注释**

把 `memory.rs` 头部 `NOTE:` 段改为（追加部分减半的诚实说明）：

```rust
//! NOTE: this runs on the host (64-bit, usize = 8B). The embedded thumbv7
//! target is 32-bit (usize = 4B). On thumbv7 the usize-dependent parts
//! (Vec/String/Box/pointers) roughly halve, but i32/u32-fixed parts (Rect,
//! ObjRef, and Style's Option<i32> fields) do not — expect ~20-30% lower,
//! not a full halving. Absolute embedded sizes come from
//! `cargo size --target thumbv7em-none-eabihf`. This bench gives the
//! RELATIVE cost shape and a regression gate.
```

- [ ] **Step 3: 测量优化后基线（旧阈值仍过）**

Run: `cargo bench -p qingui --bench memory`
Expected: 全表打印（静态尺寸现在显示 Node ~520、WidgetKind ~40、List 变体 8 B）+ 四档（minimal/small/medium/large）+ 旧阈值断言全过（优化后更小，不会触发）。把四档 peak/live 与 Node/WidgetKind/Style 新基线记入报告。

- [ ] **Step 4: 重校阈值常量**

把 const 块全部替换为新基线 ×2（`max(新基线 × 2, 原常量)`，原常量只会更大——直接用新基线 ×2 即可；minimal 无旧值，直接 ×2）：

```rust
// Thresholds recalibrated <date> after the memory optimization: new baseline x 2.
const LIMIT_WIDGETKIND: usize = /* 新 WidgetKind x 2 */;
const LIMIT_STYLE: usize = /* 新 Style x 2（Style 未变则保持 336） */;
const LIMIT_NODE: usize = /* 新 Node x 2 */;
const LIMIT_PEAK_MINIMAL: usize = /* minimal peak x 2 */;
const LIMIT_LIVE_MINIMAL: usize = /* minimal live x 2 */;
const LIMIT_PEAK_SMALL: usize = /* small peak x 2 */;
const LIMIT_LIVE_SMALL: usize = /* small live x 2 */;
const LIMIT_PEAK_MEDIUM: usize = /* medium peak x 2 */;
const LIMIT_LIVE_MEDIUM: usize = /* medium live x 2 */;
const LIMIT_PEAK_LARGE: usize = /* large peak x 2 */;
const LIMIT_LIVE_LARGE: usize = /* large live x 2 */;
```

（逐项用 Step 3 实测值 ×2 填入；`Style` 本身未改，`LIMIT_STYLE` 保持 336 即可。）

- [ ] **Step 5: 验证**

Run: `cargo bench -p qingui --bench memory`
Expected: 四档表 + 新阈值断言全过，退出码 0。

Run: `cargo test -p qingui`
Run: `cargo check -p qingui --all-targets`
Expected: 全绿、无新 warning（除 pre-existing roller_ghost）。

- [ ] **Step 6: Commit**

```bash
git add qingui/benches/memory.rs
git commit -m "bench: add minimal tier, 32-bit note, and recalibrated thresholds"
```

---

## Self-Review

**Spec 覆盖：**
- ① Style 覆盖独立分配 → Task 1（node/render/ui + 单测）。
- ② WidgetKind Box 化 → Task 2（宏 + 注册表 + 3 构造点）。
- bench Minimal 档 + 32 位说明 → Task 3（Step 1-2）。
- 阈值重校 → Task 3（Step 3-4）。
- 验收：`cargo test` 绿、no_std 过、`Node`/`WidgetKind` 达标、API 零变化 → 各任务验证步骤 + Task 3 的基准变化。

**占位符扫描：** 无 TBD/TODO。Task 3 的常量值由 Step 3 实测基线驱动（`新基线 × 2`），是确定性程序；每处标了来源。

**类型一致性：**
- `wtype!` 选择器在 enum 变体与匹配处一致使用；`.as_ref()/.as_mut()` 对 inline/boxed 统一。
- `as_xxx`/`downcast_mut` 公开签名不变（`Option<&ListState>` 等）——Task 1/2 之后所有 `ui.as_list(obj)` 等调用点继续编译。
- `Node.style_pressed/focused/selected` 在 node.rs（定义）、render.rs（读）、ui.rs（写）、render.rs 测试（写）四处签名一致（`Option<Box<Style>>`）。
- `Tier::{Minimal,Small,Medium,Large}` 在 build_scene 与 bench_scene 两处 match 一致。
- 三个 Box 化变体与三个构造点一一对应（list/itemlist/roller）。
