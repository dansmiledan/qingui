use qingui::style::{theme_base, theme_button, theme_button_focused, Style};
use qingui::widgets::obj::ObjCfg;
use qingui::{Color, Ui};

#[test]
fn merge_some_overrides_none_keeps() {
    let base = Style::new().bg(Color::RED).radius(4);
    let merged = base.merge(Style::new().bg(Color::BLUE));
    assert_eq!(merged.bg_color, Some(Color::BLUE)); // other's Some overrides
    assert_eq!(merged.radius, Some(4)); // other's None keeps base
    assert_eq!(merged.bg_opa, None); // both None stays None
}

#[test]
fn merge_opa_field() {
    let mut base = Style::new();
    base.opa = Some(128);
    let merged = base.clone().merge(Style::new().bg(Color::RED));
    assert_eq!(merged.opa, Some(128)); // opa kept
    assert_eq!(merged.bg_color, Some(Color::RED));
    let mut other = Style::new();
    other.opa = Some(64);
    let m2 = base.merge(other);
    assert_eq!(m2.opa, Some(64)); // opa can also be overridden
}

#[test]
fn theme_base_provides_common_defaults() {
    let b = theme_base();
    assert_eq!(b.text_color, Some(Color::WHITE));
    assert_eq!(b.bg_opa, Some(255));
    assert_eq!(b.radius, Some(4));
}

#[test]
fn composed_theme_button_matches_expected() {
    // theme_button is composed from theme_base; its field values must match the composition semantics
    let b = theme_button();
    assert_eq!(b.bg_color, Some(Color::rgb(60, 90, 160)));
    assert_eq!(b.radius, Some(6));
    assert_eq!(b.border_color, Some(Color::rgb(90, 120, 200)));
    assert_eq!(b.border_width, Some(1));
    assert_eq!(b.text_color, Some(Color::WHITE)); // from theme_base
    assert_eq!(b.bg_opa, Some(255)); // from theme_base
}

#[test]
fn default_button_resolves_theme() {
    let mut ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let b = ObjCfg::new().build(&mut ui, scr);
    ui.set_style(b, theme_button());
    let r = ui.resolved_style(b);
    assert_eq!(r.bg_color, theme_button().bg_color.unwrap());
    assert_eq!(r.bg_opa, 255);
    assert_eq!(r.border_width, theme_button().border_width.unwrap());
}

#[test]
fn base_style_field_fallback() {
    let mut ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let o = ObjCfg::new().build(&mut ui, scr);
    let mut s = Style::default();
    s.bg_color = Some(Color::RED);
    ui.set_style(o, s);
    let r = ui.resolved_style(o);
    assert_eq!(r.bg_color, Color::RED);
    assert_eq!(r.bg_opa, 255); // unset fields fall back to the default
}

#[test]
fn state_override_wins_then_falls_back() {
    let mut ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let b = ObjCfg::new().build(&mut ui, scr);
    let mut base = theme_button();
    base.bg_color = Some(Color::BLUE);
    ui.set_style(b, base.clone());
    let mut focused = theme_button_focused();
    focused.bg_color = Some(Color::GREEN);
    ui.set_style_focused(b, focused);
    assert_eq!(ui.resolved_style(b).bg_color, Color::BLUE);

    ui.set_state(b, qingui::node::State::FOCUSED, true);
    assert_eq!(ui.resolved_style(b).bg_color, Color::GREEN);
    // fields not overridden by the focused overlay still fall back to base
    assert_eq!(ui.resolved_style(b).radius, base.radius.unwrap());
}
