use crate::arena::{Arena, ObjRef};
use crate::dirty::DirtyQueue;
use crate::display::Flush;
use crate::geometry::Rect;
use crate::node::{Flag, Node, State};
use crate::style::ResolvedStyle;
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::RgbColor;

/// Takes the dirty rects and renders each in chunks (PFB). A plain free function: `Ui` calls
/// it with disjoint fields.
pub(crate) fn render<C: RgbColor>(
    screen: ObjRef,
    arena: &mut Arena<Node<C>>,
    buf: &mut [C],
    dirty: &mut DirtyQueue,
    flush: &mut Option<alloc::boxed::Box<dyn Flush<C>>>,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    let areas = dirty.take();
    for area in areas {
        render_area(screen, arena, buf, flush, area, font, time_ms);
    }
}

fn render_area<C: RgbColor>(
    screen: ObjRef,
    arena: &mut Arena<Node<C>>,
    buf: &mut [C],
    flush: &mut Option<alloc::boxed::Box<dyn Flush<C>>>,
    area: Rect,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    // Chunk width = the dirty rect's own width (mirrors LVGL: buffer rows are derived from the area width)
    let max_rows = (buf.len() as i32 / area.w.max(1)).max(1);
    let mut y = area.y;
    while y < area.bottom() {
        let h = max_rows.min(area.bottom() - y);
        let chunk = Rect::new(area.x, y, area.w, h);
        render_chunk(screen, arena, buf, flush, chunk, font, time_ms);
        y += h;
    }
}

fn render_chunk<C: RgbColor>(
    screen: ObjRef,
    arena: &mut Arena<Node<C>>,
    buf: &mut [C],
    flush: &mut Option<alloc::boxed::Box<dyn Flush<C>>>,
    chunk: Rect,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    let len = (chunk.w * chunk.h) as usize;
    // 1) Background: the screen's resolved bg
    let screen_style = resolved_style(arena, screen, font);
    {
        let mut d = crate::canvas::Canvas {
            pixels: &mut buf[..len],
            area: chunk,
            stride: chunk.w,
        };
        d.clear(screen_style.bg_color.unwrap_or(C::BLACK));
    }
    // 2) Draw the object tree in pre-order (the screen itself is not drawn; its background was handled above)
    let nkids = arena.get(screen).map(|n| n.children.len()).unwrap_or(0);
    for i in 0..nkids {
        let Some(c) = arena.get(screen).and_then(|n| n.children.get(i).copied()) else { break };
        draw_node(arena, buf, c, chunk, chunk, len, font, time_ms);
    }
    // 3) Flush
    if let Some(f) = flush.as_mut() {
        f.flush(chunk, &buf[..len]);
    }
}

/// `frame` is the screen region the pixel buffer maps to (the Canvas coordinate system/stride),
/// and `clip` is the draw clip rect;
/// they are the same at the top level; a CLIP_CHILDREN parent shrinks its subtree's clip while
/// the frame stays unchanged.
fn draw_node<C: RgbColor>(
    arena: &mut Arena<Node<C>>,
    buf: &mut [C],
    obj: ObjRef,
    frame: Rect,
    clip: Rect,
    len: usize,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    let Some((abs, flags, resolved)) = node_draw_info(arena, obj, font) else {
        return;
    };
    if flags.contains(Flag::HIDDEN) {
        return;
    }
    if abs.intersect(&clip).is_some() {
        let edited = node_state(arena, obj).contains(State::EDITED);
        let mut d = crate::canvas::Canvas {
            pixels: &mut buf[..len],
            area: frame,
            stride: frame.w,
        };
        let Some(n) = arena.get_mut(obj) else { return };
        if let Some(bg) = resolved.bg_color {
            d.fill_rounded(abs, resolved.radius, bg, clip);
        }
        let ctx = crate::widgets::WidgetCtx { abs, resolved: &resolved, edited, now: time_ms };
        n.kind.draw(&ctx, &mut d, clip);
        // Overlay draw hook (generalized from the old Canvas mechanism)
        if let Some(hook) = n.draw_hook.as_mut() {
            hook(&mut d, abs, clip, time_ms);
        }
        // The border is drawn last (mirrors LVGL: border above content) so widget content
        // does not cover it
        if resolved.border_width > 0 {
            d.draw_border(abs, resolved.border_width, resolved.radius, resolved.border_color, clip);
        }
    }
    // Viewport clipping: the subtree's clip shrinks to this object's rect; if disjoint, the
    // whole subtree is skipped
    let child_clip = if flags.contains(Flag::CLIP_CHILDREN) {
        match clip.intersect(&abs) {
            Some(c) => c,
            None => return,
        }
    } else {
        clip
    };
    let nkids = arena.get(obj).map(|n| n.children.len()).unwrap_or(0);
    for i in 0..nkids {
        let Some(c) = arena.get(obj).and_then(|n| n.children.get(i).copied()) else { break };
        draw_node(arena, buf, c, frame, child_clip, len, font, time_ms);
    }
}

fn node_draw_info<C: RgbColor>(
    arena: &Arena<Node<C>>,
    obj: ObjRef,
    font: &'static MonoFont<'static>,
) -> Option<(Rect, Flag, ResolvedStyle<C>)> {
    arena.get(obj).map(|n| {
        let resolved = resolved_style(arena, obj, font);
        (abs_rect(arena, obj), n.flags, resolved)
    })
}

/// Absolute coordinates: accumulates local coordinates and translates up the parent chain
/// (shared helper, delegated to by `Ui`).
pub(crate) fn abs_rect<C>(arena: &Arena<Node<C>>, obj: ObjRef) -> Rect {
    let mut r = arena.get(obj).map(|n| n.rect).unwrap_or_default();
    let mut cur = arena.get(obj).and_then(|n| n.parent);
    while let Some(p) = cur {
        let n = arena.get(p).unwrap();
        r = r.translate(n.rect.x + n.translate.x, n.rect.y + n.translate.y);
        cur = n.parent;
    }
    if let Some(n) = arena.get(obj) {
        r = r.translate(n.translate.x, n.translate.y);
    }
    r
}

/// Style resolution (edited > focused > selected, mutually exclusive; shared helper,
/// delegated to by `Ui`). While edited, a custom `style_edited` overlay wins; without
/// one the focus overlay applies. The edited look itself is a style concern: widgets
/// set their `style_edited` at build time (see `style::theme_edited`).
pub(crate) fn resolved_style<C: RgbColor>(arena: &Arena<Node<C>>, obj: ObjRef, font: &'static MonoFont<'static>) -> ResolvedStyle<C> {
    let Some(n) = arena.get(obj) else {
        return ResolvedStyle::default();
    };
    let edited = n.state.contains(State::EDITED);
    let overlay = if edited {
        n.style_edited.as_deref().or(n.style_focused.as_deref())
    } else if n.state.contains(State::FOCUSED) {
        n.style_focused.as_deref()
    } else if n.state.contains(State::SELECTED) {
        n.style_selected.as_deref()
    } else {
        None
    };
    crate::style::resolve(&n.style, overlay, font)
}

fn node_state<C>(arena: &Arena<Node<C>>, obj: ObjRef) -> State {
    arena.get(obj).map(|n| n.state).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::Rgb888;
    use crate::widgets::obj::Manual;
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    const FONT: &'static MonoFont<'static> = crate::font::DEFAULT_FONT;

    #[derive(Default)]
    struct Rec { chunks: Vec<(Rect, Vec<Rgb888>)> }
    struct FakeFlush(Rc<RefCell<Rec>>);
    impl Flush for FakeFlush {
        fn flush(&mut self, area: Rect, pixels: &[Rgb888]) {
            self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
        }
    }
    fn px(rec: &Rc<RefCell<Rec>>, x: i32, y: i32) -> Rgb888 {
        for (area, buf) in rec.borrow().chunks.iter().rev() {
            if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
                return buf[((y - area.y) * area.w + (x - area.x)) as usize];
            }
        }
        panic!("pixel not flushed");
    }
    fn style(bg: Rgb888) -> crate::style::Style<Rgb888> {
        let mut s = crate::style::Style::default();
        s.bg_color = Some(bg);
        s
    }
    /// Builds a screen + a full-screen solid-color child, renders, and asserts pixels
    fn render_fixture(scr_style: crate::style::Style, child_style: crate::style::Style, w: i32, h: i32) -> (Arena<Node>, Rc<RefCell<Rec>>) {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, w, h), alloc::boxed::Box::new(Manual)));
        arena.get_mut(screen).unwrap().style = scr_style;
        let child = arena.insert(Node::new(Some(screen), Rect::new(0, 0, w, h), alloc::boxed::Box::new(Manual)));
        arena.get_mut(screen).unwrap().children.push(child);
        arena.get_mut(child).unwrap().style = child_style;
        let mut dirty = DirtyQueue::new(Rect::new(0, 0, w, h), 16);
        dirty.add(Rect::new(0, 0, w, h));
        let mut buf = alloc::vec![Rgb888::BLACK; (w * h) as usize];
        let rec = Rc::new(RefCell::new(Rec::default()));
        render(screen, &mut arena, &mut buf, &mut dirty, &mut Some(Box::new(FakeFlush(rec.clone()))), FONT, 0);
        (arena, rec)
    }

    #[test]
    fn renders_child_over_screen_bg() {
        let (_, rec) = render_fixture(style(Rgb888::BLACK), style(Rgb888::WHITE), 40, 30);
        assert_eq!(px(&rec, 5, 5), Rgb888::WHITE);   // child covers the screen background
        assert_eq!(px(&rec, 35, 25), Rgb888::WHITE);
    }

    #[test]
    fn hidden_subtree_is_skipped() {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, 40, 30), alloc::boxed::Box::new(Manual)));
        arena.get_mut(screen).unwrap().style = style(Rgb888::BLACK);
        let child = arena.insert(Node::new(Some(screen), Rect::new(0, 0, 40, 30), alloc::boxed::Box::new(Manual)));
        arena.get_mut(screen).unwrap().children.push(child);
        arena.get_mut(child).unwrap().style = style(Rgb888::WHITE);
        arena.get_mut(child).unwrap().flags |= crate::node::Flag::HIDDEN;
        let mut dirty = DirtyQueue::new(Rect::new(0, 0, 40, 30), 16);
        dirty.add(Rect::new(0, 0, 40, 30));
        let mut buf = alloc::vec![Rgb888::BLACK; 40 * 30];
        let rec = Rc::new(RefCell::new(Rec::default()));
        render(screen, &mut arena, &mut buf, &mut dirty, &mut Some(Box::new(FakeFlush(rec.clone()))), FONT, 0);
        assert_eq!(px(&rec, 5, 5), Rgb888::BLACK); // HIDDEN child is not drawn → screen background
    }

    #[test]
    fn clip_children_limits_child() {
        let (_arena, rec) = {
            let mut arena = Arena::new();
            let screen = arena.insert(Node::new(None, Rect::new(0, 0, 40, 30), alloc::boxed::Box::new(Manual)));
            arena.get_mut(screen).unwrap().style = style(Rgb888::BLACK);
            let vp = arena.insert(Node::new(Some(screen), Rect::new(0, 0, 20, 30), alloc::boxed::Box::new(Manual)));
            arena.get_mut(vp).unwrap().flags |= crate::node::Flag::CLIP_CHILDREN;
            arena.get_mut(screen).unwrap().children.push(vp);
            let child = arena.insert(Node::new(Some(vp), Rect::new(0, 0, 40, 30), alloc::boxed::Box::new(Manual)));
            arena.get_mut(child).unwrap().style = style(Rgb888::WHITE);
            arena.get_mut(vp).unwrap().children.push(child);
            let mut dirty = DirtyQueue::new(Rect::new(0, 0, 40, 30), 16);
            dirty.add(Rect::new(0, 0, 40, 30));
            let mut buf = alloc::vec![Rgb888::BLACK; 40 * 30];
            let rec = Rc::new(RefCell::new(Rec::default()));
            render(screen, &mut arena, &mut buf, &mut dirty, &mut Some(Box::new(FakeFlush(rec.clone()))), FONT, 0);
            (arena, rec)
        };
        assert_eq!(px(&rec, 5, 5), Rgb888::WHITE);   // visible inside the viewport
        assert_eq!(px(&rec, 25, 5), Rgb888::BLACK);  // clipped outside the viewport → screen background
    }

    #[test]
    fn abs_rect_accumulates_parent_and_translate() {
        let mut arena: Arena<Node> = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, 100, 100), alloc::boxed::Box::new(Manual)));
        let p = arena.insert(Node::new(Some(screen), Rect::new(10, 20, 50, 50), alloc::boxed::Box::new(Manual)));
        arena.get_mut(screen).unwrap().children.push(p);
        arena.get_mut(p).unwrap().translate = crate::geometry::Point { x: 5, y: 0 };
        let c = arena.insert(Node::new(Some(p), Rect::new(3, 4, 10, 10), alloc::boxed::Box::new(Manual)));
        arena.get_mut(p).unwrap().children.push(c);
        assert_eq!(abs_rect(&arena, c), Rect::new(18, 24, 10, 10)); // 10+5+3, 20+0+4
    }

    #[test]
    fn resolved_style_state_precedence() {
        // The two states are mutually exclusive: focused > selected
        let mut arena: Arena<Node> = Arena::new();
        let r = arena.insert(Node::new(None, Rect::new(0, 0, 10, 10), alloc::boxed::Box::new(Manual)));
        let n = arena.get_mut(r).unwrap();
        n.state |= crate::node::State::SELECTED | crate::node::State::FOCUSED;
        n.style_selected = Some(Box::new(style(Rgb888::new(1, 0, 0))));
        n.style_focused = Some(Box::new(style(Rgb888::new(2, 0, 0))));
        assert_eq!(resolved_style(&arena, r, FONT).bg_color, Some(Rgb888::new(2, 0, 0))); // FOCUSED takes priority

        arena.get_mut(r).unwrap().state = crate::node::State::SELECTED;
        assert_eq!(resolved_style(&arena, r, FONT).bg_color, Some(Rgb888::new(1, 0, 0))); // only SELECTED left
    }

    #[test]
    fn edited_falls_back_to_focus_overlay() {
        // Without a custom style_edited the focus overlay applies while edited;
        // the edited look itself is a style concern, not a render rule
        let mut arena: Arena<Node> = Arena::new();
        let r = arena.insert(Node::new(None, Rect::new(0, 0, 10, 10), alloc::boxed::Box::new(Manual)));
        let mut f = crate::style::Style::default();
        f.border_color = Some(Rgb888::WHITE);
        f.border_width = Some(2);
        arena.get_mut(r).unwrap().style_focused = Some(Box::new(f));
        arena.get_mut(r).unwrap().state = crate::node::State::FOCUSED | crate::node::State::EDITED;
        let rs = resolved_style(&arena, r, FONT);
        assert_eq!(rs.border_color, Rgb888::WHITE); // falls back to the focus overlay
        assert_eq!(rs.border_width, 2);
    }

    #[test]
    fn style_edited_overlay_wins_while_edited() {
        // A custom style_edited overlay takes precedence over the focus overlay and
        // disables the default amber border tint
        let mut arena: Arena<Node> = Arena::new();
        let r = arena.insert(Node::new(None, Rect::new(0, 0, 10, 10), alloc::boxed::Box::new(Manual)));
        let mut f = crate::style::Style::default();
        f.border_color = Some(Rgb888::WHITE);
        f.border_width = Some(2);
        let mut e = crate::style::Style::default();
        e.border_color = Some(Rgb888::new(1, 2, 3));
        e.border_width = Some(4);
        e.bg_color = Some(Rgb888::new(4, 5, 6));
        arena.get_mut(r).unwrap().style_focused = Some(Box::new(f));
        arena.get_mut(r).unwrap().style_edited = Some(Box::new(e));
        arena.get_mut(r).unwrap().state = crate::node::State::FOCUSED | crate::node::State::EDITED;
        let rs = resolved_style(&arena, r, FONT);
        assert_eq!(rs.border_color, Rgb888::new(1, 2, 3)); // edited overlay wins, no amber tint
        assert_eq!(rs.border_width, 4);
        assert_eq!(rs.bg_color, Some(Rgb888::new(4, 5, 6)));
    }
}
