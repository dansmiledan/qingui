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

#[cfg(target_arch = "arm")]
extern crate alloc;

#[cfg(target_arch = "arm")]
mod allocator;
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
#[entry]
fn main() -> ! {
    hprintln!("qemu-time: scene modules wired in Task 3");
    exit(EXIT_SUCCESS);
    loop {}
}

#[cfg(not(target_arch = "arm"))]
fn main() {
    println!("qemu-time targets the bare-metal Cortex-M4F; build and run it for the embedded target:");
    println!("  cargo run -p qemu-time --target thumbv7em-none-eabihf");
}
