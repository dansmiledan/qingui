# PixelFormat Slimming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Slim `PixelFormat` to a named bound with default methods delegating to embedded-graphics' own `From` conversions, deleting all hand-written conversion code — which also fixes a live bug in the untested `Rgb555/Rgb666/Bgr*` impls.

**Architecture:** `pub trait PixelFormat: PixelColor + Copy + PartialEq + Default + Into<Color> + From<Color>` with `to_color`/`from_color` as default methods (`self.into()` / `c.into()`). The 8 supported formats get empty impls. Call sites (`C::from_color` / `x.to_color()`) are unchanged.

**Tech Stack:** Rust edition 2024, `no_std` + `alloc`, embedded-graphics 0.8.2 (e-g-core 0.4.1), cargo test.

**Spec:** `docs/superpowers/specs/2026-08-14-pixelformat-slim-design.md`

## Verified Facts (checked against registry sources during planning)

- e-g-core 0.4.1 `conversion.rs` provides `From` conversions between ALL 8 RGB/BGR types via `convert_channel` (rounding, fixed-point).
- e-g-core 0.4.1 `RgbColor::new(r, g, b)` and `r()/g()/b()` use **native bit depth** (Rgb565 red is 0..=31), NOT 8-bit. Therefore the current macro bodies in `qingui/src/pixel.rs` — `<$t>::new(c.r(), c.g(), c.b())` and `Color::new(self.r(), self.g(), self.b())` — are **wrong for `Rgb555/Rgb666/Bgr888/Bgr565/Bgr555/Bgr666`** (low-bit masking on encode, native-depth-as-8-bit on decode). Only `Rgb888` (native depth 8) and the hand-written `Rgb565` impl are correct today. The slimming deletes the buggy code.
- e-g's 565→888 expansion equals the classic bit-replication values (bit replication IS the exact rounding expansion), so `blit565`'s decode output is unchanged.
- e-g's 888→565 quantization ROUNDS (old qingui code truncated): mid-range values can differ by 1 LSB (e.g. `r=250` → 30, was 31). Full-scale values (0x0000/0xFFFF/0xF800…) are identical under both.

## Global Constraints

- Crate `qingui` is `#![no_std]` with `extern crate alloc`; no new dependencies.
- Call-site signatures unchanged: `C::from_color(c)` / `px.to_color()` keep working via the default methods.
- The only tolerated assertion-value changes are in 565-quantization tests, and only where rounding vs truncation differs (mid-range values); every such change needs a one-line justification in the implementer's report. All other tests must pass with UNCHANGED assertions.
- qingui-codegen's inlined 5-6-5 encode formula (truncation) is NOT touched — encode-side quantization is arbitrary, decode expands losslessly. Only its stale comment (referencing the deleted `color_to_rgb565`) is updated.
- Code comments and commit messages in English (Conventional Commits). Commits are local only; **never `git push`**. Per `AGENTS.md`, the controller asks the user before commit batches.

---

### Task 1: Slim `PixelFormat` + fix the native-depth bug

**Files:**
- Rewrite: `qingui/src/pixel.rs` (whole file, 95 lines)
- Modify: `qingui/src/canvas.rs` (`blit565` decode at line 73; test assertion at line 378; imports as needed)
- Modify: `qingui-codegen/src/lib.rs` (stale comments at ~54 and ~103 only — NO code change)
- Modify: `qingui/tests/geometry.rs` (stale comment at line 45 only — the assertions use full-scale values, identical under both quantizations)
- Test: `qingui/src/pixel.rs` test module (rewritten, below)

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces (unchanged signatures, all existing code relies on them):
  - `pub trait PixelFormat: PixelColor + Copy + PartialEq + Default + Into<Color> + From<Color> { fn to_color(self) -> Color; fn from_color(c: Color) -> Self; }` — default bodies delegate to e-g `From`.
  - `impl PixelFormat` (empty) for `Rgb888, Rgb565, Rgb555, Rgb666, Bgr888, Bgr565, Bgr555, Bgr666`.
  - REMOVED: `qingui::pixel::{color_to_rgb565, color_from_rgb565}` (pub(crate)) — `blit565` switches to e-g types directly.

- [ ] **Step 1: Write the failing tests**

Replace the whole test module in `qingui/src/pixel.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::pixelcolor::{Bgr555, Bgr565, Bgr666, Bgr888, Rgb555, Rgb565, Rgb666, Rgb888, RgbColor};

    #[test]
    fn color_identity() {
        let c = Color::new(80, 140, 255);
        assert_eq!(PixelFormat::to_color(c), c);
        assert_eq!(<Color as PixelFormat>::from_color(c), c);
    }

    #[test]
    fn rgb888_lossless_roundtrip() {
        let c = Color::new(1, 128, 255);
        assert_eq!(Rgb888::from_color(c).to_color(), c);
    }

    #[test]
    fn rgb565_full_scale_values() {
        assert_eq!(Rgb565::from_color(Color::WHITE), Rgb565::WHITE);
        assert_eq!(Rgb565::from_color(Color::BLACK), Rgb565::BLACK);
        assert_eq!(Rgb565::from_color(Color::RED), Rgb565::RED);
    }

    #[test]
    fn rgb565_quantization_rounds_like_eg() {
        // e-g converts 8->5 bits with rounding (not truncation): 250*31/255 rounds to 30.
        let raw = RawU16::from(Rgb565::from_color(Color::new(250, 0, 0))).into_inner();
        assert_eq!(raw, 30 << 11);
    }

    #[test]
    fn rgb565_decode_matches_bit_replication() {
        // 565 -> 888 expansion equals the classic bit-replication values.
        assert_eq!(Rgb565::from(RawU16::new(0xF800)).to_color(), Color::new(255, 0, 0));
        // r5 = 16 -> (16<<3)|(16>>2) = 132
        assert_eq!(Rgb565::from(RawU16::new(16 << 11)).to_color(), Color::new(132, 0, 0));
    }

    #[test]
    fn all_formats_roundtrip_midrange() {
        // Regression: the old hand-written macro bodies passed 8-bit values through
        // native-depth new()/r() accessors, corrupting every format except Rgb888 and
        // the hand-written Rgb565. A mid-range color must survive a quantize
        // round-trip within one target-depth LSB (8-bit space) for every format.
        fn check<T: PixelFormat + core::fmt::Debug>(c: Color, tol: i16) {
            let back = T::from_color(c).to_color();
            assert!((back.r() as i16 - c.r() as i16).abs() <= tol, "r drift: {c:?} -> {back:?}");
            assert!((back.g() as i16 - c.g() as i16).abs() <= tol, "g drift: {c:?} -> {back:?}");
            assert!((back.b() as i16 - c.b() as i16).abs() <= tol, "b drift: {c:?} -> {back:?}");
        }
        let c = Color::new(80, 140, 255);
        check::<Rgb888>(c, 0);
        check::<Bgr888>(c, 0);
        check::<Rgb666>(c, 4); // 6-bit: 1 LSB = 4 in 8-bit space
        check::<Bgr666>(c, 4);
        check::<Rgb565>(c, 8); // 5/6-bit
        check::<Bgr565>(c, 8);
        check::<Rgb555>(c, 8);
        check::<Bgr555>(c, 8);
    }

    #[test]
    fn color_is_pixel_color() {
        // Compile-time proof that Color: PixelColor, usable as the default framebuffer format.
        fn assert_pc<T: PixelColor>() {}
        assert_pc::<Color>();
    }
}
```

Note: `Rgb565::from(RawU16::new(16 << 11))` — `16 << 11` is `0x8000`; r5=16 expands to `(16<<3)|(16>>2) = 132`. If e-g's rounding expansion differs for this input, compute the actual value from `convert_channel::<31,255>(16) = round(16*255/31) = round(131.61) = 132` — it is 132; if the assertion still fails, read e-g-core's `conversion.rs` and recalibrate with the real value plus justification.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qingui pixel`
Expected: FAIL — `all_formats_roundtrip_midrange` fails for `Rgb555`/`Bgr666`/etc. under the old macro bodies (and `rgb565_quantization_rounds_like_eg` fails: old truncation gives `31 << 11`). This proves the live bug exists before the fix.

- [ ] **Step 3: Rewrite pixel.rs**

Replace the entire file (keep the module doc, updated) with:

```rust
//! Framebuffer pixel formats: the bridge between qingui's internal RGB888 `Color`
//! and the device-native pixel type stored in the framebuffer.

use crate::geometry::Color;
use embedded_graphics::pixelcolor::{Bgr555, Bgr565, Bgr666, Bgr888, PixelColor, Rgb555, Rgb565, Rgb666, Rgb888};

/// A framebuffer pixel format: convertible to/from the internal RGB888 `Color`.
///
/// Implemented for `Color` (which IS `Rgb888`, the default) and for the other
/// embedded-graphics RGB/BGR color types, so the framebuffer can directly use
/// the display's native format (e.g. `Rgb565`). Conversions delegate to
/// embedded-graphics' own `From` impls (rounding quantization).
pub trait PixelFormat: PixelColor + Copy + PartialEq + Default + Into<Color> + From<Color> {
    /// Converts a framebuffer pixel to the internal RGB888 `Color`.
    fn to_color(self) -> Color {
        self.into()
    }
    /// Converts an internal RGB888 `Color` to a framebuffer pixel (quantizes).
    fn from_color(c: Color) -> Self {
        c.into()
    }
}

macro_rules! impl_pixel_format {
    ($($t:ty),* $(,)?) => {$( impl PixelFormat for $t {} )*};
}

impl_pixel_format!(Rgb888, Rgb565, Rgb555, Rgb666, Bgr888, Bgr565, Bgr555, Bgr666);
```

followed by the test module from Step 1.

- [ ] **Step 4: Adapt `canvas.rs`**

1. `blit565` (line 73): `self.put(px, py, crate::pixel::color_from_rgb565(v));` →

```rust
                self.put(px, py, Color::from(Rgb565::from(RawU16::new(v))));
```

Add to canvas.rs's top imports: `use embedded_graphics::pixelcolor::{raw::RawU16, Rgb565};` (check what's already imported; the test module's imports are separate).

2. Test at line 378: `assert_eq!(RawU16::from(d.pixels[2 * 10 + 2]).into_inner(), crate::pixel::color_to_rgb565(Color::WHITE));` →

```rust
        assert_eq!(RawU16::from(d.pixels[2 * 10 + 2]).into_inner(), 0xFFFF); // Color::WHITE quantizes to full-scale 565
```

- [ ] **Step 5: Fix the two stale comments (no code changes)**

- `qingui-codegen/src/lib.rs:54` and `:103`: the comments reference qingui's `color_to_rgb565`, which no longer exists. Reword to describe the formula on its own terms, e.g. "5-6-5 truncation (encode-side quantization; qingui's `blit565` decode expands losslessly)".
- `qingui/tests/geometry.rs:45`: the comment "The public Rgb565 PixelFormat impl wraps the crate-internal color_to_rgb565" → "The public Rgb565 PixelFormat impl delegates to e-g's From conversions (rounding quantization)". The three assertions (0xFFFF/0x0000/0xF800) stay — full-scale values are identical under both quantizations.

- [ ] **Step 6: Run the full suite**

Run: `cargo test -p qingui && cargo check --workspace --examples --benches --tests`
Expected: PASS, zero warnings. Every test passes with UNCHANGED assertions except the pixel.rs test module itself (rewritten) and the canvas.rs:378 literal. If any OTHER assertion value fails, investigate — likely a real behavior change you didn't intend.

- [ ] **Step 7: Commit**

```bash
git add qingui/src/pixel.rs qingui/src/canvas.rs qingui-codegen/src/lib.rs qingui/tests/geometry.rs
git commit -m "refactor: slim PixelFormat to default methods over e-g conversions; fix native-depth conversion bug"
```

---

### Task 2: README migration note + full verification

**Files:**
- Modify: `qingui/README.md` (append to the "Unreleased / 0.3 breaking changes" section)

- [ ] **Step 1: README bullet**

Append to the breaking-changes list in `qingui/README.md` (English, matching existing bullets):

```markdown
- `PixelFormat` now requires `Into<Color> + From<Color>` (conversions delegate to embedded-graphics; custom `PixelFormat` impls outside the built-in formats must satisfy the e-g conversion bounds). 888-to-565 quantization now rounds (e-g semantics) instead of truncating — mid-range values can shift by 1 LSB; 565-to-888 expansion and the `blit565` round-trip are unchanged.
```

- [ ] **Step 2: Full verification**

Run in order:
1. `cargo test --workspace` — PASS.
2. `cargo check --workspace --examples --benches --tests` — PASS, zero warnings.
3. `cargo build -p qingui --target thumbv7em-none-eabihf` — PASS. If the target is missing, STOP and ask the user before `rustup target add`.
4. `cargo bench -p qingui --bench time` vs main — expect noise-level deltas (conversion happens only at write points). Report numbers.
5. `cargo doc -p qingui --no-deps` — clean.

- [ ] **Step 3: Commit**

```bash
git add qingui/README.md
git commit -m "docs: note PixelFormat supertrait change and rounding quantization"
```

---

## Self-Review Notes (already applied)

- **Spec coverage:** §1 pixel.rs 重写 → T1 Step 3; §2 blit565 → T1 Step 4.1; §3 行为变化 → T1 Steps 1-2（rounding 用例先红后绿）+ T2 README; §4 测试 → T1 Steps 1/4.2/5; §5 文档 → T2 Step 1; §6 验证 → T2 Step 2。
- **Bug framing:** T1 Step 2 requires the new regression test to FAIL against the old code first (proof of the live bug), then pass after the rewrite — TDD evidence for the fix, not just the refactor.
- **Type consistency:** `to_color(self) -> Color { self.into() }` requires `Self: Into<Color>`; `from_color(c: Color) -> Self { c.into() }` requires `Self: From<Color>` — both supertraits are on the trait, and all 8 formats satisfy them via e-g's conversion macro (verified: `impl_rgb_conversion!` covers every pair, `Color` = `Rgb888` reflexive via core).
- **Known trap (inline in T1 Step 1 note):** the `16 << 11` expansion expectation (132) is computed from e-g's rounding formula; if the assertion disagrees, recalibrate from the registry source with justification — do not assume.
