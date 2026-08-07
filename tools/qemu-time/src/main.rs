#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

//! 32-bit runtime bench for qingui, run on QEMU (`-machine mps2-an386`,
//! Cortex-M4F, thumbv7em-none-eabihf, `-icount shift=3`).
//!
//! Prints layout / render (full + partial dirty) / full-frame / primitive
//! timing as deterministic SysTick ticks. Semihosting carries the output; the
//! exit code reflects whether all asserts passed.
//!
//! Build/run for the target:
//!
//! ```text
//! cargo run -p qemu-time --target thumbv7em-none-eabihf
//! ```
//!
//! On host builds (workspace `cargo test`/`cargo build`) this compiles to a
//! stub so the crate stays a clean workspace member.
//!
//! BASELINE 2026-08-07 (QEMU mps2-an386, `-icount shift=3`,
//! thumbv7em-none-eabihf, dev profile, SysTick ticks; deterministic, verified
//! identical across 2 runs):
//!   layout (40 children, 320x240)             =      95667
//!   render full  Minimal  (3 nodes, 19200 px) =    2029561  partial =   678765  frame = 2042022
//!   render full  Small    (16 nodes, 76800 px)=    7488230  partial =  4261180  frame = 7517158
//!   render full  Medium   (50 nodes, 76800 px)=    2140347  partial = 15166486  frame = 2170896
//!   render full  Large    (140 nodes, 76800px)=   15776983  partial = 10606006  frame = 15846535
//!   fill_rect=64015 draw_line=67425 draw_line_many=160474 draw_circle=209946
//!   fill_circle=54871 fill_rounded=24960 draw_border=213775 draw_arc=152215
//!   draw_text=141124 blit565=68643

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

#[cfg(target_arch = "arm")]
fn report_layout() {
    hprintln!("== layout (flex column, 40 children, 320x240) ==");
    let mut ui = scenes::build_layout_scene(40);
    let mut now = || timer::elapsed();
    let t = scenes::time_layout(&mut ui, &mut now);
    hprintln!("  layout           {:>10} ticks", t);
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
    println!("  cargo run -p qemu-time --target thumbv7em-none-eabihf");
}
