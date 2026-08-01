use qingui::widgets::obj::ObjBuilder;
use qingui::{Rect, Ui};

#[test]
fn create_and_hierarchy() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjBuilder::new().build(&mut ui, screen);
    let b = ObjBuilder::new().build(&mut ui, a);
    assert_eq!(ui.children(screen), vec![a]);
    assert_eq!(ui.children(a), vec![b]);
}

#[test]
fn delete_invalidates_handle_and_reparents_nothing() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjBuilder::new().build(&mut ui, screen);
    let b = ObjBuilder::new().build(&mut ui, a);
    ui.set_pos(b, 10, 10);
    ui.delete(a);
    assert!(!ui.is_valid(a));
    assert!(!ui.is_valid(b)); // 删除父对象级联删除子树
    // 悬垂句柄操作安全 no-op
    ui.set_pos(a, 5, 5);
    assert_eq!(ui.children(screen).len(), 0);
}

#[test]
fn generation_recycled_slot_is_safe() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjBuilder::new().build(&mut ui, screen);
    ui.delete(a);
    let b = ObjBuilder::new().build(&mut ui, screen); // 复用 slot
    assert_eq!(a.index, b.index);
    assert_ne!(a, b);
    assert!(!ui.is_valid(a));
    assert!(ui.is_valid(b));
}

#[test]
fn abs_rect_accumulates_parents() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjBuilder::new().build(&mut ui, screen);
    ui.set_pos(a, 10, 20);
    ui.set_size(a, 100, 80);
    let b = ObjBuilder::new().build(&mut ui, a);
    ui.set_pos(b, 5, 5);
    ui.set_size(b, 30, 30);
    assert_eq!(ui.rect(b), Rect::new(5, 5, 30, 30));
    assert_eq!(ui.abs_rect(b), Rect::new(15, 25, 30, 30));
}
