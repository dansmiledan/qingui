use qingui::widgets::button::ButtonCfg;
use qingui::widgets::chart::{ChartCfg, ChartState};
use qingui::widgets::table::{TableCfg, TableState};
use qingui::prelude::*;
use qingui::Ui;
use qingui::widgets::spinner::{SpinnerCfg, SpinnerState};
use qingui::widgets::roller::{RollerCfg, RollerState};
use qingui::widgets::list::{ListCfg, ListState};
use qingui::widgets::arc::{ArcCfg, ArcState};
use qingui::widgets::slider::{SliderCfg, SliderState};
use qingui::widgets::checkbox::{CheckboxCfg, CheckboxState};
use qingui::widgets::dropdown::{DropdownCfg, DropdownState};
use qingui::input::Key;
use qingui::widgets::obj::ObjCfg;
use qingui::widgets::scrollview::{ScrollViewCfg, ScrollViewState};

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

#[test]
fn dropdown_popup_props_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = DropdownCfg::new(&["x", "y"]).build(&mut ui, scr);
    let s = ui.widget::<DropdownState>(a).unwrap();
    assert_eq!((s.popup_rows, s.popup_row_h, s.popup_min_w), (5, 16, 80));
    let b = DropdownCfg::new(&["x", "y"]).popup_rows(3).popup_row_h(20).popup_min_w(120).build(&mut ui, scr);
    let s = ui.widget::<DropdownState>(b).unwrap();
    assert_eq!((s.popup_rows, s.popup_row_h, s.popup_min_w), (3, 20, 120));
}

#[test]
fn table_cell_props_default_and_override() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let a = TableCfg::new(2, 3).build(&mut ui, scr);
    assert_eq!((ui.rect(a).w, ui.rect(a).h), (2 * qingui::widgets::table::CELL_W, 3 * qingui::widgets::table::CELL_H));
    let s = ui.widget::<TableState>(a).unwrap();
    assert_eq!((s.cell_w, s.cell_h), (60, 16));
    let b = TableCfg::new(2, 3).cell_w(40).cell_h(20).build(&mut ui, scr);
    assert_eq!((ui.rect(b).w, ui.rect(b).h), (80, 60));
    let s = ui.widget::<TableState>(b).unwrap();
    assert_eq!((s.cell_w, s.cell_h), (40, 20));
}

#[test]
fn scrollview_step_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let sv = ScrollViewCfg::new().size(60, 60).step(8).build(&mut ui, scr);
    assert_eq!(ui.widget::<ScrollViewState>(sv).unwrap().step, 8);
    let content = ui.scrollview_content(sv).unwrap();
    // Content taller than the viewport so there is room to scroll
    let _tall = ObjCfg::new().size(60, 200).build(&mut ui, content);
    ui.group_add(sv);
    ui.group_focus(sv);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.translate(content).y, -8);
}

#[test]
fn chart_line_width_default_and_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ChartCfg::new().build(&mut ui, scr);
    assert_eq!(ui.widget::<ChartState>(a).unwrap().line_width, 2);
    let b = ChartCfg::new().line_width(4).build(&mut ui, scr);
    assert_eq!(ui.widget::<ChartState>(b).unwrap().line_width, 4);
}

#[test]
fn button_content_pad_override() {
    let mut ui = Ui::new(160, 120, 120);
    let scr = ui.screen();
    let a = ButtonCfg::new("Go").build(&mut ui, scr);
    let b = ButtonCfg::new("Go").content_pad(40, 20).build(&mut ui, scr);
    let (ra, rb) = (ui.rect(a), ui.rect(b));
    // Same text: the size delta equals the content_pad delta from the default (24, 12)
    assert_eq!((rb.w - ra.w, rb.h - ra.h), (40 - 24, 20 - 12));
}
