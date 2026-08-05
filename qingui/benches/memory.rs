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
    println!("  largest-variant tax = {} B (WidgetKind - ObjState)", size_of::<WidgetKind>() - size_of::<obj::ObjState>());
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

fn main() {
    report_static_sizes();
}
