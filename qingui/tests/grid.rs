use qingui::layout::{Grid, Track};
use qingui::style::Layout;
use qingui::widgets::obj::ObjBuilder;
use qingui::Ui;

fn grid(cols: Vec<Track>, rows: Vec<Track>, gap: i32) -> Layout {
    Layout::Grid(Grid { cols, rows, col_gap: gap, row_gap: gap })
}

#[test]
fn px_tracks_place_children() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjBuilder::new().build(&mut ui, scr);
    c.set_pos(&mut ui, 0, 0);
    c.set_size(&mut ui, 300, 200);
    c.set_layout(&mut ui, grid(vec![Track::Px(100), Track::Px(100)], vec![Track::Px(50), Track::Px(50)], 10));
    let a = ObjBuilder::new().build(&mut ui, c);
    a.set_size(&mut ui, 10, 10);
    a.set_grid_cell(&mut ui, (0, 1), (0, 1));
    let b = ObjBuilder::new().build(&mut ui, c);
    b.set_size(&mut ui, 10, 10);
    b.set_grid_cell(&mut ui, (1, 1), (1, 1));
    ui.timer_handler();
    assert_eq!((a.rect(&ui).x, a.rect(&ui).y), (0, 0));
    assert_eq!((b.rect(&ui).x, b.rect(&ui).y), (110, 60)); // 100+gap, 50+gap
}

#[test]
fn fr_shares_remaining_space() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjBuilder::new().build(&mut ui, scr);
    c.set_size(&mut ui, 300, 100);
    c.set_layout(&mut ui, grid(vec![Track::Px(100), Track::Fr(1), Track::Fr(2)], vec![Track::Px(50)], 0));
    let a = ObjBuilder::new().build(&mut ui, c);
    a.set_grid_cell(&mut ui, (1, 1), (0, 1));
    let b = ObjBuilder::new().build(&mut ui, c);
    b.set_grid_cell(&mut ui, (2, 1), (0, 1));
    ui.timer_handler();
    // 剩余 200，fr1=66（200/3 取整），fr2=134
    assert_eq!(a.rect(&ui).x, 100);
    let fr1 = b.rect(&ui).x - 100;
    assert!((fr1 - 66).abs() <= 1);
}

#[test]
fn content_track_sizes_to_child() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjBuilder::new().build(&mut ui, scr);
    c.set_size(&mut ui, 300, 100);
    c.set_layout(&mut ui, grid(vec![Track::Content, Track::Px(10)], vec![Track::Px(50)], 0));
    let a = ObjBuilder::new().build(&mut ui, c);
    a.set_size(&mut ui, 42, 10);
    a.set_grid_cell(&mut ui, (0, 1), (0, 1));
    let b = ObjBuilder::new().build(&mut ui, c);
    b.set_grid_cell(&mut ui, (1, 1), (0, 1));
    ui.timer_handler();
    assert_eq!(b.rect(&ui).x, 42); // content 轨道 = 最宽子对象 42
}

#[test]
fn span_places_across_tracks() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjBuilder::new().build(&mut ui, scr);
    c.set_size(&mut ui, 300, 100);
    c.set_layout(&mut ui, grid(vec![Track::Px(50), Track::Px(50)], vec![Track::Px(50)], 10));
    let a = ObjBuilder::new().build(&mut ui, c);
    a.set_size(&mut ui, 10, 10);
    a.set_grid_cell(&mut ui, (0, 2), (0, 1)); // 跨 2 列
    let b = ObjBuilder::new().build(&mut ui, c);
    b.set_grid_cell(&mut ui, (1, 1), (0, 1));
    ui.timer_handler();
    assert_eq!(a.rect(&ui).x, 0);
    assert_eq!(b.rect(&ui).x, 60);
}

#[test]
fn ignore_layout_child_not_managed() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjBuilder::new().build(&mut ui, scr);
    c.set_size(&mut ui, 300, 100);
    c.set_layout(&mut ui, grid(vec![Track::Content, Track::Px(10)], vec![Track::Px(50)], 0));
    let a = ObjBuilder::new().build(&mut ui, c);
    a.set_size(&mut ui, 42, 10);
    a.set_grid_cell(&mut ui, (0, 1), (0, 1));
    let b = ObjBuilder::new().build(&mut ui, c);
    b.set_grid_cell(&mut ui, (1, 1), (0, 1));
    // 浮动对象：不参与布局（包括 content 轨道测量与定位）
    let f = ObjBuilder::new().build(&mut ui, c);
    f.set_size(&mut ui, 200, 200);
    f.set_ignore_layout(&mut ui, true);
    ui.timer_handler();
    assert_eq!(b.rect(&ui).x, 42); // content 轨道只算 a（不含 f 的 200）
    assert_eq!(f.rect(&ui).x, 0); // f 不被重新定位
}
