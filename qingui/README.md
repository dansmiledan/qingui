# qingui

轻量嵌入式 GUI 库（Rust, `no_std` + `alloc`），灵感来自 LVGL 的子集：Arena 对象树、PFB（局部帧缓冲）渲染、脏矩形、动画、按键焦点组。

![gallery 动画展示](assets/qingui_gallery.gif)

## 特性

- **PFB 渲染**：调用方提供任意大小的像素缓冲（如 1/10 屏），RAM 占用与屏幕分辨率解耦，分块渲染 + `Flush` trait 推送
- **脏矩形**：属性变化自动标脏，只重绘变化区域（合并/裁剪/上限坍缩）
- **动画**：`tick_inc` 驱动的时间线动画，6 种 easing，支持 delay/repeat/playback/完成回调
- **按键交互**：仿 LVGL 的 keypad + 焦点组（group），Slider 编辑态、Switch 切换、List 项导航与滚动
- **控件**：Obj、Label、Button、Slider、Switch、Bar、List、Arc、Checkbox、Chart、Dropdown、Image、ItemList、Led、Msgbox、Roller、ScrollView、Spinbox、Spinner、Table
- **用户控件**：实现 `widgets::Widget` trait 后经 `ui.create_widget(parent, w, h, Box::new(w))` 挂载，与内置控件完全同权；查询/更新状态用 `ui.widget::<T>(obj)` / `ui.update::<T, _>(obj, |w| ...)`（示例见 `tests/custom_widget.rs`）
- **布局**：手动定位 + Flex（row/column、wrap、对齐）+ Grid（px/fr/content 轨道）
- **样式**：扁平样式结构 + 按状态（Focused/Edited/Selected/Disabled）覆盖
- **字体**：多字体——embedded-graphics MonoFont，内置 FONT_6X10 默认，可插 eg 生态字体（ASCII，非 ASCII 回落 `?`）

## 快速开始

```rust
use qingui::display::Flush;
use qingui::input::Key;
use qingui::widgets::slider::SliderCfg;
use qingui::{Color, Rect, Ui};

struct MyFlush;
impl Flush for MyFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        // 把 pixels 写入屏幕的 area 区域（RGB888）
    }
}

let mut ui = Ui::new(320, 240, 24); // 屏幕 320x240，PFB 缓冲 24 行
ui.set_flush(Box::new(MyFlush));

// Builder：默认尺寸/样式（通用 theme_base + 控件专属）可链式覆盖；
// 控件专属几何/时序参数（行高 row_h、动画时长 roll_dur/fx_dur、线宽 line_width、
// 旋钮宽 knob_w、弹层 popup_* 等）也各有链式 setter，默认值见各控件源码常量
let scr = ui.screen();
let slider = SliderCfg::new(0, 100)
    .size(140, 14)
    .value(50)
    .style_with(|s| s.bg(Color::new(90, 90, 120)))
    .build(&mut ui, scr);
ui.set_pos(slider, 20, 20);

loop {
    ui.tick_inc(16);
    // ui.keypad_input(Key::Next); // 有按键时
    ui.timer_handler(); // 动画 → 布局 → 脏矩形渲染 → flush
}
```

## Pixel formats

`Ui` is generic over the framebuffer pixel format `C` (default: qingui's RGB888 `Color`). To render directly in the display's native format, pick it at construction and implement `Flush` for it:

```rust
use embedded_graphics::pixelcolor::Rgb565;
use qingui::display::Flush;
use qingui::{Rect, Ui};

struct MyFlush;
impl Flush<Rgb565> for MyFlush {
    fn flush(&mut self, area: Rect, pixels: &[Rgb565]) {
        // Write `pixels` to the screen's `area` (RGB565).
    }
}

let mut ui = Ui::<Rgb565>::new(320, 240, 24);
ui.set_flush(Box::new(MyFlush));
```

Supported formats: the eight embedded-graphics RGB/BGR color types (`Rgb888`/`Rgb666`/`Rgb565`/`Rgb555`, `Bgr888`/`Bgr666`/`Bgr565`/`Bgr555`) plus qingui's `Color` (default, RGB888). Embedded-graphics code draws into a qingui canvas of the same format via `DrawTarget<Color = Rgb565>` on `Canvas<'_, Rgb565>`.

**Caveat — `Ui::new` type inference:** the default type parameter `C = Color` does not participate in expression-level inference, so `let mut ui = Ui::new(...)` followed only by generic builder calls fails with E0283 (nothing pins `C`). Annotate the binding (`let mut ui: Ui = Ui::new(...)`) or use `Ui::<Color>::new(...)`.

**Migration — `DrawTarget` color type:** `Canvas`'s `DrawTarget` implementation now has `type Color = C` (was `Rgb888` before). Downstream embedded-graphics code that drew `Pixel<Rgb888>` into a default canvas must switch to `Canvas<'_, Rgb888>` (or qingui's `Color`).

## Unreleased / 0.3 breaking changes

Rendering was reworked to delegate all drawing to embedded-graphics primitives; qingui's custom rasterizer (`draw.rs`) and the whole alpha/opacity system are gone.

- `Canvas` drawing methods lost the `opa` parameter; `draw_text_opa` removed (use `draw_text`). There is no alpha blending anywhere.
- `Style.bg_opa`/`Style.opa` removed; the background now paints iff `bg_color` is `Some` (`ResolvedStyle.bg_color: Option<Color>`).
- `Ui::set_opa`, `AnimProp::Opa`, and the list delete-ghost effect removed.
- `WidgetCtx::ap` removed (was `pub`): opacity no longer exists — custom widgets calling `ctx.ap(...)` should simply drop the wrapper and pass colors to `Canvas` methods directly.
- The list widget's `Ghost` struct and `ListFx::ghost` field (both `pub`) removed along with the delete-ghost effect.
- Nodes with no explicit style previously defaulted to an opaque black background; they now default to no background (paint only when `bg_color` is `Some`).
- `Canvas::draw_arc` no longer wraps `end <= start` (previously `(270°, 90°)` drew a 180° arc by adding 360°); now `end <= start` draws nothing — express wrap as `end > 360`.
- `qingui::Point` is now embedded-graphics' `Point`; `Rect` ↔ `Rectangle` `From` conversions added.
- Visual change: no anti-aliasing (aliased corners/arcs/lines), no translucency.
- Rendering now delegates to embedded-graphics primitives (`Rectangle`/`RoundedRectangle`/`Circle`/`Arc`/`Line`/…); the framebuffer, dirty-rect, and `Flush` pipeline are unchanged.
- `Color` is now a re-export of e-g's `Rgb888` (`pub use Rgb888 as Color`). `Color::rgb(r, g, b)` → `Color::new(r, g, b)`; `Color::GRAY`/`LIGHT_GRAY`/`DARK_GRAY` moved to `qingui::geometry::{GRAY, LIGHT_GRAY, DARK_GRAY}`; `Color::blend(a, b, t)` → free function `qingui::geometry::blend(a, b, t)`; `to_rgb565`/`from_rgb565` are now crate-internal. `Color::WHITE` etc. keep working via `Rgb888`'s constants. Note: `Color::WHITE` etc. are `RgbColor` trait consts — bring `embedded_graphics::pixelcolor::RgbColor` into scope where you use them. Direct field access `c.r`/`c.g`/`c.b` (and `Color { r, g, b }` literals/patterns) becomes the `RgbColor` accessor methods `c.r()`/`c.g()`/`c.b()` — same trait import as the constants.

## 示例（examples）

仓库内含 minifb 桌面模拟器（不发布到 crates.io）：

```
cargo run --example demo
cargo run --example gallery
```

- **demo**：控件总览——方向键/Tab 移动焦点，Enter 选择/进入编辑，Esc 退出编辑，Q 退出。绿色边框为脏矩形调试可视化。
- **gallery**：全部控件以 flex(wrap) 铺开，每 1s 末位前移（动画换位），交互控件自动演示（开关切换、进度随机、滚轮旋转、数值递增…）。

## 内存评估（memory benchmark）

零依赖内存评估，覆盖**静态类型尺寸**（`size_of` 表——每节点持有定长 `Box<dyn Widget>`（2×usize），控件状态按实际大小单独堆分配，不再按最大控件状态均摊）+ **运行时峰值堆**（三档场景的 peak/live + 阈值断言防回归）。

两种运行方式，同一套场景代码：

| 运行环境 | 命令 | ABI |
|---|---|---|
| host（64 位，快速回归） | `cargo bench -p qingui --bench memory` | `usize = 8B` |
| QEMU 裸机（真实 32 位） | `cd tools/qemu-mem && cargo run --release --target thumbv7em-none-eabihf` | `usize = 4B`（`-machine mps2-an386`，Cortex-M4F，semihosting 输出，退出码=断言结果） |

### 静态尺寸对比（host 64-bit vs QEMU thumbv7em 32-bit）

| 类型 | host (B) | QEMU 32 位 (B) |
|---|---|---|
| Rect | 16 | 16 |
| Point | 8 | 8 |
| Color | 3 | 3 |
| Style | 40 | 140 \* |
| ResolvedStyle | 32 | 112 \* |
| 4 × Style（旧内联成本） | 160 | 560 \* |
| **Node** | 272 | 280 \* |
| **kind：`Box<dyn Widget>`** | 16 | 8 \* |
| largest widget state | 152 | 112 |
| Ui | 248 | 152 \* |

\* QEMU 32 位数字为迁移前基线，trait-object 迁移（`WidgetKind` enum → `Box<dyn Widget>`）后未复测（largest widget state 一行除外：该行 QEMU 值为迁移后按 thumbv7em 目标编译期实测）；host 为迁移后实测。`Box<dyn Widget>` 定长 2×usize，各控件状态按实际大小单独堆分配（largest = List 状态）。

各控件状态（每节点一次 Box 堆分配；用户控件状态即其自身结构体大小，原 `Custom` 通道已删除）：

| 状态 | host (B) | QEMU 32 位 (B) \* | | 状态 | host (B) | QEMU 32 位 (B) \* |
|---|---|---|---|---|---|---|
| Obj | 0 | 0 | | Spinner | 0 | 0 |
| Label | 24 | 12 | | Msgbox | 4 | 4 |
| Button | 24 | 12 | | Led | 4 | 4 |
| Slider | 12 | 12 | | Table | 32 | 16 |
| Switch | 1 | 1 | | Spinbox | 16 | 16 |
| Bar | 12 | 12 | | Roller | 56 | 40 |
| List | 152 | 112 | | ScrollView | 12 | 12 |
| Arc | 12 | 12 | | Dropdown | 32 | 16 |
| Checkbox | 32 | 16 | | Image | 24 | 16 |
| Chart | 32 | 20 | | ItemList | 56 | 152 \* |

### 峰值堆对比（场景表，peak=构造峰值 / live=树常驻）

| 档位 | 节点数 | host peak (B) | host live (B) | QEMU peak (B) \* | QEMU live (B) \* |
|---|---|---|---|---|---|
| minimal | 3 | 5,247 | 5,127 | 5,431 | 5,311 |
| small | 16 | 31,201 | 29,125 | 32,205 | 30,849 |
| medium | 50 | 55,000 | 45,960 | 59,476 | 52,380 |
| large | 140 | 147,496 | 109,192 | 165,516 | 128,588 |

\* QEMU 列为迁移前基线（trait-object 迁移后未复测）；host 为迁移后实测，对比详情见 `docs/BENCHMARK.md`。

32 位目标比 64 位 host 约低 20–25%：指针/引用（`Vec`/`String`/`Box`/`Option<usize>`）减半，但 `i32`/`u32` 固定字段不缩水，故非整体减半。QEMU 侧用自研静态 arena + 计数分配器（`tools/qemu-mem/src/allocator.rs`），`dealloc` 真实复用，peak/live 才有意义；回归测试见 `cargo test -p qemu-mem`（host 端跑同一套场景逻辑 + 分配器完整性校验）。

## License

MIT OR Apache-2.0
