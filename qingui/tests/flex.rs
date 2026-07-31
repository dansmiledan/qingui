use qingui::layout::{Align, Flex, FlexDir};
use qingui::style::Layout;
use qingui::widgets::obj::ObjBuilder;
use qingui::Ui;

fn flex(dir: FlexDir, main: Align, cross: Align, gap: i32) -> Layout {
    Layout::Flex(Flex { dir, wrap: false, main, cross, track: Align::Start, gap })
}

fn row_of(ui: &mut Ui, n: usize, w: i32, h: i32) -> Vec<qingui::ObjRef> {
    let scr = ui.screen();
    let c = ObjBuilder::new().build(ui, scr);
    c.set_pos(ui, 0, 0);
    c.set_size(ui, 200, 100);
    (0..n)
        .map(|_| {
            let ch = ObjBuilder::new().build(ui, c);
            ch.set_size(ui, w, h);
            ch
        })
        .collect()
}

#[test]
fn row_start_gap() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let scr = ui.screen();
    let c = scr.children(&ui)[0];
    c.set_layout(&mut ui, flex(FlexDir::Row, Align::Start, Align::Start, 5));
    ui.timer_handler();
    assert_eq!(kids[0].rect(&ui).x, 0);
    assert_eq!(kids[1].rect(&ui).x, 25);
    assert_eq!(kids[2].rect(&ui).x, 50);
}

#[test]
fn row_space_between() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let scr = ui.screen();
    let c = scr.children(&ui)[0];
    c.set_layout(&mut ui, flex(FlexDir::Row, Align::SpaceBetween, Align::Start, 0));
    ui.timer_handler();
    // 容器宽 200，子宽 20×3=60，剩余 140 分两间隙 = 70
    assert_eq!(kids[0].rect(&ui).x, 0);
    assert_eq!(kids[1].rect(&ui).x, 90);
    assert_eq!(kids[2].rect(&ui).x, 180);
}

#[test]
fn row_center_cross_center() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 1, 20, 10);
    let scr = ui.screen();
    let c = scr.children(&ui)[0];
    c.set_layout(&mut ui, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Center, track: Align::Center, gap: 0,
    }));
    ui.timer_handler();
    assert_eq!(kids[0].rect(&ui).x, 90); // (200-20)/2
    assert_eq!(kids[0].rect(&ui).y, 45); // (100-10)/2，track Center 把行整体居中
}

#[test]
fn column_wrap() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 4, 20, 40); // 容器高 100 → 每列 2 个
    let scr = ui.screen();
    let c = scr.children(&ui)[0];
    let mut f = flex(FlexDir::Column, Align::Start, Align::Start, 0);
    if let Layout::Flex(ref mut fl) = f {
        fl.wrap = true;
    }
    c.set_layout(&mut ui, f);
    ui.timer_handler();
    assert_eq!(kids[0].rect(&ui).y, 0);
    assert_eq!(kids[1].rect(&ui).y, 40);
    assert_eq!(kids[2].rect(&ui).y, 0); // 换列
    assert_eq!(kids[2].rect(&ui).x, 20); // 第二列 x = 列宽 20
}

#[test]
fn layout_reruns_on_size_change() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 2, 20, 10);
    let scr = ui.screen();
    let c = scr.children(&ui)[0];
    c.set_layout(&mut ui, flex(FlexDir::Row, Align::End, Align::Start, 0));
    ui.timer_handler();
    assert_eq!(kids[1].rect(&ui).x, 180);
    c.set_size(&mut ui, 100, 100); // 容器变小 → 布局标脏 → 下一帧重算
    ui.timer_handler();
    assert_eq!(kids[1].rect(&ui).x, 80);
}

#[test]
fn reorder_children_relayouts() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let scr = ui.screen();
    let c = scr.children(&ui)[0];
    c.set_layout(&mut ui, flex(FlexDir::Row, Align::Start, Align::Start, 0));
    ui.timer_handler();
    assert_eq!(kids[0].rect(&ui).x, 0);
    // 把最后一个移到最前 → 重排后位置更新
    kids[2].move_child_to_index(&mut ui, 0);
    ui.timer_handler();
    assert_eq!(kids[2].rect(&ui).x, 0);
    assert_eq!(kids[0].rect(&ui).x, 20);
    assert_eq!(kids[1].rect(&ui).x, 40);
}
