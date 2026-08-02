# Demo 新增 ItemList 页设计

日期:2026-08-01
状态:已与用户确认

## 目标

在 demo(qingui/examples/demo.rs)中新增一个 "ItemList" 页,展示复杂 ItemList(每 item 三控件)与流式 chart(上下两个折线图,不同颜色)。纯 example 改动,不动库代码。

## 已确认的决策

| 决策点 | 结论 |
|---|---|
| 页面安排 | 菜单加第 6 项 "ItemList"(idx 5),chart 与 itemlist 同一页 |
| chart 数据 | 流式滚动(run_with_tick 周期 push 正弦数据) |
| item 交互 | Enter 翻转选中 item 的 checkbox,LED 亮灭跟随 |

## 布局与结构

- 菜单数组(demo.rs:62)加 `"ItemList"`;页面切换逻辑(demo.rs:242-259)扩展 idx 5。
- `page_itemlist` 挂在 panel 下,column flex:上半两个 chart(各 GROW 宽、均分高度),下半 ItemList(GROW)。
- Chart ×2:上蓝色、下橙色,各 `range(0,100)`、单序列、容量 48。
- ItemList:8 个 item;每 item 内设 row flex(cross center, gap 8):LED + Label("Sensor 01"…)+ Checkbox。

## 数据流

- `main` 由 `sim::run(build)` 改为 `sim::run_with_tick(build, tick)`;build 与 tick 闭包通过 `Rc<RefCell<...>>` 共享两个 chart 的 ObjRef(demo 为 std 环境)。
- tick 每 ~100ms 向两个 chart 各 push 一点:两条相位差 π/2 的正弦波(0..100)。
- 交互:ItemList 的 Enter 走 `KeyOutcome::Pass` → 默认发 Clicked(itemlist.rs:20-26 已确认);监听 itemlist 的 Clicked,按 `itemlist_selected` 索引翻转对应 item 的 checkbox(`set_value(cb, 1-v)`),LED 跟随(`set_value(led, v*255)`)。item 的 (led, checkbox) ObjRef 对存入 `Rc<RefCell<Vec<...>>>` 供回调索引。

## 验证

- `cargo check --examples` 通过。
- 手动:sim 中进入 ItemList 页,两条折线滚动、颜色不同;上下键移动选中,Enter 翻转 checkbox 且 LED 联动。

## 影响面

仅 `qingui/examples/demo.rs`。
