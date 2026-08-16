# 去透明度/抗锯齿 + e-g 统一渲染设计

日期：2026-08-14
状态：已批准（待实现）
前置：`2026-08-14-generic-pixel-format-design.md`（已合并 main，`a48397a`）

## 背景与动机

qingui 目前自带一套软件光栅化器：`draw.rs` 的覆盖率/AA 数学（265 行）、`canvas.rs` 里的超采样圆角与 alpha 混合路径、贯穿样式/渲染/动画的 `opa` 链路。e-g 0.8 的绘制模型恰好是不透明、无 AA（`DrawTarget` 只写不读），并提供了完整的图元库（`Rectangle`、`RoundedRectangle`、`Circle`、`Ellipse`、`Arc`、`Sector`、`Line`、`Polyline`、`Triangle` + `PrimitiveStyle`）。

目标：删除自研光栅化与透明度体系，`Canvas` 绘制方法全部委托 e-g primitives，数据结构尽可能对齐 e-g。收益：

1. 删掉 `draw.rs` 全部和 `canvas.rs` 大部分渲染复杂度，渲染正确性外包给 e-g。
2. 与 e-g 生态深度对齐（配合已有的 `PixelFormat` 泛型化，`Canvas<C>` 本身就是 `DrawTarget<Color = C>`）。
3. 二进制体积与维护面缩小。

明确**不**做的事：

- 不做「直接渲染到任意 e-g DrawTarget（显示驱动即后端）」——PFB 帧缓冲 + dirty rect + `Flush` 架构保留。（未来如需零帧缓冲，再单独立项。）
- 不保留任何形式的透明度（包括预混色阶、点阵仿透明）。
- 不做兼容层——0.2.0 直接破坏性变更。

## 设计

### 1. 删除清单

**Style 层**（`qingui/src/style.rs`）：
- `Style.bg_opa`、`Style.opa` 字段及对应 builder 方法删除。
- `ResolvedStyle.bg_color` 改为 `Option<Color>`：`None` = 不画背景，吸收现有 `bg_opa(0)` 的布尔语义；`ResolvedStyle.opa` 字段删除。
- 各 widget 默认样式中的 `bg_opa: Some(0)` 改为 `bg_color = None`（语义等价迁移）；`bg_opa: Some(255)` 直接删除（有 bg_color 即画）。
- 主题（`theme_base` 等）同步迁移。

**Ui/动画层**：
- `Ui::set_opa` 删除；`AnimProp::Opa` 删除；render.rs 的节点 opa 乘数（`ap()` 闭包）删除。
- roller 的选中行 ghost 残影、layout transition 的 ghost 效果删除（对应测试 `roller_ghost.rs`/`transition_ghost.rs` 随功能删除或改写）。

**Canvas 层**（`qingui/src/canvas.rs`）：
- 所有绘制方法的 `opa: u8` 参数删除；`draw_text_opa` 合并回 `draw_text`（删旧 `draw_text` 薄封装）。
- `Color::blend` 删除（`geometry.rs`）；`put`/`put_fast` 只剩 opaque 写入；`fill_rect` 的半透明分支删除。

**`qingui/src/draw.rs` 整个删除**（覆盖率/AA 数学：`circle_cov16`、`ThickLine`、`ArcGeom`、`cov16` 等）。

### 2. Canvas 薄壳 → e-g 委托映射

`Canvas<'a, C = Color>` 保留：帧缓冲、`area`/`stride` 坐标偏移、`clip` 语义、`DrawTarget<Color = C>` 实现（像素最终都落到唯一的 opaque `put`）。每个方法变为薄壳——先按现有 clip/area 语义计算目标区域，再经 `DrawTargetExt::clipped` 把 e-g primitive 画进 `self`：

| Canvas 方法 | 委托 |
|---|---|
| `fill_rect(r, c, clip)` | `Rectangle` + fill |
| `fill_rounded(r, radius, c, clip)` | `RoundedRectangle` + fill |
| `draw_border(r, width, radius, c, clip)` | radius > 0 时 `RoundedRectangle` + stroke，否则 `Rectangle` + stroke |
| `fill_circle(center, radius, c, clip)` | `Circle` + fill |
| `draw_circle(center, radius, width, c, clip)` | `Circle` + stroke |
| `draw_arc(c, r, width, start, sweep, color, clip)` | `Arc` + stroke；现有圆头端点语义改为方头（e-g Arc 无圆头），如个别 widget 视觉依赖圆头，在端点补 `Circle` |
| `draw_line(p1, p2, width, c, clip)` | `Line` + stroke_width；width ≥ 2 的圆头改为两端补 `Circle`（e-g 粗线为方头） |
| `draw_text(...)` | 现状已是 e-g MonoText，仅去掉 opa |
| `blit565(...)` | 保留自写循环（opaque put 逐像素转换），不套 e-g `Image`（`ImageRawLE<Rgb565>` 与泛型 `C` 颜色类型不匹配，强行套用得不偿失） |
| `clear` | 保留现状（`pixels.fill`） |

实现要点：
- `fill_solid`/`fill_contiguous` 快路径保留（占比最大的纯色填充不走逐像素 `draw_iter`）。
- 委托是在 `self` 上 draw：e-g 逐像素回调进 `Canvas::draw_iter` → `put`（带 bounds check）。裁剪由 `clipped()` 与现有 area 交集双重保证。

### 3. 数据结构统一

- **`Point`**：删除自有 struct，改为 `pub use embedded_graphics::geometry::Point;`（字段/语义完全一致：i32 x/y）。所有 `crate::geometry::Point` 引用改为 re-export 后的同一类型；`Point::new(x, y)` 构造器调用点适配（e-g `Point::new` 同为 const fn）。
- **`Rect` 保留自有类型**：i32 宽高、空矩形语义、`intersect`/`union`/`translate`/`contains` 被裁剪与布局深度依赖，e-g `Rectangle`（`Point + Size(u32)`、`intersection` 语义不同）对不齐。把 canvas.rs/draw.rs 现有的 `eg_rect`/`from_eg_rect` 内部 helper 提升为 `geometry.rs` 里的公共 `From<Rect> for Rectangle` / `From<Rectangle> for Rect`。
- **`Color` 保留**：样式层 RGB888 工作色 + 已是 e-g `PixelColor`（`Raw = RawU24`）；默认帧缓冲格式角色不变。
- **字体**：已是 e-g `MonoFont`，不变。

### 4. 渲染管线调整（`qingui/src/render.rs`）

- 背景绘制条件从 `resolved.bg_opa > 0` 改为 `if let Some(bg) = resolved.bg_color`。
- 删除 `ap()` 乘数闭包，所有 widget 绘制调用直接传 `clip`。
- PFB 分块、dirty rect、`Flush<C>` 不变。

### 5. 破坏性变更清单（写入 README 迁移说明）

- `Canvas` 全部绘制方法去掉 `opa` 参数；`draw_text_opa` 删除（用 `draw_text`）。
- `Style`/`ResolvedStyle` 删除 `bg_opa`/`opa` 字段；背景语义改为 `bg_color: Option<Color>`。
- `Ui::set_opa`、`AnimProp::Opa`、`Color::blend` 删除。
- `geometry::Point` 变为 e-g 类型的 re-export（字段相同，行为等价，但类型同一性变化——`Point {}` 字面量构造不受影响）。
- 视觉效果变化：无 AA（圆角/弧线/斜线边缘锯齿化）、无半透明。

### 6. 测试策略

- Canvas 每个委托方法配像素级单测（含 clip 行为），断言 e-g 光栅化的实际输出。
- 现有视觉测试（`qingui/tests/*.rs` 中大量像素断言）逐个重校：AA 边缘消失 + e-g 光栅化差异导致断言值变化，属机械性更新；ghost 相关测试随功能删除。
- `qingui/tests/rgb565.rs` 端到端测试保持绿（其中混合用例 `rgb565_blend_roundtrips_through_rgb888` 随 blend 删除而移除，量化/托管用例保留）。
- demo/gallery 模拟器（minifb）编译通过并人工跑一遍确认视觉可接受。

### 7. 风险与回退

- **e-g stroke 语义差异**：`Arc`/`RoundedRectangle` 的 stroke 绘制方向（向内/居中）与现有视觉可能不同——实现时逐方法用像素测试校准。
- **性能**：e-g 逐像素 `draw_iter` 慢于现有批量行填充。缓解：`fill_solid`/`fill_contiguous` 快路径保留；完成后跑 `cargo bench -p qingui --bench time` 对比 main 基线，回归明显时对热点方法保留自写快路径（在 spec 允许范围内作为实现细节）。
- **回退**：整条改动在独立分支进行，main 可随时回退。

## 涉及文件

- 删除：`qingui/src/draw.rs`
- 重写/大改：`qingui/src/canvas.rs`（委托化 + 去 opa）、`qingui/src/style.rs`（字段删除 + Option 化）、`qingui/src/render.rs`（去 opa 乘数）
- 修改：`qingui/src/geometry.rs`（Point 替换、Rect 转换、删 blend）、`qingui/src/anim.rs`（删 Opa）、`qingui/src/ui.rs`（删 set_opa）、全部 widgets（调用点去 opa 参数 + 默认样式迁移）、`qingui/src/lib.rs`（删 draw 模块）
- 测试：`qingui/tests/*.rs` 像素断言重校；`rgb565.rs` 删混合用例
- 文档：`qingui/README.md` 迁移说明
