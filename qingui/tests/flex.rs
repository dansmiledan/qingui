use qingui::layout::{Align, Flex, FlexDir};
use qingui::widgets::obj::ObjCfg;
use qingui::Ui;

fn flex(dir: FlexDir, main: Align, cross: Align, gap: i32) -> Flex {
    Flex { dir, wrap: false, main, cross, track: Align::Start, gap }
}

fn row_of(ui: &mut Ui, n: usize, w: i32, h: i32) -> Vec<qingui::ObjRef> {
    let scr = ui.screen();
    let c = ObjCfg::new().build(ui, scr);
    ui.set_pos(c, 0, 0);
    ui.set_size(c, 200, 100);
    (0..n)
        .map(|_| {
            let ch = ObjCfg::new().build(ui, c);
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
    ui.set_flex(c, flex(FlexDir::Row, Align::Start, Align::Start, 5));
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
    ui.set_flex(c, flex(FlexDir::Row, Align::SpaceBetween, Align::Start, 0));
    ui.timer_handler();
    // Container width 200, children 20×3=60, the remaining 140 split across two gaps = 70
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
    ui.set_flex(c, Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Center, track: Align::Center, gap: 0,
    });
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).x, 90); // (200-20)/2
    assert_eq!(ui.rect(kids[0]).y, 45); // (100-10)/2, track Center centers the whole row
}

#[test]
fn column_wrap() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 4, 20, 40); // container height 100 → 2 per column
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    let mut f = flex(FlexDir::Column, Align::Start, Align::Start, 0);
    f.wrap = true;
    ui.set_flex(c, f);
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).y, 0);
    assert_eq!(ui.rect(kids[1]).y, 40);
    assert_eq!(ui.rect(kids[2]).y, 0); // wraps to a new column
    assert_eq!(ui.rect(kids[2]).x, 20); // second column x = column width 20
}

#[test]
fn layout_reruns_on_size_change() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 2, 20, 10);
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    ui.set_flex(c, flex(FlexDir::Row, Align::End, Align::Start, 0));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[1]).x, 180);
    ui.set_size(c, 100, 100); // container shrinks → layout marked dirty → recomputed next frame
    ui.timer_handler();
    assert_eq!(ui.rect(kids[1]).x, 80);
}

#[test]
fn reorder_children_relayouts() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let scr = ui.screen();
    let c = ui.children(scr)[0];
    ui.set_flex(c, flex(FlexDir::Row, Align::Start, Align::Start, 0));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).x, 0);
    // Moving the last child to the front → positions update after reorder
    ui.move_child_to_index(kids[2], 0);
    ui.timer_handler();
    assert_eq!(ui.rect(kids[2]).x, 0);
    assert_eq!(ui.rect(kids[0]).x, 20);
    assert_eq!(ui.rect(kids[1]).x, 40);
}

#[test]
fn padded_container_at_nonzero_pos() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let c = ObjCfg::new().build(&mut ui, scr);
    ui.set_pos(c, 50, 30);
    ui.set_size(c, 200, 100);
    ui.set_pad(c, (10, 0, 5, 0));
    let ch = ObjCfg::new().build(&mut ui, c);
    ui.set_size(ch, 20, 10);
    ui.set_flex(c, flex(FlexDir::Row, Align::Start, Align::Start, 0));
    ui.timer_handler();
    // Child origin is the pad offsets in the container's LOCAL space;
    // the container's own position (50, 30) must not shift it.
    assert_eq!((ui.rect(ch).x, ui.rect(ch).y), (10, 5));
}
