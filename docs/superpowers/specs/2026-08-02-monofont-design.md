# 多位图字体支持(MonoFont)设计

日期:2026-08-02
状态:已与用户确认

## 目标

字体系统从单一 font8x8 升级为 embedded-graphics 的 `MonoFont` 抽象:内置多字号可选,用户可插任何 eg 生态字体 crate 或自己 const 定义字体。

## 已确认的决策

| 决策点 | 结论 |
|---|---|
| 需求方向 | 多位图字体(字型+字号可选) |
| 字体定义 | 直接依赖 embedded-graphics 0.8,用 `MonoFont` 类型 |
| 默认字体 | 换成内置 `FONT_6X10`(最接近 8x8 紧凑度),删除 font8x8 依赖,接受外观/测试变化 |
| 比例字体 | 不做(MonoFont 只支持等宽;profont 同为等宽,仍可用) |

## 设计

### 依赖

- `qingui/Cargo.toml`:删 `font8x8`,加 `embedded-graphics = { version = "0.8", default-features = false }`(no_std;固件侧死代码由链接器消除)。

### 渲染(font.rs 重写 + draw.rs)

- 自写 MonoFont 文本绘制,不引入 e-g 的 Text renderer:
  - `font::glyph_index(font, ch) -> usize`:经 `font.glyph_mapping` 查字模索引,未收录字符回落到映射表第一个索引(e-g 替换字符语义)。
  - atlas 网格:`glyphs_per_row = font.image.width() / font.character_size.x`,按索引定位字模左上角。
  - 1bpp 按位绘制:set bit → `put`(fg);bg 不写(保持现状:文本背景透明)。沿用现有 put/clip/opa 路径,无分配。
- `font::text_size(font, s) -> (w, h)`:逐字 advance = `character_size.x + character_spacing`;行宽 = n × (cw + sp) − sp(n>0,末字不计 spacing,与 e-g 渲染宽度一致);高 = 行数 × `character_size.y`;支持 '\n',空串 (0, character_size.y)。

### 字体选择

- `Style.font: Option<&'static MonoFont>`(None → 默认);resolved 链原样叠加。
- `Ui::set_default_font(&'static MonoFont)`;`Ui` 存 `default_font: &'static MonoFont`,初始为 `&FONT_6X10`(embedded-graphics `mono_font::ascii::FONT_6X10`)。
- resolved_style 时若 style.font 为 None 用 default_font。
- builder 不加新方法(经 `.style()` 设置)。

### 适配

- `draw.rs`:`draw_text`/`draw_text_opa` 签名加 `font: &'static MonoFont` 参数,调用点一律显式传 `ctx.resolved.font`。
- 8 个文本 widget(label/button/checkbox/list/spinbox/roller/dropdown/table):draw 从 `ctx.resolved` 取字体传入;text_size 调用点同步带字体。
- spinbox:等宽字体下光标定位逻辑不变,仅 advance 来源从常量改为 font metrics。
- 接受外观变化:Content sizing(label/button/checkbox/roller)随 6x10 变化,受影响测试更新预期;demo 布局观察后微调。

### WidgetCtx 变化

- `ResolvedStyle` 加 `font: &'static MonoFont`(resolve 时填充),widget draw 直接用。

## 明确不做(YAGNI)

- 比例字体、抗锯齿、TTF/矢量子集化、underline/strikethrough 装饰线、中文
- 字体继承链(样式系统本无继承)
- builder 加 .font() 便捷方法

## 测试(host 端)

- text_size:单/多行/空串,spacing 语义。
- draw_text 像素断言:已知字符(如 '0'/'A')的 set-bit 像素颜色正确;clip 裁剪;opa 合成。
- 默认字体:resolved_style 无 font 字段时为 FONT_6X10 尺寸;set_default_font 生效;style.font 覆盖默认。
- 受影响既有测试:按新外观更新预期(逐个核对,不允许放松断言为"能过就行")。

## 影响面

- `qingui/Cargo.toml`:依赖换血
- `qingui/src/font.rs`:重写(MonoFont 渲染 + text_size)
- `qingui/src/draw.rs`:draw_text(_opa) 带字体
- `qingui/src/style.rs`:Style.font + ResolvedStyle.font
- `qingui/src/ui.rs`:default_font 字段 + set_default_font + resolve 填充
- `qingui/src/widgets/{label,button,checkbox,list,spinbox,roller,dropdown,table}.rs`:调用点适配
- `qingui/tests/*`:受影响测试更新 + tests/font.rs 新建
- `qingui/examples/*`:视觉变化(无需改代码,布局观察后微调)
