# ItemList 设计：容器型列表控件 + 视口裁剪 + 选中态样式

日期：2026-08-01
状态：已获用户批准（讨论式 brainstorming 后确认）

## 背景与动机

现有 `List` 是轻量虚拟文本列表：一个节点 + `Vec<String>`，自绘文本行（行高固定 16px），选择高亮/滚动/ensure_visible/增删 fx 全部自绘。它服务长列表场景（100 项 = 1 个节点），但 item 只能是文本。

用户需求：item 可以是一棵子 UI 树（图标+文字、文字+开关、多行、不等高），即 LVGL 的 `lv_list` 形态（容器 + button 子项）。

## 决策记录

- **方案 A（并存）**：文本 `List` 原样保留服务长列表；新增容器型控件 `ItemList` 服务菜单型场景。否决方案 B（改造 List 为容器型：失去轻量选项、API 全破、churn 大）与方案 C（双模式：两套语义交织）。
- **选中态用 `State::SELECTED`，不复用 FOCUSED**：FOCUSED 由焦点系统占有（focus_to 自动设置/清除）且是输入路由的视觉对应物；选中与焦点生命周期正交（选中应在焦点移出后保持；item 内可聚焦子件会造成两个"focused"视觉无法区分）。LVGL 的 `LV_STATE_CHECKED` 独立于 `LV_STATE_FOCUSED` 同构。

## 设计

### 1. 视口裁剪基础设施（通用能力）

- `Flag` 加 `CLIP_CHILDREN = 1 << 3`。
- `Ui::draw_node` 递归子节点前：父节点带此标志时，`clip = clip ∩ 父 abs`（矩形裁剪，忽略圆角）；不相交则整棵子树跳过。
- 提供 `ui.set_clip_children(obj, bool)`（pub）+ 句柄无关（Ui 中心 API）。
- 标脏逻辑不变。未来 ScrollView、长 Dropdown 弹层可复用。

### 2. 选中态样式基础设施（通用能力）

- `State` 加 `SELECTED = 1 << 4`。
- `Node` 加 `style_selected: Style` 字段（`Node::new` 初始化为 `Style::default()`）。
- `resolved_style` 叠加链：pressed > focused > selected（保持现有互斥取一语义）。
- `ui.set_style_selected(obj, style)`（pub）；`set_state` 走现有通用入口。

### 3. ItemList 结构与状态

三层节点结构：

```
ItemList 节点（WidgetKind::ItemList，视口；CLIP_CHILDREN；样式背景/边框归用户/theme）
└── content 容器（WidgetKind::Obj，Flex column，宽 GROW；滚动 = 它的 translate.y）
    ├── item 0（Obj 容器，用户往里搭任意内容）
    ├── item 1
    └── …
```

```rust
// widgets/itemlist.rs
pub struct ItemListState {
    pub selected: usize,
    pub(crate) content: ObjRef,
}
```

- 不假设行高：item 高度由内容/布局决定，天然支持不等高。
- 选中切换 = 旧项 `set_state(SELECTED, false)` + 新项 `set_state(SELECTED, true)`，样式叠加自动生效，零自绘。
- `ItemListState::on_key`：Up/Down 循环移动选中（与文本 List 一致）+ `ensure_visible`（按选中 item 的 rect 与视口高度比较，瞬调 content 的 `translate.y`，无动画）；Enter → `KeyOutcome::Clicked`-等价（见下）；其余 `Pass`。
- `KeyOutcome` 需要支持"发 Clicked"：优先复用现有机制——`Pass` 后默认行为 Enter 即 `send_event(Clicked)`，故 ItemList 的 Enter 直接返回 `Pass`，无需新增 outcome 变体。

### 4. API（Ui 中心风格，与现状一致）

```rust
// Builder（widgets/itemlist.rs，形态对齐现有 17 个 Builder）
pub struct ItemListBuilder { size, style, style_selected, sizing, transition, events }
ItemListBuilder::new()
    .size(120, 100)
    .style_selected(Style::new().bg(Color::rgb(50, 70, 120))) // 默认值即此（对齐文本 List 高亮色）
    .build(&mut ui, parent) -> ObjRef

// Ui 方法
pub fn itemlist_add_item(&mut self, il: ObjRef) -> ObjRef   // 建 item 容器（Obj，宽 GROW，透明背景），挂 content 下，返回之
pub fn itemlist_remove_selected(&mut self, il: ObjRef) -> bool // 删除选中 item（空列表 false），selected 收敛到合法范围
pub fn itemlist_select(&mut self, il: ObjRef, idx: usize)   // clamp 到合法范围；变化才切换 + 发 ValueChanged
pub fn itemlist_selected(&self, il: ObjRef) -> usize        // 非 ItemList 返回 0（对齐 list_selected 语义）
pub fn itemlist_len(&self, il: ObjRef) -> usize             // content 的 children 数
```

- **事件语义（与文本 List 有意不同）**：任何选中变化（`itemlist_select` 或键盘导航）都发 `EventKind::ValueChanged`（支持"移动即预览"）；Enter 发 `EventKind::Clicked`（确认）。文本 List 的导航不发 ValueChanged，保持不变。
- item 内容完全归用户（Builder 任意搭）；ItemList 不碰 item 基础样式。
- `WidgetKind` 的 `value/set_value`：`ItemList` 接入现有值抽象（value = selected as i32；set_value = select），使属性动画与通用值 API 自然可用。
- `ensure_visible` 算法（对齐文本 List 语义，瞬时无动画）：选中 item 的 rect 顶边在视口顶之上 → 上滚使其顶对齐（`translate.y += 差值`）；底边在视口底之下 → 下滚使其底对齐（`translate.y -= 差值`）；已可见不动；item 高于视口时优先顶部对齐。

### 5. 非目标（YAGNI）

- 不做 item 增删动画、选中滑动动画（瞬时切换，对齐 LVGL 菜单）。
- 不做 item 回收/复用（虚拟化）。
- 不改文本 `List` 的任何行为与 API。
- 不做 item 内子件的焦点组管理（item 内容要可聚焦由用户自行 group_add，语义与 SELECTED 正交）。

### 6. 测试

- 像素级：SELECTED 样式叠加（选中项 bg 变化、取消还原）；CLIP_CHILDREN 生效（item 超出视口部分被裁）；滚动后内容位置正确。
- 行为级：Up/Down 循环导航；不等高 item 的导航与 ensure_visible 滚动量；add/remove/selected/len；ValueChanged/Clicked 事件触发条件；`value/set_value` 接入正确；空列表按键不 panic。

## 影响面（预估）

- `qingui/src/node.rs`：Flag 1 位、State 1 位、Node 1 字段。
- `qingui/src/ui.rs`：draw_node 裁剪分支（<10 行）；set_clip_children/set_style_selected；itemlist_* 5 个方法。
- `qingui/src/widgets/itemlist.rs`：新建（ItemListState + on_key + Builder + add/remove/select 逻辑，约 300 行）。
- `qingui/src/widgets/mod.rs`：ItemList 变体 + 委托臂（draw 空/tick 无/on_key/value/set_value/as_itemlist）。
- `qingui/tests/itemlist.rs`、`qingui/tests/clip.rs`：新建。
- examples：gallery 或 demo 增加一个 ItemList 示例（图标+文字菜单）。

## 验收标准

1. `cargo test -p qingui` 全绿（含新测试），`cargo build -p qingui --target thumbv7em-none-eabihf` 通过。
2. 文本 List 相关测试（list_fx/list_nav 等）零改动通过。
3. CLIP_CHILDREN 与 SELECTED 均有独立像素级测试。
4. `grep -n "WidgetKind::" qingui/src/ui.rs` 不新增任何变体分支（itemlist_* 方法经 `as_itemlist` 访问器）。
