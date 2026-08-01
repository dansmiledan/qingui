# Chart 控件设计（折线图）

日期：2026-08-01
状态：已与用户确认

## 目标

为 qingui 新增折线图控件 `Chart`，主打嵌入式流式趋势监控场景。对外 API 保持简单，实现遵循现有 widget 模式。

## 已确认的需求决策

| 决策点 | 结论 |
|---|---|
| 图表类型 | 仅折线（line） |
| 数据模式 | 流式追加为主（固定容量，满挤最旧），附整体替换 |
| 序列数量 | 多序列（每条独立容量/颜色/数据） |
| Y 轴 | 固定范围（builder/API 显式设 min/max），点超界钳制 |
| 视觉元素 | 纯数据线，无网格/轴线/刻度标签 |
| 交互 | 无（不进焦点组、不处理按键） |
| X 语义 | 按容量定位、从左填充（数据未满时右侧留白），LVGL 默认行为 |
| 架构方案 | 一等公民 widget：`WidgetKind::Chart(ChartState)` + Builder + Ui API |

## 状态模型

```rust
// qingui/src/widgets/chart.rs
pub struct ChartState {
    pub min: i32,              // Y 轴固定范围
    pub max: i32,
    pub series: Vec<Series>,
}

pub struct Series {
    pub color: Color,
    pub capacity: usize,       // ≥1（传 0 钳到 1）
    pub points: VecDeque<i32>, // 满时 push_back + pop_front，无搬迁无重分配
}
```

- 序列 id 即 `Vec` 索引（usize）；不提供 remove_series，索引永远稳定。
- `WidgetKind::Chart(ChartState)` 挂现有枚举，走通用 draw/dirty 管线，无特殊分支。

## 坐标语义

- X：点等间距水平分布，`x_i = left + i * (w-1) / (capacity-1)`；从左填充，未满时右侧留白。`capacity == 1` 时唯一点画在水平中心。
- Y：`v` 钳到 `[min, max]` 后 `y = bottom - (v-min)*(h-1)/(max-min)`；`min == max` 时画水平中线（避免除零）。

## API

```rust
// Builder（与其他 widget 一致：size/sizing/style/events 齐备）
ChartBuilder::new()
    .range(0, 100)              // 默认 (0, 100)
    .size(120, 60)
    .series(Color::BLUE, 32)    // 可多次调用预建序列；不调用则 0 条序列
    .build(&mut ui, parent)     // -> ObjRef

// Ui 上的数据 API
ui.chart_add_series(chart, color, capacity) -> usize // 返回序列索引
ui.chart_push(chart, series, v)          // clamp 入 range；满则挤掉最旧；invalidate_obj
ui.chart_set_points(chart, series, &[i32]) // 整体替换；超容量只保留最新 capacity 个
ui.chart_clear(chart, series)
ui.chart_point_count(chart, series) -> usize
```

## 行为

- **绘制**：通用 `draw_node` 先画 style 背景/边框；chart 的 `draw` 只画数据线——相邻点 `draw_line`（宽 2，固定不可配），单点序列画一个 `fill_circle`，空序列跳过。
- **overflow() = 0**：y 钳制后连线必在节点矩形内，无越界。
- **标脏**：每次数据变更整表 `invalidate_obj`（与 LVGL 一致；流式场景整表反正要重画）。
- **tick = IDLE**：不自转，无每帧分配。
- **错误处理**：无效 ObjRef / 越界 series 索引一律静默 no-op（与现有 Ui API 风格一致）。

## 明确不做（YAGNI）

- `set_value` 不接 chart（多序列语义不明）
- 焦点/按键/游标交互
- push 不发事件（高频噪声）
- 网格、轴线、刻度标签
- remove_series、线宽配置、X 右对齐模式
- 增删点动画

## 测试（host 端，新文件 `qingui/tests/chart.rs`）

- push：未满追加、满挤最旧、值 clamp 到 range
- set_points：整体替换 + 超容量截尾（保留最新 capacity 个）
- 无效 ObjRef / 越界 series 索引 no-op 不 panic
- 渲染像素断言：push 后脏区非空；渲染后折线经过的像素为序列色（参考现有 `tests/render.rs` 的断言方式）
- builder 默认值（range (0,100)、0 条序列）

## 影响面

- `qingui/src/widgets/chart.rs`：新建（ChartState、绘制、ChartBuilder）
- `qingui/src/widgets/mod.rs`：`pub mod chart`、`WidgetKind::Chart` 变体、draw/tick 委托（tick 走默认 IDLE）
- `qingui/src/ui.rs`：chart_* 数据 API
- `qingui/tests/chart.rs`：新建
