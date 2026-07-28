use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir};
use qingui::style::Layout;
use qingui::{EventKind, Ui};

fn column() -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Column, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    })
}

pub fn build(ui: &mut Ui) {
    let screen = ui.screen();

    let title = ui.create_label(screen, "qingui demo");
    ui.set_pos(title, 8, 8);

    let menu = ui.create_list(screen, &["Settings", "About", "Animate", "LongList"]);
    ui.set_pos(menu, 8, 32);
    ui.set_size(menu, 100, 200);

    let panel = ui.create_obj(screen);
    ui.set_pos(panel, 116, 32);
    ui.set_size(panel, 196, 200);
    ui.set_layout(panel, column());

    // ---- Settings 页：Slider + Switch + preview Bar ----
    let page_settings = ui.create_obj(panel);
    ui.set_size(page_settings, 188, 192);
    ui.set_layout(page_settings, column());
    let l1 = ui.create_label(page_settings, "Brightness");
    let _ = l1;
    let slider = ui.create_slider(page_settings, 0, 100);
    ui.set_size(slider, 160, 12);
    ui.set_value(slider, 30);
    let l2 = ui.create_label(page_settings, "Enabled");
    let _ = l2;
    let sw = ui.create_switch(page_settings);
    let l3 = ui.create_label(page_settings, "Preview");
    let _ = l3;
    let preview = ui.create_bar(page_settings, 0, 100);
    ui.set_size(preview, 160, 10);
    ui.set_value(preview, 30);
    // Slider 调值 → 动画驱动 preview Bar（演示动画与控件值联动）
    ui.add_event_cb(slider, EventKind::ValueChanged, Box::new(move |ui, s, _| {
        let v = ui.value(s);
        let cur = ui.value(preview);
        ui.anim_start(Anim::new(preview, AnimProp::Value, cur, v, 300));
    }));

    // ---- About 页：多行文本 ----
    let page_about = ui.create_obj(panel);
    ui.set_size(page_about, 188, 192);
    ui.set_layout(page_about, column());
    let la = ui.create_label(
        page_about,
        "qingui subset\nPFB + dirty rect\nanim + keypad\n\narrows/tab: move\nenter: select/edit\nesc: exit edit",
    );
    let _ = la;

    // ---- Animate 页：无限往返动画的 Bar ----
    let page_animate = ui.create_obj(panel);
    ui.set_size(page_animate, 188, 192);
    ui.set_layout(page_animate, column());
    let bar = ui.create_bar(page_animate, 0, 100);
    ui.set_size(bar, 160, 10);
    let mut a = Anim::new(bar, AnimProp::Value, 0, 100, 1200);
    a.easing = Easing::EaseInOutQuad;
    a.repeat = -1;
    a.playback = true;
    ui.anim_start(a);

    // ---- LongList 页：20 项超长列表 + 增删按钮 ----
    let page_longlist = ui.create_obj(panel);
    ui.set_size(page_longlist, 188, 192);
    ui.set_layout(page_longlist, column());
    let long_list = ui.create_list(page_longlist, &[
        "Item 01", "Item 02", "Item 03", "Item 04", "Item 05",
        "Item 06", "Item 07", "Item 08", "Item 09", "Item 10",
        "Item 11", "Item 12", "Item 13", "Item 14", "Item 15",
        "Item 16", "Item 17", "Item 18", "Item 19", "Item 20",
    ]);
    ui.set_size(long_list, 160, 5 * 16 + 2);

    let btn_row = ui.create_obj(page_longlist);
    ui.set_size(btn_row, 160, 28);
    ui.set_layout(btn_row, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    }));
    let add_btn = ui.create_button(btn_row, "Add");
    let del_btn = ui.create_button(btn_row, "Del");

    // Add：在选中项下方插入（淡入 + 下方项下滑），满 20 项忽略
    let next_n = std::cell::Cell::new(21i32);
    ui.add_event_cb(add_btn, EventKind::Clicked, Box::new(move |ui, _b, _| {
        let idx = ui.list_selected(long_list) + 1;
        let name = format!("Item {:02}", next_n.get());
        if ui.list_insert(long_list, idx, &name) {
            next_n.set(next_n.get() + 1);
        }
    }));
    // Del：删除选中项（渐隐 + 下方项上移）
    ui.add_event_cb(del_btn, EventKind::Clicked, Box::new(move |ui, _b, _| {
        ui.list_remove(long_list);
    }));

    ui.set_hidden(page_about, true);
    ui.set_hidden(page_animate, true);
    ui.set_hidden(page_longlist, true);

    // 菜单点击 → 切页 + 面板滑入动画
    ui.add_event_cb(menu, EventKind::Clicked, Box::new(move |ui, m, _| {
        let idx = ui.list_selected(m);
        ui.set_hidden(page_settings, idx != 0);
        ui.set_hidden(page_about, idx != 1);
        ui.set_hidden(page_animate, idx != 2);
        ui.set_hidden(page_longlist, idx != 3);
        ui.set_pos(panel, 320, 32);
        let mut a = Anim::new(panel, AnimProp::X, 320, 116, 200);
        a.easing = Easing::EaseOutQuad;
        ui.anim_start(a);
    }));

    // 焦点组：菜单 → slider → switch → 超长列表 → Add/Del
    ui.group_add(menu);
    ui.group_add(slider);
    ui.group_add(sw);
    ui.group_add(long_list);
    ui.group_add(add_btn);
    ui.group_add(del_btn);
}
