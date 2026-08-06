use qingui::widgets::obj::ObjCfg;
use qingui::{Rect, Ui};

#[test]
fn create_and_hierarchy() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjCfg::new().build(&mut ui, screen);
    let b = ObjCfg::new().build(&mut ui, a);
    assert_eq!(ui.children(screen), vec![a]);
    assert_eq!(ui.children(a), vec![b]);
}

#[test]
fn delete_invalidates_handle_and_reparents_nothing() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjCfg::new().build(&mut ui, screen);
    let b = ObjCfg::new().build(&mut ui, a);
    ui.set_pos(b, 10, 10);
    ui.delete(a);
    assert!(!ui.is_valid(a));
    assert!(!ui.is_valid(b)); // deleting the parent cascades to the subtree
    // Dangling-handle operations are safe no-ops
    ui.set_pos(a, 5, 5);
    assert_eq!(ui.children(screen).len(), 0);
}

#[test]
fn generation_recycled_slot_is_safe() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjCfg::new().build(&mut ui, screen);
    ui.delete(a);
    let b = ObjCfg::new().build(&mut ui, screen); // reuses the slot
    assert_eq!(a.index, b.index);
    assert_ne!(a, b);
    assert!(!ui.is_valid(a));
    assert!(ui.is_valid(b));
}

#[test]
fn abs_rect_accumulates_parents() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjCfg::new().build(&mut ui, screen);
    ui.set_pos(a, 10, 20);
    ui.set_size(a, 100, 80);
    let b = ObjCfg::new().build(&mut ui, a);
    ui.set_pos(b, 5, 5);
    ui.set_size(b, 30, 30);
    assert_eq!(ui.rect(b), Rect::new(5, 5, 30, 30));
    assert_eq!(ui.abs_rect(b), Rect::new(15, 25, 30, 30));
}
