# Widget 尺寸/时间属性可配置化 设计

日期：2026-08-09
状态：已批准（brainstorming 结论）

## 背景

当前各 widget 的大量几何/时间属性是写死的常量或魔术数字（spinner 线宽 3、roller 的 `ROW_H=16`/`ROLL_DUR=150`、list 的 `ROW_H=16`/`FX_DUR=200` 等）。历史上这是有意为之——`docs/superpowers/specs/2026-07-27-rust-lvgl-subset-design.md` 明确砍掉了 LVGL 的 part/selector 机制。本设计在不引入 part 概念的前提下，把这些**尺寸/时间类**属性提到各 widget 的 `Cfg` 里，builder 时链式配置。

## 范围

- **做**：12 个 widget 的尺寸/时间属性（下表），builder 时配置。
- **不做**：零件颜色、easing 曲线（roller/list 保持线性插值）、运行时 setter（要改需重建，或用现有 `ui.update::<State>()` 自行扩展）、spinner 弧形参数、led/switch/spinbox 微调几何、image gif 缺省帧延迟。

## 统一模式

1. 现有 `pub const`（`roller::ROW_H`、`list::FX_DUR`、`table::CELL_W/CELL_H`、`arc::START_DEG/SWEEP_DEG/TRACK_W`、`scrollview::STEP` 等）**保留**，作为默认值——引用它们的 examples/tests 零破坏。
2. 每个 widget 的 `Cfg` 增加普通字段（非 `Option`），以对应常量初始化。
3. `WidgetBuilder<Cfg>` 增加同名链式 setter。
4. 字段随 `Cfg::build` 存入 State；`draw`/`tick` 改读 `self.field` 而非常量。
5. dropdown 弹层中重复的 `16`/`5` 改为引用 `list::ROW_H` 等常量作默认值（顺带消重）。

示例（spinner）：

```rust
SpinnerCfg::new().line_width(5).period_ms(1200).build(ui, parent);
```

## 属性清单

| Widget | 属性（默认值） | 说明 |
|---|---|---|
| spinner | `line_width`(3)、`period_ms`(1800) | period 取代写死的 `now/5 % 360`（5ms/° × 360°） |
| roller | `row_h`(16)、`roll_dur`(150)、`visible_rows`(3) | visible_rows 影响默认高度 |
| list | `row_h`(16)、`fx_dur`(200)、`visible_rows`(5) | fx_dur 是高亮滑动/滚动/增删动画的统一时长 |
| arc | `track_w`(4)、`start_deg`(135)、`sweep_deg`(270) | 替换 `pub const TRACK_W/START_DEG/SWEEP_DEG` 的用途 |
| slider | `knob_w`(8) | knob 宽度（现写死 `Rect(kx-4, .., 8, ..)`） |
| checkbox | `box_size`(12)、`gap`(6) | 勾形坐标随 box_size 按比例缩放 |
| dropdown | `popup_rows`(5)、`popup_row_h`(16)、`popup_min_w`(80) | popup_row_h 默认引用 `list::ROW_H` |
| table | `cell_w`(60)、`cell_h`(16) | |
| scrollview | `step`(20) | 按键滚动像素 |
| chart | `line_width`(2) | 数据线宽 |
| button | `content_pad`((24, 12)) | 文本尺寸之外的默认内扩（宽+24、高+12） |
| msgbox | `size`((200, 110)) | `MsgboxBuilder` 增加 `.size(w, h)` |

itemlist/bar/led/switch/spinbox/label/image/obj 无可开放的尺寸/时间常量（默认尺寸本就可由 `CommonBuilder::size` 覆盖），不在本次范围。

## 实现要点

- **spinner**：`period_ms` 进入 State；draw 中相位改为 `(now % period) * 360 / period`。扫描相位（`now/7 % 300`、弧长 60..210）保持写死（属弧形参数，范围外）。
- **roller/list**：`row_h` 影响默认尺寸计算（`min(visible_rows, n) * row_h + 余量`）、绘制行定位、滚动偏移；`roll_dur`/`fx_dur` 只替换插值里的常量。
- **checkbox**：勾形坐标点 `(2,6)(5,9)(10,3)` 原为相对 `BOX=12` 写死，改为按 `box_size` 等比缩放（整数运算）。
- **button**：`content_pad` 只影响默认尺寸计算（文本测量 + pad）；显式 `.size()` 时不起作用。
- **msgbox**：`MsgboxBuilder` 目前无 size 概念，新增 `.size(w, h)` 覆盖 `(200, 110)`。
- 所有字段存 State 后，现有 `ui.update::<State>()` 路径天然支持运行时修改（本次不提供封装好的 setter）。

## 测试

- 每个改动 widget 加一个测试：配置值生效（spinner 线宽变粗 / roller row_h 改变默认高度与行位置 / list fx_dur 改变动画进度 / table cell_w 改变网格位置等）。
- 默认值下行为与现状一致：现有测试套件不得修改（接口不变、默认渲染不变）。
- dropdown 消重后弹层高度默认值不变（`min(5,n)*16+2`）。

## 兼容性

- 所有 `pub const` 保留且语义不变（仍是默认值）。
- 现有 builder 调用不加新 setter 时行为完全不变。
