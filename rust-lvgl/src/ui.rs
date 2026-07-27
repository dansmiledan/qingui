use alloc::vec::Vec;
use crate::arena::{Arena, ObjRef};
use crate::geometry::Rect;
use crate::node::{flag, Node, WidgetKind};

pub struct Ui {
    pub(crate) arena: Arena<Node>,
    screen: ObjRef,
    #[allow(dead_code)]
    width: i32,
    #[allow(dead_code)]
    height: i32,
    dirty: crate::dirty::DirtyQueue,
    flush: Option<alloc::boxed::Box<dyn crate::display::Flush>>,
    buf: Vec<crate::geometry::Color>,
}

impl Ui {
    pub fn new(width: i32, height: i32, buf_rows: u32) -> Ui {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, width, height), WidgetKind::Obj));
        let mut dirty = crate::dirty::DirtyQueue::new(Rect::new(0, 0, width, height), 16);
        dirty.add(Rect::new(0, 0, width, height)); // 建屏全屏标脏
        let buf = alloc::vec![crate::geometry::Color::BLACK; (width * buf_rows as i32).max(0) as usize];
        Ui { arena, screen, width, height, dirty, flush: None, buf }
    }

    pub fn screen(&self) -> ObjRef {
        self.screen
    }

    pub fn is_valid(&self, obj: ObjRef) -> bool {
        self.arena.contains(obj)
    }

    pub fn create_obj(&mut self, parent: ObjRef) -> ObjRef {
        let r = self.arena.insert(Node::new(Some(parent), Rect::default(), WidgetKind::Obj));
        if let Some(p) = self.arena.get_mut(parent) {
            p.children.push(r);
        }
        r
    }

    pub fn delete(&mut self, obj: ObjRef) {
        if obj == self.screen || !self.is_valid(obj) {
            return;
        }
        self.invalidate_obj(obj);
        // 先级联收集子树
        let mut stack = alloc::vec![obj];
        let mut all = Vec::new();
        while let Some(r) = stack.pop() {
            if let Some(n) = self.arena.get(r) {
                stack.extend_from_slice(&n.children);
                all.push(r);
            }
        }
        // 从父对象摘链
        if let Some(n) = self.arena.get(obj) {
            if let Some(p) = n.parent {
                if let Some(pn) = self.arena.get_mut(p) {
                    pn.children.retain(|&c| c != obj);
                }
            }
        }
        for r in all {
            self.arena.remove(r);
        }
    }

    pub fn children(&self, obj: ObjRef) -> Vec<ObjRef> {
        self.arena.get(obj).map(|n| n.children.clone()).unwrap_or_default()
    }

    pub fn rect(&self, obj: ObjRef) -> Rect {
        self.arena.get(obj).map(|n| n.rect).unwrap_or_default()
    }

    pub fn abs_rect(&self, obj: ObjRef) -> Rect {
        let mut r = self.rect(obj);
        let mut cur = self.arena.get(obj).and_then(|n| n.parent);
        while let Some(p) = cur {
            let n = self.arena.get(p).unwrap();
            r = r.translate(n.rect.x, n.rect.y);
            cur = n.parent;
        }
        r
    }

    pub fn set_pos(&mut self, obj: ObjRef, x: i32, y: i32) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.x = x;
            n.rect.y = y;
        }
        self.invalidate_obj(obj);
    }

    pub fn set_size(&mut self, obj: ObjRef, w: i32, h: i32) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.w = w;
            n.rect.h = h;
        }
        self.invalidate_obj(obj);
    }

    pub fn invalidate_area(&mut self, rect: Rect) {
        self.dirty.add(rect);
    }
    pub fn invalidate_obj(&mut self, obj: ObjRef) {
        if self.is_valid(obj) {
            let r = self.abs_rect(obj);
            self.dirty.add(r);
        }
    }
    pub fn take_dirty(&mut self) -> Vec<Rect> {
        self.dirty.take()
    }
    pub fn dirty_is_empty(&self) -> bool {
        self.dirty.is_empty()
    }

    pub fn set_hidden(&mut self, obj: ObjRef, hidden: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            if hidden {
                n.flags |= flag::HIDDEN;
            } else {
                n.flags &= !flag::HIDDEN;
            }
        }
        self.invalidate_obj(obj);
    }

    pub fn set_style(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style = style;
        }
        self.invalidate_obj(obj);
    }
    pub fn set_style_pressed(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_pressed = style;
        }
        self.invalidate_obj(obj);
    }
    pub fn set_style_focused(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_focused = style;
        }
        self.invalidate_obj(obj);
    }
    pub fn set_state(&mut self, obj: ObjRef, state: u8, on: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            if on {
                n.state |= state;
            } else {
                n.state &= !state;
            }
        }
        self.invalidate_obj(obj);
    }
    pub fn state(&self, obj: ObjRef) -> u8 {
        self.arena.get(obj).map(|n| n.state).unwrap_or(0)
    }
    pub fn resolved_style(&self, obj: ObjRef) -> crate::style::ResolvedStyle {
        let Some(n) = self.arena.get(obj) else {
            return crate::style::ResolvedStyle::default();
        };
        use crate::node::state;
        // pressed 优先于 focused
        let overlay = if n.state & state::PRESSED != 0 {
            Some(&n.style_pressed)
        } else if n.state & state::FOCUSED != 0 {
            Some(&n.style_focused)
        } else {
            None
        };
        crate::style::resolve(&n.style, overlay)
    }

    pub fn set_flush(&mut self, f: alloc::boxed::Box<dyn crate::display::Flush>) {
        self.flush = Some(f);
    }

    pub fn render(&mut self) {
        let dirty = self.dirty.take();
        for area in dirty {
            self.render_area(area);
        }
    }

    fn render_area(&mut self, area: Rect) {
        // chunk 宽度 = 脏矩形自身宽度（对齐 LVGL：缓冲行数按区域宽度折算）
        let max_rows = (self.buf.len() as i32 / area.w.max(1)).max(1);
        let mut y = area.y;
        while y < area.bottom() {
            let h = max_rows.min(area.bottom() - y);
            let chunk = Rect::new(area.x, y, area.w, h);
            self.render_chunk(chunk);
            y += h;
        }
    }

    fn render_chunk(&mut self, chunk: Rect) {
        let len = (chunk.w * chunk.h) as usize;
        // 1) 背景：screen 的 resolved bg
        let screen_style = self.resolved_style(self.screen);
        {
            let mut d = crate::draw::DrawBuf {
                pixels: &mut self.buf[..len],
                area: chunk,
                stride: chunk.w,
            };
            d.clear(screen_style.bg_color);
        }
        // 2) 先序遍历对象树绘制（screen 本身不画，背景已在上面处理）
        let roots = self.children(self.screen);
        for r in roots {
            self.draw_node(r, chunk, len);
        }
        // 3) flush
        if let Some(f) = self.flush.as_mut() {
            f.flush(chunk, &self.buf[..len]);
        }
    }

    fn draw_node(&mut self, obj: ObjRef, clip: Rect, len: usize) {
        let Some((abs, flags, resolved)) = self.node_draw_info(obj) else {
            return;
        };
        if flags & crate::node::flag::HIDDEN != 0 {
            return;
        }
        if abs.intersect(&clip).is_some() {
            let mut d = crate::draw::DrawBuf {
                pixels: &mut self.buf[..len],
                area: clip,
                stride: clip.w,
            };
            if resolved.bg_opa > 0 {
                d.fill_rounded(abs, resolved.radius, resolved.bg_color, resolved.bg_opa, clip);
            }
            if resolved.border_width > 0 {
                d.draw_border(abs, resolved.border_width, resolved.radius, resolved.border_color, 255, clip);
            }
        }
        for c in self.children(obj) {
            self.draw_node(c, clip, len);
        }
    }

    fn node_draw_info(&self, obj: ObjRef) -> Option<(Rect, u8, crate::style::ResolvedStyle)> {
        self.arena.get(obj).map(|n| {
            (self.abs_rect(obj), n.flags, self.resolved_style(obj))
        })
    }
}
