# Widget trait object 化重构设计

日期：2026-08-08
状态：已落地（2026-08-08，branch widget-trait-object）

## 1. 背景与目标

当前 `WidgetKind` 是 `define_widgets!` 宏生成的封闭 enum（18+ 变体，小状态 inline、大状态 boxed）。它为内存优化服务，但带来三个结构性问题：

- **扩展封闭**：用户控件只能走 `Custom(Box<dyn custom::Widget>)` 这个二等公民通道，与内置控件不同权。
- **Node 字段均摊税**：`z_index`（2B）、`grid_col/grid_row`（4B）等"部分场景才用"的字段由每个节点无条件背负；`style.layout` 让"任何节点都能挂布局"的语义散落在样式系统里。
- **真实存在的 alloc churn**：`render.rs` 的 `children_z_sorted` 为支持 `z_index`，每帧每块每个节点分配一个 `Vec` 排序。

目标：把 `WidgetKind` 整体替换为**单一 trait object 模型**——`Node` 持有通用数据，`Box<dyn Widget>` 持有行为与非通用数据（downcast 访问）。用户控件与内置控件完全同权，`define_widgets!` 宏与注册表整体删除。

### 非目标（YAGNI，明确不做）

- Layer / sys_layer 体系（modal 的根治方案，后续独立任务立项；本任务仅在结构上不冲突）
- `move_before / move_after` 精确插序 API（有真实需求再加）
- small-kind 内联优化（仅在 benchmark 红线触发时立项，见 §9）
- eg 逐像素绘制路径的性能优化（eg 定位为生态兼容层）

## 2. 核心对象模型

```rust
// node.rs
pub struct Node {
    // —— 树结构 ——
    pub parent: Option<ObjRef>,
    pub children: Vec<ObjRef>,          // 顺序即 z 序（§6）
    pub rect: Rect,
    // —— 状态 ——
    pub state: State,
    pub flags: Flag,
    // —— 行为（唯一 trait object）——
    pub kind: Box<dyn Widget>,          // 16B，替代整个 WidgetKind enum
    // —— 纯视觉样式 ——
    pub style: Style,                   // bg/border/radius/text_color/font/opa
    pub style_pressed: Option<Box<Style>>,
    pub style_focused: Option<Box<Style>>,
    pub style_selected: Option<Box<Style>>,
    // —— 布局属性（从 Style 挪入，运行时值而非 Option 覆盖）——
    pub pad: (i32, i32, i32, i32),      // l,r,t,b，默认 0
    pub sizing_w: Sizing,               // 默认 Content
    pub sizing_h: Sizing,
    pub aspect_ratio: Option<u32>,
    pub transition: Option<(u32, Easing)>,
    pub item_props: ItemProps,          // 子节点对父布局的约束（§4.2）
    pub translate: Point,
    pub floating: Option<(ObjRef, Attach)>,
    // —— 杂项 ——
    pub events: Vec<(EventKind, EventCb)>,
    pub draw_hook: Option<DrawHook>,    // 保留：overlay 绘制 / 边框 debug 等
    pub tick_hook: Option<TickHook>,    // 保留：节点级每帧 hook，与 kind 正交
    pub laid_out: bool,
    // 删除: z_index, opa, grid_col, grid_row
}
```

**单 trait 而非 Layout/View 双 trait**（关键决策）：原设想 `enum Kind { Layout(Box<dyn LayoutTrait>), View(Box<dyn ViewTrait>) }` 互斥二分。评估后发现：按"能力并集"（容器也需要 tick/on_key，如 ScrollView）两个 trait 的方法集几乎完全重合，区别仅剩"哪个方法必需"——而这用默认实现即可表达。且现有代码没有任何位置需要在类型上区分叶子/容器节点（render/layout 统一遍历）。因此收敛为单 trait：

- `draw` 默认空实现 → 布局类控件（Flex/Grid）不覆盖它
- `layout` 默认手动定位 → 叶子控件不覆盖它，正好等于现在 `Layout::None` 的语义
- ScrollView 这类"容器 + 自身行为"的混合体天然成立，无需特殊分类

## 3. Widget trait

`custom.rs` 的 `Widget` trait 转正为全库唯一行为接口：

```rust
pub trait Widget {
    // —— 绘制与测量（&self，不参与 take-out）——
    fn draw(&self, ctx: &WidgetCtx, c: &mut Canvas, clip: Rect) {}      // 默认空：布局类无需绘制内容
    fn measure(&self, ctx: &MeasureCtx) -> (i32, i32);                  // 必需：内容固有尺寸（flex/grid content 轨道依赖）

    // —— 行为（&mut self，经 take-out 拿 &mut Ui，§5）——
    fn layout(&mut self, _ui: &mut Ui, _obj: ObjRef) {}                 // 默认手动定位（现状 Layout::None 语义）
    fn tick(&mut self, _ui: &mut Ui, _obj: ObjRef, _now: u64) -> TickOut { TickOut::IDLE }
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, _key: Key) -> KeyOutcome { KeyOutcome::Pass }

    // —— 属性动画 Value 通道 ——
    fn value(&self) -> i32 { 0 }
    fn set_value(&mut self, _v: i32) -> bool { false }
    fn set_range(&mut self, _min: i32, _max: i32) {}
    fn overflow(&self) -> i32 { 0 }

    // —— downcast ——
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

要点：

- **背景/边框/圆角/opa 仍由 Ui 统一绘制**，`draw` 只画控件内容，契约不变。
- `MeasureCtx` 携带测量所需的只读上下文：解析后的字体（Node style font 或 Ui 默认）与可用空间约束；Label/Table 等文本控件的固有尺寸依赖它。
- **downcast 统一**：`ui.widget::<Slider>(obj) -> Option<&Slider>` 等访问器基于 `as_any` + TypeId，替代宏生成的 18 组 `as_xxx`。`Ui::update` 受控 mutation 入口机制保留不变。
- **内置布局成为 Widget 实现**：`ManualLayout`（吸收 Obj）、`FlexLayout`、`GridLayout`、`ScrollView` 覆盖 `layout`；视觉控件覆盖 `draw`。
- `draw` 签名中的 `DrawBuf` 参数随 §7 升级为 `Canvas`。

## 4. 字段归属决策

### 4.1 Style 纯视觉化

挪出：`pad_*`（→ `Node.pad`）、`sizing_w/h`、`aspect_ratio`、`transition`（→ Node 同名字段）、`layout` 字段删除（布局方式由 kind 决定）。
挪入：`opa: Option<u8>`（自 `Node.opa`；None 按 255，语义不变，且白获状态覆盖改透明度的能力）。
保留：`bg_color/bg_opa/border_*/radius/text_color/font`。

**已确认的后果**：样式状态覆盖（pressed/focused/selected）不再能改变布局参数。现有主题的 overlay 只改颜色，无实际影响；文档写明。

Builder 各控件 `default_style()` 中的 padding/sizing 默认值改为 build 时直接写 Node 字段。

### 4.2 子节点布局约束：ItemProps enum

`grid_col/grid_row` 从独立字段收敛为：

```rust
pub enum ItemProps {
    None,                                   // 父布局不消费约束（默认）
    Grid { col: (u8, u8), row: (u8, u8) },  // (start, span)，父为 GridLayout 时设置
    // 将来可扩展: Flex { grow: u8, shrink: u8 }
}
// Node 上: pub item_props: ItemProps        // ~6B，零堆分配
```

理由：约束属于子节点、跟随子节点生命周期（删除/reparent 零管理）；父布局读取时 match，类型不符按默认值。否决了"GridLayout 内部维护 ObjRef→cell 映射表"（悬挂条目清理成本 + 绕路 API）和"删除显式占位改自动流"（丢失跨格能力）。

## 5. take-out 通道（行为方法拿 &mut Ui）

`layout`/`tick`/`on_key` 三个 `&mut self` 方法需要操作 Ui（如 Dropdown 开弹窗、Msgbox `clear_modal`、布局写子节点 rect）。机制：

- 调用前 `core::mem::replace(&mut node.kind, Box::new(NoopWidget))` 换出 kind，调用后换回。`NoopWidget` 是 ZST，Box 分配不占堆，换入换出零分配。
- 换出期间的规则（写进 trait 文档）：
  - 自身状态直接改 `self`（签名已有 `&mut self`）；
  - 操作其他节点不受限；
  - `ui.update(自身)` 会静默 no-op（kind 不在场）——与现状 Custom 的 caveat 相同。
- `draw`/`measure` 是 `&self`，render/measure 阶段直接不可变借用，不参与 take-out。
- 帧循环结构不变：`timer_handler` = 动画 → 遍历树 tick（`kind.tick` + 节点级 `tick_hook`）→ 布局 → 脏矩形渲染。分发从 enum match 变 dyn 调用，每次一次 vtable 间接，可忽略。

此通道直接消除 Dropdown/Msgbox 现有的回调绕路。

## 6. z_index 删除 / children 即 z 序

- `Node.z_index` 删除；`render.rs` 的 `children_z_sorted` 删除，render 直接按 `&node.children` 顺序遍历——**每帧每块的 Vec 分配归零**。
- 叠放语义：children Vec 顺序即 z 序（靠后在上层），与 LVGL v9 一致。
- API：`set_z_index` 删除，替换为 `move_to_front(obj)`（移到末尾/最上层）、`move_to_back(obj)`（移到开头）。
- 现有使用者：Msgbox / Dropdown 弹窗改创建后 `move_to_front`（等价语义，本就后创建排末尾）。
- Modal 的 `Option<ObjRef>` 焦点锁本任务不动；Layer 体系后续独立任务，结构上无冲突。

## 7. Canvas + eg DrawTarget

`DrawBuf` 升级为正式公开的 `Canvas`：所有 draw 方法的宿主（控件 draw、draw_hook、用户直接渲染共用）。

```rust
pub struct Canvas<'a> {
    buf: &'a mut [Color],   // PFB 当前块
    chunk: Rect,            // 当前块的屏幕绝对区域（原点偏移 + 块边界）
    clip: Rect,             // 当前裁剪（chunk ∩ 节点裁剪）
}
```

- 现有优化过的原语全部挂到 Canvas：`fill_rect`（行批量）、`draw_line`（SDF 粗线）、`fill_rounded`、`circle/arc`（scanline + fringe 采样）——实现不变，只归口。
- 新增 `draw_text(pos, text, font, color)`：font.rs 的 MonoFont 栅格化逻辑搬入，Label 等改调它。
- **eg DrawTarget 兼容层**（`embedded-graphics` 已是依赖，零新增成本）：

```rust
impl DrawTarget for Canvas<'_> {
    type Color = Rgb888;
    type Error = core::convert::Infallible;
    fn draw_iter(&mut self, pixels: I) -> Result<(), Infallible>;  // 偏移 + clip + 写入
    fn fill_contiguous(&mut self, area, colors) -> ...;            // 走行填充快路径
    fn clear(&mut self, color) -> ...;                             // fill_rect 整块
}
```

- 定位写明：**自有快路径优先，eg 是生态兼容层**；逐像素路径不做性能承诺。
- 旧 `canvas.rs` 控件（Obj + draw_hook 薄壳）删除，其用途由 draw_hook 继续覆盖。

## 8. 删除项清单

- `define_widgets!` 宏与整个 widget 注册表（`widgets/mod.rs` 宏部分）
- `WidgetKind` enum 及 18 组 `as_xxx` 访问器（由统一 downcast 替代）
- `widgets/canvas.rs`、`widgets/custom.rs`（CustomState 占位；trait 本身转正）
- `Node.z_index`、`Node.opa`、`Node.grid_col/grid_row`
- `Style.layout`、`Style.pad_*`、`Style.sizing_*`、`Style.aspect_ratio`、`Style.transition`
- `render.rs::children_z_sorted`、`Ui::set_z_index`

## 9. 迁移批次（双轨渐进，每批可编译 + `cargo test` 全绿）

| 批次 | 内容 | 验证重点 |
|---|---|---|
| 0 · 骨架 | `Widget` trait 定义；`Node.kind: Box<dyn Widget>` + take-out（`NoopWidget`）；Node/Style 字段重排；`children_z_sorted` 删除 + `move_to_front/back`；旧宏控件经临时适配层继续编译 | 编译通过，现有测试全绿 |
| 1 · 最简链路 | `ManualLayout`（吸收 Obj）、Label、Button 迁为 trait 实现 | draw/measure/动画 value 通道；demo 可运行；**跑 memory/time benchmark，拿 Box 化首批对比数据** |
| 2 · 布局 | `FlexLayout`、`GridLayout`（读 ItemProps）、ScrollView（验证 Layout 带 on_key/tick） | flex/grid/滚动行为不变 |
| 3 · 交互控件 | Slider/Switch/Checkbox/Spinbox/Arc/Bar/Led | on_key take-out + `&mut Ui` 通道 |
| 4 · 复合控件 | List/ItemList/Roller/Dropdown/Msgbox/Table/Chart/Image/Spinner | Dropdown/Msgbox 去绕路；弹窗改 `move_to_front` |
| 5 · Canvas + 收尾 | DrawBuf → Canvas + `draw_text` + eg DrawTarget；删宏/`canvas.rs`/`custom.rs`；examples/tests/benches 适配 | eg 字体可绘；**benchmark 终测** |

**测试与基准红线**：

- 现有测试套件（含属性测试、QEMU mem bench）每批全跑；只做接口适配，不改断言语义。
- benchmark 在 Batch 1 后、Batch 5 后各记一次基线，对比 2026-08-05 内存基线。
- 红线：Node 静态尺寸应缩小；运行时峰值堆允许上涨（每节点一次 Box），**三档场景峰值涨幅 >15% 则暂停**，评估 small-kind 内联优化（小状态合入单 enum 变体的预案，不预做）。
- 完成后更新 `docs/BENCHMARK.md` 与 README 相关描述。

## 10. 决策记录

| 议题 | 结论 | 理由 |
|---|---|---|
| Layout/View 双 trait 互斥 | 否决，改单 trait | 能力并集下方法集重合，区别可用默认实现表达；无代码消费类型级区分 |
| 行为方法副作用通道 | 统一 take-out，tick/on_key/layout 拿 `&mut Ui` | Custom 已验证；消除 Dropdown/Msgbox 绕路 |
| padding/sizing 等归属 | 全部挪到 Node | Style 纯视觉；代价是覆盖不能改布局（已确认可接受） |
| opa 归属 | 挪入 Style（Option<u8>） | 纯视觉属性，且获得状态覆盖能力；LVGL v9 同款 |
| grid 占位存储 | ItemProps enum 存子节点 | 生命周期安全、零分配；否决父侧映射表与自动流 |
| z_index | 本任务删除，children 即 z 序 | 消除每帧每块 Vec 分配；Layer 留后续任务 |
| Canvas/eg | 并入本任务 | DrawBuf 升级 + DrawTarget，避免二次翻修 |
| 迁移路径 | 双轨渐进（6 批） | 每步可编译可测，benchmark 分批守回归 |
