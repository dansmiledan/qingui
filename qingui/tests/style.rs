use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use qingui::style::{theme_base, theme_button, theme_button_focused, Style};
use qingui::widgets::obj::ObjCfg;
use qingui::Ui;

#[test]
fn merge_some_overrides_none_keeps() {
    let base = Style::new().bg(Rgb888::RED).radius(4);
    let merged = base.merge(Style::new().bg(Rgb888::BLUE));
    assert_eq!(merged.bg_color, Some(Rgb888::BLUE)); // other's Some overrides
    assert_eq!(merged.radius, Some(4)); // other's None keeps base
}

#[test]
fn theme_base_provides_common_defaults() {
    let b = theme_base();
    assert_eq!(b.text_color, Some(Rgb888::WHITE));
    assert_eq!(b.bg_color, None); // no bg_color = transparent background
    assert_eq!(b.radius, Some(4));
}

#[test]
fn composed_theme_button_matches_expected() {
    // theme_button is composed from theme_base; its field values must match the composition semantics
    let b = theme_button();
    assert_eq!(b.bg_color, Some(Rgb888::new(60, 90, 160)));
    assert_eq!(b.radius, Some(6));
    assert_eq!(b.border_color, Some(Rgb888::new(90, 120, 200)));
    assert_eq!(b.border_width, Some(1));
    assert_eq!(b.text_color, Some(Rgb888::WHITE)); // from theme_base
}

#[test]
fn default_button_resolves_theme() {
    let mut ui: Ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let b = ObjCfg::new().build(&mut ui, scr);
    ui.set_style(b, theme_button());
    let r = ui.resolved_style(b);
    assert_eq!(r.bg_color, theme_button().bg_color);
    assert_eq!(r.border_width, theme_button::<Rgb888>().border_width.unwrap());
}

#[test]
fn base_style_field_fallback() {
    let mut ui: Ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    let mut s = Style::default();
    s.bg_color = Some(Rgb888::RED);
    ui.set_style(o, s);
    let r = ui.resolved_style(o);
    assert_eq!(r.bg_color, Some(Rgb888::RED));
    assert_eq!(r.radius, 0); // unset fields fall back to the default
}

#[test]
fn state_override_wins_then_falls_back() {
    let mut ui: Ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let b = ObjCfg::new().build(&mut ui, scr);
    let mut base = theme_button();
    base.bg_color = Some(Rgb888::BLUE);
    ui.set_style(b, base.clone());
    let mut focused = theme_button_focused();
    focused.bg_color = Some(Rgb888::GREEN);
    ui.set_style_focused(b, focused);
    assert_eq!(ui.resolved_style(b).bg_color, Some(Rgb888::BLUE));

    ui.set_state(b, qingui::node::State::FOCUSED, true);
    assert_eq!(ui.resolved_style(b).bg_color, Some(Rgb888::GREEN));
    // fields not overridden by the focused overlay still fall back to base
    assert_eq!(ui.resolved_style(b).radius, base.radius.unwrap());
}
