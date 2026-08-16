# Color Alias to Rgb888 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace qingui's own `Color` struct with a re-export of embedded-graphics' `Rgb888`, relocating its remaining helpers (gray constants, `blend`, RGB565 conversion) to their proper homes — zero behavior change.

**Architecture:** `pub use embedded_graphics::pixelcolor::Rgb888 as Color;` in `geometry.rs`. The generic pixel-format system (`Canvas<'a, C = Color>`, `Flush<C>`, `Ui<C>`, `PixelFormat`) is untouched — the default `C = Color` now resolves to `Rgb888` (identical layout). All call-site adaptations are mechanical renames driven by the compiler.

**Tech Stack:** Rust edition 2024, `no_std` + `alloc`, embedded-graphics 0.8.2 (default-features = false), cargo test.

**Spec:** `docs/superpowers/specs/2026-08-14-color-alias-rgb888-design.md`

## Global Constraints

- Crate `qingui` is `#![no_std]` with `extern crate alloc`; no new dependencies.
- **Zero behavior change**: every existing test must pass with UNCHANGED assertions. The only permitted test edits are call-syntax adaptations (`Color::rgb(...)` → `Color::new(...)`, `c.blend(o, t)` → `blend(c, o, t)`, etc.).
- The generic pixel-format system must work exactly as before, including the `Rgb565` framebuffer path and the bit-consistency between `color_to_rgb565`/`color_from_rgb565` and the previous `Color::to_rgb565`/`from_rgb565` (same bit math, verbatim).
- Every task ends with `cargo test -p qingui` fully green and `cargo check --workspace --examples --benches --tests` with zero warnings.
- Code comments and commit messages in English (Conventional Commits). Commits are local only; **never `git push`**. Per `AGENTS.md`, the controller asks the user before commit batches.

---

### Task 1: Alias `Color` to `Rgb888` + mechanical call-site sweep

This is one atomic change: the moment `geometry.rs` flips, every call site must follow before the crate compiles. One task, one commit.

**Files:**
- Modify: `qingui/src/geometry.rs` (replace the `Color` struct block at lines 81-150)
- Modify: `qingui/src/pixel.rs` (delete duplicate impl lines 20-23, macro body 30-37, add 565 helpers, adapt Rgb565 impl 45-52, adapt test module 54-95)
- Modify: `qingui/src/canvas.rs` (`blit565`'s `Color::from_rgb565(v)` call)
- Modify: `qingui/src/widgets/led.rs` (blend call site, ~line 25)
- Modify: every file with `Color::rgb(`, `Color::GRAY`/`LIGHT_GRAY`/`DARK_GRAY`, or direct color field access (`.r`/`.g`/`.b` on a `Color`) — widgets, style.rs, render.rs, tests, examples, benches, tools. Find them with the greps in Step 3.
- Test: existing suite is the test (plus the adapted `pixel.rs`/`geometry.rs` unit tests below)

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces:
  - `qingui::Color` = `embedded_graphics::pixelcolor::Rgb888` (re-export alias in `geometry.rs`); `lib.rs`'s `pub use geometry::{Color, Point, Rect};` keeps working unchanged.
  - `qingui::geometry::{GRAY, LIGHT_GRAY, DARK_GRAY}: Color` (top-level consts, values 128/200/40 grayscale — NOT e-g's `CSS_*` grays).
  - `qingui::geometry::blend(bg: Color, fg: Color, t: u8) -> Color` (free fn; same math as the old method: result = `bg*(255-t)/255 + fg*t/255`, rounded).
  - `qingui::pixel::{color_to_rgb565(c: Color) -> u16, color_from_rgb565(v: u16) -> Color}` (both `pub(crate)`; bit math verbatim from the old `Color` methods).
  - `Color::WHITE`/`BLACK`/`RED`/`GREEN`/`BLUE` keep working via the alias (Rgb888 inherent consts). `Color::new(r, g, b)` replaces `Color::rgb(r, g, b)`.

- [ ] **Step 1: Rewrite the `Color` section of `geometry.rs`**

Replace lines 81-150 (the `Color` struct, its inherent impl, both `From` impls, and the `PixelColor` impl) with:

```rust
/// The working color type: embedded-graphics' RGB888.
pub use embedded_graphics::pixelcolor::Rgb888 as Color;

/// Medium gray.
pub const GRAY: Color = Color::new(128, 128, 128);
/// Light gray.
pub const LIGHT_GRAY: Color = Color::new(200, 200, 200);
/// Dark gray.
pub const DARK_GRAY: Color = Color::new(40, 40, 40);

/// Mixes `fg` onto `bg` by weight `t` (0..=255), producing an opaque color.
/// This is plain color mixing (used for LED brightness), not alpha compositing —
/// qingui has no translucency; the result fully replaces the pixel.
pub fn blend(bg: Color, fg: Color, t: u8) -> Color {
    let a = t as u32;
    let inv = 255 - a;
    let m = |s: u8, o: u8| ((s as u32 * inv + o as u32 * a + 127) / 255) as u8;
    Color::new(m(bg.r(), fg.r()), m(bg.g(), fg.g()), m(bg.b(), fg.b()))
}
```

(The `use embedded_graphics::pixelcolor::RgbColor;` at line 1 stays — `blend` uses the `.r()/.g()/.b()` accessors.)

- [ ] **Step 2: Adapt `pixel.rs`**

1. Delete `impl PixelFormat for Color` (lines 20-23) — it is now a duplicate of the macro-generated `Rgb888` impl (same type).
2. Update the trait doc (lines 8-12): "Implemented for `Color` (which IS `Rgb888`, the default) and for the other embedded-graphics RGB/BGR color types, so the framebuffer can directly use the display's native format (e.g. `Rgb565`)."
3. Macro body — switch from field access to accessors:

```rust
macro_rules! impl_pixel_format_rgb {
    ($($t:ty),* $(,)?) => {$(
        impl PixelFormat for $t {
            fn to_color(self) -> Color {
                use embedded_graphics::pixelcolor::RgbColor;
                Color::new(self.r(), self.g(), self.b())
            }
            fn from_color(c: Color) -> Self {
                use embedded_graphics::pixelcolor::RgbColor;
                <$t>::new(c.r(), c.g(), c.b())
            }
        }
    )*};
}
```

4. Add the RGB565 helpers (verbatim bit math from the old `Color` methods) and rewire the `Rgb565` impl:

```rust
/// Color -> RGB565 (5-6-5).
pub(crate) fn color_to_rgb565(c: Color) -> u16 {
    use embedded_graphics::pixelcolor::RgbColor;
    (((c.r() as u16) & 0xF8) << 8) | (((c.g() as u16) & 0xFC) << 3) | ((c.b() as u16) >> 3)
}

/// RGB565 (5-6-5) -> Color (bit-copy expansion, lossless round-trip).
pub(crate) fn color_from_rgb565(v: u16) -> Color {
    let r = ((v >> 11) & 0x1F) as u8;
    let g = ((v >> 5) & 0x3F) as u8;
    let b = (v & 0x1F) as u8;
    Color::new((r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2))
}

// Rgb565 is implemented via raw storage so it stays bit-consistent with
// `color_to_rgb565`/`color_from_rgb565`, which `Canvas::blit565` relies on.
impl PixelFormat for Rgb565 {
    fn to_color(self) -> Color {
        color_from_rgb565(RawU16::from(self).into_inner())
    }
    fn from_color(c: Color) -> Self {
        Rgb565::from(RawU16::new(color_to_rgb565(c)))
    }
}
```

5. Test module: `Color::rgb(...)` → `Color::new(...)` (3 spots); in `rgb565_matches_color_helpers`, `c.to_rgb565()` → `color_to_rgb565(c)` and `Color::from_rgb565(...)` → `color_from_rgb565(...)`. The `color_identity` and `color_is_pixel_color` tests stay meaningful (Color = Rgb888 is still the default format) — keep them, syntax-adapted.

- [ ] **Step 3: Mechanical sweep (compiler-driven)**

1. `canvas.rs`: `blit565`'s `crate::geometry::Color::from_rgb565(v)` → `crate::pixel::color_from_rgb565(v)`.
2. `led.rs`: `Color::BLACK.blend(color, bright)` → `blend(Color::BLACK, color, bright)` with `use crate::geometry::blend;` added to its imports.
3. Global renames (apply with your editor across `qingui/src`, `qingui/tests`, `qingui/examples`, `qingui/benches`, `qingui-codegen`, `tools`):
   - `Color::rgb(` → `Color::new(`
   - `Color::GRAY` → `GRAY`, `Color::LIGHT_GRAY` → `LIGHT_GRAY`, `Color::DARK_GRAY` → `DARK_GRAY` (add `use crate::geometry::{GRAY, ...};` / `use qingui::geometry::{GRAY, ...};` as needed per file)
4. Compile and let the compiler enumerate the rest: `cargo check -p qingui 2>&1 | grep "^error" | sort -u`. Expected leftovers: direct field accesses (`c.r`/`c.g`/`c.b` on a `Color` → `.r()/.g()/.b()`), any `Color { r, g, b }` struct literals (→ `Color::new(r, g, b)`), any missed `to_rgb565`/`from_rgb565` method calls.
5. `grep -rn "Color {" qingui/src qingui/tests --include '*.rs' | grep -v "//" ` to find struct literals (exclude `Canvas {`, `Rect {` etc. — read each hit).

- [ ] **Step 4: Run the full suite**

Run: `cargo test -p qingui && cargo check --workspace --examples --benches --tests`
Expected: PASS with **zero assertion changes** — every test edit is call-syntax only. If any assertion VALUE needs to change, you made a semantic mistake (most likely in the 565 helpers' bit math or blend's argument order): fix the implementation, never the expectation.

- [ ] **Step 5: Commit**

```bash
git add -A qingui qingui-codegen tools
git commit -m "refactor: alias Color to e-g Rgb888; relocate gray consts, blend, and RGB565 helpers"
```

---

### Task 2: README migration note + full verification

**Files:**
- Modify: `qingui/README.md` (append to the "Unreleased / 0.3 breaking changes" section)

- [ ] **Step 1: README bullet**

Append to the breaking-changes list in `qingui/README.md` (English, matching existing bullets):

```markdown
- `Color` is now a re-export of e-g's `Rgb888` (`pub use Rgb888 as Color`). `Color::rgb(r, g, b)` → `Color::new(r, g, b)`; `Color::GRAY`/`LIGHT_GRAY`/`DARK_GRAY` moved to `qingui::geometry::{GRAY, LIGHT_GRAY, DARK_GRAY}`; `Color::blend(a, b, t)` → free function `qingui::geometry::blend(a, b, t)`; `to_rgb565`/`from_rgb565` are now crate-internal. `Color::WHITE` etc. keep working via `Rgb888`'s constants.
```

- [ ] **Step 2: Full verification**

Run in order:
1. `cargo test --workspace` — PASS.
2. `cargo check --workspace --examples --benches --tests` — PASS, zero warnings.
3. `cargo build -p qingui --target thumbv7em-none-eabihf` — PASS. If the target is missing, STOP and ask the user before `rustup target add`.
4. `cargo bench -p qingui --bench time` vs main — the alias is a zero-cost abstraction; any delta beyond noise (±10%) means a real regression slipped in (most likely a missed inline or an accidental extra conversion): investigate and report.
5. `cargo doc -p qingui --no-deps` — clean (the re-export and moved items produce doc links; check for broken intra-doc links).

- [ ] **Step 3: Commit**

```bash
git add qingui/README.md
git commit -m "docs: note Color aliasing to Rgb888 in breaking changes"
```

---

## Self-Review Notes (already applied)

- **Spec coverage:** §1 核心变更 → T1 Step 1; §2 565 helpers → T1 Step 2.4; §3 宏体适配 → T1 Step 2.3; §4 调用点适配 → T1 Steps 1-3; §5 不受影响项 → Global Constraints（零行为变化）+ T1 Step 4 的零断言门禁; §6 文档 → T2 Step 1。
- **Atomicity:** 别名化不可拆（geometry.rs 一翻，全库调用点必须同 commit 跟进），故全库 sweep 与核心变更同在 T1；T2 只有文档和验证。
- **Type consistency:** `blend(bg, fg, t)` 的参数顺序与旧方法 `self.blend(over, opa)` 的语义一一对应（self→bg, over→fg, opa→t），led.rs 的调用点翻译 `Color::BLACK.blend(color, bright)` → `blend(Color::BLACK, color, bright)` 在 T1 Step 3.2 显式给出。
- **已知陷阱（已在步骤中内联警告）：** e-g 的 `CSS_LIGHT_GRAY` (211) ≠ qingui `LIGHT_GRAY` (200)——灰色常量保留自定义值，不换 CSS 常量；`Rgb888` 的 `Default` derive 已存在（`Ui::new` 的 `C::default()` 缓冲初始化依赖它，第一轮的 Rgb565 测试已证明 e-g 颜色类型带 `Default`）。
