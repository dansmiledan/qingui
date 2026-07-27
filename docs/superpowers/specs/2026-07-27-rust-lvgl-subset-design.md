# Rust LVGL 子集 设计文档

日期：2026-07-27
状态：已批准（待实现）

## 1. 目标

用 Rust 实现一个 LVGL（参考 `/Users/yintan/Documents/workspace/github/lvgl`，v9.x）的子集，第一阶段交付四个核心特性：

- **PFB（局部帧缓冲）渲染**：RAM 占用与屏幕分辨率解耦，分块渲染 + flush 推送。
- **脏矩形**：属性变化标脏，渲染只重画变化区域，可可视化验证。
- **动画**：时间线动画（值插值 + easing + 重复/往返），与脏矩形联动。
- **按键交互**：仿 LVGL 的 keypad indev + 焦点组（group），方向键导航、Enter 触发。

验收标准：`cargo run -p rust-lvgl-sim` 打开桌面模拟器窗口，运行综合 demo（List 菜单 + Slider/Switch 设置页），键盘可导航焦点，动画平滑，脏矩形可视化显示只重绘变化区域；`cargo test` 全绿；核心库以 `no_std` 配置编译通过。

## 2. 范围

### 2.1 包含

- 对象树（retained mode，仿 LVGL `lv_obj`）与事件系统。
- 精简样式系统（扁平结构 + 按状态覆盖）。
- 7 个控件：Obj（容器）、Label、Button、Slider、Switch、List、Bar。
- 布局：手动定位 + Flex + Grid（对齐 LVGL 语义）。
- 内置点阵字体：一款等宽 ASCII 位图字体（编译期生成），UTF-8 解码但只覆盖基本字符。
- 软件光栅化渲染：填充矩形、圆角矩形、边框、文字 blit，全部支持任意 clip rect。
- 桌面模拟器后端（minifb），键盘映射为按键码。

### 2.2 不包含（第一阶段）

- 触摸/指针输入。
- 图片解码、渐变、阴影、抗锯齿圆角之外的复杂 draw 单元。
- part/selector 样式链、样式继承。
- 多 display、多 indev、异步 DMA 双缓冲（接口预留扩展点）。
- TTF 字体、文本换行/省略号之外的排版（Label 支持换行，更复杂的排版不做）。

## 3. 总体架构

Cargo workspace，两个 crate：

- `rust-lvgl`（核心库，`no_std` + `alloc`）：对象树、样式、渲染、脏矩形、动画、输入、焦点组、布局、字体、控件。不依赖任何平台 API，通过 trait 对接。
- `rust-lvgl-sim`（桌面模拟器，`std`）：用 `minifb` 开窗口显示 flush 输出，键盘事件映射为按键码，内含综合 demo。

核心使用循环（与 LVGL 的 `lv_timer_handler` 用法一致）：

```
loop {
    ui.tick_inc(elapsed_ms);      // 平台喂时间
    ui.keypad_input(key);         // 有按键时
    let next = ui.timer_handler(); // 推进动画 → 重算布局 → 渲染脏矩形 → flush
    sleep(next);
}
```

## 4. 对象模型：Arena + 句柄

- `Ui` 是根上下文，持有 `Arena`（`Vec<Node>`）、脏矩形队列、动画列表、输入设备、焦点组。
- `ObjRef = (index: u32, generation: u32)`；删除对象后代际递增，悬垂句柄操作安全失效。
- `Node` 包含：几何（x/y/w/h）、样式（基础值 + 按状态覆盖）、父/子索引链、状态标志（hidden、pressed、focused、disabled 等）、`WidgetKind` 枚举（Obj/Label/Button/Slider/Switch/List/Bar）。
- 事件：`Ui::add_event_cb(obj, EventKind, callback)`，回调拿 `&mut Ui` + 目标句柄。第一阶段事件类型：Clicked、ValueChanged、Focused、Defocused、Key。

## 5. 渲染、PFB 与脏矩形

### 5.1 绘制原语

软件光栅化，核心内部用 RGB888 计算，flush 时按后端要求转换为 RGB888 或 RGB565。原语：清屏/填充矩形、圆角矩形、边框、文字位图 blit。所有原语接受任意 clip rect。

### 5.2 PFB（局部帧缓冲）

- 核心库不持有全屏缓冲。创建 `Display` 时由调用方传入像素缓冲（`&'static mut [Color]` 或编译期大小的数组），大小任意，典型 `320×40`（约 1/8 屏）。RAM 占用 = 该缓冲 + 对象树。
- 每帧渲染流程（仿 LVGL `lv_refr` 分块路径）：
  1. 收集并合并脏矩形；
  2. 每个脏矩形按缓冲容量沿行切分成若干 chunk（缓冲能装几行切几行，最后一块可不满）；
  3. 每个 chunk：清背景 → 遍历对象树，所有绘制以 chunk 矩形为 clip → `flush(chunk_rect, buf)`；
  4. 对象跨多个 chunk 时会被绘制多次，靠 clip 保证正确——这是 PFB 路径的主要复杂度所在。
- flush 第一阶段为同步阻塞式；双缓冲 + 异步 DMA 为扩展点（flush 返回前不复用缓冲）。
- 全屏缓冲是"缓冲行数 = 屏幕行数"的特例，同一套代码支持。
- 模拟器同样走 PFB 路径（约 1/10 屏缓冲），验证分块渲染正确性。

### 5.3 脏矩形

- 对象属性变化（位置、大小、样式、文本、值）时 `invalidate_area(rect)`；脏矩形入队后合并/裁剪：超出屏幕裁掉，重叠合并，队列设上限（超限时合并为整屏，防退化）。
- 调试开关：模拟器里把当帧脏矩形用半透明色框画出，作为验收的可视化证据。

## 6. 样式系统（精简版）

- `Style` 为扁平结构体，字段带 `Option`：`bg_color`、`bg_opa`、`border_color/width`、`radius`、`pad_*`、`text_color`、`text_font`，以及布局属性（见 §8）。
- 每个对象持有基础 `Style` + 按状态（Pressed/Focused/Disabled）的覆盖表；解析时逐字段回落：状态覆盖 → 基础值 → 主题默认。
- 无 part、无 selector 链、无继承（文本颜色等也不继承，全部显式设置或取主题默认）。
- 提供深色/浅色两套默认主题常量，创建控件时套用。

## 7. 控件（7 个）

| 控件 | 行为要点 |
|------|---------|
| Obj | 容器，可布局子对象 |
| Label | 文本渲染，支持 `\n` 换行 |
| Button | pressed/released/clicked，子对象可放 Label |
| Slider | 范围值，键盘 Left/Right 调值（编辑态），ValueChanged 事件 |
| Switch | 开/关，Enter 切换，切换动画 |
| List | 选项列表，Up/Down 项内导航，Enter 选中 |
| Bar | 进度条，用于展示动画 |

## 8. 布局（Flex + Grid，对齐 LVGL）

- 容器样式带 `layout: None | Flex { dir, wrap, main_align, cross_align, track_align } | Grid { col_dsc, row_dsc, align }`；子对象样式带 `grid_cell` 等定位属性。
- 轨道描述符支持 `px`、`fr`、`content` 三种（对齐 LVGL 的 `LV_GRID_FR`/`LV_GRID_CONTENT`）。
- 即时计算：样式或子树变化时标脏，下一帧渲染前重算；不做增量布局引擎。

## 9. 输入与焦点（仿 LVGL indev + group）

- 平台侧调用 `Ui::keypad_input(Key)`；`Key` 枚举：Prev/Next/Up/Down/Left/Right/Enter/Esc。模拟器映射 minifb 键盘，嵌入式接 GPIO/矩阵键盘。
- `Group`：对象可加入焦点组；Prev/Next 循环移动焦点，Up/Down/Left/Right 默认同 Prev/Next（List 内做上下项导航）。
- Enter 向焦点对象发 Clicked；Switch 收到 Clicked 直接切换开/关；Slider 上 Enter 进入"编辑态"，此后 Left/Right 调值，Enter/Esc 退出编辑态（对齐 LVGL editing 语义）。
- 焦点变化触发 Focused/Defocused 事件并刷新样式状态；焦点移动可选播放过渡动画。

## 10. 动画

- `Anim`：目标句柄 + 插值属性（x/y/w/h/opa/控件值等可枚举）+ 起止值 + 时长 + easing（linear、ease-in、ease-out、ease-in-out、bounce、overshoot）+ 可选延迟、重复次数、往返（playback）、完成回调。
- 时间由 `Ui::tick_inc(ms)` 驱动，核心不依赖系统时钟；`Ui::timer_handler()` 推进动画、处理布局标脏、执行渲染，返回下次需唤醒的毫秒数。
- 动画每帧改属性 → 标脏 → 走 PFB 分块渲染，与脏矩形天然联动。

## 11. 字体

- 内置一款等宽 ASCII 位图字体（编译期由构建脚本或宏生成字模表）。
- UTF-8 解码，基本字符之外的码点渲染为占位符（`?`）。
- Label 支持 `\n` 换行，不做复杂排版。

## 12. 测试策略

- 核心库：宿主端单元测试。无头渲染到内存缓冲，断言像素内容、脏矩形队列行为（入队/合并/裁剪/上限）、焦点转移、编辑态进出、动画数值曲线、Flex/Grid 布局结果。
- 模拟器：人工验收，不写自动化测试。
- CI 层面（本地执行）：`cargo test` 全绿；`cargo build -p rust-lvgl --target thumbv7em-none-eabihf`（或等价 no_std 检查）通过。

## 13. 验收 demo

`rust-lvgl-sim` 内置综合 demo：

- List 菜单 + Slider/Switch 设置页，方向键导航焦点，Enter 触发/进入编辑态。
- 页面切换与焦点移动带动画。
- 角落叠加 FPS 与当帧脏矩形可视化框。
