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
- **字体**：内置 8x8 位图字体（ASCII，非 ASCII 回落 `?`）

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
slider.set_pos(&mut ui, 20, 20);

loop {
    ui.tick_inc(16);
    // ui.keypad_input(Key::Next); // 有按键时
    ui.timer_handler(); // 动画 → 布局 → 脏矩形渲染 → flush
}
```

## 桌面模拟器 demo

仓库内含 minifb 模拟器（不发布到 crates.io）：

```
cargo run --example demo
```

方向键/Tab 移动焦点，Enter 选择/进入编辑，Esc 退出编辑，Q 退出。绿色边框为脏矩形调试可视化。

## License

MIT OR Apache-2.0
