//! Full-widget gallery: all widgets laid out with flex(wrap), every 1s the last one is moved to the front (animated reorder),
//! while every interactive widget "self-demonstrates": switches toggle, progress randomizes, wheels spin, values increment...
//!
//! Run: cargo run --example gallery

mod sim;

use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir};
use qingui::prelude::*;
use qingui::widgets::arc::ArcCfg;
use qingui::widgets::bar::BarCfg;
use qingui::widgets::button::ButtonCfg;
use qingui::widgets::checkbox::CheckboxCfg;
use qingui::widgets::dropdown::DropdownCfg;
use qingui::widgets::itemlist::ItemListCfg;
use qingui::widgets::label::LabelCfg;
use qingui::widgets::led::LedCfg;
use qingui::widgets::list::ListCfg;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::roller::RollerCfg;
use qingui::widgets::slider::SliderCfg;
use qingui::widgets::spinbox::SpinboxCfg;
use qingui::widgets::spinner::SpinnerCfg;
use qingui::widgets::switch::SwitchCfg;
use qingui::widgets::table::TableCfg;
use qingui::{Color, ObjRef, Ui};

fn main() {
    let mut demo = Demo::default();
    let mut built = false;
    sim::run_with_tick(
        |_| {},
        move |ui| {
            if !built {
                demo.build(ui);
                built = true;
            }
            demo.tick(ui);
        },
    );
}

/// Simple xorshift random number generator (avoids adding a rand dependency)
struct Rng(u32);
impl Rng {
    fn next(&mut self, max: i32) -> i32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % max.max(1) as u32) as i32
    }
}

#[derive(Default)]
struct Demo {
    // Handles to each widget (filled at build time)
    switch: Option<ObjRef>,
    checkbox: Option<ObjRef>,
    slider: Option<ObjRef>,
    bar: Option<ObjRef>,
    spinbox: Option<ObjRef>,
    roller: Option<ObjRef>,
    list: Option<ObjRef>,
    itemlist: Option<ObjRef>,
    dropdown: Option<ObjRef>,
    table: Option<ObjRef>,
    // Scheduling
    next_reorder: u64,
    next: [u64; 10],
    rng: Rng,
    table_val: i32,
}

impl Default for Rng {
    fn default() -> Self {
        Rng(0x2545F491)
    }
}

impl Demo {
    fn build(&mut self, ui: &mut Ui) {
        let screen = ui.screen();
        // Screen: flex row + wrap, padding 8, gap 8
        ui.set_style(screen, qingui::style::theme_screen());
        ui.set_pad(screen, (8, 8, 8, 8));
        ui.set_flex(screen, Flex {
            dir: FlexDir::Row,
            wrap: true,
            main: Align::Start,
            cross: Align::Start,
            track: Align::Start,
            gap: 8,
        });

        let mut kids: Vec<ObjRef> = Vec::new();

        let b = ButtonCfg::new("OK").build(ui, screen);
        kids.push(b);
        let l = LabelCfg::new("label").build(ui, screen);
        kids.push(l);

        let cb = CheckboxCfg::new("check").build(ui, screen);
        self.checkbox = Some(cb);
        kids.push(cb);

        let sw = SwitchCfg::new().build(ui, screen);
        self.switch = Some(sw);
        kids.push(sw);

        let sl = SliderCfg::new(0, 100)
            .size(70, 12)
            .value(40)
            .build(ui, screen);
        self.slider = Some(sl);
        kids.push(sl);

        let bar = BarCfg::new(0, 100)
            .size(70, 10)
            .value(60)
            .build(ui, screen);
        self.bar = Some(bar);
        kids.push(bar);

        let arc = ArcCfg::new(0, 100).build(ui, screen);
        ui.set_size(arc, 56, 56);
        // Arc rotates in an infinite loop
        ui.anim_start(Anim::new(arc, AnimProp::Value, 0, 100, 3000).repeat(-1));
        kids.push(arc);

        let sp = SpinnerCfg::new().build(ui, screen);
        ui.set_size(sp, 26, 26);
        kids.push(sp);

        let led = LedCfg::new(Color::rgb(60, 180, 90)).build(ui, screen);
        // LED breathing
        ui.anim_start(
            Anim::new(led, AnimProp::Value, 40, 255, 1200)
                .repeat(-1)
                .playback(true)
                .easing(Easing::EaseInOutQuad),
        );
        kids.push(led);

        let sb = SpinboxCfg::new(0, 999, 3).build(ui, screen);
        ui.set_value(sb, 42);
        self.spinbox = Some(sb);
        kids.push(sb);

        let roller = RollerCfg::new(&["A", "B", "C"])
            .size(56, 56)
            .build(ui, screen);
        self.roller = Some(roller);
        kids.push(roller);

        let dd = DropdownCfg::new(&["Red", "Green"]).build(ui, screen);
        ui.set_size(dd, 80, 20);
        self.dropdown = Some(dd);
        kids.push(dd);

        let list = ListCfg::new(&["item 1", "item 2", "item 3"]).build(ui, screen);
        ui.set_size(list, 80, 50);
        self.list = Some(list);
        kids.push(list);

        // ItemList: menu-style list (each item is a Led icon + Label text);
        // viewport height 60 < content height 5*16=80, scrolling the selection demonstrates content scrolling
        let menu = ItemListCfg::new().size(140, 60).build(ui, screen);
        for (color, name) in [
            (Color::GREEN, "Wi-Fi"),
            (Color::BLUE, "Bluetooth"),
            (Color::RED, "Airplane"),
            (Color::rgb(255, 200, 0), "Location"),
            (Color::WHITE, "About"),
        ] {
            let it = ui.itemlist_add_item(menu).unwrap();
            ui.set_flex(it, Flex {
                dir: FlexDir::Row,
                wrap: false,
                main: Align::Start,
                cross: Align::Center,
                track: Align::Start,
                gap: 6,
            });
            ui.set_size(it, 140, 16);
            LedCfg::new(color).size(8, 8).build(ui, it);
            LabelCfg::new(name).build(ui, it);
        }
        ui.group_add(menu);
        self.itemlist = Some(menu);
        kids.push(menu);

        let table = TableCfg::new(2, 2)
            .cell(0, 0, "id")
            .cell(0, 1, "val")
            .cell(1, 0, "01")
            .cell(1, 1, "0")
            .build(ui, screen);
        self.table = Some(table);
        kids.push(table);

        // Canvas: an arc spinning with now (a plain Obj with a draw hook; transparent
        // background like the old canvas widget's default)
        let cv = ObjCfg::new()
            .size(36, 36)
            .build(ui, screen);
        ui.set_draw_hook(cv, Some(Box::new(|d, abs, clip, now| {
            let c = qingui::Point { x: abs.x + 18, y: abs.y + 18 };
            let end = (now / 10) as i32 % 360;
            d.draw_arc(c, 14, 4, 0, end, Color::rgb(80, 140, 255), 255, clip);
            d.fill_circle(c, 3, Color::WHITE, 255, clip);
        })));
        kids.push(cv);
        // tick_hook drives the canvas to redraw every frame
        ui.set_tick_hook(cv, Some(Box::new(|ui, cv, _now| {
            ui.invalidate_obj(cv);
            true // redraw every frame
        })));

        let obj = ObjCfg::new().build(ui, screen);
        ui.set_size(obj, 40, 40);
        ui.set_style(obj, qingui::style::theme_obj());
        kids.push(obj);

        // Enable layout transitions on all widgets: reorders animate automatically
        for &k in &kids {
            ui.set_transition(k, Some((300, Easing::EaseInOutQuad)));
        }

        // Scheduling start point (offset phases)
        self.next_reorder = 1000;
        for (i, n) in self.next.iter_mut().enumerate() {
            *n = 800 + i as u64 * 300;
        }
    }

    fn tick(&mut self, ui: &mut Ui) {
        let now = ui.time();

        // Every 1s: move the last widget to the front
        if now >= self.next_reorder {
            self.next_reorder += 3000;
            let scr = ui.screen();
            let kids = ui.children(scr);
            if let Some(&last) = kids.last() {
                ui.move_child_to_index(last, 0);
            }
        }

        let fire = |i: usize, period: u64, now: u64, next: &mut [u64; 10]| -> bool {
            if now >= next[i] {
                next[i] = now + period;
                true
            } else {
                false
            }
        };

        // Bar: random progress (animated)
        if fire(0, 1500, now, &mut self.next) {
            let b = self.bar.unwrap();
            let cur = ui.value(b);
            let target = self.rng.next(101);
            ui.anim_start(Anim::new(b, AnimProp::Value, cur, target, 700).easing(Easing::EaseOutQuad));
        }
        // Slider: random value (animated)
        if fire(1, 1800, now, &mut self.next) {
            let s = self.slider.unwrap();
            let cur = ui.value(s);
            let target = self.rng.next(101);
            ui.anim_start(Anim::new(s, AnimProp::Value, cur, target, 800).easing(Easing::EaseInOutQuad));
        }
        // Switch: toggle
        if fire(2, 2300, now, &mut self.next) {
            ui.toggle_switch(self.switch.unwrap());
        }
        // Checkbox: toggle
        if fire(3, 2600, now, &mut self.next) {
            ui.toggle_checkbox(self.checkbox.unwrap());
        }
        // Roller: next item (wrap-around handled manually with modulo)
        if fire(4, 2000, now, &mut self.next) {
            let r = self.roller.unwrap();
            ui.set_value(r, (ui.value(r) + 1) % 3);
        }
        // List: next item
        if fire(5, 1900, now, &mut self.next) {
            let l = self.list.unwrap();
            ui.list_select(l, (ui.list_selected(l) + 1) % 3);
        }
        // Dropdown: next item
        if fire(6, 2200, now, &mut self.next) {
            let d = self.dropdown.unwrap();
            ui.set_value(d, (ui.value(d) + 1) % 2);
        }
        // Spinbox: increment
        if fire(7, 2100, now, &mut self.next) {
            let sb = self.spinbox.unwrap();
            ui.set_value(sb, (ui.value(sb) + 7) % 1000);
        }
        // Table: increment value
        if fire(8, 1500, now, &mut self.next) {
            self.table_val += 1;
            ui.table_set_cell(self.table.unwrap(), 1, 1, &self.table_val.to_string());
        }
        // ItemList: next item (5-item cycle, out-of-range demonstrates scrolling)
        if fire(9, 1700, now, &mut self.next) {
            let m = self.itemlist.unwrap();
            ui.itemlist_select(m, (ui.itemlist_selected(m) + 1) % 5);
        }
    }
}
