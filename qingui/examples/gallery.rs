//! 全控件画廊：所有控件按 flex(wrap) 排布，每 1s 把最后一个移到最前（动画换位），
//! 同时每个可操作控件都在"自演示"：开关切换、进度随机、滚轮转动、数值自增……
//!
//! 运行：cargo run --example gallery

mod sim;

use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir};
use qingui::style::Layout;
use qingui::widgets::arc::ArcBuilder;
use qingui::widgets::bar::BarBuilder;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::canvas::CanvasBuilder;
use qingui::widgets::checkbox::CheckboxBuilder;
use qingui::widgets::dropdown::DropdownBuilder;
use qingui::widgets::label::LabelBuilder;
use qingui::widgets::led::LedBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::obj::ObjBuilder;
use qingui::widgets::roller::RollerBuilder;
use qingui::widgets::slider::SliderBuilder;
use qingui::widgets::spinbox::SpinboxBuilder;
use qingui::widgets::spinner::SpinnerBuilder;
use qingui::widgets::switch::SwitchBuilder;
use qingui::widgets::table::TableBuilder;
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
        screen.set_style(
            ui,
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

        let b = ButtonBuilder::new("OK").build(ui, screen);
        kids.push(b);
        let l = LabelBuilder::new("label").build(ui, screen);
        kids.push(l);

        let cb = CheckboxBuilder::new("check").build(ui, screen);
        self.checkbox = Some(cb);
        kids.push(cb);

        let sw = SwitchBuilder::new().build(ui, screen);
        self.switch = Some(sw);
        kids.push(sw);

        let sl = SliderBuilder::new(0, 100)
            .size(70, 12)
            .value(40)
            .build(ui, screen);
        self.slider = Some(sl);
        kids.push(sl);

        let bar = BarBuilder::new(0, 100)
            .size(70, 10)
            .value(60)
            .build(ui, screen);
        self.bar = Some(bar);
        kids.push(bar);

        let arc = ArcBuilder::new(0, 100).build(ui, screen);
        arc.set_size(ui, 56, 56);
        // Arc 无限循环旋转
        ui.anim_start(Anim::new(arc, AnimProp::Value, 0, 100, 3000).repeat(-1));
        kids.push(arc);

        let sp = SpinnerBuilder::new().build(ui, screen);
        sp.set_size(ui, 26, 26);
        kids.push(sp);

        let led = LedBuilder::new(Color::rgb(60, 180, 90)).build(ui, screen);
        // LED 呼吸
        ui.anim_start(
            Anim::new(led, AnimProp::Value, 40, 255, 1200)
                .repeat(-1)
                .playback(true)
                .easing(Easing::EaseInOutQuad),
        );
        kids.push(led);

        let sb = SpinboxBuilder::new(0, 999, 3).build(ui, screen);
        sb.set_value(ui, 42);
        self.spinbox = Some(sb);
        kids.push(sb);

        let roller = RollerBuilder::new(&["A", "B", "C"])
            .size(56, 56)
            .build(ui, screen);
        self.roller = Some(roller);
        kids.push(roller);

        let dd = DropdownBuilder::new(&["Red", "Green"]).build(ui, screen);
        dd.set_size(ui, 80, 20);
        self.dropdown = Some(dd);
        kids.push(dd);

        let list = ListBuilder::new(&["item 1", "item 2", "item 3"]).build(ui, screen);
        list.set_size(ui, 80, 50);
        self.list = Some(list);
        kids.push(list);

        let table = TableBuilder::new(2, 2)
            .cell(0, 0, "id")
            .cell(0, 1, "val")
            .cell(1, 0, "01")
            .cell(1, 1, "0")
            .build(ui, screen);
        self.table = Some(table);
        kids.push(table);

        // Canvas：用 now 自转的圆弧
        let cv = CanvasBuilder::new(Box::new(|d, abs, clip, now| {
            let c = qingui::Point { x: abs.x + 18, y: abs.y + 18 };
            let end = (now / 10) as i32 % 360;
            d.draw_arc(c, 14, 4, 0, end, Color::rgb(80, 140, 255), 255, clip);
            d.fill_circle(c, 3, Color::WHITE, 255, clip);
        }))
        .size(36, 36)
        .build(ui, screen);
        kids.push(cv);
        // tick_hook 驱动 Canvas 逐帧重绘
        cv.on_tick(ui, Box::new(|ui, cv, _now| {
            cv.invalidate(ui);
            true // 每帧重绘
        }));

        let obj = ObjBuilder::new().build(ui, screen);
        obj.set_size(ui, 40, 40);
        obj.set_style(ui, qingui::style::theme_obj());
        kids.push(obj);

        // 全部控件开启布局过渡：重排时自动动画换位
        for &k in &kids {
            k.set_transition(ui, Some((300, Easing::EaseInOutQuad)));
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
            let scr = ui.screen();
            let kids = scr.children(ui);
            if let Some(&last) = kids.last() {
                last.move_child_to_index(ui, 0);
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
            let cur = b.value(ui);
            let target = self.rng.next(101);
            ui.anim_start(Anim::new(b, AnimProp::Value, cur, target, 700).easing(Easing::EaseOutQuad));
        }
        // Slider：随机值（动画）
        if fire(1, 1800, now, &mut self.next) {
            let s = self.slider.unwrap();
            let cur = s.value(ui);
            let target = self.rng.next(101);
            ui.anim_start(Anim::new(s, AnimProp::Value, cur, target, 800).easing(Easing::EaseInOutQuad));
        }
        // Switch：切换
        if fire(2, 2300, now, &mut self.next) {
            self.switch.unwrap().toggle_switch(ui);
        }
        // Checkbox：切换
        if fire(3, 2600, now, &mut self.next) {
            self.checkbox.unwrap().toggle_checkbox(ui);
        }
        // Roller：下一项（首尾循环由手动取模）
        if fire(4, 2000, now, &mut self.next) {
            let r = self.roller.unwrap();
            r.set_value(ui, (r.value(ui) + 1) % 3);
        }
        // List：下一项
        if fire(5, 1900, now, &mut self.next) {
            let l = self.list.unwrap();
            l.list_select(ui, (l.list_selected(ui) + 1) % 3);
        }
        // Dropdown：下一项
        if fire(6, 2200, now, &mut self.next) {
            let d = self.dropdown.unwrap();
            d.set_value(ui, (d.value(ui) + 1) % 2);
        }
        // Spinbox：自增
        if fire(7, 2100, now, &mut self.next) {
            let sb = self.spinbox.unwrap();
            sb.set_value(ui, (sb.value(ui) + 7) % 1000);
        }
        // Table：数值自增
        if fire(8, 1500, now, &mut self.next) {
            self.table_val += 1;
            self.table.unwrap().table_set_cell(ui, 1, 1, &self.table_val.to_string());
        }
    }
}
