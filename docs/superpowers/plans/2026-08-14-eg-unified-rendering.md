# e-g Unified Rendering (No Alpha / No AA) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove qingui's custom rasterizer (AA coverage math), alpha blending, and the whole `opa` chain; make every `Canvas` drawing method delegate to embedded-graphics primitives; align data structures with e-g.

**Architecture:** `Canvas<'a, C = Color>` stays the single `DrawTarget<Color = C>` (framebuffer + clip + coordinate offset + PFB chunking all unchanged), but its drawing methods become thin shells that build e-g primitives and draw them into `self` via `DrawTargetExt::clipped`. Terminal write paths (`put`, row batch fill, `fill_solid`/`fill_contiguous` overrides) never delegate — no recursion. Style drops `bg_opa`/`opa`; background semantics become `bg_color: Option<Color>` (`None` = don't paint). True-transparency features (fade anim, node opa multiplier, list ghost) are removed outright.

**Tech Stack:** Rust edition 2024, `no_std` + `alloc`, embedded-graphics 0.8.2 (default-features = false), cargo test.

**Spec:** `docs/superpowers/specs/2026-08-14-eg-unified-rendering-design.md`

## Global Constraints

- Crate `qingui` is `#![no_std]` with `extern crate alloc`; no new dependencies.
- embedded-graphics 0.8.2. Confirmed API signatures (registry source): `Circle::new(top_left: Point, diameter: u32)`, `Line::new(start: Point, end: Point)`, `Arc::new(top_left: Point, diameter: u32, start_angle: Angle, sweep_angle: Angle)`, `RoundedRectangle::new(rectangle: Rectangle, corners: CornerRadii)`, `CornerRadii::new(radius: Size)`, `Angle::from_degrees(f32)`, `PrimitiveStyle::with_fill(c)` / `PrimitiveStyle::with_stroke(c, width: u32)`, styling via `.into_styled(...)`, `DrawTargetExt::clipped(&mut target, &Rectangle)`.
- This is a **breaking** refactor (0.2.0): NO compatibility shims, NO deprecated leftovers. Removed APIs disappear cleanly.
- Every task ends with `cargo test -p qingui` fully green (and `cargo check --examples --benches` clean). Intermediate red states are not committed.
- Recalibrated visual-test assertions must each carry a one-line justification (what changed and why the new value is correct) in the implementer's report; silent snapshot updates are a review finding.
- Code comments and commit messages in English (Conventional Commits). Commits are local only; **never `git push`**. Per `AGENTS.md`, the controller asks the user before commit batches.
- The `PixelFormat` generic (`Canvas<'a, C>`, `Flush<C>`, `Ui<C>`, `Rgb565` path) from the previous refactor is preserved intact. `Rgb565` conversion stays bit-consistent with `Color::to_rgb565`/`from_rgb565` while `blit565` exists.

---

### Task 1: `geometry.rs` — `Point` → e-g, `Rect` ↔ `Rectangle` conversions

**Files:**
- Modify: `qingui/src/geometry.rs` (replace the `Point` struct at lines 3-10; append conversions at end)
- Test: `qingui/src/geometry.rs` (`#[cfg(test)] mod tests` appended)

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces:
  - `qingui::Point` is now `embedded_graphics::geometry::Point` (same `pub x: i32, y: i32` fields; `Point::new(x, y)` is a const fn; `Default`/`PartialEq`/`Debug` all exist) — all existing `crate::geometry::Point` usage compiles unchanged.
  - `impl From<Rect> for embedded_graphics::primitives::Rectangle` (clamps negative w/h to 0)
  - `impl From<embedded_graphics::primitives::Rectangle> for Rect`
  - `Color::blend` still exists (Task 3 removes it); canvas.rs's local `from_eg_rect` and draw.rs's `eg_rect` stay until Task 3.

- [ ] **Step 1: Write the failing tests**

Append to `qingui/src/geometry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::primitives::Rectangle as EgRect;

    #[test]
    fn point_is_eg_point() {
        // Compile-time proof: qingui::Point IS embedded-graphics' Point.
        let p: Point = embedded_graphics::geometry::Point::new(3, 4);
        assert_eq!((p.x, p.y), (3, 4));
        let q = Point::new(3, 4);
        assert_eq!(p, q);
    }

    #[test]
    fn rect_eg_roundtrip() {
        let r = Rect::new(2, 3, 10, 20);
        let eg: EgRect = r.into();
        assert_eq!(eg.top_left, Point::new(2, 3));
        assert_eq!((eg.size.width, eg.size.height), (10, 20));
        let back: Rect = eg.into();
        assert_eq!(back, r);
    }

    #[test]
    fn rect_to_eg_clamps_negative_size() {
        let eg: EgRect = Rect::new(5, 5, -3, 7).into();
        assert_eq!((eg.size.width, eg.size.height), (0, 7));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qingui geometry`
Expected: FAIL — `Point` has no `new` (it's the local struct), and the `From` impls don't exist.

- [ ] **Step 3: Implement**

In `qingui/src/geometry.rs`:

1. Replace lines 1-10 (`use embedded_graphics::pixelcolor::RgbColor;` + the `Point` struct) with:

```rust
use embedded_graphics::pixelcolor::RgbColor;

/// A 2D point in screen coordinates (re-exported from embedded-graphics).
pub use embedded_graphics::geometry::Point;
```

2. Append at the end of the file:

```rust
impl From<Rect> for embedded_graphics::primitives::Rectangle {
    fn from(r: Rect) -> Self {
        embedded_graphics::primitives::Rectangle::new(
            Point::new(r.x, r.y),
            embedded_graphics::geometry::Size::new(r.w.max(0) as u32, r.h.max(0) as u32),
        )
    }
}

impl From<embedded_graphics::primitives::Rectangle> for Rect {
    fn from(r: embedded_graphics::primitives::Rectangle) -> Self {
        Rect::new(r.top_left.x, r.top_left.y, r.size.width as i32, r.size.height as i32)
    }
}
```

- [ ] **Step 4: Run the full suite**

Run: `cargo test -p qingui && cargo check --examples --benches`
Expected: PASS — `Point` swap is source-compatible (field literals `Point { x, y }`, `.x`/`.y`, `Point::new`, `Default`, `PartialEq`, `Debug` all exist on e-g's `Point`).

- [ ] **Step 5: Commit**

```bash
git add qingui/src/geometry.rs
git commit -m "refactor: replace Point with e-g Point, add Rect<->Rectangle conversions"
```

---

### Task 2: Remove the opacity system (style/anim/ui/render/widgets)

**Files:**
- Modify: `qingui/src/style.rs` (Style fields 9-10 & 21-22, builders 36-39, merge 60/66, ResolvedStyle 73-105, resolve 109-130, theme_base 135-137, theme_label 149-152)
- Modify: `qingui/src/render.rs` (`node_draw_info` tuple, `ap()` closure ~100-101, bg block ~108-111, border call ~119-120, test helpers 224/346)
- Modify: `qingui/src/widgets/mod.rs` (delete `WidgetCtx::ap` 45-50)
- Modify: `qingui/src/anim.rs` (delete `AnimProp::Opa` variant 16-17)
- Modify: `qingui/src/ui.rs` (delete `set_opa` ~736, delete `AnimProp::Opa` match arm ~818-824)
- Modify: `qingui/src/widgets/list.rs` (remove the ghost effect: `fx.ghost` field 51, `Ghost` struct, fade draw block ~143-, ghost set ~230, ghost branches in `fx_active`/`tick` ~63-86, comment ~328)
- Modify: every widget file that references `bg_opa`, `ctx.ap(`, or `ap(` (checkbox, led, spinner, table, image, itemlist, scrollview, msgbox, slider, arc, dropdown, …)
- Modify: tests and examples that reference `set_opa`, `AnimProp::Opa`, `bg_opa`, `style.opa`, or list ghost (find with grep, see Step 4)
- Test: existing suite is the test

**Interfaces:**
- Consumes: Task 1 (unchanged canvas signatures — canvas still takes `opa: u8` everywhere in this task).
- Produces:
  - `Style { bg_color: Option<Color>, border_color, border_width, radius, text_color, font }` — no `bg_opa`, no `opa`. `bg_color: None` now means "no background" at resolve time (NOT "inherit").
  - `ResolvedStyle { bg_color: Option<Color>, border_color, border_width, radius, text_color, font }` — no `bg_opa`, no `opa`; `ResolvedStyle::default().bg_color == None`.
  - `resolve(base, overlay, font)`: `bg_color` = overlay's `Some` else base's (no default fill).
  - No `WidgetCtx::ap`, no `Ui::set_opa`, no `AnimProp::Opa`, no list ghost. Canvas calls pass literal `255` (or the former base value) for `opa` — Task 3 deletes the parameter.
  - `theme_base()` = `Style::new().text_color(Color::WHITE).radius(4)`; `theme_label()` = `theme_base()` (doc: "no bg_color = transparent background").

- [ ] **Step 1: style.rs surgery**

Apply exactly:

- Delete `Style.bg_opa` (field + doc) and `Style.opa` (field + doc); delete the `bg_opa` builder method; delete the two `merge` lines for them.
- `ResolvedStyle`: delete `bg_opa`/`opa` fields; change `pub bg_color: Color` → `pub bg_color: Option<Color>`; `Default` impl: `bg_color: None`, drop the two deleted fields.
- `resolve()`: delete the `pick_u8` closure; the returned struct becomes:

```rust
    ResolvedStyle {
        bg_color: pick(overlay, |s| s.bg_color),
        border_color: pick(overlay, |s| s.border_color).unwrap_or(d.border_color),
        border_width: pick_i(overlay, |s| s.border_width).unwrap_or(d.border_width),
        radius: pick_i(overlay, |s| s.radius).unwrap_or(d.radius),
        text_color: pick(overlay, |s| s.text_color).unwrap_or(d.text_color),
        font: overlay.and_then(|s| s.font).or(base.font).unwrap_or(default),
    }
```

(`d` keeps serving the remaining fields; `d.bg_color` is `None`.)

- `theme_base()`: `Style::new().text_color(Color::WHITE).radius(4)`.
- `theme_label()`: `theme_base()` with the doc comment updated to "Default style for a label (no bg_color = transparent background)".

- [ ] **Step 2: render.rs surgery**

- `node_draw_info`: drop `resolved.opa` from the tuple — new return type `Option<(Rect, Flag, ResolvedStyle)>`; update the two callers (`draw_node`).
- `draw_node`: delete the `ap` closure; bg block becomes:

```rust
        if let Some(bg) = resolved.bg_color {
            d.fill_rounded(abs, resolved.radius, bg, 255, clip);
        }
```

- Border call: `d.draw_border(abs, resolved.border_width, resolved.radius, resolved.border_color, 255, clip);`
- The test-module helpers at ~224/~346 (`s.bg_opa = Some(255)` / `e.bg_opa = Some(255)`): delete those lines (a `Some(bg_color)` now implies painting).

- [ ] **Step 3: widgets + anim + ui + list ghost**

- `widgets/mod.rs`: delete `WidgetCtx::ap` (lines 45-50).
- Every widget file: `ctx.ap(255)` → `255`; any `ap(x)`/`ctx.ap(x)` with a non-literal → `x`; `bg_opa: Some(0)` in style literals → delete the field; guards like `if s.bg_opa.is_none() { s.bg_opa = Some(0); }` → delete (a style without `bg_color` is transparent; a highlight overlay with `bg_color: Some(..)` paints). **Read each site's comment before deleting** — itemlist.rs:62/108 document exactly this contract; update the comments to the new semantics ("the highlight sets bg_color explicitly; the item base leaves it None").
- `anim.rs`: delete the `Opa` variant and its doc line.
- `ui.rs`: delete `set_opa`; delete the `AnimProp::Opa =>` match arm.
- `list.rs`: remove the ghost effect entirely — the `Ghost` struct, `fx.ghost` field, the fade-draw block, the `self.fx.ghost = Some(...)` assignment (item deletion becomes immediate shift-up + dirty), and the ghost branches in `fx_active`/`tick`/`needs_redraw`-style checks. Adjust comments (line ~47's doc and ~328's comment lose the ghost mention).

- [ ] **Step 4: sweep tests/examples**

Run: `grep -rn "set_opa\|AnimProp::Opa\|bg_opa\|\.opa\b" qingui/tests qingui/examples qingui/benches`
For each hit: delete the usage or translate to the new semantics (e.g. a fade-animation test switches to animating `AnimProp::X`; a `bg_opa(0)` test style drops the line). Do not change what any test *means* beyond the removed feature.
`qingui/tests/list_fx.rs`: ghost assertions → rewrite to assert immediate disappearance of the deleted item.

- [ ] **Step 5: Run the full suite**

Run: `cargo test -p qingui && cargo check --workspace --examples --benches --tests`
Expected: PASS. Rendering output is pixel-identical everywhere except where the ghost/opa features were removed — any other assertion failure means a migration mistake; investigate, don't recalibrate.

- [ ] **Step 6: Commit**

```bash
git add -A qingui
git commit -m "refactor: remove the opacity system (style opa, node opa, fade anim, list ghost)"
```

---

### Task 3: `Canvas` delegation to e-g primitives + delete `draw.rs` + strip `opa` params

**Files:**
- Rewrite: `qingui/src/canvas.rs` (method bodies; signatures lose `opa: u8`)
- Delete: `qingui/src/draw.rs`
- Modify: `qingui/src/lib.rs` (remove `pub(crate) mod draw;`)
- Modify: every call site of a `Canvas` drawing method (widgets, render.rs, tests) — drop the `opa` argument
- Modify: `qingui/tests/rgb565.rs` (delete `rgb565_blend_roundtrips_through_rgb888` — blend is gone)
- Modify: `qingui/tests/canvas.rs` (drop opa args; recalibrate rasterization-sensitive assertions)
- Test: `qingui/src/canvas.rs` test module (rewritten — see Step 2)

**Interfaces:**
- Consumes: Tasks 1-2 (`Point` is e-g `Point`; `From<Rect> for Rectangle`; no `ap()` anywhere).
- Produces (final Canvas API — all later code relies on these signatures):

```rust
impl<C: PixelFormat> Canvas<'_, C> {
    pub fn clear(&mut self, c: Color);
    pub(crate) fn put(&mut self, x: i32, y: i32, c: Color);        // opaque, bounds-checked
    pub fn fill_rect(&mut self, r: Rect, c: Color, clip: Rect);
    pub fn blit565(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u8], clip: Rect);
    pub fn fill_rounded(&mut self, r: Rect, radius: i32, c: Color, clip: Rect);
    pub fn draw_border(&mut self, r: Rect, width: i32, radius: i32, c: Color, clip: Rect);
    pub fn draw_text(&mut self, pos: Point, font: &'static MonoFont<'static>, s: &str, c: Color, clip: Rect);
    pub fn fill_circle(&mut self, center: Point, radius: i32, c: Color, clip: Rect);
    pub fn draw_circle(&mut self, center: Point, radius: i32, width: i32, c: Color, clip: Rect);
    pub fn draw_arc(&mut self, center: Point, radius: i32, width: i32, start_deg: i32, end_deg: i32, c: Color, clip: Rect);
    pub fn draw_line(&mut self, p1: Point, p2: Point, width: i32, c: Color, clip: Rect);
}
```

- [ ] **Step 1: Rewrite canvas.rs**

Structure after the rewrite (write it in this order):

1. Imports: drop everything from `crate::draw`; add

```rust
use crate::geometry::{Color, Point, Rect};
use crate::pixel::PixelFormat;
use embedded_graphics::draw_target::DrawTargetExt;
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::primitives::{Circle, CornerRadii, Line, PrimitiveStyle, RoundedRectangle};
use embedded_graphics::geometry::{Angle, Size};
```

2. Struct, `clear`, `put` (opaque only), `put_fast` (opaque only): keep; delete the `opa > 0` blend branches; delete `put_clipped` (no longer used — e-g `clipped` handles it).

3. Private terminal batch fill (no delegation — this is the floor all paths land on):

```rust
    /// Batch-fills the pre-clipped rect rows (terminal write path, no delegation).
    fn fill_rows(&mut self, r: Rect, c: C) {
        let area_x = self.area.x;
        let area_y = self.area.y;
        let stride = self.stride;
        let w = r.w as usize;
        for y in r.y..r.bottom() {
            let row = ((y - area_y) * stride + (r.x - area_x)) as usize;
            self.pixels[row..row + w].fill(c);
        }
    }
```

4. `fill_rect` keeps its current fast path (it IS what e-g's `fill_solid` funnels into — do not delegate, that would recurse):

```rust
    pub fn fill_rect(&mut self, r: Rect, c: Color, clip: Rect) {
        let Some(r) = r.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        self.fill_rows(r, C::from_color(c));
    }
```

5. `blit565`: drop `opa`; inner write is `self.put(px, py, Color::from_rgb565(v));`. Rest unchanged.

6. Delegating methods — the shared pattern is: compute/clamp, convert the color once (`C::from_color(c)`), then draw the primitive into `DrawTargetExt::clipped(&mut *self, &clip.into())`. Write them exactly:

```rust
    /// Filled rounded rectangle (aliased edges — no AA).
    pub fn fill_rounded(&mut self, r: Rect, radius: i32, c: Color, clip: Rect) {
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        if radius == 0 {
            self.fill_rect(r, c, clip);
            return;
        }
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = RoundedRectangle::new(r.into(), CornerRadii::new(Size::new(radius as u32, radius as u32)))
            .into_styled(PrimitiveStyle::with_fill(C::from_color(c)))
            .draw(&mut t);
    }

    /// Border inside `r` (aliased). `width <= 0` draws nothing.
    pub fn draw_border(&mut self, r: Rect, width: i32, radius: i32, c: Color, clip: Rect) {
        if width <= 0 {
            return;
        }
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        let style = PrimitiveStyle::with_stroke(C::from_color(c), width as u32);
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        if radius == 0 {
            let _ = embedded_graphics::primitives::Rectangle::from(r).into_styled(style).draw(&mut t);
        } else {
            let _ = RoundedRectangle::new(r.into(), CornerRadii::new(Size::new(radius as u32, radius as u32)))
                .into_styled(style)
                .draw(&mut t);
        }
    }

    /// Filled circle (aliased).
    pub fn fill_circle(&mut self, center: Point, radius: i32, c: Color, clip: Rect) {
        if radius <= 0 {
            return;
        }
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = Circle::new(center - Point::new(radius, radius), (radius * 2) as u32)
            .into_styled(PrimitiveStyle::with_fill(C::from_color(c)))
            .draw(&mut t);
    }

    /// Circle outline with stroke `width` (aliased).
    pub fn draw_circle(&mut self, center: Point, radius: i32, width: i32, c: Color, clip: Rect) {
        if radius <= 0 || width <= 0 {
            return;
        }
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = Circle::new(center - Point::new(radius, radius), (radius * 2) as u32)
            .into_styled(PrimitiveStyle::with_stroke(C::from_color(c), width as u32))
            .draw(&mut t);
    }

    /// Arc (LVGL angle convention: 0 deg at 3 o'clock, positive clockwise), stroke `width`,
    /// square ends (e-g arcs have no round caps).
    pub fn draw_arc(&mut self, center: Point, radius: i32, width: i32, start_deg: i32, end_deg: i32, c: Color, clip: Rect) {
        if radius <= 0 || width <= 0 || end_deg <= start_deg {
            return;
        }
        let arc = embedded_graphics::primitives::Arc::new(
            center - Point::new(radius, radius),
            (radius * 2) as u32,
            Angle::from_degrees(start_deg as f32),
            Angle::from_degrees((end_deg - start_deg) as f32),
        );
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = arc
            .into_styled(PrimitiveStyle::with_stroke(C::from_color(c), width as u32))
            .draw(&mut t);
    }

    /// Thick line with round caps (width >= 2 adds a circle cap at each end); 1px is a plain e-g line.
    pub fn draw_line(&mut self, p1: Point, p2: Point, width: i32, c: Color, clip: Rect) {
        if width <= 0 {
            return;
        }
        let c = C::from_color(c);
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = Line::new(p1, p2)
            .into_styled(PrimitiveStyle::with_stroke(c, width as u32))
            .draw(&mut t);
        if width >= 2 {
            let off = Point::new(-width / 2, -width / 2);
            let cap = PrimitiveStyle::with_fill(c);
            let _ = Circle::new(p1 + off, width as u32).into_styled(cap).draw(&mut t);
            let _ = Circle::new(p2 + off, width as u32).into_styled(cap).draw(&mut t);
        }
    }

    /// Mono text (top-baseline), clipped. No background: only glyph pixels are drawn.
    pub fn draw_text(&mut self, pos: Point, font: &'static MonoFont<'static>, s: &str, c: Color, clip: Rect) {
        let style = embedded_graphics::mono_font::MonoTextStyle::new(font, C::from_color(c));
        let mut t = DrawTargetExt::clipped(&mut *self, &clip.into());
        let _ = embedded_graphics::text::Text::with_baseline(
            s,
            pos,
            style,
            embedded_graphics::text::Baseline::Top,
        )
        .draw(&mut t);
    }
```

(The old `EgTarget` shim and `draw_text_opa` are deleted — e-g mono text with no background color emits only glyph pixels, so text draws directly in `C`.)

7. `DrawTarget` impl: `draw_iter` keeps calling `self.put(p.x, p.y, color.to_color())`. `fill_solid` and the new `fill_contiguous` override land on the terminal paths (no recursion):

```rust
    fn fill_solid(&mut self, area: &embedded_graphics::primitives::Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let r: Rect = (*area).into();
        if let Some(r) = r.intersect(&self.area) {
            self.fill_rows(r, color);
        }
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &embedded_graphics::primitives::Rectangle, mut colors: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = Self::Color>,
    {
        // Row-wise writes without per-pixel bounds checks (e-g's default falls back to draw_iter).
        let r: Rect = (*area).into();
        let Some(r) = r.intersect(&self.area) else {
            // Consume the iterator contract: e-g guarantees exactly area-pixels; nothing to draw.
            for _ in colors {}
            return Ok(());
        };
        let stride = self.stride;
        let area_x = self.area.x;
        let area_y = self.area.y;
        let full_w = r.w as usize;
        for y in r.y..r.bottom() {
            let row = ((y - area_y) * stride + (r.x - area_x)) as usize;
            // `colors` yields pixels for the UNclipped area row by row; skip the clipped-off prefix.
            let skip = (r.x - area.x) as usize;
            let take = full_w;
            for (i, px) in colors.by_ref().skip(skip).take(take).enumerate() {
                self.pixels[row + i] = px;
            }
            // Discard the clipped-off suffix of this source row.
            let consumed = skip + take;
            let row_total = (area.size.width as i32) as usize;
            for _ in colors.by_ref().take(row_total.saturating_sub(consumed)) {}
        }
        Ok(())
    }
```

**Note for the implementer:** `fill_contiguous`'s iterator contract (row-major over the *unclipped* `area`, exactly `w*h` items) is subtle — verify against e-g 0.8.2's default implementation in the registry source (`src/draw_target/mod.rs`) before finalizing; if the contract differs, simplify to the default per-pixel loop (correctness first, note it in the report).

8. Delete `qingui/src/draw.rs`; remove `pub(crate) mod draw;` from `qingui/src/lib.rs`.

9. `Geometry`/`Dimensions` impl and the `blit565`/geometry helpers stay; canvas.rs's local `from_eg_rect` helper is replaced by the `From` impls from Task 1 (delete the local fn).

- [ ] **Step 2: Rewrite the canvas unit tests**

Replace the test module appended in the PixelFormat task with property-based assertions that don't guess e-g's rasterization grid:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{Circle as EgCircle, PrimitiveStyle as EgStyle};

    fn canvas565(buf: &mut [Rgb565]) -> Canvas<'_, Rgb565> {
        Canvas { pixels: buf, area: Rect::new(0, 0, 10, 10), stride: 10 }
    }

    #[test]
    fn rgb565_opaque_fill_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(255, 0, 0), clip);
        assert!(d.pixels.iter().all(|&p| p == Rgb565::RED));
    }

    #[test]
    fn fill_rect_respects_clip() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::WHITE, Rect::new(2, 2, 4, 4));
        assert_eq!(d.pixels[2 * 10 + 2], Color::WHITE);
        assert_eq!(d.pixels[0], Color::BLACK);
        assert_eq!(d.pixels[6 * 10 + 6], Color::BLACK); // just outside the clip
    }

    #[test]
    fn fill_rounded_fills_center_and_spares_corners() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rounded(Rect::new(1, 1, 8, 8), 3, Color::WHITE, clip);
        assert_eq!(d.pixels[5 * 10 + 5], Color::WHITE); // center
        assert_eq!(d.pixels[1 * 10 + 1], Color::BLACK); // rounded-off corner
        assert_eq!(d.pixels[0], Color::BLACK);          // outside the rect
    }

    #[test]
    fn fill_circle_covers_center_not_corner() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_circle(Point::new(5, 5), 3, Color::WHITE, clip);
        assert_eq!(d.pixels[5 * 10 + 5], Color::WHITE);
        assert_eq!(d.pixels[0], Color::BLACK);
    }

    #[test]
    fn draw_line_hits_endpoints() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.draw_line(Point::new(1, 1), Point::new(8, 8), 1, Color::WHITE, clip);
        assert_eq!(d.pixels[1 * 10 + 1], Color::WHITE);
        assert_eq!(d.pixels[8 * 10 + 8], Color::WHITE);
        assert_eq!(d.pixels[1 * 10 + 8], Color::BLACK); // off the diagonal
    }

    #[test]
    fn draw_border_paints_edges_not_center() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.draw_border(Rect::new(2, 2, 6, 6), 1, 0, Color::WHITE, clip);
        assert_eq!(d.pixels[2 * 10 + 2], Color::WHITE); // top-left edge
        assert_eq!(d.pixels[5 * 10 + 5], Color::BLACK); // center untouched
    }

    #[test]
    fn draw_arc_paints_ring_pixels() {
        let mut buf = [Color::BLACK; 100];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.draw_arc(Point::new(5, 5), 3, 1, 0, 360, Color::WHITE, clip);
        assert_eq!(d.pixels[5 * 10 + 8], Color::WHITE); // 3 o'clock point on the ring
        assert_eq!(d.pixels[5 * 10 + 5], Color::BLACK); // center hollow
    }

    #[test]
    fn draw_text_draws_glyph_pixels_only() {
        let mut buf = [Color::BLACK; 200];
        let mut d = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 20, 10), stride: 20 };
        let clip = Rect::new(0, 0, 20, 10);
        d.draw_text(Point::new(0, 0), crate::font::DEFAULT_FONT, "I", Color::WHITE, clip);
        let white = buf.iter().filter(|&&p| p == Color::WHITE).count();
        assert!(white > 0 && white < 50, "glyph pixels drawn, background untouched ({white})");
    }

    #[test]
    fn draw_target_accepts_native_rgb565() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        EgCircle::new(embedded_graphics::geometry::Point::new(0, 0), 5)
            .into_styled(EgStyle::with_fill(Rgb565::GREEN))
            .draw(&mut d)
            .unwrap();
        assert_eq!(d.pixels[1 * 10 + 1], Rgb565::GREEN);
    }

    #[test]
    fn rgb565_put_quantizes() {
        let mut buf = [Rgb565::BLACK; 100];
        let mut d = canvas565(&mut buf);
        d.put(2, 2, Color::WHITE);
        assert_eq!(RawU16::from(d.pixels[2 * 10 + 2]).into_inner(), Color::WHITE.to_rgb565());
    }

    #[test]
    fn default_canvas_still_rgb888() {
        let mut buf = [Color::BLACK; 100];
        let mut d: Canvas<'_> = Canvas { pixels: &mut buf, area: Rect::new(0, 0, 10, 10), stride: 10 };
        let clip = Rect::new(0, 0, 10, 10);
        d.fill_rect(Rect::new(0, 0, 10, 10), Color::rgb(80, 140, 255), clip);
        assert!(d.pixels.iter().all(|&p| p == Color::rgb(80, 140, 255)));
    }
}
```

- [ ] **Step 3: Strip the `opa` argument at every call site**

Run: `grep -rn "ap(255)\|, 255, clip\|draw_text_opa\|fill_rounded(\|draw_border(\|fill_circle(\|draw_circle(\|draw_arc(\|draw_line(\|fill_rect(\|blit565(" qingui/src qingui/tests --include '*.rs' -l`
For every call site: delete the `opa` argument (now always `255` after Task 2); `draw_text_opa(...)` → `draw_text(...)` (drop the opa arg). In `qingui/tests/rgb565.rs` delete the `rgb565_blend_roundtrips_through_rgb888` test.
Expected failure mode if you miss one: compile error (argument count) — the compiler is the checklist.

- [ ] **Step 4: Recalibrate visual assertions**

Run: `cargo test -p qingui`
Tests asserting exact pixels on AA edges (rounded corners, arcs, thick lines, circle edges) will fail. For EACH failing assertion: look at the failing output, confirm the new pixels are the correct aliased rendering of the same shape (not a wrong shape/offset/clip bug), update the expected value, and record the before/after with a one-line justification in your report. If a failure looks like a REAL rendering bug (wrong position, missing stroke, wrong size), STOP and fix the canvas method instead — common causes: e-g stroke centering vs the old inside/outward width semantics (`draw_arc`/`draw_circle` stroke may sit `width/2` off; adjust the radius by `- width / 2 + ...` as the old code's semantics require — check the old `draw.rs` `ArcGeom` in git history if unsure).

- [ ] **Step 5: Run the full suite + all targets**

Run: `cargo test -p qingui && cargo check --workspace --examples --benches --tests`
Expected: PASS, zero new warnings.

- [ ] **Step 6: Commit**

```bash
git add -A qingui
git commit -m "refactor: delegate Canvas drawing to e-g primitives; remove AA rasterizer and alpha blending"
```

---

### Task 4: Docs, migration notes, and full verification

**Files:**
- Modify: `qingui/README.md` (migration section)
- Modify: `qingui/src/geometry.rs` if `Color::blend` still exists (delete it — Task 3 should have; double-check)

- [ ] **Step 1: Delete `Color::blend` if anything still references it**

Run: `grep -rn "blend" qingui/src qingui/tests`
Expected: zero hits (Task 3 removed the blend paths). If `geometry.rs::blend` remains as dead code, delete it (and any test of it).

- [ ] **Step 2: README migration section**

Add to `qingui/README.md` (English, matching existing style; the repo-root README is a symlink to this file) a "Unreleased / 0.3 breaking changes" section listing:
- Canvas drawing methods lost the `opa` parameter; `draw_text_opa` removed (use `draw_text`); no alpha blending anywhere.
- `Style.bg_opa`/`Style.opa` removed; background now paints iff `bg_color` is `Some` (`ResolvedStyle.bg_color: Option<Color>`).
- `Ui::set_opa`, `AnimProp::Opa`, `Color::blend`, and the list delete-ghost effect removed.
- `qingui::Point` is now e-g's `Point`; `Rect` ↔ `Rectangle` `From` conversions added.
- Visual change: no anti-aliasing (aliased corners/arcs/lines), no translucency.
- Rendering now delegates to embedded-graphics primitives.

- [ ] **Step 3: Full verification**

Run in order:
1. `cargo test --workspace` — PASS.
2. `cargo check --workspace --examples --benches --tests` — PASS, zero warnings vs baseline.
3. `cargo build -p qingui --target thumbv7em-none-eabihf` — PASS. If the target is missing, STOP and ask the user before `rustup target add`.
4. `cargo bench -p qingui --bench time` on this branch AND on main (`git worktree add` a scratch main checkout, or use the main checkout at the repo root if clean) — compare. Delegated fills should hit the `fill_solid`/`fill_contiguous` fast paths; if a benchmark regresses > 15%, note it in the report with numbers (the fix, if needed: keep a hand-written fast path for that method — this is spec-sanctioned, see spec §7).
5. Build and (if a display is available) run the `demo`/`gallery` examples; at minimum `cargo build --examples` must pass and the gallery simulator must start without panicking.

- [ ] **Step 4: Commit**

```bash
git add qingui/README.md qingui/src/geometry.rs
git commit -m "docs: migration notes for e-g unified rendering (no alpha, no AA)"
```

---

## Self-Review Notes (already applied)

- **Spec coverage:** §1 删除清单 → T2 (+T3 blend); §2 委托映射 → T3 (fill_rect 保留自写快路径并在计划中说明原因：它是 fill_solid 的落点，委托会递归); §3 数据结构 → T1; §4 render 管线 → T2 Step 2; §5 破坏性清单 → T4 README; §6 测试策略 → T2/T3 各任务的测试步骤 + T3 Step 4 重校规程; §7 性能回退条款 → T4 Step 3.4。
- **Order:** T1 (Point/From) before T2/T3 because T3's delegation code uses `Point::new`/`r.into()`; T2 before T3 because T3's call-site sweep assumes `ap()` is already gone (every opa argument is a literal `255` by then).
- **Type consistency:** `Canvas` 方法签名在 T3 Produces 块一次定义，T3 的测试与调用点扫描都以它为准；`ResolvedStyle.bg_color: Option<Color>` 在 T2 Produces 定义，T2 的 render.rs 代码块使用 `if let Some(bg) = resolved.bg_color`。
- **Recursion audit:** 委托方法 → e-g → `draw_iter`/`fill_solid`/`fill_contiguous` → `put`/`fill_rows`（终端，不委托）。`fill_rect` 不委托。无环。
- **Known judgment calls left to the implementer (with stop rules):** `fill_contiguous` 迭代器契约核对（T3 Step 1.7 注释）；arc/circle stroke 居中语义差（T3 Step 4 停线规则）。
