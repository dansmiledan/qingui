use qingui::widgets::obj::ObjCfg;
use qingui::Ui;

/// Example third-party layout constraint, as an external layout algorithm
/// would attach to a child via `set_item_custom`.
#[derive(Debug, PartialEq)]
struct DockProps {
    edge: u8,
    weight: i32,
}

#[test]
fn custom_constraints_roundtrip_and_mutate() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let child = ObjCfg::new().build(&mut ui, scr);

    ui.set_item_custom(child, Box::new(DockProps { edge: 1, weight: 10 }));

    // Read back by type.
    let p = ui.item_custom::<DockProps>(child).unwrap();
    assert_eq!(p, &DockProps { edge: 1, weight: 10 });
    // Wrong type returns None.
    assert!(ui.item_custom::<u32>(child).is_none());

    // Mutate in place.
    let r = ui.update_item_custom::<DockProps, _>(child, |p| {
        p.weight += 5;
        p.weight
    });
    assert_eq!(r, Some(15));
    assert_eq!(ui.item_custom::<DockProps>(child).unwrap().weight, 15);
}

#[test]
fn non_custom_node_returns_none() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let child = ObjCfg::new().build(&mut ui, scr);

    assert!(ui.item_custom::<DockProps>(child).is_none());
    assert!(ui.update_item_custom::<DockProps, _>(child, |p| p.weight = 1).is_none());

    // A grid-placed child is not custom either.
    ui.set_grid_cell(child, (0, 1), (0, 1));
    assert!(ui.item_custom::<DockProps>(child).is_none());
}

#[test]
fn custom_replaces_grid_placement() {
    let mut ui = Ui::new(320, 240, 240);
    let scr = ui.screen();
    let child = ObjCfg::new().build(&mut ui, scr);

    ui.set_grid_cell(child, (2, 1), (3, 2));
    assert_eq!(ui.grid_cell(child), ((2, 1), (3, 2)));

    // Mutual exclusivity: attaching custom constraints replaces `specific`,
    // so the grid placement falls back to the default.
    ui.set_item_custom(child, Box::new(DockProps { edge: 0, weight: 1 }));
    assert_eq!(ui.grid_cell(child), ((0, 1), (0, 1)));
}
