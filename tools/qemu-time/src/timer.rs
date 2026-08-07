//! SysTick-based deterministic cycle counter for QEMU timing.
//!
//! QEMU's Cortex-M model does NOT implement the DWT cycle counter (source:
//! `hw/intc/armv7m_nvic.c` has `TODO: Implement debug registers`). SysTick
//! advances deterministically only when QEMU runs with `-icount shift=3`
//! (verified: 4 identical runs, 10k->8003 / 1M->800002 ticks, exactly 100x
//! linear). Each tick = 2^shift = 8 virtual ns; absolute hardware cycles come
//! from real-hardware DWT — this gives a deterministic relative shape.

use core::sync::atomic::{AtomicBool, Ordering};

const SYST_CSR: usize = 0xE000_E010;
const SYST_RVR: usize = 0xE000_E014;
const SYST_CVR: usize = 0xE000_E018;
const RELOAD: u32 = 0x00FF_FFFF;

static INIT: AtomicBool = AtomicBool::new(false);
// Bare metal, single thread: static mut is fine here.
static mut PREV: u32 = RELOAD;
static mut ACC: u64 = 0;

#[inline(never)]
fn mmio_write(addr: usize, val: u32) {
    unsafe { core::ptr::write_volatile(addr as *mut u32, val) };
}
#[inline(never)]
fn mmio_read(addr: usize) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

/// Enables SysTick: max reload, core clock source, count down to zero.
pub fn init() {
    mmio_write(SYST_RVR, RELOAD);
    mmio_write(SYST_CVR, 0);
    mmio_write(SYST_CSR, 0x5); // CLKSOURCE=1 (core clock), ENABLE=1
    unsafe { PREV = mmio_read(SYST_CVR) & 0xFF_FFFF; }
    INIT.store(true, Ordering::Relaxed);
}

/// Monotonic elapsed ticks since `init()` (or first call), wrap-aware:
/// the 24-bit SysTick wraps back to RELOAD; we add the wrapped distance.
/// The accumulator is correct for at most one wrap between reads: a double
/// wrap (a measurement > ~33.5M ticks, 2 * RELOAD) would undercount. Today's
/// largest measurement (~15.8M, render full Large) is ~94% of one reload,
/// leaving ~2.1x headroom before a double-wrap undercount.
pub fn elapsed() -> u64 {
    if !INIT.load(Ordering::Relaxed) {
        init();
    }
    let cur = mmio_read(SYST_CVR) & 0xFF_FFFF;
    unsafe {
        let prev = PREV;
        PREV = cur;
        if cur <= prev {
            ACC += (prev - cur) as u64;
        } else {
            ACC += prev as u64 + (RELOAD - cur) as u64;
        }
        ACC
    }
}
