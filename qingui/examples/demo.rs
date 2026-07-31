mod sim;

use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir, Grid, Sizing, Track};
use qingui::style::{Layout, Style};
use qingui::widgets::arc::ArcBuilder;
use qingui::widgets::bar::BarBuilder;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::checkbox::CheckboxBuilder;
use qingui::widgets::dropdown::DropdownBuilder;
use qingui::widgets::label::LabelBuilder;
use qingui::widgets::led::LedBuilder;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::msgbox::MsgboxBuilder;
use qingui::widgets::obj::ObjBuilder;
use qingui::widgets::roller::RollerBuilder;
use qingui::widgets::slider::SliderBuilder;
use qingui::widgets::spinbox::SpinboxBuilder;
use qingui::widgets::spinner::SpinnerBuilder;
use qingui::widgets::switch::SwitchBuilder;
use qingui::widgets::table::TableBuilder;
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
    screen.set_style(ui, ss);

    let title = LabelBuilder::new("qingui demo").build(ui, screen);
    title.set_grid_cell(ui, (0, 2), (0, 1));

    let menu = ListBuilder::new(&["Settings", "About", "Animate", "LongList", "P1 Demo"])
        .build(ui, screen);
    menu.set_grid_cell(ui, (0, 1), (1, 1));
    menu.set_sizing(ui, Some(Sizing::GROW), Some(Sizing::GROW));

    let panel = ObjBuilder::new().build(ui, screen);
    panel.set_grid_cell(ui, (1, 1), (1, 1));
    panel.set_style(ui, qingui::style::theme_obj());
    panel.set_sizing(ui, Some(Sizing::GROW), Some(Sizing::GROW));
    panel.set_layout(ui, column());

    // ---- Settings 页：Slider + Switch + preview Bar ----
    let page_settings = ObjBuilder::new().build(ui, panel);
    page_settings.set_style(ui, transparent());
    page_settings.set_sizing(ui, Some(Sizing::GROW), Some(Sizing::GROW));
    page_settings.set_layout(ui, column());
    let l1 = LabelBuilder::new("Brightness").build(ui, page_settings);
    let _ = l1;
    let slider = SliderBuilder::new(0, 100)
        .size(160, 12)
        .value(30)
        .build(ui, page_settings);
    let l2 = LabelBuilder::new("Enabled").build(ui, page_settings);
    let _ = l2;
    let sw = SwitchBuilder::new().build(ui, page_settings);
    let cb = CheckboxBuilder::new("Notify me").build(ui, page_settings);
    let l3 = LabelBuilder::new("Preview").build(ui, page_settings);
    let _ = l3;
    let preview = BarBuilder::new(0, 100)
        .size(160, 10)
        .value(30)
        .build(ui, page_settings);
    // Slider 调值 → 动画驱动 preview Bar（演示动画与控件值联动）
    slider.on(ui, EventKind::ValueChanged, Box::new(move |ui, s, _| {
        let v = s.value(ui);
        let cur = preview.value(ui);
        ui.anim_start(Anim::new(preview, AnimProp::Value, cur, v, 300));
    }));

    // ---- About 页：多行文本 + 布局过渡演示 ----
    let page_about = ObjBuilder::new().build(ui, panel);
    page_about.set_style(ui, transparent());
    page_about.set_sizing(ui, Some(Sizing::GROW), Some(Sizing::GROW));
    page_about.set_layout(ui, column());
    let la = LabelBuilder::new(
        "qingui subset\nPFB + dirty rect\nanim + keypad\n\narrows/tab: move\nenter: select/edit\nesc: exit edit",
    )
    .build(ui, page_about);
    let _ = la;
    // 布局过渡演示：切换左侧菜单列宽，界面平滑重排
    let wide = std::cell::Cell::new(false);
    let wide_btn = ButtonBuilder::new("Wide").build(ui, page_about);
    wide_btn.on(ui, EventKind::Clicked, Box::new(move |ui, _b, _| {
        let w = !wide.get();
        wide.set(w);
        let scr = ui.screen();
        scr.set_layout(ui, Layout::Grid(Grid {
            cols: vec![Track::Px(if w { 180 } else { 108 }), Track::Fr(1)],
            rows: vec![Track::Content, Track::Fr(1)],
            col_gap: 8,
            row_gap: 8,
        }));
    }));

    // ---- Animate 页：无限往返动画的 Bar + 圆弧仪表盘 ----
    let page_animate = ObjBuilder::new().build(ui, panel);
    page_animate.set_style(ui, transparent());
    page_animate.set_sizing(ui, Some(Sizing::GROW), Some(Sizing::GROW));
    page_animate.set_layout(ui, column());
    let bar = BarBuilder::new(0, 100).build(ui, page_animate);
    bar.set_size(ui, 160, 10);
    ui.anim_start(
        Anim::new(bar, AnimProp::Value, 0, 100, 1200)
            .easing(Easing::EaseInOutQuad)
            .repeat(-1)
            .playback(true),
    );

    // Arc 表盘：值动画驱动（无限循环 0..360）
    let arc = ArcBuilder::new(0, 360).build(ui, page_animate);
    arc.set_sizing(ui, Some(Sizing::GROW), None);
    arc.set_aspect(ui, Some(1000)); // 1:1
    ui.anim_start(Anim::new(arc, AnimProp::Value, 0, 360, 2400).repeat(-1));

    let spinner = SpinnerBuilder::new().build(ui, page_animate);
    let _ = spinner;

    // ---- LongList 页：20 项超长列表 + 增删按钮 ----
    let page_longlist = ObjBuilder::new().build(ui, panel);
    page_longlist.set_style(ui, transparent());
    page_longlist.set_sizing(ui, Some(Sizing::GROW), Some(Sizing::GROW));
    page_longlist.set_layout(ui, column());
    let long_list = ListBuilder::new(&[
        "Item 01", "Item 02", "Item 03", "Item 04", "Item 05",
        "Item 06", "Item 07", "Item 08", "Item 09", "Item 10",
        "Item 11", "Item 12", "Item 13", "Item 14", "Item 15",
        "Item 16", "Item 17", "Item 18", "Item 19", "Item 20",
    ])
    .build(ui, page_longlist);
    long_list.set_size(ui, 160, 5 * 16 + 2);

    let btn_row = ObjBuilder::new().build(ui, page_longlist);
    btn_row.set_style(ui, transparent());
    btn_row.set_size(ui, 160, 28);
    btn_row.set_layout(ui, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    }));
    let add_btn = ButtonBuilder::new("Add").build(ui, btn_row);
    let del_btn = ButtonBuilder::new("Del").build(ui, btn_row);

    // Add：在选中项下方插入（淡入 + 下方项下滑），demo 侧限制最多 20 项
    let next_n = std::cell::Cell::new(21i32);
    add_btn.on(ui, EventKind::Clicked, Box::new(move |ui, _b, _| {
        if long_list.list_len(ui) >= 20 {
            return;
        }
        let idx = long_list.list_selected(ui) + 1;
        let name = format!("Item {:02}", next_n.get());
        long_list.list_insert(ui, idx, &name);
        next_n.set(next_n.get() + 1);
    }));
    // Del：删除选中项（渐隐 + 下方项上移）
    del_btn.on(ui, EventKind::Clicked, Box::new(move |ui, _b, _| {
        long_list.list_remove(ui);
    }));

    // LongList 项点击 → Msgbox（模态消息框）
    long_list.on(ui, EventKind::Clicked, Box::new(move |ui, l, _| {
        let idx = l.list_selected(ui);
        let screen = ui.screen();
        let prev = ui.focused();
        let mb = MsgboxBuilder::new(
            "Clicked",
            &format!("Item {:02}", idx + 1),
        )
        .buttons(&["OK"])
        .build(ui, screen);
        // 关闭后还原焦点
        mb.on(ui, EventKind::ValueChanged, Box::new(move |ui, _t, _| {
            if let Some(p) = prev {
                ui.group_focus(p);
            }
        }));
    }));

    // ---- P1 Demo 页：Roller / Dropdown / Spinbox / LED / Table ----
    let page_p1 = ObjBuilder::new().build(ui, panel);
    page_p1.set_style(ui, transparent());
    page_p1.set_sizing(ui, Some(Sizing::GROW), Some(Sizing::GROW));
    page_p1.set_layout(ui, column());
    let roller = RollerBuilder::new(&["One", "Two", "Three", "Four", "Five"])
        .size(90, 56)
        .build(ui, page_p1);
    let dropdown = DropdownBuilder::new(&["Red", "Green", "Blue"]).build(ui, page_p1);
    let spinbox = SpinboxBuilder::new(0, 999, 3).build(ui, page_p1);
    let led_row = ObjBuilder::new().build(ui, page_p1);
    led_row.set_style(ui, transparent());
    led_row.set_size(ui, 120, 18);
    led_row.set_layout(ui, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Center, track: Align::Start, gap: 6,
    }));
    let led = LedBuilder::new(Color::rgb(60, 180, 90)).build(ui, led_row);
    let _led_lbl = LabelBuilder::new("status").build(ui, led_row);
    // LED 亮度跟随 spinbox 值（演示控件联动）
    spinbox.on(ui, EventKind::ValueChanged, Box::new(move |ui, sb, _| {
        let v = sb.value(ui);
        led.set_value(ui, v * 255 / 999);
    }));
    let _table = TableBuilder::new(3, 2)
        .cell(0, 0, "id")
        .cell(0, 1, "val")
        .cell(0, 2, "unit")
        .cell(1, 0, "01")
        .cell(1, 1, "42")
        .cell(1, 2, "ms")
        .build(ui, page_p1);

    page_about.set_hidden(ui, true);
    page_animate.set_hidden(ui, true);
    page_longlist.set_hidden(ui, true);
    page_p1.set_hidden(ui, true);

    // 布局过渡：菜单/面板/页面的位置尺寸变化自动动画
    for &o in &[menu, panel, page_settings, page_about, page_animate, page_longlist, page_p1] {
        o.set_transition(ui, Some((250, Easing::EaseInOutQuad)));
    }

    // 菜单点击 → 切页 + 面板滑入动画（translate：布局子对象的正确动画通道）
    menu.on(ui, EventKind::Clicked, Box::new(move |ui, m, _| {
        let idx = m.list_selected(ui);
        page_settings.set_hidden(ui, idx != 0);
        page_about.set_hidden(ui, idx != 1);
        page_animate.set_hidden(ui, idx != 2);
        page_longlist.set_hidden(ui, idx != 3);
        page_p1.set_hidden(ui, idx != 4);
        ui.anim_start(Anim::new(panel, AnimProp::TranslateX, 204, 0, 200).easing(Easing::EaseOutQuad));
    }));

    // 焦点组：菜单 → slider → switch → checkbox → Wide → 超长列表 → Add/Del → P1 控件组
    menu.group_add(ui);
    slider.group_add(ui);
    sw.group_add(ui);
    cb.group_add(ui);
    wide_btn.group_add(ui);
    long_list.group_add(ui);
    add_btn.group_add(ui);
    del_btn.group_add(ui);
    roller.group_add(ui);
    dropdown.group_add(ui);
    spinbox.group_add(ui);
}
