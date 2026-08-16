# Color 别名化为 Rgb888 设计

日期：2026-08-14
状态：已批准（待实现）
前置：`2026-08-14-generic-pixel-format-design.md`、`2026-08-14-eg-unified-rendering-design.md`（均已合并 main）

## 背景与动机

经过前两轮重构（帧缓冲像素格式泛型化、去透明度/AA + e-g 统一渲染），qingui 自有的 `Color` struct（RGB888 三通道）职责大幅缩水，剩余职责均可用 e-g 的 `Rgb888` 加少量辅助代码替代。用户已确认目标设备需要 Rgb565 帧缓冲，因此**泛型参数 `C` 保留不动**；本设计只处理 `Color` 的别名化。

收益：样式层直接就是 e-g 颜色类型，互操作零心智转换；少一层自有类型，API 面更小。行为零变化（同布局同值）。

## 设计

### 1. 核心变更（`qingui/src/geometry.rs`）

- 删除 `pub struct Color { r, g, b }` 及其 inherent impl，改为：

```rust
/// The working color type: embedded-graphics' RGB888.
pub use embedded_graphics::pixelcolor::Rgb888 as Color;
```

- 灰色常量保留自定义值（e-g `CSS_LIGHT_GRAY` 为 211，qingui 为 200，不等价），改为顶层常量：

```rust
/// Medium gray.
pub const GRAY: Color = Color::new(128, 128, 128);
/// Light gray.
pub const LIGHT_GRAY: Color = Color::new(200, 200, 200);
/// Dark gray.
pub const DARK_GRAY: Color = Color::new(40, 40, 40);
```

（`BLACK/WHITE/RED/GREEN/BLUE` 由 `Rgb888` 自带关联常量提供，`Color::WHITE` 等调用点经别名照常工作，无需修改。）

- `blend` 改为自由函数（doc 保持「LED 亮度用的不透明混色，非 alpha」的澄清）：

```rust
/// Mixes `fg` onto `bg` at ratio `t` (0..=255): plain color mixing (used for LED
/// brightness), not alpha compositing — qingui has no translucency.
pub fn blend(bg: Color, fg: Color, t: u8) -> Color { ... }
```

- 删除：`From<Color> for Rgb888` / `From<Rgb888> for Color`（同一类型，与核心自反 impl 冲突）、`impl PixelColor for Color`（`Rgb888` 自身已是 `PixelColor<Raw = RawU24>`）。

### 2. `to_rgb565`/`from_rgb565` 移入 `qingui/src/pixel.rs`

```rust
/// Color -> RGB565 (5-6-5).
pub(crate) fn color_to_rgb565(c: Color) -> u16 { ... }
/// RGB565 (5-6-5) -> Color (bit-copy expansion, lossless round-trip).
pub(crate) fn color_from_rgb565(v: u16) -> Color { ... }
```

位级逻辑原样搬运（与现有 `Color::to_rgb565`/`from_rgb565` 完全一致，blit565 无损往返不变）。`Rgb565` 的 `PixelFormat` impl 和 `canvas.rs` 的 `blit565` 改用这两个函数。

### 3. `pixel.rs` 宏体适配

- 删除 `impl PixelFormat for Color`（与宏生成的 `Rgb888` impl 重复——同一类型）。
- 宏体从字段访问改为访问器方法：`from_color(c: Color)` 内 `<$t>::new(c.r(), c.g(), c.b())`（`RgbColor` trait 提供）；`to_color` 内 `Color::new(self.r(), self.g(), self.b())`。

### 4. 调用点机械适配（编译器兜底清单）

- `Color::rgb(a, b, c)` → `Color::new(a, b, c)`（全库，含 const 上下文如 `EDIT_ACCENT`）。
- `Color::GRAY` / `Color::LIGHT_GRAY` / `Color::DARK_GRAY` → `GRAY` / `LIGHT_GRAY` / `DARK_GRAY`（加 `use crate::geometry::GRAY` 等导入）。
- 直接字段访问 `c.r`/`c.g`/`c.b` → `c.r()`/`c.g()`/`c.b()`。
- `led.rs`：`Color::BLACK.blend(color, bright)` → `blend(Color::BLACK, color, bright)`。
- 测试同步（如 `tests/geometry.rs` 的 `color_blend` 改为自由函数调用）。

### 5. 不受影响的部分

- 泛型体系：`Canvas<'a, C = Color>`、`Flush<C>`、`Ui<C>`、`Widget<C>` 等所有默认参数原样工作（`Color` 现在解析为 `Rgb888`，内存布局相同）。
- `PixelFormat` 的 8 种 e-g 类型 impl、Rgb565 位级一致性、`DrawTarget<Color = C>` 互操作。
- PFB/dirty-rect/Flush 渲染管线。

### 6. 文档

`qingui/README.md` 的 "Unreleased / 0.3 breaking changes" 追加一条：`Color` 现在是 `Rgb888` 的再导出；`Color::rgb(...)` → `Color::new(...)`；`Color::GRAY` 等灰色常量移到 `geometry` 模块顶层（`qingui::geometry::GRAY`）；`Color::blend` 变为自由函数 `geometry::blend`；`to_rgb565`/`from_rgb565` 变为 crate 内部实现细节。

## 测试

- 全套现有测试必须**零断言变化**通过（别名前后同类型同值；唯一允许的测试修改是调用语法适配，如 `color_blend` 测试改用自由函数）。
- `cargo check --workspace --examples --benches --tests` 零警告。
- `cargo build -p qingui --target thumbv7em-none-eabihf` 通过。
- time bench 与 main 对比无可感回归（别名化为零成本抽象，预期噪声内）。

## 涉及文件

- 修改：`qingui/src/geometry.rs`（核心变更）、`qingui/src/pixel.rs`（宏体 + 565 helper + 删重复 impl）、`qingui/src/canvas.rs`（blit565 调用点）
- 机械适配：全部引用 `Color::rgb` / `Color::GRAY` 系 / 颜色字段访问的文件（widgets、style、render、tests、examples、benches、tools）
- 文档：`qingui/README.md`
