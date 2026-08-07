#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

//! 32-bit runtime bench for qingui, run on QEMU (`-machine mps2-an386`,
//! Cortex-M4F, thumbv7em-none-eabihf, `-icount shift=3`).
//!
//! Prints layout / render (full + partial dirty) / full-frame / primitive
//! timing as deterministic SysTick ticks. Semihosting carries the output; the
//! exit code reflects whether all asserts passed.
//!
//! Build/run for the target (must run from the package dir so the `-icount
//! shift=3` runner in `.cargo/config.toml` applies):
//!
//! ```text
//! (cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)
//! ```
//!
//! IMPORTANT: run with `--release`. The dev profile (plain `cargo run`)
//! produces distorted numbers — unoptimized code + `debug_assertions` skew
//! the relative primitive costs (e.g. dev showed `draw_border`/`draw_circle`
//! more expensive than `draw_arc`, and `fill_rounded` cheaper than
//! `fill_rect`, which is impossible; release reverses both). The threshold
//! asserts below are calibrated against the release baselines, so a dev run
//! will (correctly) fail them.
//!
//! On host builds (workspace `cargo test`/`cargo build`) this compiles to a
//! stub so the crate stays a clean workspace member.
//!
//! BASELINE 2026-08-07 (QEMU mps2-an386, `-icount shift=3`,
//! thumbv7em-none-eabihf, RELEASE profile, SysTick ticks; deterministic,
//! verified identical across 2 runs):
//!   layout (40 children, 320x240)             =       8784
//!   render full  Minimal  (3 nodes, 19200 px) =     72619  partial =  25126  frame = 72844
//!   render full  Small    (16 nodes, 76800 px)=    339889  partial = 156030  frame = 340891
//!   render full  Medium   (50 nodes, 76800 px)=    777823  partial = 535563  frame = 780024
//!   render full  Large    (140 nodes, 76800px)=   1945304  partial = 1539392  frame = 1950620
//!   fill_rect=215729 draw_line=190899 draw_line_many=36081 draw_circle=161012
//!   fill_circle=166656 fill_rounded=226371 draw_border=60274 draw_arc=173846
//!   draw_text=9733 blit565=9178

#[cfg(target_arch = "arm")]
extern crate alloc;

#[cfg(target_arch = "arm")]
mod allocator;
#[cfg(target_arch = "arm")]
mod scenes;
#[cfg(target_arch = "arm")]
mod timer;

#[cfg(target_arch = "arm")]
use cortex_m_rt::entry;
#[cfg(target_arch = "arm")]
use cortex_m_semihosting::debug::{exit, EXIT_SUCCESS};
#[cfg(target_arch = "arm")]
use cortex_m_semihosting::hprintln;

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = hprintln!("PANIC: {}", info);
    exit(cortex_m_semihosting::debug::EXIT_FAILURE);
    loop {}
}

// Thresholds calibrated 2026-08-07: QEMU baseline x 2 (see spec
// docs/superpowers/specs/2026-08-07-runtime-bench-design.md).
#[cfg(target_arch = "arm")]
const LIMIT_LAYOUT: u64 = 17_568; // 2 * 8784
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_FULL_MINIMAL: u64 = 145_238; // 2 * 72619
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_PARTIAL_MINIMAL: u64 = 50_252; // 2 * 25126
#[cfg(target_arch = "arm")]
const LIMIT_FRAME_MINIMAL: u64 = 145_688; // 2 * 72844
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_FULL_SMALL: u64 = 679_778; // 2 * 339889
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_PARTIAL_SMALL: u64 = 312_060; // 2 * 156030
#[cfg(target_arch = "arm")]
const LIMIT_FRAME_SMALL: u64 = 681_782; // 2 * 340891
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_FULL_MEDIUM: u64 = 1_555_646; // 2 * 777823
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_PARTIAL_MEDIUM: u64 = 1_071_126; // 2 * 535563
#[cfg(target_arch = "arm")]
const LIMIT_FRAME_MEDIUM: u64 = 1_560_048; // 2 * 780024
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_FULL_LARGE: u64 = 3_890_608; // 2 * 1945304
#[cfg(target_arch = "arm")]
const LIMIT_RENDER_PARTIAL_LARGE: u64 = 3_078_784; // 2 * 1539392
#[cfg(target_arch = "arm")]
const LIMIT_FRAME_LARGE: u64 = 3_901_240; // 2 * 1950620
#[cfg(target_arch = "arm")]
const LIMIT_FILL_RECT: u64 = 431_458; // 2 * 215729
#[cfg(target_arch = "arm")]
const LIMIT_DRAW_LINE: u64 = 381_798; // 2 * 190899
#[cfg(target_arch = "arm")]
const LIMIT_DRAW_LINE_MANY: u64 = 72_162; // 2 * 36081
#[cfg(target_arch = "arm")]
const LIMIT_DRAW_CIRCLE: u64 = 322_024; // 2 * 161012
#[cfg(target_arch = "arm")]
const LIMIT_FILL_CIRCLE: u64 = 333_312; // 2 * 166656
#[cfg(target_arch = "arm")]
const LIMIT_FILL_ROUNDED: u64 = 452_742; // 2 * 226371
#[cfg(target_arch = "arm")]
const LIMIT_DRAW_BORDER: u64 = 120_548; // 2 * 60274
#[cfg(target_arch = "arm")]
const LIMIT_DRAW_ARC: u64 = 347_692; // 2 * 173846
#[cfg(target_arch = "arm")]
const LIMIT_DRAW_TEXT: u64 = 19_466; // 2 * 9733
#[cfg(target_arch = "arm")]
const LIMIT_BLIT565: u64 = 18_356; // 2 * 9178

#[cfg(target_arch = "arm")]
fn report_layout() {
    hprintln!("== layout (flex column, 40 children, 320x240) ==");
    let mut ui = scenes::build_layout_scene(40);
    let mut now = || timer::elapsed();
    let t = scenes::time_layout(&mut ui, &mut now);
    hprintln!("  layout           {:>10} ticks", t);
    assert!(t < LIMIT_LAYOUT, "layout {} ticks exceeds {}", t, LIMIT_LAYOUT);
}

#[cfg(target_arch = "arm")]
fn report_render() {
    use scenes::Tier;
    for tier in [Tier::Minimal, Tier::Small, Tier::Medium, Tier::Large] {
        let mut sc = scenes::build_render_scene(tier);
        let (nodes, area) = scenes::scene_label(&sc);
        hprintln!("== render ({:?}, {} nodes, {} px) ==", tier, nodes, area);
        let mut now = || timer::elapsed();
        let full = scenes::time_render_full(&mut sc.ui, &mut now);
        let partial = scenes::time_render_partial(&mut sc.ui, sc.leaf, &mut now);
        let frame = scenes::time_frame(&mut sc.ui, &mut now);
        hprintln!("  render full      {:>10} ticks", full);
        hprintln!("  render partial   {:>10} ticks", partial);
        hprintln!("  full frame       {:>10} ticks", frame);
        let (fl, pl, frl) = match tier {
            Tier::Minimal => (
                LIMIT_RENDER_FULL_MINIMAL,
                LIMIT_RENDER_PARTIAL_MINIMAL,
                LIMIT_FRAME_MINIMAL,
            ),
            Tier::Small => (
                LIMIT_RENDER_FULL_SMALL,
                LIMIT_RENDER_PARTIAL_SMALL,
                LIMIT_FRAME_SMALL,
            ),
            Tier::Medium => (
                LIMIT_RENDER_FULL_MEDIUM,
                LIMIT_RENDER_PARTIAL_MEDIUM,
                LIMIT_FRAME_MEDIUM,
            ),
            Tier::Large => (
                LIMIT_RENDER_FULL_LARGE,
                LIMIT_RENDER_PARTIAL_LARGE,
                LIMIT_FRAME_LARGE,
            ),
        };
        assert!(full < fl, "render full {tier:?}: {full} exceeds {fl}");
        assert!(partial < pl, "render partial {tier:?}: {partial} exceeds {pl}");
        assert!(frame < frl, "frame {tier:?}: {frame} exceeds {frl}");
    }
}

#[cfg(target_arch = "arm")]
fn report_primitives() {
    hprintln!("== primitives (DrawBuf 320x240, {} draws each) ==", scenes::PRIM_ITERS);
    let mut now = || timer::elapsed();
    let p = scenes::run_primitives(&mut now);
    hprintln!("  fill_rect        {:>10} ticks", p.fill_rect);
    hprintln!("  draw_line        {:>10} ticks", p.draw_line);
    hprintln!("  draw_line_many   {:>10} ticks", p.draw_line_many);
    hprintln!("  draw_circle      {:>10} ticks", p.draw_circle);
    hprintln!("  fill_circle      {:>10} ticks", p.fill_circle);
    hprintln!("  fill_rounded     {:>10} ticks", p.fill_rounded);
    hprintln!("  draw_border      {:>10} ticks", p.draw_border);
    hprintln!("  draw_arc         {:>10} ticks", p.draw_arc);
    hprintln!("  draw_text        {:>10} ticks", p.draw_text);
    hprintln!("  blit565          {:>10} ticks", p.blit565);
    assert!(p.fill_rect < LIMIT_FILL_RECT, "fill_rect {} exceeds {}", p.fill_rect, LIMIT_FILL_RECT);
    assert!(p.draw_line < LIMIT_DRAW_LINE, "draw_line {} exceeds {}", p.draw_line, LIMIT_DRAW_LINE);
    assert!(
        p.draw_line_many < LIMIT_DRAW_LINE_MANY,
        "draw_line_many {} exceeds {}",
        p.draw_line_many,
        LIMIT_DRAW_LINE_MANY
    );
    assert!(
        p.draw_circle < LIMIT_DRAW_CIRCLE,
        "draw_circle {} exceeds {}",
        p.draw_circle,
        LIMIT_DRAW_CIRCLE
    );
    assert!(
        p.fill_circle < LIMIT_FILL_CIRCLE,
        "fill_circle {} exceeds {}",
        p.fill_circle,
        LIMIT_FILL_CIRCLE
    );
    assert!(
        p.fill_rounded < LIMIT_FILL_ROUNDED,
        "fill_rounded {} exceeds {}",
        p.fill_rounded,
        LIMIT_FILL_ROUNDED
    );
    assert!(
        p.draw_border < LIMIT_DRAW_BORDER,
        "draw_border {} exceeds {}",
        p.draw_border,
        LIMIT_DRAW_BORDER
    );
    assert!(p.draw_arc < LIMIT_DRAW_ARC, "draw_arc {} exceeds {}", p.draw_arc, LIMIT_DRAW_ARC);
    assert!(p.draw_text < LIMIT_DRAW_TEXT, "draw_text {} exceeds {}", p.draw_text, LIMIT_DRAW_TEXT);
    assert!(p.blit565 < LIMIT_BLIT565, "blit565 {} exceeds {}", p.blit565, LIMIT_BLIT565);
}

#[cfg(target_arch = "arm")]
#[entry]
fn main() -> ! {
    timer::init();
    report_layout();
    report_render();
    report_primitives();
    exit(EXIT_SUCCESS);
    loop {}
}

#[cfg(not(target_arch = "arm"))]
fn main() {
    println!("qemu-time targets the bare-metal Cortex-M4F; build and run it for the embedded target:");
    println!("  (cd tools/qemu-time && cargo run --release --target thumbv7em-none-eabihf)");
}
