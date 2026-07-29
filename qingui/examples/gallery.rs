//! 全控件画廊：所有控件按 flex(wrap) 排布，每 1s 把最后一个移到最前，
//! 布局变化通过 transition 自动动画换位。
//!
//! 运行：cargo run --example gallery

mod sim;

use qingui::anim::Easing;
use qingui::layout::{Align, Flex, FlexDir};
use qingui::style::Layout;
use qingui::{Color, ObjRef, Ui};

fn main() {
    sim::run_with_tick(build, tick);
}

fn tick(ui: &mut Ui) {
    // 每 1000ms 触发一次：最后一个子对象移到最前
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1000);
    let now = ui.time();
    let next = NEXT.load(std::sync::atomic::Ordering::Relaxed);
    if now >= next {
        NEXT.store(next + 1000, std::sync::atomic::Ordering::Relaxed);
        let kids = ui.children(ui.screen());
        if let Some(&last) = kids.last() {
            ui.move_child_to_index(last, 0);
        }
    }
}

fn build(ui: &mut Ui) {
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
    kids.push(cb);

    let sw = ui.create_switch(screen);
    kids.push(sw);

    let sl = ui.create_slider(screen, 0, 100);
    ui.set_size(sl, 70, 12);
    ui.set_value(sl, 40);
    kids.push(sl);

    let bar = ui.create_bar(screen, 0, 100);
    ui.set_size(bar, 70, 10);
    ui.set_value(bar, 60);
    kids.push(bar);

    let arc = ui.create_arc(screen, 0, 100);
    ui.set_size(arc, 56, 56);
    ui.set_value(arc, 70);
    kids.push(arc);

    let sp = ui.create_spinner(screen);
    ui.set_size(sp, 26, 26);
    kids.push(sp);

    let led = ui.create_led(screen, Color::rgb(60, 180, 90));
    kids.push(led);

    let sb = ui.create_spinbox(screen, 0, 999, 3);
    ui.set_value(sb, 42);
    kids.push(sb);

    let roller = ui.create_roller(screen, &["A", "B", "C"]);
    ui.set_size(roller, 56, 56);
    kids.push(roller);

    let dd = ui.create_dropdown(screen, &["Red", "Green"]);
    ui.set_size(dd, 80, 20);
    kids.push(dd);

    let list = ui.create_list(screen, &["item 1", "item 2", "item 3"]);
    ui.set_size(list, 80, 50);
    kids.push(list);

    let table = ui.create_table(screen, 2, 2);
    ui.table_set_cell(table, 0, 0, "id");
    ui.table_set_cell(table, 0, 1, "val");
    ui.table_set_cell(table, 1, 0, "01");
    ui.table_set_cell(table, 1, 1, "42");
    kids.push(table);

    // Canvas：手绘小圆弧
    let cv = ui.create_canvas(screen, 36, 36, Box::new(|d, abs, clip, _| {
        let c = qingui::Point { x: abs.x + 18, y: abs.y + 18 };
        d.draw_arc(c, 14, 4, 0, 270, Color::rgb(80, 140, 255), 255, clip);
        d.fill_circle(c, 3, Color::WHITE, 255, clip);
    }));
    kids.push(cv);

    // 纯容器块
    let obj = ui.create_obj(screen);
    ui.set_size(obj, 40, 40);
    ui.set_style(obj, qingui::style::theme_obj());
    kids.push(obj);

    // 全部控件开启布局过渡：重排时自动动画换位
    for &k in &kids {
        ui.set_transition(k, Some((300, Easing::EaseInOutQuad)));
    }
}
