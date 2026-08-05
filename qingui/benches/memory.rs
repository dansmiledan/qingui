//! Memory benchmark: static type sizes + peak heap of representative scenes.
//!
//! NOTE: this runs on the host (64-bit, usize = 8B). The embedded thumbv7
//! target is 32-bit (usize = 4B), so absolute numbers differ. This bench gives
//! the RELATIVE cost shape and a regression gate; absolute embedded sizes come
//! from `cargo size --target thumbv7em-none-eabihf`.
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
    println!("4 x Style     {:>6} B", 4 * size_of::<Style>());
    println!("Node          {:>6} B", size_of::<Node>());
    println!("WidgetKind    {:>6} B", size_of::<WidgetKind>());
    println!("  largest state (ItemListState) = {} B", size_of::<itemlist::ItemListState>());
    println!("  discriminator overhead = {} B (WidgetKind - largest state)", size_of::<WidgetKind>() - size_of::<itemlist::ItemListState>());
    println!("  NOTE: every node carries WidgetKind ({} B) for kind regardless of its state", size_of::<WidgetKind>());
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
}

enum Tier { Small, Medium, Large }

fn build_scene(tier: Tier) -> qingui::Ui {
    use qingui::prelude::*;
    use qingui::widgets::button::ButtonBuilder;
    use qingui::widgets::chart::ChartBuilder;
    use qingui::widgets::itemlist::ItemListBuilder;
    use qingui::widgets::list::ListBuilder;
    use qingui::widgets::slider::SliderBuilder;
    use qingui::{Color, Ui};

    let (n_items, n_chart_pts) = match tier {
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
    // Keep handles alive so the built tree (and its allocations) stays resident.
    let _list = ListBuilder::new(&refs).build(&mut ui, scr);
    for i in 0..n_items {
        ButtonBuilder::new(&format!("btn{i}")).build(&mut ui, scr);
    }
    for _ in 0..n_items / 4 {
        SliderBuilder::new(0, 100).build(&mut ui, scr);
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
    let nodes = node_count(&ui);
    let peak = peak();
    let live = current();
    drop(ui);
    println!("{label:<8} {nodes:>5} nodes  peak {peak:>9} B  live {live:>9} B");
}

fn main() {
    report_static_sizes();
    bench_scene("small", Tier::Small);
    bench_scene("medium", Tier::Medium);
    bench_scene("large", Tier::Large);
}
