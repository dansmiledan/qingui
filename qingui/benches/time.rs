//! Runtime benchmark (host, wall-clock): layout / render / frame / primitives.
//!
//! Uses the SAME scene + measurement code as the QEMU tool via `#[path]`
//! (single source of truth, see
//! `tools/qemu-time/src/scenes.rs` and the spec
//! `docs/superpowers/specs/2026-08-07-runtime-bench-design.md`).
//!
//! Wall-clock is noisy; we warm up then take N samples and report min/median.
//! No assertions here — regression gates live in the deterministic QEMU tool.

extern crate alloc;

#[path = "../../tools/qemu-time/src/scenes.rs"]
mod scenes;

use std::time::Instant;

const SAMPLES: usize = 100;
const WARMUP: usize = 5;

fn now_from(base: &Instant) -> impl FnMut() -> u64 + '_ {
    move || base.elapsed().as_nanos() as u64
}

/// Runs `measure` SAMPLES times after WARMUP, returns (min_us, median_us).
fn bench<F: FnMut(&mut dyn FnMut() -> u64) -> u64>(mut measure: F) -> (f64, f64) {
    let base = Instant::now();
    let mut now = now_from(&base);
    for _ in 0..WARMUP {
        measure(&mut now);
    }
    let mut v: Vec<u64> = (0..SAMPLES).map(|_| measure(&mut now)).collect();
    v.sort_unstable();
    let min = *v.first().unwrap();
    let median = v[v.len() / 2];
    (min as f64 / 1000.0, median as f64 / 1000.0) // ns -> us
}

fn main() {
    println!("== runtime bench (host wall-clock, {} samples, min/median us) ==", SAMPLES);

    // ---- layout ----
    let (min, med) = bench(|now| {
        let mut ui = scenes::build_layout_scene(40);
        scenes::time_layout(&mut ui, now)
    });
    println!("layout (flex, 40 children)   min {min:>8.1} us  median {med:>8.1} us");

    // ---- render / frame ----
    for tier in [scenes::Tier::Minimal, scenes::Tier::Small, scenes::Tier::Medium, scenes::Tier::Large] {
        let (nodes, area) = {
            let sc = scenes::build_render_scene(tier);
            scenes::scene_label(&sc)
        };
        let (fmin, fmed) = bench(|now| {
            let mut sc = scenes::build_render_scene(tier);
            scenes::time_render_full(&mut sc.ui, now)
        });
        let (pmin, pmed) = bench(|now| {
            let mut sc = scenes::build_render_scene(tier);
            scenes::time_render_partial(&mut sc.ui, sc.leaf, now)
        });
        let (frmin, frmed) = bench(|now| {
            let mut sc = scenes::build_render_scene(tier);
            scenes::time_frame(&mut sc.ui, now)
        });
        println!("render {:?} ({} nodes, {} px)", tier, nodes, area);
        println!("  full     min {fmin:>8.1} us  median {fmed:>8.1} us");
        println!("  partial  min {pmin:>8.1} us  median {pmed:>8.1} us");
        println!("  frame    min {frmin:>8.1} us  median {frmed:>8.1} us");
    }

    // ---- primitives ----
    println!("== primitives (DrawBuf 320x240, {} draws each) ==", scenes::PRIM_ITERS);
    let report = |name: &str, f: &mut dyn FnMut(&mut dyn FnMut() -> u64) -> u64| {
        let (min, med) = bench(f);
        println!("  {name:<16} min {min:>8.1} us  median {med:>8.1} us");
    };
    report("fill_rect", &mut |now| scenes::run_primitives(now).fill_rect);
    report("draw_line", &mut |now| scenes::run_primitives(now).draw_line);
    report("draw_line_many", &mut |now| scenes::run_primitives(now).draw_line_many);
    report("draw_circle", &mut |now| scenes::run_primitives(now).draw_circle);
    report("fill_circle", &mut |now| scenes::run_primitives(now).fill_circle);
    report("fill_rounded", &mut |now| scenes::run_primitives(now).fill_rounded);
    report("draw_border", &mut |now| scenes::run_primitives(now).draw_border);
    report("draw_arc", &mut |now| scenes::run_primitives(now).draw_arc);
    report("draw_text", &mut |now| scenes::run_primitives(now).draw_text);
    report("blit565", &mut |now| scenes::run_primitives(now).blit565);
}
