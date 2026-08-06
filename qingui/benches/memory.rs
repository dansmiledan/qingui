//! Memory benchmark: static type sizes + peak heap of representative scenes.
//!
//! NOTE: this runs on the host (64-bit, usize = 8B). The embedded thumbv7
//! target is 32-bit (usize = 4B). On thumbv7 the usize-dependent parts
//! (Vec/String/Box/pointers) roughly halve, but i32/u32-fixed parts (Rect,
//! ObjRef, and Style's Option<i32> fields) do not — expect ~20-30% lower,
//! not a full halving. Absolute embedded sizes come from
//! `cargo size --target thumbv7em-none-eabihf`. This bench gives the
//! RELATIVE cost shape and a regression gate.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counting allocator: tracks current live bytes and the running peak.
struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let cur = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(cur, Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static G: Counting = Counting;

fn current() -> usize { CURRENT.load(Ordering::Relaxed) }
fn peak() -> usize { PEAK.load(Ordering::Relaxed) }
/// Resets the counters before a measured segment (excludes std runtime noise).
fn reset() {
    CURRENT.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
}

// Thresholds recalibrated 2026-08-05 after the memory optimization: new baseline x 2.
// See spec docs/superpowers/specs/2026-08-05-memory-bench-design.md.
const LIMIT_WIDGETKIND: usize = 80;
const LIMIT_STYLE: usize = 336;
const LIMIT_NODE: usize = 752;
const LIMIT_PEAK_MINIMAL: usize = 11_742;
const LIMIT_LIVE_MINIMAL: usize = 11_502;
const LIMIT_PEAK_SMALL: usize = 70_138;
const LIMIT_LIVE_SMALL: usize = 66_090;
const LIMIT_PEAK_MEDIUM: usize = 141_600;
const LIMIT_LIVE_MEDIUM: usize = 121_472;
const LIMIT_PEAK_LARGE: usize = 419_152;
const LIMIT_LIVE_LARGE: usize = 318_992;

fn report_static_sizes() {
    use core::mem::size_of;
    use qingui::geometry::{Color, Point, Rect};
    use qingui::node::Node;
    use qingui::style::{ResolvedStyle, Style};
    use qingui::widgets::{
        arc, bar, button, chart, checkbox, custom, dropdown, image, itemlist, label, led,
        list, msgbox, obj, roller, scrollview, slider, spinbox, spinner, switch, table,
    };
    use qingui::widgets::WidgetKind;

    println!("== static sizes (host 64-bit) ==");
    println!("Rect          {:>6} B", size_of::<Rect>());
    println!("Point         {:>6} B", size_of::<Point>());
    println!("Color         {:>6} B", size_of::<Color>());
    println!("Style         {:>6} B", size_of::<Style>());
    println!("ResolvedStyle {:>6} B", size_of::<ResolvedStyle>());
    println!("4 x Style (old inline cost) {:>6} B", 4 * size_of::<Style>());
    println!("Node          {:>6} B", size_of::<Node>());
    println!("WidgetKind    {:>6} B", size_of::<WidgetKind>());
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
    println!("  largest inline state  = {max_state} B");
    println!("  discriminator overhead = {} B (WidgetKind - largest inline state)", size_of::<WidgetKind>() - max_state);
    println!("  NOTE: List/ItemList/Roller states are boxed (heap-allocated), so the enum stays small for every node");
    macro_rules! row {
        ($name:literal, $t:ty) => { println!("  {:<14} {:>6} B", $name, size_of::<$t>()); };
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
    println!("Ui            {:>6} B", size_of::<qingui::Ui>());
    assert!(size_of::<WidgetKind>() < LIMIT_WIDGETKIND, "WidgetKind {} B exceeds limit", size_of::<WidgetKind>());
    assert!(size_of::<Style>() < LIMIT_STYLE, "Style {} B exceeds limit", size_of::<Style>());
    assert!(size_of::<Node>() < LIMIT_NODE, "Node {} B exceeds limit", size_of::<Node>());
}

#[derive(Clone, Copy)]
enum Tier { Minimal, Small, Medium, Large }

fn build_scene(tier: Tier) -> qingui::Ui {
    use qingui::prelude::*;
    use qingui::widgets::button::ButtonCfg;
    use qingui::widgets::chart::ChartBuilder;
    use qingui::widgets::itemlist::ItemListBuilder;
    use qingui::widgets::label::LabelCfg;
    use qingui::widgets::list::ListBuilder;
    use qingui::widgets::slider::SliderCfg;
    use qingui::{Color, Ui};

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
    // ListBuilder::new takes &[&str]; build the label strings first (their allocation
    // is counted, which is representative of real use). Same pattern as dropdown.rs.
    let texts: Vec<String> = (0..n_items).map(|i| format!("item{i}")).collect();
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    // Bind the ObjRef handles and label strings to silence unused-variable warnings;
    // the tree itself stays resident in the Ui arena held by the caller.
    let _list = ListBuilder::new(&refs).build(&mut ui, scr);
    for i in 0..n_items {
        ButtonCfg::new(&format!("btn{i}")).build(&mut ui, scr);
    }
    for _ in 0..n_items / 4 {
        SliderCfg::new(0, 100).build(&mut ui, scr);
    }
    let _chart = ChartBuilder::new().series(Color::RED, n_chart_pts).build(&mut ui, scr);
    let _il = ItemListBuilder::new().build(&mut ui, scr);
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

fn node_count(ui: &qingui::Ui) -> usize {
    let mut n = 0;
    let mut stack = vec![ui.screen()];
    while let Some(o) = stack.pop() {
        n += 1;
        stack.extend(ui.children(o));
    }
    n
}

fn bench_scene(label: &str, tier: Tier) {
    reset();
    let ui = build_scene(tier);
    let peak = peak();
    let live = current();
    let nodes = node_count(&ui);
    drop(ui);
    println!("{label:<8} {nodes:>5} nodes  peak {peak:>9} B  live {live:>9} B");
    let (peak_limit, live_limit) = match tier {
        Tier::Minimal => (LIMIT_PEAK_MINIMAL, LIMIT_LIVE_MINIMAL),
        Tier::Small => (LIMIT_PEAK_SMALL, LIMIT_LIVE_SMALL),
        Tier::Medium => (LIMIT_PEAK_MEDIUM, LIMIT_LIVE_MEDIUM),
        Tier::Large => (LIMIT_PEAK_LARGE, LIMIT_LIVE_LARGE),
    };
    assert!(peak < peak_limit, "{label}: peak {peak} B exceeds {peak_limit} B");
    assert!(live < live_limit, "{label}: live {live} B exceeds {live_limit} B");
}

fn main() {
    report_static_sizes();
    bench_scene("minimal", Tier::Minimal);
    bench_scene("small", Tier::Small);
    bench_scene("medium", Tier::Medium);
    bench_scene("large", Tier::Large);
}
