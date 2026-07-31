use qingui::widgets::label::LabelBuilder;
use qingui::widgets::obj::ObjBuilder;
use qingui::{EventKind, Ui};

#[test]
fn handle_methods_roundtrip() {
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.screen();
    let l = LabelBuilder::new("hi").build(&mut ui, s);
    l.set_pos(&mut ui, 5, 7);
    assert_eq!(l.rect(&ui).x, 5);
    assert_eq!(l.rect(&ui).y, 7);
    l.set_text(&mut ui, "hello");
    assert_eq!(l.text(&ui), "hello");
    l.set_hidden(&mut ui, true);
    assert!(l.is_hidden(&ui));

    let c = ObjBuilder::new().size(50, 20).build(&mut ui, s);
    assert_eq!(c.rect(&ui).w, 50);
    let child = LabelBuilder::new("kid").build(&mut ui, c);
    assert_eq!(c.children(&ui), vec![child]);
}

#[test]
fn handle_event_and_value() {
    use std::cell::Cell;
    use std::rc::Rc;
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.screen();
    let sl = qingui::widgets::slider::SliderBuilder::new(0, 100).build(&mut ui, s);
    let hits = Rc::new(Cell::new(0));
    let h = hits.clone();
    sl.on(&mut ui, EventKind::ValueChanged, Box::new(move |_, _, _| h.set(h.get() + 1)));
    sl.set_value(&mut ui, 30);
    assert_eq!(sl.value(&ui), 30);
    assert_eq!(hits.get(), 1);
}
