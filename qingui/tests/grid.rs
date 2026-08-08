use qingui::layout::{Grid, Track};
use qingui::widgets::obj::ObjCfg;
use qingui::Ui;

fn grid(cols: Vec<Track>, rows: Vec<Track>, gap: i32) -> Grid {
    Grid { cols, rows, col_gap: gap, row_gap: gap }
}

#[test]
fn px_tracks_place_children() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_pos(c, 0, 0);
    ui.set_size(c, 300, 200);
    ui.set_grid(c, grid(vec![Track::Px(100), Track::Px(100)], vec![Track::Px(50), Track::Px(50)], 10));
    let a = ObjCfg::new().build(&mut ui, c);
    ui.set_size(a, 10, 10);
    ui.set_grid_cell(a, (0, 1), (0, 1));
    let b = ObjCfg::new().build(&mut ui, c);
    ui.set_size(b, 10, 10);
    ui.set_grid_cell(b, (1, 1), (1, 1));
    ui.timer_handler();
    assert_eq!((ui.rect(a).x, ui.rect(a).y), (0, 0));
    assert_eq!((ui.rect(b).x, ui.rect(b).y), (110, 60)); // 100+gap, 50+gap
}

#[test]
fn fr_shares_remaining_space() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_size(c, 300, 100);
    ui.set_grid(c, grid(vec![Track::Px(100), Track::Fr(1), Track::Fr(2)], vec![Track::Px(50)], 0));
    let a = ObjCfg::new().build(&mut ui, c);
    ui.set_grid_cell(a, (1, 1), (0, 1));
    let b = ObjCfg::new().build(&mut ui, c);
    ui.set_grid_cell(b, (2, 1), (0, 1));
    ui.timer_handler();
    // Remaining 200, fr1=66 (200/3 truncated), fr2=134
    assert_eq!(ui.rect(a).x, 100);
    let fr1 = ui.rect(b).x - 100;
    assert!((fr1 - 66).abs() <= 1);
}

#[test]
fn content_track_sizes_to_child() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_size(c, 300, 100);
    ui.set_grid(c, grid(vec![Track::Content, Track::Px(10)], vec![Track::Px(50)], 0));
    let a = ObjCfg::new().build(&mut ui, c);
    ui.set_size(a, 42, 10);
    ui.set_grid_cell(a, (0, 1), (0, 1));
    let b = ObjCfg::new().build(&mut ui, c);
    ui.set_grid_cell(b, (1, 1), (0, 1));
    ui.timer_handler();
    assert_eq!(ui.rect(b).x, 42); // content track = widest child 42
}

#[test]
fn span_places_across_tracks() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_size(c, 300, 100);
    ui.set_grid(c, grid(vec![Track::Px(50), Track::Px(50)], vec![Track::Px(50)], 10));
    let a = ObjCfg::new().build(&mut ui, c);
    ui.set_size(a, 10, 10);
    ui.set_grid_cell(a, (0, 2), (0, 1)); // spans 2 columns
    let b = ObjCfg::new().build(&mut ui, c);
    ui.set_grid_cell(b, (1, 1), (0, 1));
    ui.timer_handler();
    assert_eq!(ui.rect(a).x, 0);
    assert_eq!(ui.rect(b).x, 60);
}

#[test]
fn ignore_layout_child_not_managed() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_size(c, 300, 100);
    ui.set_grid(c, grid(vec![Track::Content, Track::Px(10)], vec![Track::Px(50)], 0));
    let a = ObjCfg::new().build(&mut ui, c);
    ui.set_size(a, 42, 10);
    ui.set_grid_cell(a, (0, 1), (0, 1));
    let b = ObjCfg::new().build(&mut ui, c);
    ui.set_grid_cell(b, (1, 1), (0, 1));
    // Floating object: does not participate in layout (including content-track measuring and positioning)
    let f = ObjCfg::new().build(&mut ui, c);
    ui.set_size(f, 200, 200);
    ui.set_ignore_layout(f, true);
    ui.timer_handler();
    assert_eq!(ui.rect(b).x, 42); // the content track only counts a (not f's 200)
    assert_eq!(ui.rect(f).x, 0); // f is not repositioned
}
