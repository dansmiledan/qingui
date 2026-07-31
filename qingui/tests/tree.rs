use qingui::widgets::obj::ObjBuilder;
use qingui::{Rect, Ui};

#[test]
fn create_and_hierarchy() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjBuilder::new().build(&mut ui, screen);
    let b = ObjBuilder::new().build(&mut ui, a);
    assert_eq!(screen.children(&ui), vec![a]);
    assert_eq!(a.children(&ui), vec![b]);
}

#[test]
fn delete_invalidates_handle_and_reparents_nothing() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjBuilder::new().build(&mut ui, screen);
    let b = ObjBuilder::new().build(&mut ui, a);
    b.set_pos(&mut ui, 10, 10);
    a.delete(&mut ui);
    assert!(!ui.is_valid(a));
    assert!(!ui.is_valid(b)); // 删除父对象级联删除子树
    // 悬垂句柄操作安全 no-op
    a.set_pos(&mut ui, 5, 5);
    assert_eq!(screen.children(&ui).len(), 0);
}

#[test]
fn generation_recycled_slot_is_safe() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ObjBuilder::new().build(&mut ui, screen);
    a.delete(&mut ui);
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
    a.set_pos(&mut ui, 10, 20);
    a.set_size(&mut ui, 100, 80);
    let b = ObjBuilder::new().build(&mut ui, a);
    b.set_pos(&mut ui, 5, 5);
    b.set_size(&mut ui, 30, 30);
    assert_eq!(b.rect(&ui), Rect::new(5, 5, 30, 30));
    assert_eq!(b.abs_rect(&ui), Rect::new(15, 25, 30, 30));
}
