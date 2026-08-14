# Canvas 像素格式泛型化设计（e-g 互操作）

日期：2026-08-14
状态：已批准（待实现）

## 背景与动机

qingui 目前内部全部使用自定义的 `geometry::Color`（RGB888 三通道）作为帧缓冲像素格式：

- `Canvas` 的帧缓冲是 `&mut [Color]`（`qingui/src/canvas.rs:13`）。
- 与设备的边界是 `Flush::flush(area, pixels: &[Color])`（`qingui/src/display.rs:7`），驱动拿到 RGB888 后自行转换成设备格式（如 Rgb565）。
- `Canvas` 已实现 e-g 的 `DrawTarget<Color = Rgb888>`（`qingui/src/canvas.rs:558`），但颜色类型固定为 Rgb888，无法与使用 Rgb565 等设备原生颜色类型的 e-g 生态代码直接组合。

目标：让 qingui 的帧缓冲像素格式可按设备选择（`Rgb565`、`Rgb888` 等 e-g `PixelColor` 类型），从而：

1. e-g 生态代码可以用设备原生颜色类型直接画进 qingui 画布（`DrawTarget<Color = C>`）。
2. flush 输出即为设备格式，对 Rgb565 屏实现零转换推送。
3. 附带收益：Rgb565 帧缓冲内存减半（如 320x240 从 225KB 降到 150KB）。

明确**不**做的事：

- 不把 qingui 内部渲染（样式、主题、widget 绘制、AA/混合）改成低色深空间——混合需要逐通道运算，RGB888 内部表示保持不变。
- 本期不支持 `Gray2/Gray4/Gray8/BinaryColor`（单色/灰阶屏上 AA 混合无意义；后续可用 luma 映射另立项目支持）。
- 不让 qingui 渲染到任意外部 `DrawTarget`（e-g `DrawTarget` 是只写接口，无法实现需要读-改-写的 alpha 混合）。

## 设计

核心思路：**像素格式泛型只渗透到存储层，绘制 API 保持 RGB888 不变。**

### 1. 新模块 `qingui/src/pixel.rs`

```rust
/// A framebuffer pixel format: convertible to/from the internal RGB888 `Color`.
pub trait PixelFormat: PixelColor + Copy + PartialEq + Default {
    fn to_color(self) -> Color;      // device pixel → RGB888
    fn from_color(c: Color) -> Self; // RGB888 → device pixel (quantizes)
}
```

实现对象：e-g 的 8 种 RGB 格式（`Rgb888`、`Rgb565`、`Rgb555`、`Rgb666`、`Bgr888`、`Bgr565`、`Bgr555`、`Bgr666`），以及 qingui 自己的 `Color`（恒等转换，作为默认格式保证现有行为完全不变）。

同时给 `geometry::Color` 实现 e-g 的 `PixelColor`（`type Raw = RawU24`），使默认格式满足 `PixelFormat: PixelColor` 约束。

### 2. `Canvas<'a, C: PixelFormat = Color>`

- `pixels: &'a mut [C]`。
- **所有绘制方法签名不变**（仍收 `Color` + opa），转换收敛在 `put`/`put_fast`/`fill_rect` 内部：
  - opaque 路径：`pixels[idx] = C::from_color(c)`（`fill_rect` 的行填充只转换一次再批量填充）；
  - 混合路径：`C::from_color(pixels[idx].to_color().blend(c, opa))`。
- 因此 800+ 处 widget/样式调用点零改动。

### 3. e-g `DrawTarget` 泛型实现

替换现有 `DrawTarget<Color = Rgb888>` 实现为单一泛型实现：

```rust
impl<C: PixelFormat> DrawTarget for Canvas<'_, C> {
    type Color = C;
    // draw_iter / fill_solid / clear 走 put / fill_rect 的快路径，与现状一致
}
```

`draw_text_opa` 内部的 `EgTarget`（`BinaryColor`）相应泛型化，逻辑不变。

### 4. `Flush<C = Color>` 与 `Ui<C = Color>`

- `Flush`：`fn flush(&mut self, area: Rect, pixels: &[C]);`，文档中的 "RGB888" 描述改为 "pixels in the canvas pixel format `C`"。
- `Ui`：`buf: Vec<C>`，`flush: Option<Box<dyn Flush<C>>>`；`Ui::<Rgb565>::new(...)` 即选定设备格式。
- `render::render` / `render_area` / `render_chunk` / `draw_node` 内部泛型化（crate 私有，不影响 API）。
- 缓冲初始化 `vec![Color::BLACK; ...]` 改为 `vec![C::default(); ...]`（各格式 default 均为黑）。

### 5. 兼容性

三处泛型参数默认值均为 `Color`：

- `impl Flush for X`（不带参数即 `Flush<Color>`）——现有实现无需修改。
- `Canvas { pixels, area, stride }` 字面量——`pixels: &mut [Color]` 时推断为默认。
- `Ui::new(...)`——返回 `Ui<Color>`。

现有用户代码零修改、行为零变化。

### 6. 精度与性能权衡

- 混合在低色深格式下为「读出 → RGB888 混合 → 量化写回」，有轻微精度损失（LVGL 的 16bit 软件渲染采用同样做法）。
- opaque 填充每像素一次位运算转换（几个 shift/mask），可忽略；`fill_rect` 批量路径不受影响。

## 错误处理

无新增运行时错误路径。`PixelFormat` 转换均为全函数；`blit565` 等现有接口行为不变。

## 测试

- `pixel.rs`：各格式的 `to_color`/`from_color` 往返测试；`Rgb565` 量化与 `Color::to_rgb565`/`from_rgb565` 已有逻辑的一致性校验（可复用其实现）。
- `canvas.rs`：新增 `Canvas<Rgb565>` 的绘制/混合/`DrawTarget` 冒烟测试。
- 现有 `render.rs` 测试走默认 `Color` 路径，保持不变且必须全绿。

## 涉及文件

- 新增：`qingui/src/pixel.rs`
- 修改：`qingui/src/canvas.rs`、`qingui/src/display.rs`、`qingui/src/render.rs`、`qingui/src/ui.rs`、`qingui/src/geometry.rs`（`PixelColor` 实现）、`qingui/src/lib.rs`（导出新模块）
- 不动：全部 widgets、`style.rs`、主题
