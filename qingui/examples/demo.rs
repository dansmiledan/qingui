mod sim;

use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir, Grid, Sizing, Track};
use qingui::style::{Layout, Style};
use qingui::widgets::arc::ArcBuilder;
use qingui::widgets::bar::BarBuilder;
use qingui::widgets::button::ButtonBuilder;
use qingui::widgets::chart::ChartBuilder;
use qingui::widgets::checkbox::CheckboxBuilder;
use qingui::widgets::dropdown::DropdownBuilder;
use qingui::widgets::itemlist::ItemListBuilder;
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
use qingui::{Color, EventKind, ObjRef, Ui};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    // build 与 tick 共享两个 chart 的引用（流式数据源）
    let charts: Rc<RefCell<Vec<ObjRef>>> = Rc::new(RefCell::new(Vec::new()));
    let charts_tick = charts.clone();
    let mut frame = 0u32;
    sim::run_with_tick(move |ui| build(ui, &charts), move |ui| {
        // 60fps 下每 6 帧 push 一次（~100ms）：两条相位差 π/2 的正弦
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

/// 透明容器样式（只做布局，不画背景）
fn transparent() -> Style {
    let mut s = Style::default();
    s.bg_opa = Some(0);
    s
}

pub fn build(ui: &mut Ui, charts: &Rc<RefCell<Vec<ObjRef>>>) {
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

    let title = LabelBuilder::new("qingui demo").build(ui, screen);
    ui.set_grid_cell(title, (0, 2), (0, 1));

    let menu = ListBuilder::new(&["Settings", "About", "Animate", "LongList", "P1 Demo", "ItemList"])
        .build(ui, screen);
    ui.set_grid_cell(menu, (0, 1), (1, 1));
    ui.set_sizing(menu, Some(Sizing::GROW), Some(Sizing::GROW));

    let panel = ObjBuilder::new().build(ui, screen);
    ui.set_grid_cell(panel, (1, 1), (1, 1));
    ui.set_style(panel, qingui::style::theme_obj());
    ui.set_sizing(panel, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(panel, column());

    // ---- Settings 页：Slider + Switch + preview Bar ----
    let page_settings = ObjBuilder::new().build(ui, panel);
    ui.set_style(page_settings, transparent());
    ui.set_sizing(page_settings, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_settings, column());
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
    ui.add_event_cb(slider, EventKind::ValueChanged, Box::new(move |ui, s, _| {
        let v = ui.value(s);
        let cur = ui.value(preview);
        ui.anim_start(Anim::new(preview, AnimProp::Value, cur, v, 300));
    }));

    // ---- About 页：多行文本 + 布局过渡演示 ----
    let page_about = ObjBuilder::new().build(ui, panel);
    ui.set_style(page_about, transparent());
    ui.set_sizing(page_about, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_about, column());
    let la = LabelBuilder::new(
        "qingui subset\nPFB + dirty rect\nanim + keypad\n\narrows/tab: move\nenter: select/edit\nesc: exit edit",
    )
    .build(ui, page_about);
    let _ = la;
    // 布局过渡演示：切换左侧菜单列宽，界面平滑重排
    let wide = std::cell::Cell::new(false);
    let wide_btn = ButtonBuilder::new("Wide").build(ui, page_about);
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

    // ---- Animate 页：无限往返动画的 Bar + 圆弧仪表盘 ----
    let page_animate = ObjBuilder::new().build(ui, panel);
    ui.set_style(page_animate, transparent());
    ui.set_sizing(page_animate, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_animate, column());
    let bar = BarBuilder::new(0, 100).build(ui, page_animate);
    ui.set_size(bar, 160, 10);
    ui.anim_start(
        Anim::new(bar, AnimProp::Value, 0, 100, 1200)
            .easing(Easing::EaseInOutQuad)
            .repeat(-1)
            .playback(true),
    );

    // Arc 表盘：值动画驱动（无限循环 0..360）
    let arc = ArcBuilder::new(0, 360).build(ui, page_animate);
    ui.set_sizing(arc, Some(Sizing::GROW), None);
    ui.set_aspect(arc, Some(1000)); // 1:1
    ui.anim_start(Anim::new(arc, AnimProp::Value, 0, 360, 2400).repeat(-1));

    let spinner = SpinnerBuilder::new().build(ui, page_animate);
    let _ = spinner;

    // ---- LongList 页：20 项超长列表 + 增删按钮 ----
    let page_longlist = ObjBuilder::new().build(ui, panel);
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

    let btn_row = ObjBuilder::new().build(ui, page_longlist);
    ui.set_style(btn_row, transparent());
    ui.set_size(btn_row, 160, 28);
    ui.set_layout(btn_row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    }));
    let add_btn = ButtonBuilder::new("Add").build(ui, btn_row);
    let del_btn = ButtonBuilder::new("Del").build(ui, btn_row);

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
        let mb = MsgboxBuilder::new(
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
    let page_p1 = ObjBuilder::new().build(ui, panel);
    ui.set_style(page_p1, transparent());
    ui.set_sizing(page_p1, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_p1, column());
    let roller = RollerBuilder::new(&["One", "Two", "Three", "Four", "Five"])
        .size(90, 56)
        .build(ui, page_p1);
    let dropdown = DropdownBuilder::new(&["Red", "Green", "Blue"]).build(ui, page_p1);
    let spinbox = SpinboxBuilder::new(0, 999, 3).build(ui, page_p1);
    let led_row = ObjBuilder::new().build(ui, page_p1);
    ui.set_style(led_row, transparent());
    ui.set_size(led_row, 120, 18);
    ui.set_layout(led_row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Center, track: Align::Start, gap: 6,
    }));
    let led = LedBuilder::new(Color::rgb(60, 180, 90)).build(ui, led_row);
    let _led_lbl = LabelBuilder::new("status").build(ui, led_row);
    // LED 亮度跟随 spinbox 值（演示控件联动）
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

    // ---- ItemList 页：上下两个流式 chart + 三控件 item 的 ItemList ----
    let page_itemlist = ObjBuilder::new().build(ui, panel);
    ui.set_style(page_itemlist, transparent());
    ui.set_sizing(page_itemlist, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page_itemlist, column());

    // 上下两个折线图：不同颜色，数据由 main 的 tick 周期 push
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

    // 复杂 ItemList：每 item = LED + Label + Checkbox
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
        let _lbl = LabelBuilder::new(&format!("Sensor {:02}", i + 1)).build(ui, item);
        let cb = CheckboxBuilder::new("").build(ui, item);
        item_controls.borrow_mut().push((led, cb));
    }
    // Enter 触发 itemlist 的 Clicked：翻转选中 item 的 checkbox，LED 亮灭跟随
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

    // 布局过渡：菜单/面板/页面的位置尺寸变化自动动画
    for &o in &[menu, panel, page_settings, page_about, page_animate, page_longlist, page_p1, page_itemlist] {
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
        ui.set_hidden(page_itemlist, idx != 5);
        ui.anim_start(Anim::new(panel, AnimProp::TranslateX, 204, 0, 200).easing(Easing::EaseOutQuad));
    }));

    // 焦点组：菜单 → slider → switch → checkbox → Wide → 超长列表 → Add/Del → P1 控件组 → ItemList
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
    ui.group_add(il);
}
