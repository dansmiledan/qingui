mod sim;
mod images;

use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir, Grid, Sizing, Track};
use qingui::prelude::*;
use qingui::style::{Layout, Style};
use qingui::widgets::arc::ArcCfg;
use qingui::widgets::bar::BarCfg;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::chart::ChartBuilder;
use qingui::widgets::checkbox::CheckboxCfg;
use qingui::widgets::dropdown::DropdownBuilder;
use qingui::widgets::image::ImageBuilder;
use qingui::widgets::itemlist::ItemListBuilder;
use qingui::widgets::label::LabelCfg;
use qingui::widgets::led::LedBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::msgbox::MsgboxBuilder;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::roller::RollerBuilder;
use qingui::widgets::scrollview::ScrollViewBuilder;
use qingui::widgets::slider::SliderCfg;
use qingui::widgets::spinbox::SpinboxCfg;
use qingui::widgets::spinner::SpinnerBuilder;
use qingui::widgets::switch::SwitchCfg;
use qingui::widgets::table::TableBuilder;
use qingui::{Color, EventKind, ObjRef, Ui};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    // build and tick share references to the two charts (streaming data source)
    let charts: Rc<RefCell<Vec<ObjRef>>> = Rc::new(RefCell::new(Vec::new()));
    let charts_tick = charts.clone();
    let mut frame = 0u32;
    sim::run_with_tick(move |ui| build(ui, &charts), move |ui| {
        // At 60fps, push every 6 frames (~100ms): two sine waves with a π/2 phase offset
        frame += 1;
        if frame % 6 != 0 {
            return;
        }
        let t = (frame / 6) as f32 * 0.15;
        let cs = charts_tick.borrow();
        if cs.len() == 2 {
            ui.chart_push(cs[0], 0, (50.0 + t.sin() * 45.0) as i32);
            ui.chart_push(cs[1], 0, (50.0 + t.cos() * 45.0) as i32);
        }
    });
}

fn column() -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Column, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    })
}

/// Transparent container style (layout only, no background drawn)
fn transparent() -> Style {
    let mut s = Style::default();
    s.bg_opa = Some(0);
    s
}

pub fn build(ui: &mut Ui, charts: &Rc<RefCell<Vec<ObjRef>>>) {
    let screen = ui.screen();

    // Screen-level Grid: title row (content height) + main row (Fr); fixed-width menu on the left, adaptive panel on the right
    let mut ss = qingui::style::theme_screen();
    ss.pad_left = Some(8);
    ss.pad_top = Some(8);
    ss.pad_right = Some(8);
    ss.pad_bottom = Some(8);
    ss.layout = Some(Layout::Grid(Grid {
        cols: vec![Track::Px(108), Track::Fr(1)],
        rows: vec![Track::Content, Track::Fr(1)],
        col_gap: 8,
        row_gap: 8,
    }));
    ui.set_style(screen, ss);

    let title = LabelCfg::new("qingui demo").build(ui, screen);
    ui.set_grid_cell(title, (0, 2), (0, 1));

    let menu = ListBuilder::new(&["Settings", "About", "Animate", "LongList", "P1 Demo", "ItemList"])
        .build(ui, screen);
    ui.set_grid_cell(menu, (0, 1), (1, 1));
    ui.set_sizing(menu, Some(Sizing::GROW), Some(Sizing::GROW));

    let panel = ObjCfg::new().build(ui, screen);
    ui.set_grid_cell(panel, (1, 1), (1, 1));
    ui.set_style(panel, qingui::style::theme_obj());
    ui.set_sizing(panel, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(panel, column());

    // ---- Settings page: Slider + Switch + preview Bar ----
    let page_settings = ObjCfg::new().build(ui, panel);
    ui.set_style(page_settings, transparent());
    ui.set_sizing(page_settings, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_settings, column());
    let l1 = LabelCfg::new("Brightness").build(ui, page_settings);
    let _ = l1;
    let slider = SliderCfg::new(0, 100)
        .size(160, 12)
        .value(30)
        .build(ui, page_settings);
    let l2 = LabelCfg::new("Enabled").build(ui, page_settings);
    let _ = l2;
    let sw = SwitchCfg::new().build(ui, page_settings);
    let cb = CheckboxCfg::new("Notify me").build(ui, page_settings);
    let l3 = LabelCfg::new("Preview").build(ui, page_settings);
    let _ = l3;
    let preview = BarCfg::new(0, 100)
        .size(160, 10)
        .value(30)
        .build(ui, page_settings);
    // Slider value change → animation drives the preview Bar (demonstrates animation linking to widget values)
    ui.add_event_cb(slider, EventKind::ValueChanged, Box::new(move |ui, s, _| {
        let v = ui.value(s);
        let cur = ui.value(preview);
        ui.anim_start(Anim::new(preview, AnimProp::Value, cur, v, 300));
    }));

    // ---- About page: Wide button + ScrollView (About text and two images scroll past the visible viewport) ----
    let page_about = ObjCfg::new().build(ui, panel);
    ui.set_style(page_about, transparent());
    ui.set_sizing(page_about, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_about, column());
    // Layout transition demo: toggling the left menu column width smoothly re-lays out the UI
    let wide = std::cell::Cell::new(false);
    let wide_btn = ButtonCfg::new("Wide").build(ui, page_about);
    ui.add_event_cb(wide_btn, EventKind::Clicked, Box::new(move |ui, _b, _| {
        let w = !wide.get();
        wide.set(w);
        let scr = ui.screen();
        ui.set_layout(scr, Layout::Grid(Grid {
            cols: vec![Track::Px(if w { 180 } else { 108 }), Track::Fr(1)],
            rows: vec![Track::Content, Track::Fr(1)],
            col_gap: 8,
            row_gap: 8,
        }));
    }));
    // Scrolling content: text + image widget examples (static image + gif frame animation);
    // When content exceeds the viewport, focus the ScrollView and scroll with Up/Down
    let sv = ScrollViewBuilder::new().build(ui, page_about);
    ui.set_sizing(sv, Some(Sizing::GROW), Some(Sizing::GROW));
    let sv_content = ui.scrollview_content(sv).unwrap();
    // Multi-font demo: default FONT_6X10 side by side with an overridden FONT_10X20
    let small = LabelCfg::new("FONT_6X10 small").build(ui, sv_content);
    let mut big_style = qingui::style::Style::default();
    big_style.font = Some(&embedded_graphics::mono_font::ascii::FONT_10X20);
    let big = LabelCfg::new("FONT_10X20").build(ui, sv_content);
    ui.set_style(big, big_style);
    let _ = small;
    let la = LabelCfg::new(
        "qingui subset\nPFB + dirty rect\nanim + keypad\n\narrows/tab: move\nenter: select/edit\nesc: exit edit",
    )
    .build(ui, sv_content);
    let _ = la;
    let _logo = ImageBuilder::new(&images::HAIZEI).build(ui, sv_content);
    let _anim = ImageBuilder::new(&images::MIAO).build(ui, sv_content);

    // ---- Animate page: infinitely ping-ponging Bar + arc gauge ----
    let page_animate = ObjCfg::new().build(ui, panel);
    ui.set_style(page_animate, transparent());
    ui.set_sizing(page_animate, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_animate, column());
    let bar = BarCfg::new(0, 100).build(ui, page_animate);
    ui.set_size(bar, 160, 10);
    ui.anim_start(
        Anim::new(bar, AnimProp::Value, 0, 100, 1200)
            .easing(Easing::EaseInOutQuad)
            .repeat(-1)
            .playback(true),
    );

    // Arc dial: driven by a value animation (infinite loop 0..360)
    let arc = ArcCfg::new(0, 360).build(ui, page_animate);
    ui.set_sizing(arc, Some(Sizing::GROW), None);
    ui.set_aspect(arc, Some(1000)); // 1:1
    ui.anim_start(Anim::new(arc, AnimProp::Value, 0, 360, 2400).repeat(-1));

    let spinner = SpinnerBuilder::new().build(ui, page_animate);
    let _ = spinner;

    // ---- LongList page: 20-item long list + add/del buttons ----
    let page_longlist = ObjCfg::new().build(ui, panel);
    ui.set_style(page_longlist, transparent());
    ui.set_sizing(page_longlist, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_longlist, column());
    let long_list = ListBuilder::new(&[
        "Item 01", "Item 02", "Item 03", "Item 04", "Item 05",
        "Item 06", "Item 07", "Item 08", "Item 09", "Item 10",
        "Item 11", "Item 12", "Item 13", "Item 14", "Item 15",
        "Item 16", "Item 17", "Item 18", "Item 19", "Item 20",
    ])
    .build(ui, page_longlist);
    ui.set_size(long_list, 160, 5 * 16 + 2);

    let btn_row = ObjCfg::new().build(ui, page_longlist);
    ui.set_style(btn_row, transparent());
    ui.set_size(btn_row, 160, 28);
    ui.set_layout(btn_row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    }));
    let add_btn = ButtonCfg::new("Add").build(ui, btn_row);
    let del_btn = ButtonCfg::new("Del").build(ui, btn_row);

    // Add: insert below the selected item (fade in + items below slide down), the demo caps at 20 items
    let next_n = std::cell::Cell::new(21i32);
    ui.add_event_cb(add_btn, EventKind::Clicked, Box::new(move |ui, _b, _| {
        if ui.list_len(long_list) >= 20 {
            return;
        }
        let idx = ui.list_selected(long_list) + 1;
        let name = format!("Item {:02}", next_n.get());
        ui.list_insert(long_list, idx, &name);
        next_n.set(next_n.get() + 1);
    }));
    // Del: delete the selected item (fade out + items below slide up)
    ui.add_event_cb(del_btn, EventKind::Clicked, Box::new(move |ui, _b, _| {
        ui.list_remove(long_list);
    }));

    // Clicking a LongList item → Msgbox (modal message box)
    ui.add_event_cb(long_list, EventKind::Clicked, Box::new(move |ui, l, _| {
        let idx = ui.list_selected(l);
        let screen = ui.screen();
        let prev = ui.focused();
        let mb = MsgboxBuilder::new(
            "Clicked",
            &format!("Item {:02}", idx + 1),
        )
        .buttons(&["OK"])
        .build(ui, screen);
        // Restore focus after it closes
        ui.add_event_cb(mb, EventKind::ValueChanged, Box::new(move |ui, _t, _| {
            if let Some(p) = prev {
                ui.group_focus(p);
            }
        }));
    }));

    // ---- P1 Demo page: Roller / Dropdown / Spinbox / LED / Table ----
    let page_p1 = ObjCfg::new().build(ui, panel);
    ui.set_style(page_p1, transparent());
    ui.set_sizing(page_p1, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_p1, column());
    let roller = RollerBuilder::new(&["One", "Two", "Three", "Four", "Five"])
        .size(90, 56)
        .build(ui, page_p1);
    let dropdown = DropdownBuilder::new(&["Red", "Green", "Blue"]).build(ui, page_p1);
    let spinbox = SpinboxCfg::new(0, 999, 3).build(ui, page_p1);
    let led_row = ObjCfg::new().build(ui, page_p1);
    ui.set_style(led_row, transparent());
    ui.set_size(led_row, 120, 18);
    ui.set_layout(led_row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Center, track: Align::Start, gap: 6,
    }));
    let led = LedBuilder::new(Color::rgb(60, 180, 90)).build(ui, led_row);
    let _led_lbl = LabelCfg::new("status").build(ui, led_row);
    // LED brightness follows the spinbox value (demonstrates widget linkage)
    ui.add_event_cb(spinbox, EventKind::ValueChanged, Box::new(move |ui, sb, _| {
        let v = ui.value(sb);
        ui.set_value(led, v * 255 / 999);
    }));
    let _table = TableBuilder::new(3, 2)
        .cell(0, 0, "id")
        .cell(0, 1, "val")
        .cell(0, 2, "unit")
        .cell(1, 0, "01")
        .cell(1, 1, "42")
        .cell(1, 2, "ms")
        .build(ui, page_p1);

    // ---- ItemList page: two streaming charts (top/bottom) + an ItemList of 3-control items ----
    let page_itemlist = ObjCfg::new().build(ui, panel);
    ui.set_style(page_itemlist, transparent());
    ui.set_sizing(page_itemlist, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_itemlist, column());

    // Two line charts (top/bottom): different colors, data pushed periodically by main's tick
    let chart1 = ChartBuilder::new()
        .range(0, 100)
        .series(Color::rgb(80, 140, 255), 48)
        .build(ui, page_itemlist);
    ui.set_sizing(chart1, Some(Sizing::GROW), None);
    ui.set_size(chart1, 160, 56);
    let chart2 = ChartBuilder::new()
        .range(0, 100)
        .series(Color::rgb(255, 160, 60), 48)
        .build(ui, page_itemlist);
    ui.set_sizing(chart2, Some(Sizing::GROW), None);
    ui.set_size(chart2, 160, 56);
    charts.borrow_mut().extend([chart1, chart2]);

    // Complex ItemList: each item = LED + Label + Checkbox
    let il = ItemListBuilder::new().build(ui, page_itemlist);
    ui.set_sizing(il, Some(Sizing::GROW), Some(Sizing::GROW));
    let item_controls: Rc<RefCell<Vec<(ObjRef, ObjRef)>>> = Rc::new(RefCell::new(Vec::new()));
    for i in 0..8 {
        let item = ui.itemlist_add_item(il).unwrap();
        ui.set_layout(item, Layout::Flex(Flex {
            dir: FlexDir::Row, wrap: false,
            main: Align::Start, cross: Align::Center, track: Align::Start, gap: 8,
        }));
        let led = LedBuilder::new(Color::rgb(60, 180, 90)).size(10, 10).build(ui, item);
        let _lbl = LabelCfg::new(&format!("Sensor {:02}", i + 1)).build(ui, item);
        let cb = CheckboxCfg::new("").build(ui, item);
        item_controls.borrow_mut().push((led, cb));
    }
    // Enter triggers the itemlist's Clicked: toggles the selected item's checkbox, the LED follows on/off
    let ics = item_controls.clone();
    ui.add_event_cb(il, EventKind::Clicked, Box::new(move |ui, il, _| {
        let idx = ui.itemlist_selected(il);
        let (led, cb) = ics.borrow()[idx];
        let v = 1 - ui.value(cb);
        ui.set_value(cb, v);
        ui.set_value(led, v * 255);
    }));

    ui.set_hidden(page_about, true);
    ui.set_hidden(page_animate, true);
    ui.set_hidden(page_longlist, true);
    ui.set_hidden(page_p1, true);
    ui.set_hidden(page_itemlist, true);

    // Layout transition: position/size changes of the menu/panel/pages are animated automatically
    for &o in &[menu, panel, page_settings, page_about, page_animate, page_longlist, page_p1, page_itemlist] {
        ui.set_transition(o, Some((250, Easing::EaseInOutQuad)));
    }

    // Menu click → page switch + panel slide-in animation (translate: the correct animation channel for layout children)
    ui.add_event_cb(menu, EventKind::Clicked, Box::new(move |ui, m, _| {
        let idx = ui.list_selected(m);
        ui.set_hidden(page_settings, idx != 0);
        ui.set_hidden(page_about, idx != 1);
        ui.set_hidden(page_animate, idx != 2);
        ui.set_hidden(page_longlist, idx != 3);
        ui.set_hidden(page_p1, idx != 4);
        ui.set_hidden(page_itemlist, idx != 5);
        ui.anim_start(Anim::new(panel, AnimProp::TranslateX, 204, 0, 200).easing(Easing::EaseOutQuad));
    }));

    // Focus group: menu → slider → switch → checkbox → Wide → ScrollView → long list → Add/Del → P1 widgets → ItemList
    ui.group_add(menu);
    ui.group_add(slider);
    ui.group_add(sw);
    ui.group_add(cb);
    ui.group_add(wide_btn);
    ui.group_add(sv);
    ui.group_add(long_list);
    ui.group_add(add_btn);
    ui.group_add(del_btn);
    ui.group_add(roller);
    ui.group_add(dropdown);
    ui.group_add(spinbox);
    ui.group_add(il);
}
