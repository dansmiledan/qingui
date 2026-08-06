// Regression: no rendering ghosting after layout transitions (moving a container must mark its subtree dirty)
use qingui::display::Flush;
use qingui::layout::{Align, Flex, FlexDir, Grid, Sizing, Track};
use qingui::style::Layout;
use qingui::widgets::label::LabelCfg;
use qingui::widgets::list::ListBuilder;
use qingui::widgets::obj::ObjCfg;
use qingui::{Color, ObjRef, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    fb: Vec<Color>,
}
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        let mut r = self.0.borrow_mut();
        let fb = &mut r.fb;
        for y in 0..area.h {
            for x in 0..area.w {
                fb[(area.y + y) as usize * 320 + (area.x + x) as usize] = pixels[(y * area.w + x) as usize];
            }
        }
    }
}

const TEXT: &str = "qingui subset\nPFB + dirty rect\nanim + keypad\n\narrows/tab: move\nenter: select/edit\nesc: exit edit";

fn build(wide: bool, with_transition: bool) -> (Ui, Rc<RefCell<RecFlush>>, ObjRef, ObjRef) {
    let rec = Rc::new(RefCell::new(RecFlush { fb: vec![Color::BLACK; 320 * 240] }));
    let mut ui = Ui::new(320, 240, 24);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let screen = ui.screen();
    let col = if wide { 108 } else { 180 };
    let mut ss = qingui::style::theme_screen();
    ss.layout = Some(Layout::Grid(Grid {
        cols: vec![Track::Px(col), Track::Fr(1)],
        rows: vec![Track::Content, Track::Fr(1)],
        col_gap: 8,
        row_gap: 8,
    }));
    ss.pad_left = Some(8);
    ss.pad_top = Some(8);
    ui.set_style(screen, ss);

    let menu = ListBuilder::new(&["Settings", "About"]).build(&mut ui, screen);
    ui.set_grid_cell(menu, (0, 1), (1, 1));
    ui.set_sizing(menu, Some(Sizing::GROW), Some(Sizing::GROW));

    let panel = ObjCfg::new().build(&mut ui, screen);
    ui.set_grid_cell(panel, (1, 1), (1, 1));
    ui.set_style(panel, qingui::style::theme_obj());
    ui.set_sizing(panel, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(panel, Layout::Flex(Flex {
        dir: FlexDir::Column, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    }));

    let page = ObjCfg::new().build(&mut ui, panel);
    let mut ps = qingui::style::Style::default();
    ps.bg_opa = Some(0);
    ui.set_style(page, ps);
    ui.set_sizing(page, Some(Sizing::GROW), Some(Sizing::GROW));
    ui.set_layout(page, Layout::Flex(Flex {
        dir: FlexDir::Column, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    }));
    let _la = LabelCfg::new(TEXT).build(&mut ui, page);

    if with_transition {
        for &o in &[menu, panel, page] {
            ui.set_transition(o, Some((250, qingui::anim::Easing::Linear)));
        }
    }
    (ui, rec, menu, panel)
}

fn fb(rec: &Rc<RefCell<RecFlush>>) -> Vec<Color> {
    rec.borrow().fb.clone()
}

#[test]
fn repro_wide_transition_text_ghost() {
    // Reference: built directly in wide mode (108), no animation
    let (mut ui_ref, rec_ref, _, _) = build(true, false);
    ui_ref.tick_inc(1);
    ui_ref.timer_handler();
    let reference = fb(&rec_ref);

    // Repro: start narrow (180) with transitions, switch to wide (108), wait for the animation to finish
    let (mut ui, rec, _, _) = build(false, true);
    ui.tick_inc(1);
    ui.timer_handler();
    let scr = ui.screen();
    ui.set_layout(scr, Layout::Grid(Grid {
        cols: vec![Track::Px(108), Track::Fr(1)],
        rows: vec![Track::Content, Track::Fr(1)],
        col_gap: 8,
        row_gap: 8,
    }));
    for _ in 0..40 {
        ui.tick_inc(16);
        ui.timer_handler();
    }
    assert!(!ui.anim_running(), "过渡应已结束");
    let got = fb(&rec);

    // Find the first mismatching pixel
    let mut bad = None;
    for y in 0..240 {
        for x in 0..320 {
            if reference[y * 320 + x] != got[y * 320 + x] {
                bad = Some((x, y, reference[y * 320 + x], got[y * 320 + x]));
                break;
            }
        }
        if bad.is_some() {
            break;
        }
    }
    if let Some((x, y, r, g)) = bad {
        panic!("像素 ({},{}) 不一致：参考 {:?} vs 实际 {:?}", x, y, r, g);
    }
}
