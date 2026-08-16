use embedded_graphics::pixelcolor::RgbColor;
use qingui::widgets::bar::BarCfg;
use qingui::widgets::chart::{ChartCfg, ChartState};
use qingui::{Color, Ui};

#[test]
fn update_mutates_and_invalidates() {
    let mut ui: Ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let c = ChartCfg::new().series(Color::BLUE, 4).build(&mut ui, s);
    ui.take_dirty();
    let r = ui.update::<ChartState, _>(c, |st| {
        st.series[0].push(7);
        st.series.len()
    });
    assert_eq!(r, Some(1));
    assert!(!ui.dirty_is_empty()); // running f marks it dirty
}

#[test]
fn update_wrong_type_is_noop() {
    let mut ui: Ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let b = BarCfg::new(0, 100).build(&mut ui, s); // BarState, not ChartState
    ui.take_dirty();
    let r = ui.update::<ChartState, _>(b, |st| st.series.len());
    assert_eq!(r, None);
    assert!(ui.dirty_is_empty());
}

#[test]
fn update_deleted_obj_is_noop() {
    let mut ui: Ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let c = ChartCfg::new().series(Color::BLUE, 4).build(&mut ui, s);
    ui.delete(c);
    ui.take_dirty();
    let r = ui.update::<ChartState, _>(c, |st| st.series.len());
    assert_eq!(r, None);
    assert!(ui.dirty_is_empty());
}
