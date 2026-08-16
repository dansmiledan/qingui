//! Shared benchmark scene builder — single source of truth for:
//! - the host memory bench (`qingui/benches/memory.rs`, via `#[path]`),
//! - the QEMU memory tool (`tools/qemu-mem`, `mod scene;`),
//! - the host allocator regression test (`tools/qemu-mem/tests/alloc_host.rs`),
//! - the runtime bench scenes (`tools/qemu-time/src/scenes.rs` wraps it),
//! so memory and runtime numbers always come from the same scene.
//!
//! no_std + alloc only; no semihosting / I/O so it compiles on every target.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use qingui::prelude::*;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::chart::ChartCfg;
use qingui::widgets::itemlist::ItemListCfg;
use qingui::widgets::label::LabelCfg;
use qingui::widgets::list::ListCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::{Color, ObjRef, Ui};

#[derive(Clone, Copy, Debug)]
pub enum Tier {
    Minimal,
    Small,
    Medium,
    Large,
}

/// A built scene plus a leaf widget handle (only the runtime bench reads
/// `leaf`, for partial-dirty timing; the memory benches ignore it).
pub struct Scene {
    pub ui: Ui,
    #[allow(dead_code)] // unread in the memory-bench consumers of this file
    pub leaf: ObjRef,
}

/// Scene per tier: a list, `n_items` buttons, `n_items/4` sliders, a chart
/// with `n_chart_pts` points and an item list, plus a few timer ticks to
/// force real allocations through the layout / render / animation paths.
pub fn build_scene(tier: Tier) -> Scene {
    let (n_items, n_chart_pts) = match tier {
        Tier::Minimal => {
            let mut ui = Ui::new(160, 120, 8);
            let scr = ui.screen();
            LabelCfg::new("hello").build(&mut ui, scr);
            let leaf = ButtonCfg::new("OK").build(&mut ui, scr);
            ui.tick_inc(16);
            ui.timer_handler();
            return Scene { ui, leaf };
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
    let mut leaf = None;
    for i in 0..n_items {
        let b = ButtonCfg::new(&format!("btn{i}")).build(&mut ui, scr);
        if leaf.is_none() {
            leaf = Some(b);
        }
    }
    for _ in 0..n_items / 4 {
        SliderCfg::new(0, 100).build(&mut ui, scr);
    }
    let _chart = ChartCfg::new().series(Color::new(255, 0, 0), n_chart_pts).build(&mut ui, scr);
    let _il = ItemListCfg::new().build(&mut ui, scr);
    for _ in 0..n_items {
        ui.itemlist_add_item(_il);
    }
    for _ in 0..5 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    Scene { ui, leaf: leaf.unwrap() }
}

#[allow(dead_code)] // unused in the qemu-time consumer of this file
pub fn node_count(ui: &Ui) -> usize {
    let mut n = 0;
    let mut stack = vec![ui.screen()];
    while let Some(o) = stack.pop() {
        n += 1;
        stack.extend(ui.children(o));
    }
    n
}
