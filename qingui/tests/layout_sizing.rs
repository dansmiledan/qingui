use qingui::layout::{Align, Flex, FlexDir, Grid, Sizing, Track};
use qingui::style::Layout;
use qingui::Ui;

fn row_flex() -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
    })
}

fn container(ui: &mut Ui, w: i32, h: i32) -> qingui::ObjRef {
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, w, h);
    ui.set_layout(c, row_flex());
    c
}

#[test]
fn flex_grow_fills_remaining() {
    let mut ui = Ui::new(320, 240, 240);
    let c = container(&mut ui, 200, 40);
    let a = ui.create_obj(c);
    ui.set_size(a, 50, 10);
    let b = ui.create_obj(c);
    ui.set_sizing(b, Some(Sizing::GROW), None);
    ui.timer_handler();
    assert_eq!(ui.rect(b).w, 150); // 200 - 50
    assert_eq!(ui.rect(b).x, 50);
}

#[test]
fn flex_two_grow_share_equally() {
    let mut ui = Ui::new(320, 240, 240);
    let c = container(&mut ui, 200, 40);
    let a = ui.create_obj(c);
    ui.set_sizing(a, Some(Sizing::GROW), None);
    let b = ui.create_obj(c);
    ui.set_sizing(b, Some(Sizing::GROW), None);
    ui.timer_handler();
    assert_eq!(ui.rect(a).w, 100);
    assert_eq!(ui.rect(b).w, 100);
    assert_eq!(ui.rect(b).x, 100);
}

#[test]
fn flex_grow_respects_max() {
    let mut ui = Ui::new(320, 240, 240);
    let c = container(&mut ui, 200, 40);
    let a = ui.create_obj(c);
    ui.set_sizing(a, Some(Sizing::Grow { min: 0, max: 100 }), None);
    ui.timer_handler();
    assert_eq!(ui.rect(a).w, 100); // 剩余 200 但 clamp 到 max
}

#[test]
fn flex_grow_cross_axis_fills_line() {
    let mut ui = Ui::new(320, 240, 240);
    let c = container(&mut ui, 200, 40);
    let a = ui.create_obj(c);
    ui.set_size(a, 50, 10);
    let b = ui.create_obj(c);
    ui.set_sizing(b, None, Some(Sizing::GROW));
    ui.timer_handler();
    assert_eq!(ui.rect(b).h, 40); // 交叉轴撑满行高
}

#[test]
fn flex_percent_sizing() {
    let mut ui = Ui::new(320, 240, 240);
    let c = container(&mut ui, 200, 40);
    let a = ui.create_obj(c);
    ui.set_sizing(a, Some(Sizing::Percent(50)), None);
    ui.timer_handler();
    assert_eq!(ui.rect(a).w, 100);
}

#[test]
fn grid_child_grow_fills_cell() {
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, 300, 100);
    ui.set_layout(c, Layout::Grid(Grid {
        cols: vec![Track::Px(100), Track::Fr(1)],
        rows: vec![Track::Fr(1)],
        col_gap: 10,
        row_gap: 0,
    }));
    let a = ui.create_obj(c);
    ui.set_grid_cell(a, (1, 1), (0, 1));
    ui.set_sizing(a, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.timer_handler();
    // Fr 列 = 300-100-10 = 190，行 = 100
    assert_eq!(ui.rect(a).w, 190);
    assert_eq!(ui.rect(a).h, 100);
    assert_eq!(ui.rect(a).x, 110);
}
