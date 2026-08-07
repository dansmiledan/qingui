#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

//! 32-bit memory bench for qingui, run on QEMU (`-machine mps2-an386`,
//! Cortex-M4F, thumbv7em-none-eabihf).
//!
//! Prints the same static size table and peak-heap scene table as the host
//! bench (`cargo bench -p qingui --bench memory`) but with the real 32-bit
//! ABI (usize = 4B), so absolute embedded sizes are meaningful. Semihosting
//! carries the output; the exit code reflects whether all asserts passed.
//!
//! Build/run for the target:
//!
//! ```text
//! cargo run --target thumbv7em-none-eabihf
//! ```
//!
//! On host builds (workspace `cargo test`/`cargo build`) this compiles to a
//! stub so the crate stays a clean workspace member.

#[cfg(target_arch = "arm")]
extern crate alloc;

#[cfg(target_arch = "arm")]
mod allocator;
#[cfg(target_arch = "arm")]
mod scene;

#[cfg(target_arch = "arm")]
use core::mem::size_of;
#[cfg(target_arch = "arm")]
use cortex_m_rt::entry;
#[cfg(target_arch = "arm")]
use cortex_m_semihosting::debug::{exit, EXIT_FAILURE, EXIT_SUCCESS};
#[cfg(target_arch = "arm")]
use cortex_m_semihosting::hprintln;
#[cfg(target_arch = "arm")]
use qingui::geometry::{Color, Point, Rect};
#[cfg(target_arch = "arm")]
use qingui::node::Node;
#[cfg(target_arch = "arm")]
use qingui::style::{ResolvedStyle, Style};
#[cfg(target_arch = "arm")]
use qingui::widgets::{
    arc, bar, button, chart, checkbox, custom, dropdown, image, itemlist, label, led, list,
    msgbox, obj, roller, scrollview, slider, spinbox, spinner, switch, table,
};
#[cfg(target_arch = "arm")]
use qingui::widgets::WidgetKind;

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = hprintln!("PANIC: {}", info);
    exit(EXIT_FAILURE);
    loop {}
}

#[cfg(target_arch = "arm")]
fn report_static_sizes() {
    hprintln!("== static sizes (thumbv7em-none-eabihf, 32-bit) ==");
    hprintln!("Rect          {:>6} B", size_of::<Rect>());
    hprintln!("Point         {:>6} B", size_of::<Point>());
    hprintln!("Color         {:>6} B", size_of::<Color>());
    hprintln!("Style         {:>6} B", size_of::<Style>());
    hprintln!("ResolvedStyle {:>6} B", size_of::<ResolvedStyle>());
    hprintln!("4 x Style (old inline cost) {:>6} B", 4 * size_of::<Style>());
    hprintln!("Node          {:>6} B", size_of::<Node>());
    hprintln!("WidgetKind    {:>6} B", size_of::<WidgetKind>());
    let max_state = [
        size_of::<arc::ArcState>(),
        size_of::<bar::BarState>(),
        size_of::<button::ButtonState>(),
        size_of::<chart::ChartState>(),
        size_of::<checkbox::CheckboxState>(),
        size_of::<custom::CustomState>(),
        size_of::<dropdown::DropdownState>(),
        size_of::<image::ImageState>(),
        size_of::<label::LabelState>(),
        size_of::<led::LedState>(),
        size_of::<msgbox::MsgboxState>(),
        size_of::<obj::ObjState>(),
        size_of::<scrollview::ScrollViewState>(),
        size_of::<slider::SliderState>(),
        size_of::<spinbox::SpinboxState>(),
        size_of::<spinner::SpinnerState>(),
        size_of::<switch::SwitchState>(),
        size_of::<table::TableState>(),
    ]
    .into_iter()
    .max()
    .unwrap();
    hprintln!("  largest inline state  = {} B", max_state);
    hprintln!("  discriminator overhead = {} B (WidgetKind - largest inline state)", size_of::<WidgetKind>() - max_state);
    hprintln!("  NOTE: List/ItemList/Roller states are boxed (heap-allocated), so the enum stays small for every node");
    macro_rules! row {
        ($name:literal, $t:ty) => {
            hprintln!("  {:<14} {:>6} B", $name, size_of::<$t>());
        };
    }
    row!("Obj", obj::ObjState);
    row!("Label", label::LabelState);
    row!("Button", button::ButtonState);
    row!("Slider", slider::SliderState);
    row!("Switch", switch::SwitchState);
    row!("Bar", bar::BarState);
    row!("List", list::ListState);
    row!("Arc", arc::ArcState);
    row!("Checkbox", checkbox::CheckboxState);
    row!("Chart", chart::ChartState);
    row!("Spinner", spinner::SpinnerState);
    row!("Msgbox", msgbox::MsgboxState);
    row!("Led", led::LedState);
    row!("Table", table::TableState);
    row!("Spinbox", spinbox::SpinboxState);
    row!("Roller", roller::RollerState);
    row!("ScrollView", scrollview::ScrollViewState);
    row!("Dropdown", dropdown::DropdownState);
    row!("Image", image::ImageState);
    row!("ItemList", itemlist::ItemListState);
    row!("Custom", custom::CustomState);
    hprintln!("Ui            {:>6} B", size_of::<qingui::Ui>());
    // Regression gates: QEMU 32-bit baselines x2 (see docs/BENCHMARK.md).
    assert!(size_of::<WidgetKind>() < 48, "WidgetKind {} B exceeds limit", size_of::<WidgetKind>());
    assert!(size_of::<Style>() < 280, "Style {} B exceeds limit", size_of::<Style>());
    assert!(size_of::<Node>() < 560, "Node {} B exceeds limit", size_of::<Node>());
}

// Regression gates: QEMU 32-bit baselines x2, recalibrated 2026-08-07 to the
// numbers this tool actually prints (they were previously copied from the
// host 64-bit bench, leaving >2x slack). See docs/BENCHMARK.md and spec
// docs/superpowers/specs/2026-08-05-memory-bench-design.md.
#[cfg(target_arch = "arm")]
const LIMIT_PEAK_MINIMAL: usize = 10_862; // 2 * 5431
#[cfg(target_arch = "arm")]
const LIMIT_LIVE_MINIMAL: usize = 10_622; // 2 * 5311
#[cfg(target_arch = "arm")]
const LIMIT_PEAK_SMALL: usize = 64_410; // 2 * 32205
#[cfg(target_arch = "arm")]
const LIMIT_LIVE_SMALL: usize = 61_698; // 2 * 30849
#[cfg(target_arch = "arm")]
const LIMIT_PEAK_MEDIUM: usize = 118_952; // 2 * 59476
#[cfg(target_arch = "arm")]
const LIMIT_LIVE_MEDIUM: usize = 104_760; // 2 * 52380
#[cfg(target_arch = "arm")]
const LIMIT_PEAK_LARGE: usize = 331_032; // 2 * 165516
#[cfg(target_arch = "arm")]
const LIMIT_LIVE_LARGE: usize = 257_176; // 2 * 128588

#[cfg(target_arch = "arm")]
fn bench_scene(label: &str, tier: scene::Tier) {
    // reset() is valid here: bare metal, nothing else is live at this point.
    allocator::reset();
    let scene::Scene { ui, .. } = scene::build_scene(tier);
    let peak = allocator::peak();
    let live = allocator::current();
    let nodes = scene::node_count(&ui);
    drop(ui);
    hprintln!("{:<8} {:>5} nodes  peak {:>9} B  live {:>9} B", label, nodes, peak, live);
    let (peak_limit, live_limit) = match tier {
        scene::Tier::Minimal => (LIMIT_PEAK_MINIMAL, LIMIT_LIVE_MINIMAL),
        scene::Tier::Small => (LIMIT_PEAK_SMALL, LIMIT_LIVE_SMALL),
        scene::Tier::Medium => (LIMIT_PEAK_MEDIUM, LIMIT_LIVE_MEDIUM),
        scene::Tier::Large => (LIMIT_PEAK_LARGE, LIMIT_LIVE_LARGE),
    };
    assert!(peak < peak_limit, "{label}: peak {peak} B exceeds {peak_limit} B");
    assert!(live < live_limit, "{label}: live {live} B exceeds {live_limit} B");
}

#[cfg(target_arch = "arm")]
#[entry]
fn main() -> ! {
    report_static_sizes();
    bench_scene("minimal", scene::Tier::Minimal);
    bench_scene("small", scene::Tier::Small);
    bench_scene("medium", scene::Tier::Medium);
    bench_scene("large", scene::Tier::Large);
    exit(EXIT_SUCCESS);
    loop {}
}

#[cfg(not(target_arch = "arm"))]
fn main() {
    println!("qemu-mem targets the bare-metal Cortex-M4F; build and run it for the embedded target:");
    println!("  cargo run --target thumbv7em-none-eabihf");
}
