# qingui 内存优化设计：Style 覆盖独立分配 + WidgetKind 大变体 Box 化

日期：2026-08-05
状态：已获用户批准（讨论式 brainstorming 后确认）

## 背景与动机

内存 bench（`docs/superpowers/specs/2026-08-05-memory-bench-design.md`）首次测量显示每节点内存成本远超预期：

- **`Node` = 1000 B**（host 64 位），其中 **`4×Style` = 672 B（67%）**、`WidgetKind` = 184 B（18%）。
- `WidgetKind` 因内嵌最大变体（`ItemListState` 184 B）形成"最大变体税"——每个纯 Obj/Label 节点都背 184 B。
- LVGL 声称最小配置 ~16 KB 可运行，其关键在于：**共享样式**（style 是静态/全局对象，节点只存引用）+ **按类精确分配**。qingui 则是每节点拷贝 4 份完整 Style + 内嵌最大变体。

目标：把每节点成本降一半以上，且完全行为等价。

## 目标与非目标

**目标**：
- 三个状态覆盖样式（pressed/focused/selected）从 Node 内联 3×168 B 改为 3×`Option<Box<Style>>`（3×8 B），未设置时零堆。Node 1000 → ~520 B。
- `List`/`ItemList`/`Roller` 三个 >40 B 变体 Box 化，`WidgetKind` 184 → ~40 B。
- bench 增加 `Tier::Minimal`（1 Label + 1 Button + 极小缓冲）作为可比 LVGL 的底线档；头部说明"32 位部分减半"（usize 相关减半、i32/固定宽度不减）。

**非目标**：
- 不改渲染缓冲配置（PFB 行缓冲属调用方可配置项，README 已说明）。
- 不盒化 32 B 以下变体（Dropdown/Table/Chart/Image，盒了不划算——边际 < 8 B）。
- 不做 Rc 共享样式（API 变更大，留待后续）。
- 不改任何公开 API。

## 设计

### 1. Node 结构（`node.rs`）

```rust
pub style: crate::style::Style,                                   // base 保持内联
pub style_pressed: Option<alloc::boxed::Box<crate::style::Style>>,
pub style_focused: Option<alloc::boxed::Box<crate::style::Style>>,
pub style_selected: Option<alloc::boxed::Box<crate::style::Style>>,
```

`Node::new` 三个覆盖默认 `None`。设了覆盖的控件（Button/Slider 等 focused）才付 1 次 168 B 堆 + 8 B 指针；纯 Obj/Label/容器零覆盖成本。

### 2. 读取与 setter

`render.rs` `resolved_style`（现 176-180）overlay 取值改 `as_deref()`：

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

`ui.rs` 三个 setter（312-330）API 签名**不变**，内部包 Box：

```rust
pub fn set_style_pressed(&mut self, obj: ObjRef, style: crate::style::Style) {
    if let Some(n) = self.arena.get_mut(obj) {
        n.style_pressed = Some(alloc::boxed::Box::new(style));
    }
    self.invalidate_obj(obj);
}
```

（focused/selected 同。语义与现在完全一致——空 overlay 在 resolve 里仍是无操作。）

`render.rs` 单测（310-312）改为构造 `Some(Box::new(style(color)))`。

**零改动**：约 15 个 widget builder 的 `ui.set_style_*(r, ...)` 调用、`tests/selected.rs`、`tests/style.rs`（API 未变）。

### 3. WidgetKind Box 化（`widgets/mod.rs` 宏 + 构造点）

利用 `AsRef`/`AsMut`（`impl<T> AsRef<T> for T` + `Box<T>: AsRef<T>`），`s.as_ref()`/`s.as_mut()` 对 `&T` 与 `&Box<T>` 统一返回 `&T`/`&mut T`——宏改动极小：

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
            // overflow / value / set_value / set_range / tick / on_key 同：s.as_ref() 或 s.as_mut()
            $(
                pub fn $as(&self) -> Option<&$state> {
                    match self { WidgetKind::$variant(s) => Some(s.as_ref()), _ => None }
                }
                pub fn $as_mut(&mut self) -> Option<&mut $state> {
                    match self { WidgetKind::$variant(s) => Some(s.as_mut()), _ => None }
                }
            )+
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

注册表新增第 4 参：

```rust
define_widgets! {
    Obj(obj::ObjState, as_obj, as_obj_mut, inline),
    Label(label::LabelState, as_label, as_label_mut, inline),
    // ... 其余小状态 inline ...
    List(list::ListState, as_list, as_list_mut, boxed),
    ItemList(itemlist::ItemListState, as_itemlist, as_itemlist_mut, boxed),
    Roller(roller::RollerState, as_roller, as_roller_mut, boxed),
}
```

构造点 3 处加 `Box::new(...)`：`list.rs:336`、`itemlist.rs:113`、`roller.rs:184`。

**公开 API 零变化**：`as_list() -> Option<&ListState>` 签名不变；`downcast_mut` 经 `Box::as_mut` 后仍返回 `&mut ListState`。

**明确不动**：`Custom(Box<dyn Widget>)`、`dropdown.rs:32` 模式匹配（Dropdown 内联）、`call_on_key` 的 `WidgetKind::Obj` 占位、Chart/Dropdown/Table/Image（32 B 以下）。

### 4. bench：Minimal 档 + 32 位说明

`Tier` 加 `Minimal`（1 Label + 1 Button，`Ui::new(160, 120, 8)` → 缓冲 3.8 KB，节点 3）；`bench_scene` match 加 Minimal 的 `LIMIT_PEAK_MINIMAL`/`LIMIT_LIVE_MINIMAL`（测基线后 ×2 校准，无初值）；`main()` 按 Minimal → Small → Medium → Large 打印。

头部注释增强：

> On thumbv7 (32-bit) the usize-dependent parts (Vec/String/Box/pointers) roughly halve, but i32/u32-fixed parts (Rect, ObjRef, and Style's Option<i32> fields) do not — expect ~20-30% lower, not a full halving. Absolute embedded sizes: `cargo size --target thumbv7em-none-eabihf`.

## 验收标准

1. `cargo test -p qingui` 全绿（211+ 测试，`resolved_style` 优先级行为逐字节等价）。
2. `cargo build -p qingui --target thumbv7em-none-eabihf` 通过（no_std 不破坏）。
3. `cargo bench -p qingui --bench memory`：`Node` 1000 → ~520 B（±10%）、`WidgetKind` 184 → ~40 B、四档表 + 32 位说明 + 全断言过。
4. 公开 API 零变化。
5. 阈值用新测量基线 ×2 校准。

## 影响面

- `qingui/src/node.rs`（Node 字段 + new()）
- `qingui/src/render.rs`（resolved_style + 单测）
- `qingui/src/ui.rs`（3 setter 内部包 Box）
- `qingui/src/widgets/mod.rs`（宏 `wtype!` + `.as_ref()/.as_mut()` + 注册表第 4 参）
- `qingui/src/widgets/{list,itemlist,roller}.rs`（构造点 + Box::new）
- `qingui/benches/memory.rs`（Minimal 档 + 32 位说明 + 阈值）

其余 widget 文件、全部 builder、`tests/` 零改动。

## 风险与对策

- **宏改动单点风险**：`wtype!` 选择器 + `.as_ref()/.as_mut()` 统一——改错全库编译错，编译即暴露；211 测试兜底。
- **行为漂移**：`resolved_style` 解引用路径——`focus_visual`/`selected`/`style` 像素测试兜底。
- **结构优化不达预期**：Node > 600 B 或 WidgetKind > 64 B 时 bench 断言红——测基线后 ×2 校准；若结构不足则扩大盒化范围（Dropdown/Table/Chart）再试。
- **32 位说明**：诚实标注"部分减半"（`Option<i32>` 两架构都是 8 B）。
