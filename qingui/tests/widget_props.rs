use qingui::prelude::*;
use qingui::Ui;
use qingui::widgets::spinner::{SpinnerCfg, SpinnerState};
use qingui::widgets::roller::{RollerCfg, RollerState};
use qingui::widgets::list::{ListCfg, ListState};
use qingui::widgets::arc::{ArcCfg, ArcState};
use qingui::widgets::slider::{SliderCfg, SliderState};
use qingui::widgets::checkbox::{CheckboxCfg, CheckboxState};

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

#[test]
fn arc_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ArcCfg::new(0, 100).build(&mut ui, scr);
    let s = ui.widget::<ArcState>(a).unwrap();
    assert_eq!((s.track_w, s.start_deg, s.sweep_deg), (4, 135, 270));
    let b = ArcCfg::new(0, 100).track_w(6).start_deg(0).sweep_deg(180).build(&mut ui, scr);
    let s = ui.widget::<ArcState>(b).unwrap();
    assert_eq!((s.track_w, s.start_deg, s.sweep_deg), (6, 0, 180));
}

#[test]
fn slider_knob_w_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = SliderCfg::new(0, 100).build(&mut ui, scr);
    assert_eq!(ui.widget::<SliderState>(a).unwrap().knob_w, 8);
    let b = SliderCfg::new(0, 100).knob_w(14).build(&mut ui, scr);
    assert_eq!(ui.widget::<SliderState>(b).unwrap().knob_w, 14);
}

#[test]
fn checkbox_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = CheckboxCfg::new("ab").build(&mut ui, scr);
    let s = ui.widget::<CheckboxState>(a).unwrap();
    assert_eq!((s.box_size, s.gap), (12, 6));
    let w_default = ui.rect(a).w;
    let b = CheckboxCfg::new("ab").box_size(20).gap(10).build(&mut ui, scr);
    let s = ui.widget::<CheckboxState>(b).unwrap();
    assert_eq!((s.box_size, s.gap), (20, 10));
    // Same text: default width grows by exactly the box/gap delta
    assert_eq!(ui.rect(b).w - w_default, (20 - 12) + (10 - 6));
}
