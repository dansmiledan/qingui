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
    ui.set_pos(c, 0, 0);
    ui.set_size(c, 200, 100);
    (0..n)
        .map(|_| {
            let ch = ObjBuilder::new().build(ui, c);
            ui.set_size(ch, w, h);
            ch
        })
        .collect()
}

#[test]
fn row_start_gap() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    ui.set_layout(c, flex(FlexDir::Row, Align::Start, Align::Start, 5));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).x, 0);
    assert_eq!(ui.rect(kids[1]).x, 25);
    assert_eq!(ui.rect(kids[2]).x, 50);
}

#[test]
fn row_space_between() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    ui.set_layout(c, flex(FlexDir::Row, Align::SpaceBetween, Align::Start, 0));
    ui.timer_handler();
    // 容器宽 200，子宽 20×3=60，剩余 140 分两间隙 = 70
    assert_eq!(ui.rect(kids[0]).x, 0);
    assert_eq!(ui.rect(kids[1]).x, 90);
    assert_eq!(ui.rect(kids[2]).x, 180);
}

#[test]
fn row_center_cross_center() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 1, 20, 10);
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    ui.set_layout(c, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Center, track: Align::Center, gap: 0,
    }));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).x, 90); // (200-20)/2
    assert_eq!(ui.rect(kids[0]).y, 45); // (100-10)/2，track Center 把行整体居中
}

#[test]
fn column_wrap() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 4, 20, 40); // 容器高 100 → 每列 2 个
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    let mut f = flex(FlexDir::Column, Align::Start, Align::Start, 0);
    if let Layout::Flex(ref mut fl) = f {
        fl.wrap = true;
    }
    ui.set_layout(c, f);
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).y, 0);
    assert_eq!(ui.rect(kids[1]).y, 40);
    assert_eq!(ui.rect(kids[2]).y, 0); // 换列
    assert_eq!(ui.rect(kids[2]).x, 20); // 第二列 x = 列宽 20
}

#[test]
fn layout_reruns_on_size_change() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 2, 20, 10);
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    ui.set_layout(c, flex(FlexDir::Row, Align::End, Align::Start, 0));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[1]).x, 180);
    ui.set_size(c, 100, 100); // 容器变小 → 布局标脏 → 下一帧重算
    ui.timer_handler();
    assert_eq!(ui.rect(kids[1]).x, 80);
}

#[test]
fn reorder_children_relayouts() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    ui.set_layout(c, flex(FlexDir::Row, Align::Start, Align::Start, 0));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).x, 0);
    // 把最后一个移到最前 → 重排后位置更新
    ui.move_child_to_index(kids[2], 0);
    ui.timer_handler();
    assert_eq!(ui.rect(kids[2]).x, 0);
    assert_eq!(ui.rect(kids[0]).x, 20);
    assert_eq!(ui.rect(kids[1]).x, 40);
}
