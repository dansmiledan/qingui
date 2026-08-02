# Image widget + qingui-codegen 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按 `docs/superpowers/specs/2026-08-01-image-widget-design.md` 实现 image widget(png/jpg 静态图 + gif 帧动画),解码全部在构建时由新 crate qingui-codegen 完成,固件只含 RGB565 位图。

**Architecture:** 库侧加 `Color::from_rgb565/to_rgb565` + `DrawBuf::blit565` 原语 + `widgets/image.rs`(Frame/ImageData/ImageState/ImageBuilder,tick 驱动播帧);host 工具 crate `qingui-codegen`(std,依赖 image crate)在构建时把 assets 转成 `images.rs`。

**Tech Stack:** 库:no_std + alloc,host 测试 `cargo test -p qingui`,嵌入式 `cargo build -p qingui --target thumbv7em-none-eabihf`。codegen:std,`image = "0.25"`(default-features=false,features=["png","jpeg","gif"])。

## Global Constraints

- 库(qingui)零新增依赖、no_std + alloc、draw 热路径无分配。
- 行为语义:单帧 IDLE(timer 睡眠);多帧按 delays_ms 循环播放;HIDDEN 子树不播不脏(复用现有管线)。
- RGB565 小端;透明通道直接丢弃;gif 每帧按完整帧处理(无 dispose/blend)。
- 无效数据(data 长度不足等)静默不画。
- 中文注释风格一致;commit message 中文。
- 每个 Task 结束:`cargo test -p qingui` 全绿 + thumbv7em 通过;Task 3/4 另需 `cargo test -p qingui-codegen` 与 `cargo check -p qingui --examples`。
- **对 spec 的有意修正(已裁决)**:demo 不在 qingui 包里加 build.rs——库的 build.rs 会对下游固件用户执行并把 image crate 拉进其 build-dependency。改为:codegen 提供 CLI example,一次性生成 `qingui/examples/images.rs` 提交进仓库。build.rs 调用模式仍是下游标准用法(codegen doc 中给出)。

---

### Task 1: Color 565 转换 + DrawBuf::blit565

**Files:**
- Modify: `qingui/src/geometry.rs`(Color impl 块,约 :92 blend 之后)
- Modify: `qingui/src/draw.rs`(DrawBuf impl 块,fill_rect 之后)
- Test: `qingui/tests/draw.rs`(追加)

**Interfaces:**
- Consumes: `DrawBuf { pixels, area, stride }`、`put`(draw.rs:86)、`Rect::intersect`、`Color::blend`。
- Produces(后续 Task 依赖):
  - `Color::from_rgb565(v: u16) -> Color`
  - `Color::to_rgb565(self) -> u16`
  - `DrawBuf::blit565(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u8], opa: u8, clip: Rect)`

- [ ] **Step 1: 写失败测试(追加到 tests/draw.rs)**

```rust
#[test]
fn rgb565_roundtrip() {
    use qingui::Color;
    // 纯色端点
    assert_eq!(Color::from_rgb565(0xF800), Color::rgb(255, 0, 0));
    assert_eq!(Color::from_rgb565(0x07E0), Color::rgb(0, 255, 0));
    assert_eq!(Color::from_rgb565(0x001F), Color::rgb(0, 0, 255));
    assert_eq!(Color::from_rgb565(0xFFFF), Color::rgb(255, 255, 255));
    assert_eq!(Color::from_rgb565(0x0000), Color::rgb(0, 0, 0));
    // 全量往返不丢位
    for v in [0x0001u16, 0x1234, 0x7BEF, 0x8C51, 0xFFFE] {
        assert_eq!(Color::from_rgb565(v).to_rgb565(), v);
    }
}

#[test]
fn blit565_pixels_clip_and_opa() {
    use qingui::draw::DrawBuf;
    use qingui::{Color, Rect};
    // 2x2 图:红 绿 / 蓝 白(565 小端字节序)
    let data: [u8; 8] = [0x00, 0xF8, 0xE0, 0x07, 0x1F, 0x00, 0xFF, 0xFF];
    let mut buf = [Color::rgb(0, 0, 0); 16];
    {
        let mut d = DrawBuf { pixels: &mut buf, area: Rect::new(0, 0, 4, 4), stride: 4 };
        d.blit565(1, 1, 2, 2, &data, 255, Rect::new(0, 0, 4, 4));
    }
    assert_eq!(buf[1 * 4 + 1], Color::rgb(255, 0, 0));
    assert_eq!(buf[1 * 4 + 2], Color::rgb(0, 255, 0));
    assert_eq!(buf[2 * 4 + 1], Color::rgb(0, 0, 255));
    assert_eq!(buf[2 * 4 + 2], Color::rgb(255, 255, 255));
    // clip 裁剪:只允许左列
    let mut buf2 = [Color::rgb(0, 0, 0); 16];
    {
        let mut d = DrawBuf { pixels: &mut buf2, area: Rect::new(0, 0, 4, 4), stride: 4 };
        d.blit565(1, 1, 2, 2, &data, 255, Rect::new(0, 0, 2, 4));
    }
    assert_eq!(buf2[1 * 4 + 1], Color::rgb(255, 0, 0));
    assert_eq!(buf2[1 * 4 + 2], Color::rgb(0, 0, 0)); // 被裁掉
    // opa=0 不写;data 不足不画不 panic
    let mut buf3 = [Color::rgb(1, 2, 3); 4];
    {
        let mut d = DrawBuf { pixels: &mut buf3, area: Rect::new(0, 0, 2, 2), stride: 2 };
        d.blit565(0, 0, 2, 2, &data, 0, Rect::new(0, 0, 2, 2));
        d.blit565(0, 0, 4, 4, &data, 255, Rect::new(0, 0, 2, 2)); // 长度不足
    }
    assert_eq!(buf3, [Color::rgb(1, 2, 3); 4]);
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p qingui --test draw 2>&1 | tail -3`
Expected: 编译失败(from_rgb565/blit565 不存在)

- [ ] **Step 3: 实现**

geometry.rs Color impl 块追加:

```rust
    /// RGB565(5-6-5)→ RGB888(位复制扩展,全量往返不丢位)
    pub fn from_rgb565(v: u16) -> Color {
        let r = ((v >> 11) & 0x1F) as u8;
        let g = ((v >> 5) & 0x3F) as u8;
        let b = (v & 0x1F) as u8;
        Color::rgb((r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2))
    }
    /// RGB888 → RGB565(高位截断)
    pub fn to_rgb565(self) -> u16 {
        ((self.r as u16 >> 3) << 11) | ((self.g as u16 >> 2) << 5) | (self.b as u16 >> 3)
    }
```

(若 Color 还含其他字段,以 Color::rgb 构造为准;字段名以现有 struct 为准。)

draw.rs DrawBuf impl 块追加:

```rust
    /// 1:1 blit RGB565(小端)位图;data 不足 w*h*2 时静默不画。无分配。
    pub fn blit565(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u8], opa: u8, clip: Rect) {
        if w <= 0 || h <= 0 || data.len() < (w as usize) * (h as usize) * 2 {
            return;
        }
        let dst = Rect::new(x, y, w, h);
        let Some(r) = dst.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        for py in r.y..r.bottom() {
            for px in r.x..r.right() {
                let sx = (px - x) as usize;
                let sy = (py - y) as usize;
                let i = (sy * w as usize + sx) * 2;
                let v = data[i] as u16 | ((data[i + 1] as u16) << 8);
                self.put(px, py, crate::geometry::Color::from_rgb565(v), opa);
            }
        }
    }
```

- [ ] **Step 4: 跑测试确认通过 + 全量回归 + thumbv7em**

Run: `cargo test -p qingui --test draw 2>&1 | tail -3`(全 PASS)
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(空)+ `cargo build -p qingui --target thumbv7em-none-eabihf`

- [ ] **Step 5: Commit**

```bash
git add qingui/src/geometry.rs qingui/src/draw.rs qingui/tests/draw.rs
git commit -m "feat(draw): Color RGB565 转换 + blit565 位图原语

位复制扩展保证 565↔888 全量往返不丢位;blit 走 put 统一处理
clip/area/opa,无分配,数据不足静默不画。"
```

---

### Task 2: widgets/image.rs + 注册

**Files:**
- Create: `qingui/src/widgets/image.rs`
- Modify: `qingui/src/widgets/mod.rs`(`pub mod image;` + define_widgets! 加一行)
- Test: `qingui/tests/image.rs`(新建)

**Interfaces:**
- Consumes: Task 1 的 `blit565`;`WidgetBehavior`(draw 必实现,tick 默认 IDLE);TickOut 结构体字面量(`TickOut { redraw, active }`,字段 pub);builder 模式参照 `qingui/src/widgets/chart.rs`。
- Produces:
  - `qingui::widgets::image::{Frame, ImageData, ImageState, ImageBuilder}`
  - `Frame { w: i32, h: i32, rgb565: &'static [u8] }`
  - `ImageData { frames: &'static [Frame], delays_ms: &'static [u16] }`
  - `ImageBuilder::new(&'static ImageData)`,默认尺寸=首帧尺寸

- [ ] **Step 1: 写失败测试(新建 tests/image.rs)**

```rust
use qingui::display::Flush;
use qingui::widgets::image::{Frame, ImageBuilder, ImageData};
use qingui::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

/// 2x2 全红图
static RED: ImageData = ImageData {
    frames: &[Frame { w: 2, h: 2, rgb565: &[0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8] }],
    delays_ms: &[0],
};
/// 两帧动画:红/蓝,各 100ms
static ANIM: ImageData = ImageData {
    frames: &[
        Frame { w: 2, h: 2, rgb565: &[0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8, 0x00, 0xF8] },
        Frame { w: 2, h: 2, rgb565: &[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00] },
    ],
    delays_ms: &[100, 100],
};

#[test]
fn builder_default_size_is_first_frame() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let im = ImageBuilder::new(&RED).build(&mut ui, s);
    assert_eq!(ui.rect(im), Rect::new(0, 0, 2, 2));
}

#[test]
fn static_image_sleeps() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    ImageBuilder::new(&RED).build(&mut ui, s);
    ui.tick_inc(16);
    ui.timer_handler();
    assert_eq!(ui.timer_handler(), u32::MAX); // 单帧无逐帧行为
}

#[test]
fn gif_advances_and_wraps() {
    #[derive(Default)]
    struct Rec { n: usize }
    struct Shared(Rc<RefCell<Rec>>);
    impl Flush for Shared {
        fn flush(&mut self, _a: Rect, _p: &[Color]) { self.0.borrow_mut().n += 1; }
    }
    let rec = Rc::new(RefCell::new(Rec::default()));
    let mut ui = Ui::new(64, 64, 16);
    ui.set_flush(Box::new(Shared(rec.clone())));
    let s = ui.screen();
    let im = ImageBuilder::new(&ANIM).build(&mut ui, s);
    ui.tick_inc(16);
    assert_eq!(ui.timer_handler(), 0); // 动画保持唤醒
    rec.borrow_mut().n = 0;
    ui.tick_inc(50); // 未到 100ms:不切帧不重绘
    ui.timer_handler();
    assert_eq!(rec.borrow().n, 0);
    ui.tick_inc(60); // 累计 110ms:切到帧 1 并重绘
    ui.timer_handler();
    assert!(rec.borrow().n > 0);
    ui.tick_inc(100); // 再 100ms:回卷到帧 0
    ui.timer_handler();
    assert!(rec.borrow().n > 0);
    let _ = im;
}
```

- [ ] **Step 2: 跑测试确认失败**(编译失败,image 模块不存在)

- [ ] **Step 3: 实现 qingui/src/widgets/image.rs**

```rust
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::Rect;
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{TickOut, WidgetBehavior, WidgetCtx, WidgetKind};

/// 一帧 RGB565(小端)位图
pub struct Frame {
    pub w: i32,
    pub h: i32,
    pub rgb565: &'static [u8],
}

/// 图片数据:静态图单帧;gif 多帧 + 逐帧延时。由 qingui-codegen 生成
pub struct ImageData {
    pub frames: &'static [Frame],
    pub delays_ms: &'static [u16],
}

pub struct ImageState {
    pub data: &'static ImageData,
    pub cur: usize,
    pub last_switch: u64,
}

impl WidgetBehavior for ImageState {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
        let Some(f) = self.data.frames.get(self.cur) else { return };
        d.blit565(ctx.abs.x, ctx.abs.y, f.w, f.h, f.rgb565, ctx.ap(255), clip);
    }
    fn tick(&mut self, now: u64) -> TickOut {
        if self.data.frames.len() <= 1 {
            return TickOut::IDLE;
        }
        let delay = self.data.delays_ms.get(self.cur).copied().unwrap_or(100) as u64;
        if now.saturating_sub(self.last_switch) >= delay {
            self.cur = (self.cur + 1) % self.data.frames.len();
            self.last_switch = now;
            TickOut { redraw: true, active: true }
        } else {
            TickOut { redraw: false, active: true }
        }
    }
}

/// Image 构建器:默认尺寸 = 首帧尺寸 + bg 透明
pub struct ImageBuilder {
    data: &'static ImageData,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl ImageBuilder {
    pub fn new(data: &'static ImageData) -> Self {
        Self { data, size: None, style: None, sizing: None, transition: None, events: Vec::new() }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (fw, fh) = self.data.frames.first().map(|f| (f.w, f.h)).unwrap_or((0, 0));
        let (w, h) = self.size.unwrap_or((fw, fh));
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Image(
            ImageState { data: self.data, cur: 0, last_switch: ui.time() },
        ));
        let mut s = self.style.unwrap_or_default();
        if s.bg_opa.is_none() {
            s.bg_opa = Some(0);
        }
        ui.set_style(r, s);
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}
```

mod.rs:`pub mod image;`(字母序 checkbox 之后 itemlist 之前);define_widgets! 加一行 `Image(image::ImageState, as_image, as_image_mut),`(字母序 Dropdown 之后 ItemList 之前)。

- [ ] **Step 4: 跑测试确认通过 + 全量回归 + thumbv7em + examples check**

- [ ] **Step 5: Commit**

```bash
git add qingui/src/widgets/image.rs qingui/src/widgets/mod.rs qingui/tests/image.rs
git commit -m "feat(image): image widget(静态图 + gif 帧动画)

ImageData 由构建时 codegen 生成;draw 走 blit565,tick 按帧延时
循环播放(单帧 IDLE 睡眠),隐藏子树不播不脏复用现有管线。"
```

---

### Task 3: qingui-codegen crate

**Files:**
- Modify: `Cargo.toml`(workspace members 加 "qingui-codegen")
- Create: `qingui-codegen/Cargo.toml`
- Create: `qingui-codegen/src/lib.rs`
- Create: `qingui-codegen/examples/convert.rs`(CLI:`<assets_dir> <out_dir>`)
- Test: `qingui-codegen/tests/convert.rs`

**Interfaces:**
- Consumes: `qingui::Color::to_rgb565`(codegen 以 path 依赖 qingui,复用同一转换,避免两处实现分叉);`qingui::widgets::image::{ImageData, Frame}` 的生成目标形状(Task 2)。
- Produces: `qingui_codegen::convert(assets_dir: &str, out_dir: &str) -> std::io::Result<()>`,在 out_dir 生成 `images.rs`。

- [ ] **Step 1: 写失败测试(qingui-codegen/tests/convert.rs)**

```rust
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, Rgba, RgbaImage};
use std::fs;

/// 现场生成 2x2 png(左上角纯红,其余纯绿)与 2 帧 gif(帧1 全红 80ms,帧2 全蓝 120ms)
fn make_assets(dir: &std::path::Path) {
    let mut png = RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255]));
    png.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    png.save(dir.join("logo.png")).unwrap();

    let f1 = Frame::from_parts(RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])),
                               0, 0, Delay::from_numer_denom_ms(80, 1));
    let f2 = Frame::from_parts(RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255])),
                               0, 0, Delay::from_numer_denom_ms(120, 1));
    let mut enc = GifEncoder::new(fs::File::create(dir.join("anim.gif")).unwrap());
    enc.encode_frames(vec![f1, f2].into_iter()).unwrap();
}

#[test]
fn convert_generates_expected_images_rs() {
    let tmp = std::env::temp_dir().join(format!("qg-codegen-{}", std::process::id()));
    let assets = tmp.join("assets");
    let out = tmp.join("out");
    fs::create_dir_all(&assets).unwrap();
    fs::create_dir_all(&out).unwrap();
    make_assets(&assets);

    qingui_codegen::convert(assets.to_str().unwrap(), out.to_str().unwrap()).unwrap();
    let gen = fs::read_to_string(out.join("images.rs")).unwrap();

    // 静态图:单帧、2x2、8 字节、delay 0
    assert!(gen.contains("pub static LOGO: qingui::widgets::image::ImageData"));
    assert!(gen.contains("delays_ms: &[0]"));
    // gif:两帧、延时 80/120
    assert!(gen.contains("pub static ANIM: qingui::widgets::image::ImageData"));
    assert!(gen.contains("delays_ms: &[80, 120]"));
    // 帧像素:png 的 (0,0) 纯红 → 0xF800 小端 = [0x00, 0xF8] 在最前
    assert!(gen.contains("0x00, 0xF8"));

    fs::remove_dir_all(&tmp).ok();
}
```

注:image crate 的 `Frame::from_parts(buffer, left, top, delay)`、`Delay::from_numer_denom_ms(numer, denom)`、`GifEncoder::encode_frames` 以 0.25 实际 API 为准;若有出入,以 crates.io 文档调整并记录偏差。

- [ ] **Step 2: 跑测试确认失败**(crate 不存在)

- [ ] **Step 3: 实现**

workspace Cargo.toml:`members = ["qingui", "qingui-codegen"]`

qingui-codegen/Cargo.toml:

```toml
[package]
name = "qingui-codegen"
version = "0.1.0"
edition = "2021"

[dependencies]
qingui = { path = "../qingui" }
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "gif"] }

[dev-dependencies]
image = { version = "0.25", default-features = false, features = ["png", "gif"] }
```

qingui-codegen/src/lib.rs:

```rust
//! 构建时图片转换:assets 目录 → images.rs(RGB565 位图数组)。
//! 用法(固件 crate 的 build.rs):
//! `qingui_codegen::convert("assets", &std::env::var("OUT_DIR").unwrap());`
//! 然后 `include!(concat!(env!("OUT_DIR"), "/images.rs"));`

use std::fmt::Write as _;
use std::path::Path;

/// 扫描 assets_dir,把 png/jpg/jpeg 转为单帧、gif 转为多帧 ImageData,
/// 在 out_dir 生成 images.rs。透明通道直接丢弃;gif 每帧按完整帧处理。
pub fn convert(assets_dir: &str, out_dir: &str) -> std::io::Result<()> {
    let mut code = String::from("// @generated by qingui-codegen; DO NOT EDIT\n");
    let mut entries: Vec<_> = std::fs::read_dir(assets_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    entries.sort();
    for path in entries {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let name = to_screaming_snake(stem);
        let frames: Vec<(i32, i32, Vec<u8>, u16)> = match ext.as_str() {
            "png" | "jpg" | "jpeg" => {
                let img = image::open(&path).map_err(to_ioe)?.to_rgba8();
                let (w, h) = (img.width() as i32, img.height() as i32);
                vec![(w, h, rgba_to_565(img.as_raw()), 0)]
            }
            "gif" => decode_gif(&path)?,
            _ => continue,
        };
        write_static(&mut code, &name, &frames);
    }
    std::fs::write(Path::new(out_dir).join("images.rs"), code)
}

fn decode_gif(path: &Path) -> std::io::Result<Vec<(i32, i32, Vec<u8>, u16)>> {
    let file = std::fs::File::open(path)?;
    let dec = image::codecs::gif::GifDecoder::new(file).map_err(to_ioe)?;
    let mut out = Vec::new();
    for f in dec.into_frames().collect::<Result<Vec<_>, _>>().map_err(to_ioe)? {
        let (numer, denom) = f.delay().numer_denom_ms();
        let ms = (numer / denom.max(1)).max(1) as u16;
        let buf = f.into_buffer();
        out.push((buf.width() as i32, buf.height() as i32, rgba_to_565(buf.as_raw()), ms));
    }
    Ok(out)
}

fn rgba_to_565(rgba: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(rgba.len() / 2);
    for px in rgba.chunks_exact(4) {
        let c = qingui::Color::rgb(px[0], px[1], px[2]).to_rgb565();
        v.push((c & 0xFF) as u8);
        v.push((c >> 8) as u8);
    }
    v
}

fn write_static(code: &mut String, name: &str, frames: &[(i32, i32, Vec<u8>, u16)]) {
    let _ = writeln!(code, "pub static {name}: qingui::widgets::image::ImageData = qingui::widgets::image::ImageData {{");
    let _ = writeln!(code, "    frames: &[");
    for (w, h, data, _ms) in frames {
        let _ = write!(code, "        qingui::widgets::image::Frame {{ w: {w}, h: {h}, rgb565: &[");
        for (i, b) in data.iter().enumerate() {
            if i > 0 { let _ = write!(code, ", "); }
            let _ = write!(code, "0x{b:02X}");
        }
        let _ = writeln!(code, "] }},");
    }
    let _ = writeln!(code, "    ],");
    let delays: Vec<String> = frames.iter().map(|f| f.3.to_string()).collect();
    let _ = writeln!(code, "    delays_ms: &[{}],", delays.join(", "));
    let _ = writeln!(code, "}};");
}

fn to_screaming_snake(stem: &str) -> String {
    let mut out = String::new();
    for (i, ch) in stem.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    out
}

fn to_ioe(e: image::ImageError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
}
```

qingui-codegen/examples/convert.rs:

```rust
//! 一次性生成 images.rs:cargo run -p qingui-codegen --example convert -- <assets_dir> <out_dir>
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: convert <assets_dir> <out_dir>");
        std::process::exit(2);
    }
    qingui_codegen::convert(&args[1], &args[2]).unwrap();
}
```

- [ ] **Step 4: 跑测试 + 回归**

Run: `cargo test -p qingui-codegen 2>&1 | tail -3`(PASS)
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(空)+ `cargo build -p qingui --target thumbv7em-none-eabihf`(codegen 不影响嵌入式构建)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml qingui-codegen
git commit -m "feat(codegen): qingui-codegen 构建时图片转换 crate

png/jpg → 单帧、gif → 多帧 ImageData(RGB565 小端);依赖 image
crate 仅主机端,固件零解码负担。附一次性 CLI example。"
```

---

### Task 4: demo 集成(About 页静态图 + gif)

**Files:**
- Create: `qingui/examples/assets/logo.png`、`qingui/examples/assets/anim.gif`(由脚本生成,二进制提交)
- Create: `qingui-codegen/examples/make_demo_assets.rs`(一次性资产生成脚本)
- Create: `qingui/examples/images.rs`(由 convert CLI 生成,@generated 提交)
- Modify: `qingui/examples/demo.rs`(About 页加静态图 + gif)

**Interfaces:**
- Consumes: Task 3 的 convert CLI;Task 2 的 `ImageBuilder`;demo.rs 的 `page_about`(column flex 容器)。
- Produces: 无新接口。

- [ ] **Step 1: 写资产生成脚本(qingui-codegen/examples/make_demo_assets.rs)**

```rust
//! 生成 demo 用测试图:cargo run -p qingui-codegen --example make_demo_assets
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, Rgba, RgbaImage};

fn main() {
    let dir = std::path::Path::new("qingui/examples/assets");
    std::fs::create_dir_all(dir).unwrap();
    // logo.png:48x24 蓝底白斜纹
    let mut img = RgbaImage::from_pixel(48, 24, Rgba([40, 80, 200, 255]));
    for x in 0..48 {
        for y in 0..24 {
            if (x + y) % 8 < 2 {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }
    img.save(dir.join("logo.png")).unwrap();
    // anim.gif:16x16,3 帧纯色(红/绿/蓝),各 300ms
    let frames: Vec<Frame> = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]]
        .into_iter()
        .map(|c| Frame::from_parts(RgbaImage::from_pixel(16, 16, Rgba([c[0], c[1], c[2], 255])),
                                   0, 0, Delay::from_numer_denom_ms(300, 1)))
        .collect();
    let mut enc = GifEncoder::new(std::fs::File::create(dir.join("anim.gif")).unwrap());
    enc.encode_frames(frames.into_iter()).unwrap();
}
```

- [ ] **Step 2: 生成资产与 images.rs**

Run: `cargo run -p qingui-codegen --example make_demo_assets`
Run: `cargo run -p qingui-codegen --example convert -- qingui/examples/assets qingui/examples`
Expected: 生成 qingui/examples/assets/{logo.png,anim.gif} 与 qingui/examples/images.rs(含 LOGO/ANIM 两个 static)

- [ ] **Step 3: demo.rs 接入**

`mod sim;` 下加 `mod images;`;imports 加 `use qingui::widgets::image::ImageBuilder;`。About 页(page_about 容器内,现有内容之后)加:

```rust
    let _logo = ImageBuilder::new(&images::LOGO).build(ui, page_about);
    let _anim = ImageBuilder::new(&images::ANIM).build(ui, page_about);
```

- [ ] **Step 4: 验证**

Run: `cargo check -p qingui --examples`(零 error)
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(空)
手动:sim About 页可见蓝底斜纹静态图与红绿蓝循环闪烁的 16x16 方块。

- [ ] **Step 5: Commit**

```bash
git add qingui-codegen/examples/make_demo_assets.rs qingui/examples/assets qingui/examples/images.rs qingui/examples/demo.rs
git commit -m "feat(demo): About 页加静态图与 gif 动画示例(image widget)"
```

---

## Self-Review 记录

- Spec 覆盖:RGB565/小端/丢 alpha(Task 1+3)、blit565(Task 1)、ImageData/Frame/ImageState/builder(Task 2)、gif 帧+延时+循环播放(Task 2 tick + Task 3 decode_gif)、codegen crate 与 build.rs 用法(Task 3 doc)、demo 集成(Task 4)、测试清单(每 Task)。
- 对 spec 的修正:demo 不用 build.rs(避免库包 build.rs 拖累下游),改一次性生成提交——已在 Global Constraints 声明。
- 占位符:无;image crate API 给了具体名称并注明"以 0.25 实际 API 为准"的fallback 记录要求。
- 类型一致性:Frame/ImageData 字段名(w/h/rgb565/frames/delays_ms)、convert 签名、blit565 签名全文一致;codegen 生成代码引用 `qingui::widgets::image::ImageData`,与 Task 2 产出一致。
- 命名:静态名 to_screaming_snake(logo.png→LOGO、anim.gif→ANIM),Task 3 测试与 Task 4 demo 引用一致。
