//! Counting first-fit free-list allocator over a static arena.
//!
//! Runs on bare metal, so it replaces `std::alloc::System`. Tracks current
//! live bytes and the running peak, mirroring the host bench's counting
//! allocator in `qingui/benches/memory.rs`. `dealloc` really reuses memory so
//! the live/peak numbers are meaningful across repeated layout passes.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

const ARENA_SIZE: usize = 1024 * 1024;
// Used by the host regression test (tests/alloc_host.rs); dead in the bin.
#[allow(dead_code)]
pub const ARENA_LIMIT: usize = ARENA_SIZE;
/// Header stored before every allocated payload and in every free block
/// (first usize of a free block is its size, second its next pointer).
const HEADER: usize = core::mem::size_of::<usize>();
const MIN_FREE: usize = 2 * HEADER;

#[repr(C, align(16))]
struct Arena([u8; ARENA_SIZE]);
// `static mut` (not `static`): LLVM const-folds an immutable all-zero static
// into a read-only section in release builds, so the allocator's writes to
// the arena would silently be ignored. `static mut` guarantees a writable slot.
static mut ARENA: Arena = Arena([0; ARENA_SIZE]);

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

static mut FREE_HEAD: *mut usize = core::ptr::null_mut();
static mut INIT: bool = false;

pub fn current() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

pub fn peak() -> usize {
    PEAK.load(Ordering::Relaxed)
}

/// Resets the counters before a measured segment (excludes runtime noise).
/// Only valid when nothing else is live, i.e. on bare metal before the scenes.
/// Used by main.rs; dead in the host test build.
#[allow(dead_code)]
pub fn reset() {
    CURRENT.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
}

/// Total bytes currently on the free list. Used by the host regression test
/// (tests/alloc_host.rs) to detect arena bytes orphaned from the free list;
/// dead in the bin.
#[allow(dead_code)]
pub fn free_bytes() -> usize {
    let mut total = 0usize;
    let mut cur = unsafe { FREE_HEAD };
    while !cur.is_null() {
        total += unsafe { *cur };
        cur = unsafe { *cur.add(1) as *mut usize };
    }
    total
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Debug-only integrity check: the free list must be sorted, non-overlapping
/// and inside the arena. Catches double-free / overlap corruption early.
#[cfg(debug_assertions)]
unsafe fn validate_free(label: &str) { unsafe {
    let base = core::ptr::addr_of_mut!(ARENA.0).cast::<u8>() as usize;
    let mut prev_end = base;
    let mut cur = FREE_HEAD;
    let mut count = 0;
    while !cur.is_null() {
        let start = cur as usize;
        assert!(start >= base && start < base + ARENA_SIZE, "{label}: free block {start:#x} out of arena");
        assert!(start >= prev_end, "{label}: free block {start:#x} overlaps/precedes previous end {prev_end:#x}");
        assert!(*cur <= ARENA_SIZE, "{label}: block size {} too large", *cur);
        prev_end = start + *cur;
        cur = *cur.add(1) as *mut usize;
        count += 1;
        assert!(count < 1_000_000, "{label}: free list cycle");
    }
}}

unsafe fn init() { unsafe {
    let base = core::ptr::addr_of_mut!(ARENA.0).cast::<u8>() as usize;
    FREE_HEAD = base as *mut usize;
    *FREE_HEAD = ARENA_SIZE;
    *FREE_HEAD.add(1) = 0;
    INIT = true;
}}

/// Insert a free block, keeping the free list sorted by address and coalescing
/// adjacent blocks so freed memory can be reused. The next pointer is stored
/// as a plain usize.
unsafe fn insert_free(addr: usize, size: usize) { unsafe {
    debug_assert!(size <= ARENA_SIZE, "insert_free bad size {size}");
    let mut prev: *mut usize = core::ptr::null_mut();
    let mut cur: *mut usize = FREE_HEAD;
    while !cur.is_null() && (cur as usize) < addr {
        prev = cur;
        cur = *cur.add(1) as *mut usize;
    }
    let mut b = addr as *mut usize;
    *b = size;
    *b.add(1) = cur as usize;
    if prev.is_null() {
        FREE_HEAD = b;
    } else {
        *prev.add(1) = b as usize;
    }
    // Coalesce backward: previous block ends right where this one starts.
    if !prev.is_null() && (prev as usize) + *prev == addr {
        *prev += size;
        *prev.add(1) = *b.add(1);
        b = prev;
    }
    // Coalesce forward: this block ends right where the next one starts.
    let next_addr = *b.add(1);
    let next = next_addr as *mut usize;
    if next_addr != 0 && (b as usize) + *b == next_addr {
        *b += *next;
        *b.add(1) = *next.add(1);
    }
}}

unsafe fn alloc_impl(layout: Layout) -> *mut u8 { unsafe {
    if layout.size() == 0 {
        return core::ptr::NonNull::dangling().as_ptr();
    }
    if !INIT {
        init();
    }
    #[cfg(debug_assertions)]
    validate_free("alloc:entry");
    let align = layout.align().max(HEADER);
    let size = layout.size();
    let mut prev: *mut usize = core::ptr::null_mut();
    let mut cur = FREE_HEAD;
    while !cur.is_null() {
        debug_assert!(*cur <= ARENA_SIZE, "free block size {} > arena", *cur);
        let fb_start = cur as usize;
        let fb_end = fb_start + *cur;
        // Align the payload within the free block. The head fragment between
        // the block start and the payload header must be either empty or
        // large enough to live on as a free block — otherwise those bytes
        // would be orphaned (leaked) from the arena on every aligned alloc.
        // head is a multiple of HEADER and align >= HEADER, so a single
        // align-step bump always turns a too-small fragment into MIN_FREE+.
        let mut payload = align_up(fb_start + HEADER, align);
        let head = payload - HEADER - fb_start;
        if head > 0 && head < MIN_FREE {
            payload += align;
        }
        let block_end = payload + size;
        if block_end <= fb_end {
            let next_addr = *cur.add(1);
            if prev.is_null() {
                FREE_HEAD = next_addr as *mut usize;
            } else {
                *prev.add(1) = next_addr;
            }
            // Return the head fragment (if any) to the free list.
            let head = payload - HEADER - fb_start;
            if head >= MIN_FREE {
                insert_free(fb_start, head);
            }
            // Store the block span right before the payload so dealloc can
            // recover it; the block starts at this header.
            // Split off the tail as a new free block. Its start must be
            // aligned for a usize header, so round up past the payload end.
            // A tail too small for a free block is absorbed into this block
            // (the stored span covers it) instead of being orphaned.
            let remain_start = align_up(block_end, HEADER);
            let remain = fb_end.saturating_sub(remain_start);
            if remain >= MIN_FREE {
                *(payload as *mut usize).sub(1) = size + HEADER;
                insert_free(remain_start, remain);
            } else {
                *(payload as *mut usize).sub(1) = fb_end - (payload - HEADER);
            }
            let cur_bytes = CURRENT.fetch_add(size, Ordering::Relaxed) + size;
            PEAK.fetch_max(cur_bytes, Ordering::Relaxed);
            return payload as *mut u8;
        }
        prev = cur;
        cur = *cur.add(1) as *mut usize;
    }
    report_oom(size)
}}

unsafe fn report_oom(wanted: usize) -> ! { unsafe {
    let mut n = 0;
    let mut total = 0usize;
    let mut max = 0usize;
    let mut cur = FREE_HEAD;
    while !cur.is_null() {
        let s = *cur;
        total = total.saturating_add(s);
        if s > max {
            max = s;
        }
        n += 1;
        cur = *cur.add(1) as *mut usize;
    }
    panic!("OOM want {wanted} B: {n} free blocks, total {total}, largest {max}");
}}

unsafe fn dealloc_impl(ptr: *mut u8, layout: Layout) { unsafe {
    if layout.size() == 0 {
        return;
    }
    CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    let block = (ptr as usize) - HEADER;
    let size = *(block as *const usize);
    insert_free(block, size);
    #[cfg(debug_assertions)]
    validate_free("dealloc:exit");
}}

pub struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { unsafe {
        alloc_impl(layout)
    }}
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { unsafe {
        dealloc_impl(ptr, layout);
    }}
}

#[global_allocator]
static G: Counting = Counting;
