use qingui::anim::{Anim, AnimProp, Easing};
use qingui::layout::{Align, Flex, FlexDir};
use qingui::style::Layout;
use qingui::{EventKind, ObjRef, Ui};

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

    // ---- Animate 页：无限往返动画的 Bar + 圆弧仪表盘 ----
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

    // 圆弧仪表盘：隐藏 Bar 驱动角度，Canvas 自定义绘制
    let angle = std::rc::Rc::new(std::cell::Cell::new(0i32));
    let angle2 = angle.clone();
    let gauge = ui.create_canvas(page_animate, 70, 70, Box::new(move |d, abs, clip, _now| {
        let cx = qingui::Point { x: abs.x + 35, y: abs.y + 35 };
        // 背景环（灰）
        d.draw_circle(cx, 28, 5, qingui::Color::rgb(60, 60, 70), 255, clip);
        // 旋转圆弧
        d.draw_arc(cx, 28, 5, 0, angle2.get(), qingui::Color::rgb(80, 140, 255), 255, clip);
        // 中心点
        d.fill_circle(cx, 4, qingui::Color::WHITE, 255, clip);
    }));
    let driver = ui.create_bar(page_animate, 0, 360);
    ui.set_hidden(driver, true);
    ui.add_event_cb(driver, EventKind::ValueChanged, Box::new(move |ui, b, _| {
        angle.set(ui.value(b));
        ui.invalidate_obj(gauge);
    }));
    let mut ga = Anim::new(driver, AnimProp::Value, 0, 360, 2400);
    ga.repeat = -1;
    ui.anim_start(ga);

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

    // LongList 项点击 → 弹出模态对话框（遮罩 + 对话框，最上层）
    let popup: std::rc::Rc<std::cell::Cell<Option<(ObjRef, Option<ObjRef>)>>> =
        std::rc::Rc::new(std::cell::Cell::new(None));
    let popup_open = popup.clone();
    ui.add_event_cb(long_list, EventKind::Clicked, Box::new(move |ui, l, _| {
        if popup_open.get().is_some() {
            return; // 已打开
        }
        let prev_focus = ui.focused();
        let idx = ui.list_selected(l);
        let screen = ui.screen();
        // 遮罩（最后创建 → 渲染在最上层）
        let mask = ui.create_obj(screen);
        ui.set_pos(mask, 0, 0);
        ui.set_size(mask, 320, 240);
        let mut ms = qingui::style::Style::default();
        ms.bg_color = Some(qingui::Color::BLACK);
        ms.bg_opa = Some(140);
        ui.set_style(mask, ms);
        // 对话框
        let dlg = ui.create_obj(mask);
        ui.set_size(dlg, 180, 90);
        ui.set_pos(dlg, (320 - 180) / 2, (240 - 90) / 2);
        ui.set_style(dlg, qingui::style::theme_obj());
        let mut ds = qingui::style::Style::default();
        ds.border_color = Some(qingui::Color::WHITE);
        ds.border_width = Some(2);
        ui.set_style(dlg, ds);
        let msg = ui.create_label(dlg, &format!("Clicked Item {:02}", idx + 1));
        ui.set_pos(msg, 12, 14);
        let ok = ui.create_button(dlg, "OK");
        ui.set_pos(ok, 62, 52);
        ui.group_add(ok);
        ui.set_modal(dlg); // 焦点锁进对话框
        // 关闭：OK 点击或 Esc，恢复之前焦点
        let close = move |ui: &mut Ui, popup: &std::rc::Rc<std::cell::Cell<Option<(ObjRef, Option<ObjRef>)>>>| {
            if let Some((m, prev)) = popup.get() {
                ui.clear_modal();
                ui.delete(m);
                popup.set(None);
                if let Some(p) = prev {
                    ui.group_focus(p);
                }
            }
        };
        let pc = popup_open.clone();
        let close2 = close.clone();
        ui.add_event_cb(ok, EventKind::Clicked, Box::new(move |ui, _b, _| close2(ui, &pc)));
        let pk = popup_open.clone();
        ui.add_event_cb(ok, EventKind::Key(qingui::input::Key::Esc), Box::new(move |ui, _b, k| {
            if k == EventKind::Key(qingui::input::Key::Esc) {
                close(ui, &pk);
            }
        }));
        popup_open.set(Some((mask, prev_focus)));
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
