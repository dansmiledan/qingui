# qingui

轻量嵌入式 GUI 库（Rust, `no_std` + `alloc`），灵感来自 LVGL 的子集：Arena 对象树、PFB（局部帧缓冲）渲染、脏矩形、动画、按键焦点组。

## 特性

- **PFB 渲染**：调用方提供任意大小的像素缓冲（如 1/10 屏），RAM 占用与屏幕分辨率解耦，分块渲染 + `Flush` trait 推送
- **脏矩形**：属性变化自动标脏，只重绘变化区域（合并/裁剪/上限坍缩）
- **动画**：`tick_inc` 驱动的时间线动画，6 种 easing，支持 delay/repeat/playback/完成回调
- **按键交互**：仿 LVGL 的 keypad + 焦点组（group），Slider 编辑态、Switch 切换、List 项导航与滚动
- **控件**：Obj、Label、Button、Slider、Switch、Bar、List
- **布局**：手动定位 + Flex（row/column、wrap、对齐）+ Grid（px/fr/content 轨道）
- **样式**：扁平样式结构 + 按状态（Pressed/Focused/Disabled）覆盖
- **字体**：多字体——embedded-graphics MonoFont，内置 FONT_6X10 默认，可插 eg 生态字体（ASCII，非 ASCII 回落 `?`）

## 快速开始

```rust
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::slider::SliderBuilder;
use qingui::{Color, Rect, Ui};

struct MyFlush;
impl Flush for MyFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        // 把 pixels 写入屏幕的 area 区域（RGB888）
    }
}

let mut ui = Ui::new(320, 240, 24); // 屏幕 320x240，PFB 缓冲 24 行
ui.set_flush(Box::new(MyFlush));

// Builder：默认尺寸/样式（通用 theme_base + 控件专属）可链式覆盖
let scr = ui.screen();
let slider = SliderBuilder::new(0, 100)
    .size(140, 14)
    .value(50)
    .style_with(|s| s.bg(Color::rgb(90, 90, 120)))
    .build(&mut ui, scr);
ui.set_pos(slider, 20, 20);

loop {
    ui.tick_inc(16);
    // ui.keypad_input(Key::Next); // 有按键时
    ui.timer_handler(); // 动画 → 布局 → 脏矩形渲染 → flush
}
```

## 示例（examples）

仓库内含 minifb 桌面模拟器（不发布到 crates.io）：

```
cargo run --example demo
cargo run --example gallery
```

- **demo**：控件总览——方向键/Tab 移动焦点，Enter 选择/进入编辑，Esc 退出编辑，Q 退出。绿色边框为脏矩形调试可视化。
- **gallery**：全部控件以 flex(wrap) 铺开，每 1s 末位前移（动画换位），交互控件自动演示（开关切换、进度随机、滚轮旋转、数值递增…）。

## 内存评估（memory benchmark）

零依赖 `cargo bench`，评估内存使用（静态类型尺寸 + 运行时峰值堆）：

```
cargo bench -p qingui --bench memory
```

报告内容：`size_of` 表（`Node`/`WidgetKind`/各控件状态/`Style`/`Ui`，含"最大变体税"——每个节点都按最大控件状态背负 `WidgetKind` 的大小）+ 三档场景（small/medium/large）的峰值/常驻堆 + 阈值断言防回归。

注意：bench 在 host（64 位）上运行，数值与 32 位 MCU 目标不同，但相对成本形状一致；嵌入式固件实际大小用 `cargo size --target thumbv7em-none-eabihf` 测量。

## License

MIT OR Apache-2.0
