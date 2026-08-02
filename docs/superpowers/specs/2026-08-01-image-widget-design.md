# Image widget + qingui-codegen 设计

日期:2026-08-01
状态:已与用户确认

## 目标

新增 image widget:png/jpg 静态图与 gif 动画。解码全部发生在构建时(主机端),固件只含 RGB565 位图数据,widget 只做 blit 与按帧延时播放。

## 已确认的决策

| 决策点 | 结论 |
|---|---|
| 解码策略 | 构建时预转换(主机端解码,固件零解码依赖/RAM) |
| gif | 拆成帧数组 + 按帧延时自动播放(tick 驱动) |
| 像素格式 | RGB565(每像素 2 字节,小端) |
| 工具形态 | workspace 新增 qingui-codegen crate,用户 build.rs 调用一行 |

## 结构

### qingui-codegen(host/std 工具 crate,不进固件)

- 依赖 `image` crate(png/jpg/gif 解码,仅主机)。
- API:`qingui_codegen::convert(assets_dir: &str, out_dir: &str)`,供用户 build.rs 调用:
  ```rust
  // 固件 crate 的 build.rs
  qingui_codegen::convert("assets", &std::env::var("OUT_DIR").unwrap());
  ```
- 扫描 assets 目录:png/jpg → 单帧 ImageData;gif → 多帧 ImageData(逐帧 RGBA 解码 + 帧延时)。
- 生成 `<out_dir>/images.rs`:每个文件一个 `pub static <文件名大写snake>: qingui::widgets::image::ImageData`;RGBA → RGB565 小端(透明通道直接丢弃)。
- 用户侧 `include!(concat!(env!("OUT_DIR"), "/images.rs"))`。

### 库侧(qingui/src/widgets/image.rs)

```rust
pub struct Frame { pub w: i32, pub h: i32, pub rgb565: &'static [u8] }
pub struct ImageData { pub frames: &'static [Frame], pub delays_ms: &'static [u16] }

pub struct ImageState {
    pub data: &'static ImageData,
    pub cur: usize,
    pub last_switch: u64,
}
```

- `ImageBuilder::new(&'static ImageData)`:默认尺寸 = 首帧尺寸;size/style/sizing/transition/events 与其他 builder 一致。
- 注册:`define_widgets!` 加一行;无专属 API,不加扩展 trait,prelude 不动(builder 与其他控件一样经 `qingui::widgets::image::ImageBuilder` 路径导出)。

### 绘制

- `DrawBuf::blit565(x, y, w, h, data: &[u8], opa: u8, clip: Rect)`:clip 裁剪,逐像素 RGB565→RGB888 写入;draw 内无分配。
- WidgetBehavior::draw:blit 当前帧;overflow = 0(必在矩形内)。

### 播放(tick)

- 单帧:`TickOut::IDLE`。
- 多帧:`now - last_switch >= delays_ms[cur]` 则切帧(cur = (cur+1) % len,last_switch = now),返回 `{ redraw: true, active: true }`;未到点返回 `{ redraw: false, active: true }`(保持唤醒)。
- 隐藏语义复用现有管线:HIDDEN 子树不 tick、不标脏,恢复显示按绝对时间续播。

## 明确不做(YAGNI)

- 缩放/旋转/9-patch
- 透明色键与 gif dispose/blend 精细语义(每帧按完整帧处理,alpha 直接丢弃)
- 设备端运行时解码(架构上不冲突,未来可作为独立迭代)
- set_value/焦点/按键接入(纯展示控件)

## 测试(host 端)

- blit565:像素正确性(565→888 转换)、clip 裁剪、opa 合成(新文件 tests/image.rs,像素断言参照 tests/render.rs 模式)。
- 播放:tick 到点切帧/未到点不动/循环回卷;单帧 IDLE(timer 睡眠)。
- codegen:临时目录放一个最小 png(测试代码内现场生成或内嵌字节)跑 convert,断言生成文件内容(帧数/尺寸/数据长度)。
- ImageBuilder:默认尺寸 = 首帧尺寸。

## 影响面

- `qingui-codegen/`:新 crate(Cargo.toml + src/lib.rs)+ workspace 成员
- `qingui/src/draw.rs`:blit565
- `qingui/src/widgets/image.rs`:新建(Frame/ImageData/ImageState/ImageBuilder)
- `qingui/src/widgets/mod.rs`:mod 声明 + define_widgets! 一行
- `qingui/tests/image.rs`:新建
- `qingui/examples/demo.rs`:demo 页或现有页加示例(assets 放 demo 用图,demo build.rs 调 convert)
