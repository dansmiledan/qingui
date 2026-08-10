//! Host regression test for the counting free-list allocator.
//!
//! Runs the same scene-building code as the QEMU bench on the host so the
//! allocator logic is exercised identically. Catches free-list corruption /
//! OOM fast without a target. A single test function keeps the measurement
//! window free of other threads (a second test thread blocked on a mutex
//! lazily allocates, which would pollute the live-byte delta).

#[path = "../src/allocator.rs"]
mod allocator;
#[path = "../src/scene.rs"]
mod scene;

use allocator::{current, peak};
use scene::Tier;

fn fast_rand() -> usize {
    static mut SEED: usize = 0x12345678;
    unsafe {
        SEED ^= SEED << 13;
        SEED ^= SEED >> 7;
        SEED ^= SEED << 17;
        SEED
    }
}

#[test]
fn allocator_stays_consistent_across_scenes_and_churn() {
    for tier in [Tier::Minimal, Tier::Small, Tier::Medium, Tier::Large] {
        // The test process shares the global allocator with the std runtime,
        // so measure scene allocations as a delta instead of reset()-ing the
        // counters (reset() is only valid on bare metal where nothing else
        // allocates).
        let base = current();
        let base_peak = peak();
        let ui = scene::build_scene(tier).ui;
        let peak_delta = peak() - base_peak;
        drop(ui);
        let live_after = current() - base;
        assert!(peak_delta < allocator::ARENA_LIMIT, "tier {tier:?} peak {peak_delta} B exceeds arena");
        assert_eq!(live_after, 0, "tier {tier:?} live {live_after} B after drop (memory not reclaimed)");
    }

    // Alloc/free churn with odd sizes exercises the alignment and coalescing
    // paths that misbehaved on the embedded target.
    let base = current();
    let mut v: Vec<Box<[u8]>> = Vec::new();
    for _ in 0..2000 {
        let n = 3 + (fast_rand() % 37);
        let b = vec![0u8; n].into_boxed_slice();
        v.push(b);
        if v.len() > 50 {
            v.remove(0);
        }
    }
    drop(v);
    assert_eq!(current() - base, 0, "churn leaked bytes");

    // Over-aligned allocations must not permanently lose arena space: the
    // alignment padding between the free-block start and the payload header
    // must be returned to the free list, otherwise each aligned alloc that
    // lands on a 16-misaligned free block orphans a few bytes for good.
    // Alloc many aligned boxes, free them all, and require the free-list
    // total to come back exactly (orphaned bytes never return).
    #[repr(align(16))]
    #[allow(dead_code)]
    struct Aligned16(u8);
    let mut v: Vec<Box<Aligned16>> = Vec::with_capacity(10_000);
    let free_before = allocator::free_bytes();
    for _ in 0..10_000 {
        v.push(Box::new(Aligned16(0)));
    }
    v.clear(); // drops the boxes, keeps the Vec capacity (no live-set change)
    assert_eq!(
        allocator::free_bytes(),
        free_before,
        "aligned allocs orphaned arena bytes (free-list total shrank)"
    );
    drop(v);
}
