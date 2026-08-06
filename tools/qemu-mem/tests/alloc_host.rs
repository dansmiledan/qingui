//! Host regression test for the counting free-list allocator.
//!
//! Runs the same scene-building code as the QEMU bench on the host so the
//! allocator logic is exercised identically. Catches free-list corruption /
//! OOM fast without a target. A single test function keeps the measurement
//! window free of other threads (a second test thread blocked on a mutex
//! lazily allocates, which would pollute the live-byte delta).

#[path = "../src/allocator.rs"]
mod allocator;

extern crate alloc;

use allocator::{current, peak};
use qingui::prelude::*;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::chart::ChartCfg;
use qingui::widgets::itemlist::ItemListCfg;
use qingui::widgets::label::LabelCfg;
use qingui::widgets::list::ListCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::{Color, Ui};

fn build_scene(tier: Tier) -> Ui {
    let (n_items, n_chart_pts) = match tier {
        Tier::Minimal => {
            let mut ui = Ui::new(160, 120, 8);
            let scr = ui.screen();
            LabelCfg::new("hello").build(&mut ui, scr);
            ButtonCfg::new("OK").build(&mut ui, scr);
            ui.tick_inc(16);
            ui.timer_handler();
            return ui;
        }
        Tier::Small => (5, 16),
        Tier::Medium => (20, 64),
        Tier::Large => (60, 256),
    };
    let mut ui = Ui::new(320, 240, 24);
    let scr = ui.screen();
    let texts: Vec<String> = (0..n_items).map(|i| format!("item{i}")).collect();
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let _list = ListCfg::new(&refs).build(&mut ui, scr);
    for i in 0..n_items {
        ButtonCfg::new(&format!("btn{i}")).build(&mut ui, scr);
    }
    for _ in 0..n_items / 4 {
        SliderCfg::new(0, 100).build(&mut ui, scr);
    }
    let _chart = ChartCfg::new().series(Color::RED, n_chart_pts).build(&mut ui, scr);
    let _il = ItemListCfg::new().build(&mut ui, scr);
    for _ in 0..n_items {
        ui.itemlist_add_item(_il);
    }
    for _ in 0..5 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    ui
}

#[derive(Clone, Copy, Debug)]
enum Tier {
    Minimal,
    Small,
    Medium,
    Large,
}

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
        let ui = build_scene(tier);
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
}
