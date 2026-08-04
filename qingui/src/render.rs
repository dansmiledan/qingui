use alloc::vec::Vec;
use crate::arena::{Arena, ObjRef};
use crate::dirty::DirtyQueue;
use crate::display::Flush;
use crate::geometry::{Color, Rect};
use crate::node::{Flag, Node, State};
use crate::style::ResolvedStyle;
use embedded_graphics::mono_font::MonoFont;

/// 取脏矩形并逐块渲染（PFB）。纯自由函数：Ui 以不相交字段调用。
pub(crate) fn render(
    screen: ObjRef,
    arena: &mut Arena<Node>,
    buf: &mut [Color],
    dirty: &mut DirtyQueue,
    flush: &mut Option<alloc::boxed::Box<dyn Flush>>,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    let areas = dirty.take();
    for area in areas {
        render_area(screen, arena, buf, flush, area, font, time_ms);
    }
}

fn render_area(
    screen: ObjRef,
    arena: &mut Arena<Node>,
    buf: &mut [Color],
    flush: &mut Option<alloc::boxed::Box<dyn Flush>>,
    area: Rect,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    // chunk 宽度 = 脏矩形自身宽度（对齐 LVGL：缓冲行数按区域宽度折算）
    let max_rows = (buf.len() as i32 / area.w.max(1)).max(1);
    let mut y = area.y;
    while y < area.bottom() {
        let h = max_rows.min(area.bottom() - y);
        let chunk = Rect::new(area.x, y, area.w, h);
        render_chunk(screen, arena, buf, flush, chunk, font, time_ms);
        y += h;
    }
}

fn render_chunk(
    screen: ObjRef,
    arena: &mut Arena<Node>,
    buf: &mut [Color],
    flush: &mut Option<alloc::boxed::Box<dyn Flush>>,
    chunk: Rect,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    let len = (chunk.w * chunk.h) as usize;
    // 1) 背景：screen 的 resolved bg
    let screen_style = resolved_style(arena, screen, font);
    {
        let mut d = crate::draw::DrawBuf {
            pixels: &mut buf[..len],
            area: chunk,
            stride: chunk.w,
        };
        d.clear(screen_style.bg_color);
    }
    // 2) 先序遍历对象树绘制（screen 本身不画，背景已在上面处理）
    let roots = children_z_sorted(arena, screen);
    for r in roots {
        draw_node(arena, buf, r, chunk, chunk, len, font, time_ms);
    }
    // 3) flush
    if let Some(f) = flush.as_mut() {
        f.flush(chunk, &buf[..len]);
    }
}

/// frame 为像素缓冲对应的屏幕区域（DrawBuf 坐标系/步长），clip 为绘制裁剪矩形；
/// 二者在顶层相同，CLIP_CHILDREN 父节点会使子树的 clip 收缩而 frame 不变
fn draw_node(
    arena: &mut Arena<Node>,
    buf: &mut [Color],
    obj: ObjRef,
    frame: Rect,
    clip: Rect,
    len: usize,
    font: &'static MonoFont<'static>,
    time_ms: u64,
) {
    let Some((abs, flags, node_opa, resolved)) = node_draw_info(arena, obj, font) else {
        return;
    };
    if flags.contains(Flag::HIDDEN) {
        return;
    }
    if abs.intersect(&clip).is_some() {
        let edited = node_state(arena, obj).contains(State::EDITED);
        // 节点 opa 作为乘数作用于本对象的所有绘制
        let ap = |base: u8| (base as u32 * node_opa as u32 / 255) as u8;
        let mut d = crate::draw::DrawBuf {
            pixels: &mut buf[..len],
            area: frame,
            stride: frame.w,
        };
        let Some(n) = arena.get_mut(obj) else { return };
        if resolved.bg_opa > 0 && ap(resolved.bg_opa) > 0 {
            d.fill_rounded(abs, resolved.radius, resolved.bg_color, ap(resolved.bg_opa), clip);
        }
        let ctx = crate::widgets::WidgetCtx { abs, resolved: &resolved, edited, opa: node_opa, now: time_ms };
        n.kind.draw(&ctx, &mut d, clip);
        // 叠加绘制钩子（原 Canvas 机制的通用化）
        if let Some(hook) = n.draw_hook.as_mut() {
            hook(&mut d, abs, clip, time_ms);
        }
        // 边框最后画（对齐 LVGL：border 在内容之上），避免被控件内容覆盖
        if resolved.border_width > 0 {
            d.draw_border(abs, resolved.border_width, resolved.radius, resolved.border_color, ap(255), clip);
        }
    }
    // 视口裁剪：子树 clip 收缩到本对象矩形内；不相交则整棵子树跳过
    let child_clip = if flags.contains(Flag::CLIP_CHILDREN) {
        match clip.intersect(&abs) {
            Some(c) => c,
            None => return,
        }
    } else {
        clip
    };
    for c in children_z_sorted(arena, obj) {
        draw_node(arena, buf, c, frame, child_clip, len, font, time_ms);
    }
}

/// 子对象按 z_index 稳定排序（小者先画，大者在上）
fn children_z_sorted(arena: &Arena<Node>, obj: ObjRef) -> Vec<ObjRef> {
    let mut kids = kids(arena, obj);
    kids.sort_by_key(|&c| arena.get(c).map(|n| n.z_index).unwrap_or(0));
    kids
}

fn node_draw_info(
    arena: &Arena<Node>,
    obj: ObjRef,
    font: &'static MonoFont<'static>,
) -> Option<(Rect, Flag, u8, ResolvedStyle)> {
    arena.get(obj).map(|n| (abs_rect(arena, obj), n.flags, n.opa, resolved_style(arena, obj, font)))
}

/// 绝对坐标：沿父链累加本地坐标与 translate（共享助手，Ui 委托调用）
pub(crate) fn abs_rect(arena: &Arena<Node>, obj: ObjRef) -> Rect {
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

/// 样式解析（pressed > focused > selected 互斥取一；共享助手，Ui 委托调用）
pub(crate) fn resolved_style(arena: &Arena<Node>, obj: ObjRef, font: &'static MonoFont<'static>) -> ResolvedStyle {
    let Some(n) = arena.get(obj) else {
        return ResolvedStyle::default();
    };
    let overlay = if n.state.contains(State::PRESSED) {
        Some(&n.style_pressed)
    } else if n.state.contains(State::FOCUSED) {
        Some(&n.style_focused)
    } else if n.state.contains(State::SELECTED) {
        Some(&n.style_selected)
    } else {
        None
    };
    crate::style::resolve(&n.style, overlay, font)
}

fn kids(arena: &Arena<Node>, obj: ObjRef) -> Vec<ObjRef> {
    arena.get(obj).map(|n| n.children.clone()).unwrap_or_default()
}

fn node_state(arena: &Arena<Node>, obj: ObjRef) -> State {
    arena.get(obj).map(|n| n.state).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::obj::ObjState;
    use crate::widgets::WidgetKind;
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use core::cell::RefCell;

    const FONT: &'static MonoFont<'static> = crate::font::DEFAULT_FONT;

    #[derive(Default)]
    struct Rec { chunks: Vec<(Rect, Vec<Color>)> }
    struct FakeFlush(Rc<RefCell<Rec>>);
    impl Flush for FakeFlush {
        fn flush(&mut self, area: Rect, pixels: &[Color]) {
            self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
        }
    }
    fn px(rec: &Rc<RefCell<Rec>>, x: i32, y: i32) -> Color {
        for (area, buf) in rec.borrow().chunks.iter().rev() {
            if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
                return buf[((y - area.y) * area.w + (x - area.x)) as usize];
            }
        }
        panic!("pixel not flushed");
    }
    fn style(bg: Color) -> crate::style::Style {
        let mut s = crate::style::Style::default();
        s.bg_color = Some(bg);
        s.bg_opa = Some(255);
        s
    }
    /// 建屏 + 挂一个覆盖全屏的纯色子节点，渲染并断言像素
    fn render_fixture(scr_style: crate::style::Style, child_style: crate::style::Style, w: i32, h: i32) -> (Arena<Node>, Rc<RefCell<Rec>>) {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, w, h), WidgetKind::Obj(ObjState)));
        arena.get_mut(screen).unwrap().style = scr_style;
        let child = arena.insert(Node::new(Some(screen), Rect::new(0, 0, w, h), WidgetKind::Obj(ObjState)));
        arena.get_mut(screen).unwrap().children.push(child);
        arena.get_mut(child).unwrap().style = child_style;
        let mut dirty = DirtyQueue::new(Rect::new(0, 0, w, h), 16);
        dirty.add(Rect::new(0, 0, w, h));
        let mut buf = alloc::vec![Color::BLACK; (w * h) as usize];
        let rec = Rc::new(RefCell::new(Rec::default()));
        render(screen, &mut arena, &mut buf, &mut dirty, &mut Some(Box::new(FakeFlush(rec.clone()))), FONT, 0);
        (arena, rec)
    }

    #[test]
    fn renders_child_over_screen_bg() {
        let (_, rec) = render_fixture(style(Color::BLACK), style(Color::WHITE), 40, 30);
        assert_eq!(px(&rec, 5, 5), Color::WHITE);   // 子对象盖住屏幕背景
        assert_eq!(px(&rec, 35, 25), Color::WHITE);
    }

    #[test]
    fn hidden_subtree_is_skipped() {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, 40, 30), WidgetKind::Obj(ObjState)));
        arena.get_mut(screen).unwrap().style = style(Color::BLACK);
        let child = arena.insert(Node::new(Some(screen), Rect::new(0, 0, 40, 30), WidgetKind::Obj(ObjState)));
        arena.get_mut(screen).unwrap().children.push(child);
        arena.get_mut(child).unwrap().style = style(Color::WHITE);
        arena.get_mut(child).unwrap().flags |= crate::node::Flag::HIDDEN;
        let mut dirty = DirtyQueue::new(Rect::new(0, 0, 40, 30), 16);
        dirty.add(Rect::new(0, 0, 40, 30));
        let mut buf = alloc::vec![Color::BLACK; 40 * 30];
        let rec = Rc::new(RefCell::new(Rec::default()));
        render(screen, &mut arena, &mut buf, &mut dirty, &mut Some(Box::new(FakeFlush(rec.clone()))), FONT, 0);
        assert_eq!(px(&rec, 5, 5), Color::BLACK); // HIDDEN 子对象不画 → 屏幕背景
    }

    #[test]
    fn clip_children_limits_child() {
        let (_arena, rec) = {
            let mut arena = Arena::new();
            let screen = arena.insert(Node::new(None, Rect::new(0, 0, 40, 30), WidgetKind::Obj(ObjState)));
            arena.get_mut(screen).unwrap().style = style(Color::BLACK);
            let vp = arena.insert(Node::new(Some(screen), Rect::new(0, 0, 20, 30), WidgetKind::Obj(ObjState)));
            arena.get_mut(vp).unwrap().flags |= crate::node::Flag::CLIP_CHILDREN;
            arena.get_mut(screen).unwrap().children.push(vp);
            let child = arena.insert(Node::new(Some(vp), Rect::new(0, 0, 40, 30), WidgetKind::Obj(ObjState)));
            arena.get_mut(child).unwrap().style = style(Color::WHITE);
            arena.get_mut(vp).unwrap().children.push(child);
            let mut dirty = DirtyQueue::new(Rect::new(0, 0, 40, 30), 16);
            dirty.add(Rect::new(0, 0, 40, 30));
            let mut buf = alloc::vec![Color::BLACK; 40 * 30];
            let rec = Rc::new(RefCell::new(Rec::default()));
            render(screen, &mut arena, &mut buf, &mut dirty, &mut Some(Box::new(FakeFlush(rec.clone()))), FONT, 0);
            (arena, rec)
        };
        assert_eq!(px(&rec, 5, 5), Color::WHITE);   // 视口内可见
        assert_eq!(px(&rec, 25, 5), Color::BLACK);  // 视口外被裁 → 屏幕背景
    }

    #[test]
    fn abs_rect_accumulates_parent_and_translate() {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, 100, 100), WidgetKind::Obj(ObjState)));
        let p = arena.insert(Node::new(Some(screen), Rect::new(10, 20, 50, 50), WidgetKind::Obj(ObjState)));
        arena.get_mut(screen).unwrap().children.push(p);
        arena.get_mut(p).unwrap().translate = crate::geometry::Point { x: 5, y: 0 };
        let c = arena.insert(Node::new(Some(p), Rect::new(3, 4, 10, 10), WidgetKind::Obj(ObjState)));
        arena.get_mut(p).unwrap().children.push(c);
        assert_eq!(abs_rect(&arena, c), Rect::new(18, 24, 10, 10)); // 10+5+3, 20+0+4
    }

    #[test]
    fn resolved_style_state_precedence() {
        // 三态互斥取一：pressed > focused > selected
        let mut arena = Arena::new();
        let r = arena.insert(Node::new(None, Rect::new(0, 0, 10, 10), WidgetKind::Obj(ObjState)));
        let n = arena.get_mut(r).unwrap();
        n.state |= crate::node::State::SELECTED | crate::node::State::FOCUSED | crate::node::State::PRESSED;
        n.style_selected.bg_color = Some(Color::rgb(1, 0, 0));
        n.style_focused.bg_color = Some(Color::rgb(2, 0, 0));
        n.style_pressed.bg_color = Some(Color::rgb(3, 0, 0));
        assert_eq!(resolved_style(&arena, r, FONT).bg_color, Color::rgb(3, 0, 0)); // PRESSED 优先

        arena.get_mut(r).unwrap().state = crate::node::State::FOCUSED | crate::node::State::SELECTED;
        assert_eq!(resolved_style(&arena, r, FONT).bg_color, Color::rgb(2, 0, 0)); // 无 PRESSED → FOCUSED

        arena.get_mut(r).unwrap().state = crate::node::State::SELECTED;
        assert_eq!(resolved_style(&arena, r, FONT).bg_color, Color::rgb(1, 0, 0)); // 只剩 SELECTED
    }
}
