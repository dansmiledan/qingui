//! 全控件画廊：所有控件按 flex(wrap) 排布，每 1s 把最后一个移到最前（动画换位），
//! 同时每个可操作控件都在"自演示"：开关切换、进度随机、滚轮转动、数值自增……
//!
//! 运行：cargo run --example gallery

mod sim;

use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir};
use qingui::style::Layout;
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

/// 简易 xorshift 随机数（避免引入 rand 依赖）
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
    // 各控件句柄（build 时填充）
    switch: Option<ObjRef>,
    checkbox: Option<ObjRef>,
    slider: Option<ObjRef>,
    bar: Option<ObjRef>,
    spinbox: Option<ObjRef>,
    roller: Option<ObjRef>,
    list: Option<ObjRef>,
    dropdown: Option<ObjRef>,
    table: Option<ObjRef>,
    // 调度
    next_reorder: u64,
    next: [u64; 9],
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
        // 屏幕：flex 行 + wrap，padding 8，gap 8
        ui.widget(screen).style(
            qingui::style::theme_screen()
                .pads(8)
                .layout(Layout::Flex(Flex {
                    dir: FlexDir::Row,
                    wrap: true,
                    main: Align::Start,
                    cross: Align::Start,
                    track: Align::Start,
                    gap: 8,
                })),
        );

        let mut kids: Vec<ObjRef> = Vec::new();

        let b = ui.create_button(screen, "OK");
        kids.push(b);
        let l = ui.create_label(screen, "label");
        kids.push(l);

        let cb = ui.create_checkbox(screen, "check");
        self.checkbox = Some(cb);
        kids.push(cb);

        let sw = ui.create_switch(screen);
        self.switch = Some(sw);
        kids.push(sw);

        let sl = ui.create_slider(screen, 0, 100);
        ui.set_size(sl, 70, 12);
        ui.set_value(sl, 40);
        self.slider = Some(sl);
        kids.push(sl);

        let bar = ui.create_bar(screen, 0, 100);
        ui.set_size(bar, 70, 10);
        ui.set_value(bar, 60);
        self.bar = Some(bar);
        kids.push(bar);

        let arc = ui.create_arc(screen, 0, 100);
        ui.set_size(arc, 56, 56);
        // Arc 无限循环旋转
        ui.anim_start(Anim::new(arc, AnimProp::Value, 0, 100, 3000).repeat(-1));
        kids.push(arc);

        let sp = ui.create_spinner(screen);
        ui.set_size(sp, 26, 26);
        kids.push(sp);

        let led = ui.create_led(screen, Color::rgb(60, 180, 90));
        // LED 呼吸
        ui.anim_start(
            Anim::new(led, AnimProp::Value, 40, 255, 1200)
                .repeat(-1)
                .playback(true)
                .easing(Easing::EaseInOutQuad),
        );
        kids.push(led);

        let sb = ui.create_spinbox(screen, 0, 999, 3);
        ui.set_value(sb, 42);
        self.spinbox = Some(sb);
        kids.push(sb);

        let roller = ui.create_roller(screen, &["A", "B", "C"]);
        ui.set_size(roller, 56, 56);
        self.roller = Some(roller);
        kids.push(roller);

        let dd = ui.create_dropdown(screen, &["Red", "Green"]);
        ui.set_size(dd, 80, 20);
        self.dropdown = Some(dd);
        kids.push(dd);

        let list = ui.create_list(screen, &["item 1", "item 2", "item 3"]);
        ui.set_size(list, 80, 50);
        self.list = Some(list);
        kids.push(list);

        let table = ui.create_table(screen, 2, 2);
        ui.table_set_cell(table, 0, 0, "id");
        ui.table_set_cell(table, 0, 1, "val");
        ui.table_set_cell(table, 1, 0, "01");
        ui.table_set_cell(table, 1, 1, "0");
        self.table = Some(table);
        kids.push(table);

        // Canvas：用 now 自转的圆弧
        let cv = ui.create_canvas(screen, 36, 36, Box::new(|d, abs, clip, now| {
            let c = qingui::Point { x: abs.x + 18, y: abs.y + 18 };
            let end = (now / 10) as i32 % 360;
            d.draw_arc(c, 14, 4, 0, end, Color::rgb(80, 140, 255), 255, clip);
            d.fill_circle(c, 3, Color::WHITE, 255, clip);
        }));
        kids.push(cv);
        // 用一个隐藏 Bar 的无限动画驱动 Canvas 逐帧重绘
        let driver = ui.create_bar(screen, 0, 360);
        ui.set_hidden(driver, true);
        ui.add_event_cb(driver, qingui::EventKind::ValueChanged, Box::new(move |ui, _b, _| {
            ui.invalidate_obj(cv);
        }));
        ui.anim_start(Anim::new(driver, AnimProp::Value, 0, 360, 10000).repeat(-1));

        let obj = ui.create_obj(screen);
        ui.set_size(obj, 40, 40);
        ui.set_style(obj, qingui::style::theme_obj());
        kids.push(obj);

        // 全部控件开启布局过渡：重排时自动动画换位
        for &k in &kids {
            ui.set_transition(k, Some((300, Easing::EaseInOutQuad)));
        }

        // 调度起点（错开相位）
        self.next_reorder = 1000;
        for (i, n) in self.next.iter_mut().enumerate() {
            *n = 800 + i as u64 * 300;
        }
    }

    fn tick(&mut self, ui: &mut Ui) {
        let now = ui.time();

        // 每 1s：最后一个移到最前
        if now >= self.next_reorder {
            self.next_reorder += 3000;
            let kids = ui.children(ui.screen());
            if let Some(&last) = kids.last() {
                ui.move_child_to_index(last, 0);
            }
        }

        let fire = |i: usize, period: u64, now: u64, next: &mut [u64; 9]| -> bool {
            if now >= next[i] {
                next[i] = now + period;
                true
            } else {
                false
            }
        };

        // Bar：随机进度（动画）
        if fire(0, 1500, now, &mut self.next) {
            let b = self.bar.unwrap();
            let cur = ui.value(b);
            let target = self.rng.next(101);
            ui.anim_start(Anim::new(b, AnimProp::Value, cur, target, 700).easing(Easing::EaseOutQuad));
        }
        // Slider：随机值（动画）
        if fire(1, 1800, now, &mut self.next) {
            let s = self.slider.unwrap();
            let cur = ui.value(s);
            let target = self.rng.next(101);
            ui.anim_start(Anim::new(s, AnimProp::Value, cur, target, 800).easing(Easing::EaseInOutQuad));
        }
        // Switch：切换
        if fire(2, 2300, now, &mut self.next) {
            ui.toggle_switch(self.switch.unwrap());
        }
        // Checkbox：切换
        if fire(3, 2600, now, &mut self.next) {
            ui.toggle_checkbox(self.checkbox.unwrap());
        }
        // Roller：下一项（首尾循环由手动取模）
        if fire(4, 2000, now, &mut self.next) {
            let r = self.roller.unwrap();
            ui.set_value(r, (ui.value(r) + 1) % 3);
        }
        // List：下一项
        if fire(5, 1900, now, &mut self.next) {
            let l = self.list.unwrap();
            ui.list_select(l, (ui.list_selected(l) + 1) % 3);
        }
        // Dropdown：下一项
        if fire(6, 2200, now, &mut self.next) {
            let d = self.dropdown.unwrap();
            ui.set_value(d, (ui.value(d) + 1) % 2);
        }
        // Spinbox：自增
        if fire(7, 2100, now, &mut self.next) {
            let sb = self.spinbox.unwrap();
            ui.set_value(sb, (ui.value(sb) + 7) % 1000);
        }
        // Table：数值自增
        if fire(8, 1500, now, &mut self.next) {
            self.table_val += 1;
            ui.table_set_cell(self.table.unwrap(), 1, 1, &self.table_val.to_string());
        }
    }
}
