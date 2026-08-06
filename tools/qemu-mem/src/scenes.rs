//! Peak-heap scenes, ported from the host bench (`qingui/benches/memory.rs`)
//! so both produce the same relative cost shape on 32-bit.
//!
//! Uses the counting allocator in `super::allocator`; output goes over
//! semihosting so the numbers show up on the QEMU console.

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use cortex_m_semihosting::hprintln;

use qingui::prelude::*;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::chart::ChartCfg;
use qingui::widgets::itemlist::ItemListCfg;
use qingui::widgets::label::LabelCfg;
use qingui::widgets::list::ListCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::{Color, Ui};

use crate::allocator::{current, peak, reset};

// Regression gates. Host 64-bit baselines x2; 32-bit numbers are lower, so
// these still bound the embedded build (see spec
// docs/superpowers/specs/2026-08-05-memory-bench-design.md).
const LIMIT_PEAK_MINIMAL: usize = 11_742;
const LIMIT_LIVE_MINIMAL: usize = 11_502;
const LIMIT_PEAK_SMALL: usize = 70_138;
const LIMIT_LIVE_SMALL: usize = 66_090;
const LIMIT_PEAK_MEDIUM: usize = 141_600;
const LIMIT_LIVE_MEDIUM: usize = 121_472;
const LIMIT_PEAK_LARGE: usize = 419_152;
const LIMIT_LIVE_LARGE: usize = 318_992;

#[derive(Clone, Copy)]
pub enum Tier {
    Minimal,
    Small,
    Medium,
    Large,
}

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
    // ListCfg::new takes &[&str]; build the label strings first (their
    // allocation is counted, which is representative of real use).
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
    // Force real allocations through layout / render / animation paths.
    for _ in 0..5 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    ui
}

fn node_count(ui: &Ui) -> usize {
    let mut n = 0;
    let mut stack = vec![ui.screen()];
    while let Some(o) = stack.pop() {
        n += 1;
        stack.extend(ui.children(o));
    }
    n
}

pub fn bench_scene(label: &str, tier: Tier) {
    reset();
    let ui = build_scene(tier);
    let peak = peak();
    let live = current();
    let nodes = node_count(&ui);
    drop(ui);
    hprintln!("{:<8} {:>5} nodes  peak {:>9} B  live {:>9} B", label, nodes, peak, live);
    let (peak_limit, live_limit) = match tier {
        Tier::Minimal => (LIMIT_PEAK_MINIMAL, LIMIT_LIVE_MINIMAL),
        Tier::Small => (LIMIT_PEAK_SMALL, LIMIT_LIVE_SMALL),
        Tier::Medium => (LIMIT_PEAK_MEDIUM, LIMIT_LIVE_MEDIUM),
        Tier::Large => (LIMIT_PEAK_LARGE, LIMIT_LIVE_LARGE),
    };
    assert!(peak < peak_limit, "{label}: peak {peak} B exceeds {peak_limit} B");
    assert!(live < live_limit, "{label}: live {live} B exceeds {live_limit} B");
}
