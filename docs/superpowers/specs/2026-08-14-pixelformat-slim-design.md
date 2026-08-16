# PixelFormat 瘦身设计（默认方法 + e-g 转换）

日期：2026-08-14
状态：已批准（待实现）
前置：`2026-08-14-color-alias-rgb888-design.md`（Color 已是 `Rgb888` 别名，已合并 main）

## 背景与动机

`PixelFormat` 作为命名 bound 必须保留（Rust 无 trait alias，30+ 文件的 `C: PixelFormat` 无法展开书写）。但其手写转换实现（宏体、`Rgb565` 特判、`color_to_rgb565`/`color_from_rgb565` helper）是冗余的：e-g-core 0.4.1（`conversion.rs`）已为全部 8 种 RGB/BGR 类型实现相互 `From` 转换，且另提供 Rgb↔Gray、Gray↔Binary 转换。

目标：删掉全部手写转换代码，trait 改为默认方法 + 空 impl。调用点签名不变。

## 设计

### 1. `qingui/src/pixel.rs` 重写

```rust
pub trait PixelFormat: PixelColor + Copy + PartialEq + Default + Into<Color> + From<Color> {
    /// Converts a framebuffer pixel to the internal RGB888 `Color`.
    fn to_color(self) -> Color { self.into() }
    /// Converts an internal RGB888 `Color` to a framebuffer pixel (quantizes).
    fn from_color(c: Color) -> Self { c.into() }
}
```

- 8 个 e-g 类型（`Rgb888/Rgb565/Rgb555/Rgb666/Bgr888/Bgr565/Bgr555/Bgr666`）由宏生成空 impl。
- 删除：`color_to_rgb565`/`color_from_rgb565`、`Rgb565` 的手写 `PixelFormat` impl、宏体里的自定义转换。
- `PixelFormat for Color`（= `Rgb888`）由宏的 `Rgb888` 空 impl 覆盖。

### 2. `canvas.rs` 的 `blit565`

u16 → Color 解码从 `color_from_rgb565(v)` 改为 `Color::from(Rgb565::from(RawU16::new(v)))`。e-g 的 565→888 展开（`convert_channel` rounding）与旧位复制展开数学等价（位复制即 5→8/6→8 位的精确 rounding），解码值不变。

### 3. 行为变化（已批准）

- 888→565 量化：截断（掩码）→ 四舍五入（e-g `convert_channel`），极端值差 1 LSB（如 r=250：31 → 30）。
- 565→888：与旧数学等价，无变化。
- `blit565` 565→888→565 往返无损性不变（位复制展开是 rounding 量化的不动点）。
- qingui-codegen 的内联 565 公式（编码端、截断）不动：编码端量化规则任意，blit565 解码端展开即还原。

### 4. 测试

- `pixel.rs` 测试模块重写：删除对照旧 helper 的 `rgb565_matches_color_helpers`；新增对照 e-g `From` 实际输出的用例，含一个固化 rounding 新语义的用例（如 `Color::new(250, 0, 0)` → `Rgb565` 的 r 通道为 30，旧截断为 31）。
- `canvas.rs` 用到旧 helper 的断言（如 `rgb565_put_quantizes`）适配为新转换的实际值。
- `tests/rgb565.rs` 端到端用例自洽（`Rgb565::from_color` 双向同一转换），预期零改动。
- 其余全套测试必须零断言变化通过。

### 5. 文档

`qingui/README.md` 破坏性清单追加：`PixelFormat` 新增 `Into<Color> + From<Color>` supertrait（下游手动实现过 `PixelFormat` 的自定义类型需改为满足 e-g 转换约束；对仅使用内置格式的用户无影响）；888→565 量化规则由截断变为四舍五入（与 e-g 一致）。

### 6. 验证

全套测试绿、`cargo check --workspace --examples --benches --tests` 零警告、`thumbv7em-none-eabihf` 构建通过、time bench 与 main 对比无回归（转换收敛在写入点，预期噪声内）。

## 涉及文件

- 重写：`qingui/src/pixel.rs`
- 修改：`qingui/src/canvas.rs`（blit565 一行 + 测试断言适配）
- 文档：`qingui/README.md`
