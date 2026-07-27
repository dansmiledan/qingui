use qingui::style::{theme_button, theme_button_pressed, Style};
use qingui::{Color, Ui};

#[test]
fn default_button_resolves_theme() {
    let mut ui = Ui::new(320, 240, 40);
    let b = ui.create_obj(ui.screen());
    ui.set_style(b, theme_button());
    let r = ui.resolved_style(b);
    assert_eq!(r.bg_color, theme_button().bg_color.unwrap());
    assert_eq!(r.bg_opa, 255);
    assert_eq!(r.border_width, theme_button().border_width.unwrap());
}

#[test]
fn base_style_field_fallback() {
    let mut ui = Ui::new(320, 240, 40);
    let o = ui.create_obj(ui.screen());
    let mut s = Style::default();
    s.bg_color = Some(Color::RED);
    ui.set_style(o, s);
    let r = ui.resolved_style(o);
    assert_eq!(r.bg_color, Color::RED);
    assert_eq!(r.bg_opa, 255); // 未设置字段落回默认
}

#[test]
fn state_override_wins_then_falls_back() {
    let mut ui = Ui::new(320, 240, 40);
    let b = ui.create_obj(ui.screen());
    let mut base = theme_button();
    base.bg_color = Some(Color::BLUE);
    ui.set_style(b, base.clone());
    let mut pressed = theme_button_pressed();
    pressed.bg_color = Some(Color::GREEN);
    ui.set_style_pressed(b, pressed);
    assert_eq!(ui.resolved_style(b).bg_color, Color::BLUE);

    ui.set_state(b, qingui::node::state::PRESSED, true);
    assert_eq!(ui.resolved_style(b).bg_color, Color::GREEN);
    // pressed 未覆盖的字段仍回落到 base
    assert_eq!(ui.resolved_style(b).radius, base.radius.unwrap());
}
