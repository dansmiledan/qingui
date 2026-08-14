# Generic Pixel Format (e-g Interop) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make qingui's framebuffer pixel format selectable per device (`Rgb565`, `Rgb888`, …) so e-g ecosystem code draws into a qingui canvas with the device-native color type and flush output needs no conversion.

**Architecture:** Add a `PixelFormat` trait bridging qingui's internal RGB888 `Color` and e-g `PixelColor` types; parameterize the storage/render path (`Canvas`, `Flush`, `Ui`, `Node`, `Widget`, builders) with a type parameter `C` that **defaults to `Color` everywhere**, so existing user code compiles unchanged. Widget drawing logic is untouched: all drawing methods still take `Color` + opa; conversion happens inside `Canvas`'s pixel-write sites only.

**Tech Stack:** Rust edition 2024, `no_std` + `alloc`, embedded-graphics 0.8 (default-features = false), cargo test.

**Spec:** `docs/superpowers/specs/2026-08-14-generic-pixel-format-design.md`

## Global Constraints

- Crate `qingui` is `#![no_std]` with `extern crate alloc`; no new dependencies.
- embedded-graphics 0.8, `default-features = false` (already in `qingui/Cargo.toml:36`).
- Every new generic parameter defaults to `Color` (`crate::geometry::Color`); existing public API must stay source-compatible and behavior-identical at `C = Color`.
- `Rgb565` conversion must stay bit-consistent with the existing `Color::to_rgb565` / `Color::from_rgb565` helpers (`qingui/src/geometry.rs:121-137`), which `Canvas::blit565` relies on.
- Code comments and commit messages in English (Conventional Commits). Commits are local only; **never `git push`**. Per `AGENTS.md`, ask the user before each commit batch.
- After every task: `cargo test -p qingui` must be green. Tasks 3-7 are behavior-preserving refactors — the existing suite IS the test; do not modify existing test expectations.
- Widget **drawing logic** (the 800+ `Color` call sites) must not change; only signatures gain `<C>`.

---

### Task 1: `PixelFormat` trait + `Color: PixelColor`

**Files:**
- Create: `qingui/src/pixel.rs`
- Modify: `qingui/src/geometry.rs` (append at end of file, after line 150)
- Modify: `qingui/src/lib.rs` (add module + re-export)
- Test: `qingui/src/pixel.rs` (`#[cfg(test)] mod tests` at the bottom)

**Interfaces:**
- Consumes: `crate::geometry::Color` (`rgb`, `to_rgb565`, `from_rgb565`, `blend`).
- Produces (all later tasks rely on these):
  - `pub trait qingui::pixel::PixelFormat: PixelColor + Copy + PartialEq + Default { fn to_color(self) -> Color; fn from_color(c: Color) -> Self; }`
  - `impl PixelFormat for Color` (identity)
  - `impl PixelFormat for Rgb888/Rgb565/Rgb555/Rgb666/Bgr888/Bgr565/Bgr555/Bgr666`
  - `impl embedded_graphics::pixelcolor::PixelColor for Color` (`type Raw = RawU24`)
  - `qingui::pixel` module is public; `PixelFormat` re-exported at crate root.

- [ ] **Step 1: Write the failing tests**

Create `qingui/src/pixel.rs` with ONLY this content:

```rust
//! Framebuffer pixel formats: the bridge between qingui's internal RGB888 `Color`
//! and the device-native pixel type stored in the framebuffer.

use crate::geometry::Color;
use embedded_graphics::pixelcolor::PixelColor;

/// A framebuffer pixel format: convertible to/from the internal RGB888 `Color`.
///
/// Implemented for qingui's own `Color` (identity, the default) and for the
/// embedded-graphics RGB/BGR color types, so the framebuffer can directly use
/// the display's native format (e.g. `Rgb565`).
pub trait PixelFormat: PixelColor + Copy + PartialEq + Default {
    /// Converts a framebuffer pixel to the internal RGB888 `Color`.
    fn to_color(self) -> Color;
    /// Converts an internal RGB888 `Color` to a framebuffer pixel (quantizes).
    fn from_color(c: Color) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::pixelcolor::{Rgb565, Rgb888};

    #[test]
    fn color_identity() {
        let c = Color::rgb(80, 140, 255);
        assert_eq!(PixelFormat::to_color(c), c);
        assert_eq!(<Color as PixelFormat>::from_color(c), c);
    }

    #[test]
    fn rgb888_lossless_roundtrip() {
        let c = Color::rgb(1, 128, 255);
        assert_eq!(Rgb888::from_color(c).to_color(), c);
    }

    #[test]
    fn rgb565_matches_color_helpers() {
        for &c in &[Color::BLACK, Color::WHITE, Color::rgb(80, 140, 255), Color::rgb(1, 2, 3), Color::rgb(255, 128, 0)] {
            let px = Rgb565::from_color(c);
            assert_eq!(RawU16::from(px).into_inner(), c.to_rgb565(), "from_color mismatch for {c:?}");
            assert_eq!(px.to_color(), Color::from_rgb565(c.to_rgb565()), "to_color mismatch for {c:?}");
        }
    }

    #[test]
    fn rgb565_quantizes() {
        assert_eq!(Rgb565::from_color(Color::RED), Rgb565::RED);
        assert_eq!(Rgb565::from_color(Color::WHITE), Rgb565::WHITE);
        assert_eq!(Rgb565::from_color(Color::BLACK), Rgb565::BLACK);
    }

    #[test]
    fn color_is_pixel_color() {
        // Compile-time proof that Color: PixelColor, usable as the default framebuffer format.
        fn assert_pc<T: PixelColor>() {}
        assert_pc::<Color>();
    }
}
```

Register the module in `qingui/src/lib.rs`: insert `pub mod pixel;` after the `pub mod node;` line, and add after line 26 (`pub use geometry::{Color, Point, Rect};`):

```rust
/// Framebuffer pixel format bridge between internal RGB888 `Color` and e-g pixel types.
pub use pixel::PixelFormat;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qingui pixel`
Expected: FAIL to compile — `PixelFormat` is not implemented for `Rgb565` etc. (`from_color`/`to_color` not found), and `Color: PixelColor` is not satisfied.

- [ ] **Step 3: Implement the trait impls**

In `qingui/src/pixel.rs`, change the top `use` to:

```rust
use crate::geometry::Color;
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::{Bgr555, Bgr565, Bgr666, Bgr888, PixelColor, Rgb555, Rgb565, Rgb666, Rgb888};
```

and add after the trait definition (before `#[cfg(test)]`):

```rust
impl PixelFormat for Color {
    fn to_color(self) -> Color { self }
    fn from_color(c: Color) -> Self { c }
}

/// Implements `PixelFormat` for an e-g RGB/BGR color type via its `RgbColor`
/// constructor/accessors (8-bit channels in, quantized storage out).
macro_rules! impl_pixel_format_rgb {
    ($($t:ty),* $(,)?) => {$(
        impl PixelFormat for $t {
            fn to_color(self) -> Color {
                use embedded_graphics::pixelcolor::RgbColor;
                Color::rgb(self.r(), self.g(), self.b())
            }
            fn from_color(c: Color) -> Self {
                <$t>::new(c.r, c.g, c.b)
            }
        }
    )*};
}

impl_pixel_format_rgb!(Rgb888, Rgb555, Rgb666, Bgr888, Bgr565, Bgr555, Bgr666);

// Rgb565 is implemented via raw storage so it stays bit-consistent with
// `Color::to_rgb565`/`from_rgb565`, which `Canvas::blit565` relies on.
impl PixelFormat for Rgb565 {
    fn to_color(self) -> Color {
        Color::from_rgb565(RawU16::from(self).into_inner())
    }
    fn from_color(c: Color) -> Self {
        Rgb565::from(RawU16::new(c.to_rgb565()))
    }
}
```

In `qingui/src/geometry.rs`, append at the end of the file:

```rust
impl embedded_graphics::pixelcolor::PixelColor for Color {
    type Raw = embedded_graphics::pixelcolor::raw::RawU24;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qingui`
Expected: PASS — new pixel tests green, entire existing suite still green.
If `rgb565_matches_color_helpers` fails because e-g's `RawU16` API differs, read the exact e-g 0.8 API in the registry source under `~/.cargo/registry/src/*/embedded-graphics-0.8*/src/pixelcolor/`; the required behavior is fixed by the test (bit-consistency with `to_rgb565`/`from_rgb565`), only the plumbing may change.

- [ ] **Step 5: Commit**

```bash
git add qingui/src/pixel.rs qingui/src/geometry.rs qingui/src/lib.rs
git commit -m "feat: add PixelFormat bridge between internal Color and e-g pixel types"
```

---

### Task 2: Generic `Canvas<'a, C>`

**Files:**
- Modify: `qingui/src/canvas.rs` (struct lines 11-18, `clear` 22-24, `put` 26-38, `put_fast` 49-58, `fill_rect` 61-84, `fill_circle` opaque span 236-245, `draw_text_opa`'s `EgTarget` 428-455, `DrawTarget` impl 558-584, `Dimensions` impl 589-592)
- Test: `qingui/src/canvas.rs` (append a `#[cfg(test)] mod tests` at the end of the file — none exists today)

**Interfaces:**
- Consumes: `crate::pixel::PixelFormat` (Task 1).
- Produces: `pub struct Canvas<'a, C = Color> { pub pixels: &'a mut [C], pub area: Rect, pub stride: i32 }`; all existing drawing methods keep their signatures (`Color` + opa in). `impl<C: PixelFormat> DrawTarget for Canvas<'_, C>` with `type Color = C`. Later tasks name `Canvas<'_, C>` in signatures.

- [ ] **Step 1: Write the failing tests**

Append to `qingui/src/canvas.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{Circle, PrimitiveStyle};

    fn canvas565(buf: &mut [Rgb565]) -> Canvas<'_, Rgb565> {
        Canvas { pixels: buf, area: Rect::new(0, 0, 10, 10), stride: 10 }
    }

    #[test]
    fn rgb565_opaque_fill_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(255, 0, 0), 255, clip);
        assert!(d.pixels.iter().all(|&p| p == Rgb565::RED));
    }

    #[test]
    fn rgb565_fill_circle_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_circle(Point { x: 5, y: 5 }, 3, Color::WHITE, 255, clip);
        assert_eq!(d.pixels[5 * 10 + 5], Rgb565::WHITE); // center pixel
    }

    #[test]
    fn rgb565_blend_roundtrips_through_rgb888() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        // Blend pure white at 50% over black: internal math yields ~(128,128,128),
        // stored quantized to 565.
        d.put(2, 2, Color::WHITE, 128);
        let expected = Color::BLACK.blend(Color::WHITE, 128);
        assert_eq!(RawU16::from(d.pixels[2 * 10 + 2]).into_inner(), expected.to_rgb565());
    }

    #[test]
    fn draw_target_accepts_native_rgb565() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        Circle::new(embedded_graphics::geometry::Point::new(0, 0), 5)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
            .draw(&mut d)
            .unwrap();
        assert_eq!(d.pixels[1 * 10 + 1], Rgb565::GREEN); // pixel (1, 1)
    }

    #[test]
    fn default_canvas_still_rgb888() {
        let mut buf = [Color::BLACK; 100];
        let mut d: Canvas<'_> = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(80, 140, 255), 255, clip);
        assert!(d.pixels.iter().all(|&p| p == Color::rgb(80, 140, 255)));
    }
}
```

Note: `fill_circle`'s exact signature — read it in `canvas.rs` before writing the test (expected: `pub fn fill_circle(&mut self, center: Point, radius: i32, c: Color, opa: u8, clip: Rect)`; if it differs, adapt the call). `put` is `pub(crate)`, accessible from the in-file test module.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qingui canvas`
Expected: FAIL to compile — `Canvas` has no type parameter `C`, `pixels` is `&mut [Color]`.

- [ ] **Step 3: Genericize the struct and the five pixel-write sites**

In `qingui/src/canvas.rs`:

1. Add to the imports at the top: `use crate::pixel::PixelFormat;`

2. Struct (lines 11-18) — note: **no `C: PixelFormat` bound on the struct itself**, so `widgets/mod.rs` can name `Canvas<'_, C>` without a bound:

```rust
pub struct Canvas<'a, C = Color> {
    /// The backing pixel storage.
    pub pixels: &'a mut [C],
    /// The absolute screen region this buffer covers.
    pub area: Rect,
    /// Row length in pixels (usually `area.w`).
    pub stride: i32,
}
```

Also append to the struct doc comment (lines 9-10): "The pixel type `C` defaults to RGB888 `Color`; set it to the display's native format (e.g. `Rgb565`) to render directly in device format."

3. `impl Canvas<'_>` (line 20) → `impl<C: PixelFormat> Canvas<'_, C>`. Inside it, change ONLY the direct `self.pixels` write sites:

`clear` (22-24):
```rust
    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(C::from_color(c));
    }
```

`put` (33-37) and `put_fast` (53-57) — identical bodies:
```rust
        if opa >= 255 {
            self.pixels[idx] = C::from_color(c);
        } else if opa > 0 {
            self.pixels[idx] = C::from_color(self.pixels[idx].to_color().blend(c, opa));
        }
```

`fill_rect` opaque fast path (65-75): convert once, then fill rows:
```rust
        if opa >= 255 {
            // Opaque fast path: batch-fill whole rows (no per-pixel bounds check,
            // no per-pixel blending).
            let c = C::from_color(c);
            let area_x = self.area.x;
            // ... rest unchanged ...
                self.pixels[row..row + w].fill(c);
```

`fill_circle` opaque span (236-245):
```rust
            if fl <= fh {
                if opa >= 255 {
                    let row = ((y - self.area.y) * self.stride + (center.x + fl - self.area.x)) as usize;
                    self.pixels[row..row + (fh - fl + 1) as usize].fill(C::from_color(c));
                } else {
                    // ... unchanged (put_fast converts) ...
```

Every other method keeps its signature and logic (they all funnel through `put`/`put_fast`/`fill_rect`).

4. `EgTarget` inside `draw_text_opa` (428-455) becomes generic:

```rust
        struct EgTarget<'a, 'b, C> {
            d: &'a mut Canvas<'b, C>,
            c: Color,
            opa: u8,
        }
        impl<C: PixelFormat> DrawTarget for EgTarget<'_, '_, C> {
            type Color = embedded_graphics::pixelcolor::BinaryColor;
            // ... body unchanged ...
        }
        impl<C> embedded_graphics::geometry::Dimensions for EgTarget<'_, '_, C> {
            // ... body unchanged ...
        }
```

5. Replace the `DrawTarget` impl (558-584) with:

```rust
impl<C: PixelFormat> embedded_graphics::draw_target::DrawTarget for Canvas<'_, C> {
    type Color = C;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        // Per-pixel path: ecosystem compatibility, no performance promise.
        for embedded_graphics::Pixel(p, color) in pixels {
            self.put(p.x, p.y, color.to_color(), 255);
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &embedded_graphics::primitives::Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // Fast path: route through the batch row fill (eg's default would fall back to draw_iter).
        let clip = self.area;
        self.fill_rect(from_eg_rect(*area), color.to_color(), 255, clip);
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        Canvas::clear(self, color.to_color());
        Ok(())
    }
}
```

6. `Dimensions` impl (589-592) → `impl<C> embedded_graphics::geometry::Dimensions for Canvas<'_, C>` (body unchanged; no bound needed).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qingui`
Expected: PASS — new canvas tests green, existing suite (render.rs tests construct `Canvas` with `&mut [Color]` → default `C`) still green.

- [ ] **Step 5: Commit**

```bash
git add qingui/src/canvas.rs
git commit -m "feat: genericize Canvas over framebuffer pixel format"
```

---

### Task 3: `Widget<C>`, `Node<C>`, hooks and `EventCb<C>`

**Files:**
- Modify: `qingui/src/widgets/mod.rs` (`Widget` trait 97-119, `NoopWidget` impl 124-127)
- Modify: `qingui/src/node.rs` (`DrawHook`/`TickHook` 8-11, `Node` 68-110, `Node::new` 112-137)
- Modify: `qingui/src/event.rs` (`EventCb` line 22)

**Interfaces:**
- Consumes: `Canvas<'_, C>` (Task 2).
- Produces:
  - `pub trait Widget<C = Color>` — `draw(&self, &WidgetCtx, &mut Canvas<'_, C>, Rect)`; `layout`/`tick`/`on_key(&mut self, &mut Ui<C>, ...)`.
  - `pub struct Node<C = Color>` with `kind: Box<dyn Widget<C>>`, `events: Vec<(EventKind, EventCb<C>)>`, `draw_hook: Option<DrawHook<C>>`, `tick_hook: Option<TickHook<C>>`.
  - `pub type DrawHook<C = Color>`, `pub type TickHook<C = Color>`, `pub type EventCb<C = Color>`.
  - `impl<C> Widget<C> for NoopWidget`.

- [ ] **Step 1: Genericize `EventCb`**

In `qingui/src/event.rs`, line 22 →

```rust
pub type EventCb<C = crate::geometry::Color> = Box<dyn FnMut(&mut Ui<C>, ObjRef, EventKind)>;
```

- [ ] **Step 2: Genericize `Widget` and `NoopWidget`**

In `qingui/src/widgets/mod.rs`:

- Change the import at line 1 to `use crate::geometry::{Color, Rect};`.
- Doc comment on `Widget` (lines 87-96): append "/// `C` is the framebuffer pixel format (default RGB888 `Color`); widget drawing code always works in `Color`, the canvas converts."
- Trait (97-119): `pub trait Widget {` → `pub trait Widget<C = Color> {`, and change four signatures:

```rust
    fn draw(&self, _ctx: &WidgetCtx, _c: &mut Canvas<'_, C>, _clip: Rect) {}
```
```rust
    fn layout(&mut self, _ui: &mut Ui<C>, _obj: ObjRef, _content: Rect) {}
```
```rust
    fn tick(&mut self, _ui: &mut Ui<C>, _obj: ObjRef, _now: u64) -> TickOut { TickOut::IDLE }
```
```rust
    fn on_key(&mut self, _ui: &mut Ui<C>, _obj: ObjRef, _key: Key) -> KeyOutcome { KeyOutcome::Pass }
```

- `impl Widget for NoopWidget` (124) → `impl<C> Widget<C> for NoopWidget`.

- [ ] **Step 3: Genericize `Node` and the hooks**

In `qingui/src/node.rs`:

```rust
/// Overlay draw hook: called after the widget draws its own content, with
/// (draw buffer, widget absolute rect, clip rect, current time ms).
pub type DrawHook<C = crate::geometry::Color> = alloc::boxed::Box<dyn FnMut(&mut crate::canvas::Canvas<'_, C>, Rect, Rect, u64)>;
/// Per-frame hook: returning `true` means still active (dirties the node and keeps the
/// timer handler awake).
pub type TickHook<C = crate::geometry::Color> = alloc::boxed::Box<dyn FnMut(&mut crate::ui::Ui<C>, ObjRef, u64) -> bool>;
```

`pub struct Node {` → `pub struct Node<C = crate::geometry::Color> {`; field changes:

```rust
    pub kind: alloc::boxed::Box<dyn crate::widgets::Widget<C>>,
```
```rust
    pub events: Vec<(crate::event::EventKind, crate::event::EventCb<C>)>,
```
```rust
    pub draw_hook: Option<DrawHook<C>>,
    pub tick_hook: Option<TickHook<C>>,
```

`impl Node {` → `impl<C> Node<C> {`; `Node::new` signature:

```rust
    pub fn new(parent: Option<ObjRef>, rect: Rect, kind: alloc::boxed::Box<dyn crate::widgets::Widget<C>>) -> Self {
```

(body unchanged)

- [ ] **Step 4: Run the full suite**

Run: `cargo test -p qingui`
Expected: PASS. Everything downstream (`Ui` = `Ui<Color>`, widgets' `impl super::Widget for X` = `Widget<Color>`, render.rs's `Arena<Node>` = `Arena<Node<Color>>`) still lines up at the default.

- [ ] **Step 5: Commit**

```bash
git add qingui/src/widgets/mod.rs qingui/src/node.rs qingui/src/event.rs
git commit -m "refactor: genericize Widget, Node, and callback hooks over pixel format"
```

---

### Task 4: `Flush<C>` + generic `render` pipeline

**Files:**
- Modify: `qingui/src/display.rs` (whole file, 8 lines)
- Modify: `qingui/src/render.rs` (`render` 11-24, `render_area` 26-44, `render_chunk` 46-76, `draw_node` 82-138, `node_draw_info` 140-149, `abs_rect` 153-165, `resolved_style` 171-186, `node_state` 188-190)

**Interfaces:**
- Consumes: `PixelFormat` (Task 1), `Canvas<'_, C>` (Task 2), `Node<C>` (Task 3).
- Produces:
  - `pub trait Flush<C = Color> { fn flush(&mut self, area: Rect, pixels: &[C]); }`
  - `pub(crate) fn render<C: PixelFormat>(screen: ObjRef, arena: &mut Arena<Node<C>>, buf: &mut [C], dirty: &mut DirtyQueue, flush: &mut Option<Box<dyn Flush<C>>>, font: &'static MonoFont<'static>, time_ms: u64)`
  - `pub(crate) fn abs_rect<C>(arena: &Arena<Node<C>>, obj: ObjRef) -> Rect`
  - `pub(crate) fn resolved_style<C>(arena: &Arena<Node<C>>, obj: ObjRef, font: &'static MonoFont<'static>) -> ResolvedStyle`

- [ ] **Step 1: Genericize `Flush`**

Replace `qingui/src/display.rs` entirely:

```rust
use crate::geometry::{Color, Rect};

/// Callback used to push rendered pixels to the display driver.
///
/// `C` is the framebuffer pixel format (default: RGB888 `Color`).
pub trait Flush<C = Color> {
    /// `area` is a rectangle in absolute screen coordinates; `pixels` holds `area.w * area.h`
    /// pixels (row-major) in the framebuffer pixel format `C`.
    fn flush(&mut self, area: Rect, pixels: &[C]);
}
```

- [ ] **Step 2: Genericize `render.rs`**

Add `use crate::pixel::PixelFormat;` to the imports, then apply these exact signature transformations (bodies unchanged; the two `Canvas` literals infer `C` from `buf`):

- `pub(crate) fn render(` → `pub(crate) fn render<C: PixelFormat>(`; `arena: &mut Arena<Node>` → `arena: &mut Arena<Node<C>>`; `buf: &mut [Color]` → `buf: &mut [C]`; `flush: &mut Option<alloc::boxed::Box<dyn Flush>>` → `flush: &mut Option<alloc::boxed::Box<dyn Flush<C>>>`.
- Same transformation for `fn render_area` and `fn render_chunk` (`<C: PixelFormat>`, `Arena<Node<C>>`, `&mut [C]`, `Box<dyn Flush<C>>`).
- `fn draw_node(` → `fn draw_node<C: PixelFormat>(`; `arena: &mut Arena<Node>` → `arena: &mut Arena<Node<C>>`; `buf: &mut [Color]` → `buf: &mut [C]`.
- `fn node_draw_info(` → `fn node_draw_info<C>(`; `arena: &Arena<Node>` → `arena: &Arena<Node<C>>`.
- `pub(crate) fn abs_rect(` → `pub(crate) fn abs_rect<C>(`; `arena: &Arena<Node>` → `arena: &Arena<Node<C>>`.
- `pub(crate) fn resolved_style(` → `pub(crate) fn resolved_style<C>(`; `arena: &Arena<Node>` → `arena: &Arena<Node<C>>`.
- `fn node_state(` → `fn node_state<C>(`; `arena: &Arena<Node>` → `arena: &Arena<Node<C>>`.

The two `crate::canvas::Canvas { pixels: &mut buf[..len], ... }` literals (lines 59-63, 102-106) need no edit — `C` is inferred from `buf`.

- [ ] **Step 3: Run the full suite**

Run: `cargo test -p qingui`
Expected: PASS. The existing `render.rs` tests (`Rec`/`FakeFlush` at lines 203-211, `render_fixture` line 226) use `Arena<Node>` and `impl Flush for FakeFlush` — both resolve to the default `C = Color` and compile unchanged.

- [ ] **Step 4: Commit**

```bash
git add qingui/src/display.rs qingui/src/render.rs
git commit -m "refactor: genericize Flush and the render pipeline over pixel format"
```

---

### Task 5: `Ui<C>` + `anim.rs` + `layout.rs`

**Files:**
- Modify: `qingui/src/ui.rs` (struct 7-22, `impl Ui` 24+, `new` 38-45, `create_widget` 88, `set_flush` 343-345, `render` 829+; plus every internal `Arena<Node>`/`Node`/`Flush`/`RunningAnim`/`Anim`/`EventCb` mention)
- Modify: `qingui/src/anim.rs` (`Anim` 80-101, `Anim::on_done` 134-137, `RunningAnim` 141-144, `eval` 159)
- Modify: `qingui/src/layout.rs` (`layout_flex` 122, `layout_grid` 360, plus any private helpers taking `&mut Ui`)
- Modify: `qingui/src/widgets/obj.rs` (ONLY the `impl super::Widget for Manual` block — needed because `Ui::new` boxes a `Manual` into `Node<C>`)

**Interfaces:**
- Consumes: all of Tasks 1-4.
- Produces:
  - `pub struct Ui<C = Color>` with `arena: Arena<Node<C>>`, `flush: Option<Box<dyn Flush<C>>>`, `buf: Vec<C>`, `anims: Vec<RunningAnim<C>>`; `impl<C: PixelFormat> Ui<C>` for the entire method set.
  - `Ui::new(width, height, buf_rows) -> Ui<C>` (buffer initialized with `C::default()`).
  - `Ui::set_flush(&mut self, f: Box<dyn Flush<C>>)`.
  - `Ui::create_widget(&mut self, parent, w, h, widget: Box<dyn Widget<C>>) -> ObjRef`.
  - `pub struct Anim<C = Color>`, `pub(crate) struct RunningAnim<C>`, `pub(crate) fn eval<C>(a: &Anim<C>, ...) -> AnimEval`.
  - `pub fn layout_flex<C: PixelFormat>(ui: &mut Ui<C>, ...)`, `pub fn layout_grid<C: PixelFormat>(ui: &mut Ui<C>, ...)`.

- [ ] **Step 1: Genericize `anim.rs`**

- `pub struct Anim {` → `pub struct Anim<C = crate::geometry::Color> {`; field: `pub on_done: Option<Box<dyn FnMut(&mut Ui<C>>)>>`.
- `impl Anim {` → `impl<C> Anim<C> {`; `pub fn on_done(mut self, cb: impl FnMut(&mut Ui<C>) + 'static) -> Self`.
- `pub(crate) struct RunningAnim {` → `pub(crate) struct RunningAnim<C> {`; field `pub anim: Anim<C>`.
- `pub(crate) fn eval(a: &Anim, ...)` → `pub(crate) fn eval<C>(a: &Anim<C>, ...)`.
- The test module (180-228) compiles unchanged: `Anim::new` / `-> Anim` resolve to `Anim<Color>` via the default.

- [ ] **Step 2: Genericize `layout.rs` entry points**

```rust
pub fn layout_flex<C: crate::pixel::PixelFormat>(ui: &mut Ui<C>, container: ObjRef, f: &Flex, content: crate::geometry::Rect) {
```
```rust
pub fn layout_grid<C: crate::pixel::PixelFormat>(ui: &mut Ui<C>, container: ObjRef, g: &Grid, content: crate::geometry::Rect) {
```

Any private helpers in `layout.rs` that take `&mut Ui` get the same `<C: crate::pixel::PixelFormat>` + `&mut Ui<C>` treatment (the compiler lists them).

- [ ] **Step 3: Genericize `Ui`**

In `qingui/src/ui.rs`:

- Imports: add `use crate::geometry::Color;` and `use crate::pixel::PixelFormat;`.
- Struct:

```rust
pub struct Ui<C = Color> {
    pub(crate) arena: Arena<Node<C>>,
    screen: ObjRef,
    width: i32,
    height: i32,
    dirty: crate::dirty::DirtyQueue,
    flush: Option<alloc::boxed::Box<dyn crate::display::Flush<C>>>,
    buf: Vec<C>,
    time_ms: u64,
    anims: Vec<crate::anim::RunningAnim<C>>,
    group: Vec<ObjRef>,
    focused_idx: Option<usize>,
    pub(crate) layout_dirty: bool,
    modal: Option<ObjRef>,
    default_font: &'static embedded_graphics::mono_font::MonoFont<'static>,
}
```

- `impl Ui {` → `impl<C: PixelFormat> Ui<C> {`.
- `pub fn new(width: i32, height: i32, buf_rows: u32) -> Ui {` → `-> Ui<C> {`; buffer line 43:

```rust
        let buf = alloc::vec![C::default(); (width * buf_rows as i32).max(0) as usize];
```

- `pub fn set_flush(&mut self, f: alloc::boxed::Box<dyn crate::display::Flush>)` → `... Flush<C>>)`.
- `pub fn create_widget(&mut self, parent: ObjRef, w: i32, h: i32, widget: alloc::boxed::Box<dyn crate::widgets::Widget>)` → `... Widget<C>>)`.
- Every remaining occurrence the compiler flags inside `ui.rs`: `Arena<Node>` → `Arena<Node<C>>`, `Box<dyn crate::widgets::Widget>` → `... Widget<C>`, `crate::anim::Anim` (e.g. a `start_anim(&mut self, a: Anim)`-style method) → `Anim<C>`, `crate::event::EventCb` (e.g. `add_event_cb`) → `EventCb<C>`, `crate::node::{DrawHook, TickHook}` → `DrawHook<C>`/`TickHook<C>`. These are type-position-only edits; no method body changes.
- The `#[cfg(test)]` module in `ui.rs` (e.g. the test widget near line 1233 with `fn on_key(&mut self, _ui: &mut Ui, ...)`) compiles unchanged: `Ui` = `Ui<Color>`, `impl Widget for X` = `Widget<Color>`.

- [ ] **Step 4: Genericize `Manual`'s `Widget` impl in `obj.rs`**

`Ui::new` inserts `Node::new(None, ..., Box::new(crate::widgets::obj::Manual))` into `Arena<Node<C>>`, which requires `Manual: Widget<C>` for all `C`. In `qingui/src/widgets/obj.rs`, change the header of `Manual`'s widget impl to:

```rust
impl<C> super::Widget<C> for Manual {
```

If the impl overrides any methods whose signature names `Ui`/`Canvas` (e.g. `layout`), update those signatures to `&mut Ui<C>` / `&mut Canvas<'_, C>` as well; bodies unchanged. Do NOT genericize the rest of `obj.rs` yet — that is Task 7. (`ObjCfg`'s `impl WidgetCfg for ObjCfg` resolves to `WidgetCfg<Color>` with `ui: &mut Ui` = `Ui<Color>` and still compiles; leave it.)

- [ ] **Step 5: Run the full suite**

Run: `cargo test -p qingui && cargo check --examples --benches`
Expected: PASS. Examples/benches use `Ui::new` and `impl Flush for SimFlush` and must compile unchanged via the defaults.

- [ ] **Step 6: Commit**

```bash
git add qingui/src/ui.rs qingui/src/anim.rs qingui/src/layout.rs qingui/src/widgets/obj.rs
git commit -m "refactor: genericize Ui, Anim, and layout entry points over pixel format"
```

---

### Task 6: Genericize the builder scaffolding

**Files:**
- Modify: `qingui/src/widgets/builder.rs` (whole file, 109 lines)

**Interfaces:**
- Consumes: `Ui<C>`, `EventCb<C>` (Tasks 3-5).
- Produces:
  - `pub(crate) struct CommonBuilder<C = Color>` (`events: Vec<(EventKind, EventCb<C>)>`, manual `Default` impl)
  - `pub(crate) trait WidgetCfg<C = Color> { fn build(self, ui: &mut Ui<C>, parent: ObjRef, common: CommonBuilder<C>) -> ObjRef; fn default_style() -> Style; }`
  - `pub struct WidgetBuilder<Cfg, C = Color>` — `build(self, ui: &mut Ui<C>, parent)` where `Cfg: WidgetCfg<C>, C: PixelFormat`.
  - Widget constructors (Task 7) return `WidgetBuilder<XxxCfg, C>` with `C` inferred at `.build(&mut ui)` — this is what lets one fluent chain serve both `Ui<Color>` and `Ui<Rgb565>`.

- [ ] **Step 1: Rewrite `builder.rs` generics**

Apply these changes to `qingui/src/widgets/builder.rs`:

- Imports: add `use crate::geometry::Color;` and `use crate::pixel::PixelFormat;`.
- `CommonBuilder` — replace the `#[derive(Default)]` with a manual impl (so no `C: Default` bound is forced onto the struct):

```rust
/// Common fields shared by every widget builder.
pub(crate) struct CommonBuilder<C = Color> {
    pub size: Option<(i32, i32)>,
    pub style: Option<Style>,
    pub style_focused: Option<Style>,
    pub style_edited: Option<Style>,
    pub layout: Option<Layout>,
    pub pad: Option<(i32, i32, i32, i32)>,
    pub sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    pub aspect: Option<u32>,
    pub transition: Option<(u32, Easing)>,
    pub events: Vec<(EventKind, EventCb<C>)>,
}

impl<C> Default for CommonBuilder<C> {
    fn default() -> Self {
        Self {
            size: None, style: None, style_focused: None, style_edited: None,
            layout: None, pad: None, sizing: None, aspect: None,
            transition: None, events: Vec::new(),
        }
    }
}

impl<C: PixelFormat> CommonBuilder<C> {
    /// Applies the sizing/transition/events tail to an inserted node.
    /// Style defaults are widget-specific and stay in each `WidgetCfg::build`;
    /// `layout` is consumed by `ObjCfg::build` (it decides the widget kind) and
    /// never reaches this tail.
    pub fn apply_tail(self, ui: &mut Ui<C>, r: ObjRef) {
        if let Some(p) = self.pad { ui.set_pad(r, p); }
        if let Some((sw, sh)) = self.sizing { ui.set_sizing(r, sw, sh); }
        if let Some(a) = self.aspect { ui.set_aspect(r, Some(a)); }
        if let Some(t) = self.transition { ui.set_transition(r, Some(t)); }
        for (k, cb) in self.events { ui.add_event_cb(r, k, cb); }
    }
}
```

- `WidgetCfg`:

```rust
/// Widget-specific build logic: default size/style and post-insert setup.
pub(crate) trait WidgetCfg<C = Color> {
    fn build(self, ui: &mut Ui<C>, parent: ObjRef, common: CommonBuilder<C>) -> ObjRef;
    fn default_style() -> Style {
        Style::default()
    }
}
```

- `WidgetBuilder` — all setters keep their bodies; only the impl header and two methods change:

```rust
/// A fluent builder for any widget. Common setters live here once.
///
/// `C` is the target UI's pixel format; it is inferred at `build` from the
/// `Ui` being built into, so constructors do not name it.
pub struct WidgetBuilder<Cfg, C = Color> {
    pub(crate) common: CommonBuilder<C>,
    pub(crate) cfg: Cfg,
}

impl<Cfg, C> WidgetBuilder<Cfg, C> {
    // ... all setters unchanged, except:

    /// Registers an event callback.
    pub fn on(mut self, kind: EventKind, cb: EventCb<C>) -> Self {
        self.common.events.push((kind, cb));
        self
    }

    /// Modifies on top of the default style.
    pub fn style_with(mut self, f: impl FnOnce(Style) -> Style) -> Self
    where
        Cfg: WidgetCfg<C>,
    {
        self.common.style = Some(f(self.common.style.take().unwrap_or_else(Cfg::default_style)));
        self
    }

    /// Builds the widget into the parent node.
    #[allow(private_bounds)]
    pub fn build(self, ui: &mut Ui<C>, parent: ObjRef) -> ObjRef
    where
        Cfg: WidgetCfg<C>,
        C: PixelFormat,
    {
        Cfg::build(self.cfg, ui, parent, self.common)
    }
}
```

(The old `#[allow(private_bounds)]` on the whole impl block is removed; it now sits only on `build`. `style_with` names the `pub(crate)` `WidgetCfg` in a where-clause — if the compiler warns `private_bounds` there too, add the same allow to `style_with`.)

- [ ] **Step 2: No widget edits in this task**

Every widget file's `impl WidgetCfg for XxxCfg { fn build(self, ui: &mut Ui, parent: ObjRef, common: CommonBuilder) -> ObjRef }` resolves via the defaults (`WidgetCfg<Color>`, `Ui<Color>`, `CommonBuilder<Color>`) and compiles UNCHANGED. Task 7 genericizes them.

- [ ] **Step 3: Run the full suite**

Run: `cargo test -p qingui && cargo check --examples --benches`
Expected: PASS — all widget cfgs resolve at `C = Color`.

- [ ] **Step 4: Commit**

```bash
git add qingui/src/widgets/builder.rs
git commit -m "refactor: genericize widget builder scaffolding over pixel format"
```

---

### Task 7: Genericize all built-in widgets

**Files:** (mechanical sweep — the same 6 rules applied to each)
- Modify: `qingui/src/widgets/obj.rs` (rest of the file: `ObjCfg` and the layout-kind `Widget` impls)
- Modify: `qingui/src/widgets/arc.rs`, `bar.rs`, `button.rs`, `chart.rs`, `checkbox.rs`, `dropdown.rs`, `flexbox.rs`, `gridbox.rs`, `image.rs`, `itemlist.rs`, `label.rs`, `led.rs`, `list.rs`, `msgbox.rs`, `roller.rs`, `scrollview.rs`, `slider.rs`, `spinbox.rs`, `spinner.rs`, `switch.rs`, `table.rs`

**Interfaces:**
- Consumes: everything above.
- Produces: every built-in widget implements `Widget<C>` for all `C: PixelFormat`, so `Ui<Rgb565>` (or any other format) can host them.

**Transformation rules** (apply per file; add `use crate::pixel::PixelFormat;` to each touched file):

- **R1 — `Widget` impl:** `impl super::Widget for XxxState {` → `impl<C: PixelFormat> super::Widget<C> for XxxState {`; inside it, `c: &mut super::Canvas` → `c: &mut super::Canvas<'_, C>` and `&mut Ui` → `&mut Ui<C>` in `draw`/`layout`/`tick`/`on_key` signatures. Bodies unchanged.
- **R2 — draw helpers:** free/helper fns taking `d: &mut Canvas` → `fn draw_xxx<C: PixelFormat>(..., d: &mut Canvas<'_, C>, ...)`. Bodies unchanged.
- **R3 — ext traits:** `impl UiXxxExt for Ui {` → `impl<C: PixelFormat> UiXxxExt for Ui<C> {`. The `pub trait UiXxxExt` declarations themselves are unchanged (they never name `Ui`/`Canvas` in their signatures).
- **R4 — `&mut Ui` / `&Ui` free fns and builder methods:** e.g. `fn build(self, ui: &mut Ui, ...)`, `pub(crate) fn create(ui: &mut Ui, ...)`, `fn ensure_visible(ui: &mut Ui, ...)`, `pub(crate) fn text(ui: &Ui, ...)` → add `<C: PixelFormat>` to the fn and change the parameter to `ui: &mut Ui<C>` / `ui: &Ui<C>`.
- **R5 — `WidgetCfg` impls:** `impl WidgetCfg for XxxCfg {` → `impl<C: PixelFormat> WidgetCfg<C> for XxxCfg {`, with `fn build(self, ui: &mut Ui<C>, parent: ObjRef, common: CommonBuilder<C>) -> ObjRef`. Where `build` boxes the state into `Node::new(...)` / `ui.create_widget(...)`, no change is needed (type inference).
- **R6 — cfg constructors:** `pub fn new(...) -> WidgetBuilder<XxxCfg>` → `pub fn new<C: PixelFormat>(...) -> WidgetBuilder<XxxCfg, C>`; body (`WidgetBuilder { common: CommonBuilder::default(), cfg: ... }`) unchanged. `C` is inferred at the `.build(&mut ui)` call site.

Suggested commit grouping (each group ends with a green `cargo test -p qingui`):

- Group A: `obj.rs`, `flexbox.rs`, `gridbox.rs`, `msgbox.rs` (containers/layout kinds).
- Group B: `arc.rs`, `bar.rs`, `button.rs`, `checkbox.rs`, `image.rs`, `label.rs`, `led.rs`, `spinner.rs`, `switch.rs` (draw-only widgets).
- Group C: `chart.rs`, `dropdown.rs`, `itemlist.rs`, `list.rs`, `roller.rs`, `scrollview.rs`, `slider.rs`, `spinbox.rs`, `table.rs` (widgets with key handling / ext traits).

- [ ] **Step 1: Apply R1-R6 to Group A**

Run: `cargo test -p qingui`
Expected: PASS.

- [ ] **Step 2: Commit Group A**

```bash
git add qingui/src/widgets/obj.rs qingui/src/widgets/flexbox.rs qingui/src/widgets/gridbox.rs qingui/src/widgets/msgbox.rs
git commit -m "refactor: genericize container widgets over pixel format"
```

- [ ] **Step 3: Apply R1-R6 to Group B**

Run: `cargo test -p qingui`
Expected: PASS.

- [ ] **Step 4: Commit Group B**

```bash
git add qingui/src/widgets/arc.rs qingui/src/widgets/bar.rs qingui/src/widgets/button.rs qingui/src/widgets/checkbox.rs qingui/src/widgets/image.rs qingui/src/widgets/label.rs qingui/src/widgets/led.rs qingui/src/widgets/spinner.rs qingui/src/widgets/switch.rs
git commit -m "refactor: genericize draw-only widgets over pixel format"
```

- [ ] **Step 5: Apply R1-R6 to Group C**

Run: `cargo test -p qingui`
Expected: PASS.

- [ ] **Step 6: Commit Group C**

```bash
git add qingui/src/widgets/chart.rs qingui/src/widgets/dropdown.rs qingui/src/widgets/itemlist.rs qingui/src/widgets/list.rs qingui/src/widgets/roller.rs qingui/src/widgets/scrollview.rs qingui/src/widgets/slider.rs qingui/src/widgets/spinbox.rs qingui/src/widgets/table.rs
git commit -m "refactor: genericize interactive widgets over pixel format"
```

Notes for the executor:
- `cargo check -p qingui` after each file pinpoints the next site; the rules above cover every pattern present in the codebase today (verified by grep: `&mut Ui`, `&mut Canvas`, `impl super::Widget for`, `impl Ui*Ext for Ui`, `WidgetCfg for`, `-> WidgetBuilder<`).
- Existing `#[cfg(test)]` modules inside widget files use the default `Color` and must keep passing unmodified.

---

### Task 8: End-to-end `Ui<Rgb565>` integration test

**Files:**
- Create: `qingui/tests/rgb565.rs`

**Interfaces:**
- Consumes: the full generic stack (Tasks 1-7): `Ui::<Rgb565>::new`, `impl Flush<Rgb565>`, `ButtonCfg::new(...).build(&mut ui, ...)` (C inferred as `Rgb565`), `Canvas<'_, Rgb565>` as `DrawTarget<Color = Rgb565>`.
- Produces: proof that the feature works end to end. No new API.

- [ ] **Step 1: Write the test**

Create `qingui/tests/rgb565.rs`:

```rust
//! End-to-end: qingui renders directly into an Rgb565 framebuffer, and e-g
//! ecosystem code draws into a qingui canvas using the device-native color type.

use std::cell::RefCell;
use std::rc::Rc;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle};

use qingui::canvas::Canvas;
use qingui::display::Flush;
use qingui::pixel::PixelFormat;
use qingui::style::Style;
use qingui::widgets::button::ButtonCfg;
use qingui::{Color, Rect, Ui};

struct Rec(Rc<RefCell<Vec<(Rect, Vec<Rgb565>)>>>);

impl Flush<Rgb565> for Rec {
    fn flush(&mut self, area: Rect, pixels: &[Rgb565]) {
        self.0.borrow_mut().push((area, pixels.to_vec()));
    }
}

fn render_solid(bg: Color) -> Vec<(Rect, Vec<Rgb565>)> {
    let mut ui = Ui::<Rgb565>::new(40, 20, 20);
    let mut s = Style::default();
    s.bg_color = Some(bg);
    s.bg_opa = Some(255);
    let screen = ui.screen();
    ui.set_style(screen, s);
    let rec = Rc::new(RefCell::new(Vec::new()));
    ui.set_flush(Box::new(Rec(rec.clone())));
    ui.render();
    Rc::try_unwrap(rec).unwrap().into_inner()
}

#[test]
fn ui_rgb565_flushes_device_native_pixels() {
    let chunks = render_solid(Color::RED);
    let total: usize = chunks.iter().map(|(_, px)| px.len()).sum();
    assert_eq!(total, 40 * 20);
    assert!(chunks.iter().all(|(_, px)| px.iter().all(|&p| p == Rgb565::RED)));
}

#[test]
fn ui_rgb565_quantizes_like_color_helpers() {
    let bg = Color::rgb(80, 140, 255);
    let chunks = render_solid(bg);
    let expected = Rgb565::from_color(bg);
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|(_, px)| px.iter().all(|&p| p == expected)));
}

#[test]
fn ui_rgb565_hosts_builtin_widgets() {
    let mut ui = Ui::<Rgb565>::new(80, 40, 40);
    let screen = ui.screen();
    ButtonCfg::new("OK").size(40, 20).build(&mut ui, screen);
    let rec = Rc::new(RefCell::new(Vec::new()));
    ui.set_flush(Box::new(Rec(rec.clone())));
    ui.render();
    let chunks = rec.borrow();
    assert!(!chunks.is_empty());
    let total: usize = chunks.iter().map(|(_, px)| px.len()).sum();
    assert_eq!(total, 80 * 40);
}

#[test]
fn eg_primitives_draw_into_rgb565_canvas() {
    let mut buf = [Rgb565::BLACK; 100];
    let mut d = Canvas { pixels: &mut buf[..], area: Rect::new(0, 0, 10, 10), stride: 10 };
    Circle::new(Point::new(0, 0), 5)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
        .draw(&mut d)
        .unwrap();
    assert_eq!(d.pixels[1 * 10 + 1], Rgb565::GREEN); // pixel (1, 1)
}
```

(If executed before Tasks 1-7 are complete, this file fails to compile — that is its "failing test" state. It is the acceptance gate for the whole feature.)

- [ ] **Step 2: Run the test**

Run: `cargo test -p qingui --test rgb565`
Expected: PASS. Failure modes and remedies:
- `ButtonCfg::new("OK")...build(&mut ui, ...)` does not infer `C = Rgb565` → check Task 7 R6 was applied to `button.rs`.
- Flushed pixels show the default theme color instead of the set bg → the test's `Style` handling, not the feature; debug via `render_solid` only.
- `Rc::try_unwrap` panics → a second `Rc` clone leaked; clone into `Rec` exactly as written.

- [ ] **Step 3: Commit**

```bash
git add qingui/tests/rgb565.rs
git commit -m "test: end-to-end Rgb565 rendering and e-g interop"
```

---

### Task 9: Full verification

- [ ] **Step 1: Full workspace test run**

Run: `cargo test --workspace`
Expected: PASS (qingui + qingui-codegen).

- [ ] **Step 2: All targets check**

Run: `cargo check --workspace --examples --benches --tests`
Expected: PASS, no new warnings versus the pre-refactor baseline.

- [ ] **Step 3: no_std target build**

Run: `cargo build -p qingui --target thumbv7em-none-eabihf`
Expected: PASS. If the target is not installed ("can't find crate for `core`"), STOP and ask the user before running `rustup target add thumbv7em-none-eabihf` (installs outside the working directory).

- [ ] **Step 4: Bench sanity**

Run: `cargo build -p qingui --benches`
Expected: benches build. Optionally run `cargo bench -p qingui --bench time` and note before/after numbers in the final summary (default `Color` path must not regress meaningfully — the conversion is identity).

---

## Self-Review Notes (already applied)

- **Spec coverage:** pixel trait (T1), Canvas generic (T2), Widget/Node/hooks (T3), Flush+render (T4), Ui/anim/layout (T5), builder (T6), widgets (T7), e2e test (T8), verification (T9). The spec's default-parameter compatibility requirement is covered by Global Constraints + T5 Step 5's examples/benches check.
- **Task order:** `render.rs` names `Node<C>`, so the `Widget`/`Node` task (T3) lands before the `Flush`/`render` task (T4); every commit compiles in this order.
- **Type consistency:** `PixelFormat::{to_color, from_color}`, `Canvas<'a, C = Color>`, `Flush<C = Color>`, `Widget<C = Color>`, `Node<C = Color>`, `Ui<C = Color>`, `EventCb/DrawHook/TickHook<C = Color>`, `Anim<C = Color>`, `RunningAnim<C>`, `CommonBuilder<C = Color>`, `WidgetCfg<C = Color>`, `WidgetBuilder<Cfg, C = Color>` — used consistently across tasks.
- **Known inference risk:** `WidgetBuilder`'s `C` is inferred at `.build(&mut ui)`; T8's `ui_rgb565_hosts_builtin_widgets` is the acceptance test for that inference.
