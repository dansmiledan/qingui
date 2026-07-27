use alloc::vec::Vec;
use crate::arena::{Arena, ObjRef};
use crate::geometry::Rect;
use crate::node::{flag, Node, WidgetKind};

pub struct Ui {
    pub(crate) arena: Arena<Node>,
    screen: ObjRef,
    width: i32,
    height: i32,
    #[allow(dead_code)]
    buf_rows: u32,
}

impl Ui {
    pub fn new(width: i32, height: i32, buf_rows: u32) -> Ui {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, width, height), WidgetKind::Obj));
        Ui { arena, screen, width, height, buf_rows }
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
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.x = x;
            n.rect.y = y;
        }
    }

    pub fn set_size(&mut self, obj: ObjRef, w: i32, h: i32) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.w = w;
            n.rect.h = h;
        }
    }

    pub fn set_hidden(&mut self, obj: ObjRef, hidden: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            if hidden {
                n.flags |= flag::HIDDEN;
            } else {
                n.flags &= !flag::HIDDEN;
            }
        }
    }

    pub fn set_style(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style = style;
        }
    }
    pub fn set_style_pressed(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_pressed = style;
        }
    }
    pub fn set_style_focused(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_focused = style;
        }
    }
    pub fn set_state(&mut self, obj: ObjRef, state: u8, on: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            if on {
                n.state |= state;
            } else {
                n.state &= !state;
            }
        }
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
}
