mod sim;

use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir, Grid, Sizing, Track};
use qingui::style::{Layout, Style};
use qingui::{Color, EventKind, Ui};

fn main() {
    sim::run(build);
}

fn column() -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Column, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    })
}

/// 透明容器样式（只做布局，不画背景）
fn transparent() -> Style {
    let mut s = Style::default();
    s.bg_opa = Some(0);
    s
}

pub fn build(ui: &mut Ui) {
    let screen = ui.screen();

    // 屏幕级 Grid：标题行（内容高）+ 主行（Fr）；左列固定宽菜单，右列自适应面板
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

    let title = ui.create_label(screen, "qingui demo");
    ui.set_grid_cell(title, (0, 2), (0, 1));

    let menu = ui.create_list(screen, &["Settings", "About", "Animate", "LongList", "P1 Demo"]);
    ui.set_grid_cell(menu, (0, 1), (1, 1));
    ui.set_sizing(menu, Some(Sizing::GROW), Some(Sizing::GROW));

    let panel = ui.create_obj(screen);
    ui.set_grid_cell(panel, (1, 1), (1, 1));
    ui.set_style(panel, qingui::style::theme_obj());
    ui.set_sizing(panel, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(panel, column());

    // ---- Settings 页：Slider + Switch + preview Bar ----
    let page_settings = ui.create_obj(panel);
    ui.set_style(page_settings, transparent());
    ui.set_sizing(page_settings, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_settings, column());
    let l1 = ui.create_label(page_settings, "Brightness");
    let _ = l1;
    let slider = qingui::widgets::slider::SliderBuilder::new(0, 100)
        .size(160, 12)
        .value(30)
        .build(ui, page_settings);
    let l2 = ui.create_label(page_settings, "Enabled");
    let _ = l2;
    let sw = ui.create_switch(page_settings);
    let cb = ui.create_checkbox(page_settings, "Notify me");
    let l3 = ui.create_label(page_settings, "Preview");
    let _ = l3;
    let preview = qingui::widgets::bar::BarBuilder::new(0, 100)
        .size(160, 10)
        .value(30)
        .build(ui, page_settings);
    // Slider 调值 → 动画驱动 preview Bar（演示动画与控件值联动）
    ui.add_event_cb(slider, EventKind::ValueChanged, Box::new(move |ui, s, _| {
        let v = ui.value(s);
        let cur = ui.value(preview);
        ui.anim_start(Anim::new(preview, AnimProp::Value, cur, v, 300));
    }));

    // ---- About 页：多行文本 + 布局过渡演示 ----
    let page_about = ui.create_obj(panel);
    ui.set_style(page_about, transparent());
    ui.set_sizing(page_about, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_about, column());
    let la = ui.create_label(
        page_about,
        "qingui subset\nPFB + dirty rect\nanim + keypad\n\narrows/tab: move\nenter: select/edit\nesc: exit edit",
    );
    let _ = la;
    // 布局过渡演示：切换左侧菜单列宽，界面平滑重排
    let wide = std::cell::Cell::new(false);
    let wide_btn = ui.create_button(page_about, "Wide");
    ui.add_event_cb(wide_btn, EventKind::Clicked, Box::new(move |ui, _b, _| {
        let w = !wide.get();
        wide.set(w);
        ui.set_layout(ui.screen(), Layout::Grid(Grid {
            cols: vec![Track::Px(if w { 180 } else { 108 }), Track::Fr(1)],
            rows: vec![Track::Content, Track::Fr(1)],
            col_gap: 8,
            row_gap: 8,
        }));
    }));

    // ---- Animate 页：无限往返动画的 Bar + 圆弧仪表盘 ----
    let page_animate = ui.create_obj(panel);
    ui.set_style(page_animate, transparent());
    ui.set_sizing(page_animate, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_animate, column());
    let bar = ui.create_bar(page_animate, 0, 100);
    ui.set_size(bar, 160, 10);
    ui.anim_start(
        Anim::new(bar, AnimProp::Value, 0, 100, 1200)
            .easing(Easing::EaseInOutQuad)
            .repeat(-1)
            .playback(true),
    );

    // Arc 表盘：值动画驱动（无限循环 0..360）
    let arc = ui.create_arc(page_animate, 0, 360);
    ui.set_sizing(arc, Some(Sizing::GROW), None);
    ui.set_aspect(arc, Some(1000)); // 1:1
    ui.anim_start(Anim::new(arc, AnimProp::Value, 0, 360, 2400).repeat(-1));

    let spinner = ui.create_spinner(page_animate);
    let _ = spinner;

    // ---- LongList 页：20 项超长列表 + 增删按钮 ----
    let page_longlist = ui.create_obj(panel);
    ui.set_style(page_longlist, transparent());
    ui.set_sizing(page_longlist, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_longlist, column());
    let long_list = ui.create_list(page_longlist, &[
        "Item 01", "Item 02", "Item 03", "Item 04", "Item 05",
        "Item 06", "Item 07", "Item 08", "Item 09", "Item 10",
        "Item 11", "Item 12", "Item 13", "Item 14", "Item 15",
        "Item 16", "Item 17", "Item 18", "Item 19", "Item 20",
    ]);
    ui.set_size(long_list, 160, 5 * 16 + 2);

    let btn_row = ui.create_obj(page_longlist);
    ui.set_style(btn_row, transparent());
    ui.set_size(btn_row, 160, 28);
    ui.set_layout(btn_row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    }));
    let add_btn = ui.create_button(btn_row, "Add");
    let del_btn = ui.create_button(btn_row, "Del");

    // Add：在选中项下方插入（淡入 + 下方项下滑），demo 侧限制最多 20 项
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
    // Del：删除选中项（渐隐 + 下方项上移）
    ui.add_event_cb(del_btn, EventKind::Clicked, Box::new(move |ui, _b, _| {
        ui.list_remove(long_list);
    }));

    // LongList 项点击 → Msgbox（模态消息框）
    ui.add_event_cb(long_list, EventKind::Clicked, Box::new(move |ui, l, _| {
        let idx = ui.list_selected(l);
        let screen = ui.screen();
        let prev = ui.focused();
        let mb = qingui::widgets::msgbox::MsgboxBuilder::new(
            "Clicked",
            &format!("Item {:02}", idx + 1),
        )
        .buttons(&["OK"])
        .build(ui, screen);
        // 关闭后还原焦点
        ui.add_event_cb(mb, EventKind::ValueChanged, Box::new(move |ui, _t, _| {
            if let Some(p) = prev {
                ui.group_focus(p);
            }
        }));
    }));

    // ---- P1 Demo 页：Roller / Dropdown / Spinbox / LED / Table ----
    let page_p1 = ui.create_obj(panel);
    ui.set_style(page_p1, transparent());
    ui.set_sizing(page_p1, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_p1, column());
    let roller = qingui::widgets::roller::RollerBuilder::new(&["One", "Two", "Three", "Four", "Five"])
        .size(90, 56)
        .build(ui, page_p1);
    let dropdown = ui.create_dropdown(page_p1, &["Red", "Green", "Blue"]);
    let spinbox = ui.create_spinbox(page_p1, 0, 999, 3);
    let led_row = ui.create_obj(page_p1);
    ui.set_style(led_row, transparent());
    ui.set_size(led_row, 120, 18);
    ui.set_layout(led_row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Center, track: Align::Start, gap: 6,
    }));
    let led = ui.create_led(led_row, Color::rgb(60, 180, 90));
    let _led_lbl = ui.create_label(led_row, "status");
    // LED 亮度跟随 spinbox 值（演示控件联动）
    ui.add_event_cb(spinbox, EventKind::ValueChanged, Box::new(move |ui, sb, _| {
        let v = ui.value(sb);
        ui.set_value(led, v * 255 / 999);
    }));
    let _table = qingui::widgets::table::TableBuilder::new(3, 2)
        .cell(0, 0, "id")
        .cell(0, 1, "val")
        .cell(0, 2, "unit")
        .cell(1, 0, "01")
        .cell(1, 1, "42")
        .cell(1, 2, "ms")
        .build(ui, page_p1);

    ui.set_hidden(page_about, true);
    ui.set_hidden(page_animate, true);
    ui.set_hidden(page_longlist, true);
    ui.set_hidden(page_p1, true);

    // 布局过渡：菜单/面板/页面的位置尺寸变化自动动画
    for &o in &[menu, panel, page_settings, page_about, page_animate, page_longlist, page_p1] {
        ui.set_transition(o, Some((250, Easing::EaseInOutQuad)));
    }

    // 菜单点击 → 切页 + 面板滑入动画（translate：布局子对象的正确动画通道）
    ui.add_event_cb(menu, EventKind::Clicked, Box::new(move |ui, m, _| {
        let idx = ui.list_selected(m);
        ui.set_hidden(page_settings, idx != 0);
        ui.set_hidden(page_about, idx != 1);
        ui.set_hidden(page_animate, idx != 2);
        ui.set_hidden(page_longlist, idx != 3);
        ui.set_hidden(page_p1, idx != 4);
        ui.anim_start(Anim::new(panel, AnimProp::TranslateX, 204, 0, 200).easing(Easing::EaseOutQuad));
    }));

    // 焦点组：菜单 → slider → switch → checkbox → Wide → 超长列表 → Add/Del → P1 控件组
    ui.group_add(menu);
    ui.group_add(slider);
    ui.group_add(sw);
    ui.group_add(cb);
    ui.group_add(wide_btn);
    ui.group_add(long_list);
    ui.group_add(add_btn);
    ui.group_add(del_btn);
    ui.group_add(roller);
    ui.group_add(dropdown);
    ui.group_add(spinbox);
}
