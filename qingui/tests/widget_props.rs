use qingui::prelude::*;
use qingui::Ui;
use qingui::widgets::spinner::{SpinnerCfg, SpinnerState};
use qingui::widgets::roller::{RollerCfg, RollerState};
use qingui::widgets::list::{ListCfg, ListState};

#[test]
fn spinner_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = SpinnerCfg::new().build(&mut ui, scr);
    let s = ui.widget::<SpinnerState>(a).unwrap();
    assert_eq!((s.line_width, s.period_ms), (3, 1800));
    let b = SpinnerCfg::new().line_width(6).period_ms(1200).build(&mut ui, scr);
    let s = ui.widget::<SpinnerState>(b).unwrap();
    assert_eq!((s.line_width, s.period_ms), (6, 1200));
}

#[test]
fn roller_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let items = ["a", "b", "c", "d", "e"];
    let a = RollerCfg::new(&items).build(&mut ui, scr);
    assert_eq!(ui.rect(a).h, 3 * qingui::widgets::roller::ROW_H + 8);
    let s = ui.widget::<RollerState>(a).unwrap();
    assert_eq!((s.row_h, s.roll_dur), (16, 150));
    let b = RollerCfg::new(&items).row_h(24).roll_dur(300).visible_rows(5).build(&mut ui, scr);
    assert_eq!(ui.rect(b).h, 5 * 24 + 8);
    let s = ui.widget::<RollerState>(b).unwrap();
    assert_eq!((s.row_h, s.roll_dur), (24, 300));
}

#[test]
fn list_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let items = ["a", "b", "c", "d", "e", "f", "g"];
    let a = ListCfg::new(&items).build(&mut ui, scr);
    assert_eq!(ui.rect(a).h, 5 * qingui::widgets::list::ROW_H + 2);
    let s = ui.widget::<ListState>(a).unwrap();
    assert_eq!((s.row_h, s.fx_dur), (16, 200));
    let b = ListCfg::new(&items).row_h(24).fx_dur(80).visible_rows(3).build(&mut ui, scr);
    assert_eq!(ui.rect(b).h, 3 * 24 + 2);
    // Row height feeds the insert shift effect offsets
    ui.list_insert(b, 0, "x");
    let s = ui.widget::<ListState>(b).unwrap();
    assert!(s.fx.item_fx.iter().any(|f| f.dy == -24));
}
