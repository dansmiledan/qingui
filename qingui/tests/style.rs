use qingui::style::{theme_base, theme_button, theme_button_pressed, Style};
use qingui::widgets::obj::ObjBuilder;
use qingui::{Color, Ui};

#[test]
fn merge_some_overrides_none_keeps() {
    let base = Style::new().bg(Color::RED).radius(4);
    let merged = base.merge(Style::new().bg(Color::BLUE));
    assert_eq!(merged.bg_color, Some(Color::BLUE)); // other 的 Some 覆盖
    assert_eq!(merged.radius, Some(4)); // other 的 None 保留 base
    assert_eq!(merged.bg_opa, None); // 双方都 None 保持 None
}

#[test]
fn merge_layout_and_sizing_fields() {
    use qingui::layout::Sizing;
    let base = Style::new().sizing(Sizing::GROW, Sizing::FIT);
    let merged = base.clone().merge(Style::new().bg(Color::RED));
    assert_eq!(merged.sizing_w, Some(Sizing::GROW)); // sizing 保留
    assert_eq!(merged.bg_color, Some(Color::RED));
    let m2 = base.merge(Style::new().sizing(Sizing::FIT, Sizing::FIT));
    assert_eq!(m2.sizing_w, Some(Sizing::FIT)); // sizing 也可被覆盖
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
    // theme_button 由 theme_base 组合而来，字段值必须与组合语义一致
    let b = theme_button();
    assert_eq!(b.bg_color, Some(Color::rgb(60, 90, 160)));
    assert_eq!(b.radius, Some(6));
    assert_eq!(b.border_color, Some(Color::rgb(90, 120, 200)));
    assert_eq!(b.border_width, Some(1));
    assert_eq!(b.text_color, Some(Color::WHITE)); // 来自 theme_base
    assert_eq!(b.bg_opa, Some(255)); // 来自 theme_base
}

#[test]
fn default_button_resolves_theme() {
    let mut ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let b = ObjBuilder::new().build(&mut ui, scr);
    b.set_style(&mut ui, theme_button());
    let r = b.resolved_style(&ui);
    assert_eq!(r.bg_color, theme_button().bg_color.unwrap());
    assert_eq!(r.bg_opa, 255);
    assert_eq!(r.border_width, theme_button().border_width.unwrap());
}

#[test]
fn base_style_field_fallback() {
    let mut ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let o = ObjBuilder::new().build(&mut ui, scr);
    let mut s = Style::default();
    s.bg_color = Some(Color::RED);
    o.set_style(&mut ui, s);
    let r = o.resolved_style(&ui);
    assert_eq!(r.bg_color, Color::RED);
    assert_eq!(r.bg_opa, 255); // 未设置字段落回默认
}

#[test]
fn state_override_wins_then_falls_back() {
    let mut ui = Ui::new(320, 240, 40);
    let scr = ui.screen();
    let b = ObjBuilder::new().build(&mut ui, scr);
    let mut base = theme_button();
    base.bg_color = Some(Color::BLUE);
    b.set_style(&mut ui, base.clone());
    let mut pressed = theme_button_pressed();
    pressed.bg_color = Some(Color::GREEN);
    b.set_style_pressed(&mut ui, pressed);
    assert_eq!(b.resolved_style(&ui).bg_color, Color::BLUE);

    b.set_state(&mut ui, qingui::node::State::PRESSED, true);
    assert_eq!(b.resolved_style(&ui).bg_color, Color::GREEN);
    // pressed 未覆盖的字段仍回落到 base
    assert_eq!(b.resolved_style(&ui).radius, base.radius.unwrap());
}
