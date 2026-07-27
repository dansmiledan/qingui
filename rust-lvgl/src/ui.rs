use alloc::vec::Vec;
use crate::arena::{Arena, ObjRef};
use crate::geometry::{Color, Rect};
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
            let kind_snap = self.kind_snapshot(obj);
            let edited = self.state(obj) & crate::node::state::EDITED != 0;
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
            match kind_snap {
                WidgetKind::Label { text } => {
                    d.draw_text(crate::geometry::Point { x: abs.x, y: abs.y }, &text, resolved.text_color, clip);
                }
                WidgetKind::Button { text } => {
                    let (tw, th) = crate::font::text_size(&text);
                    let p = crate::geometry::Point {
                        x: abs.x + (abs.w - tw) / 2,
                        y: abs.y + (abs.h - th) / 2,
                    };
                    d.draw_text(p, &text, resolved.text_color, clip);
                }
                WidgetKind::Slider { min, max, value } => {
                    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
                    let iw = (abs.w as f32 * frac) as i32;
                    if iw > 0 {
                        d.fill_rounded(Rect::new(abs.x, abs.y, iw, abs.h), resolved.radius, Color::rgb(80, 140, 255), 255, clip);
                    }
                    let kx = abs.x + iw;
                    let knob = Rect::new(kx - 4, abs.y - 2, 8, abs.h + 4);
                    let kc = if edited { Color::rgb(255, 200, 60) } else { Color::WHITE };
                    d.fill_rounded(knob, 3, kc, 255, clip);
                }
                WidgetKind::Switch { on } => {
                    let tc = if on { Color::rgb(60, 180, 90) } else { Color::rgb(90, 90, 90) };
                    d.fill_rounded(abs, abs.h / 2, tc, 255, clip);
                    let k = abs.h - 4;
                    let kx = if on { abs.right() - k - 2 } else { abs.x + 2 };
                    d.fill_rounded(Rect::new(kx, abs.y + 2, k, k), k / 2, Color::WHITE, 255, clip);
                }
                WidgetKind::Bar { min, max, value } => {
                    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
                    let iw = (abs.w as f32 * frac) as i32;
                    if iw > 0 {
                        d.fill_rounded(Rect::new(abs.x, abs.y, iw, abs.h), resolved.radius, Color::rgb(80, 140, 255), 255, clip);
                    }
                }
                WidgetKind::List { items, selected, scroll } => {
                    let row_h = 16;
                    let lclip = abs.intersect(&clip).unwrap_or(clip);
                    for (i, item) in items.iter().enumerate() {
                        let ry = abs.y + i as i32 * row_h - scroll;
                        let row = Rect::new(abs.x, ry, abs.w, row_h);
                        if !row.intersects(&lclip) {
                            continue;
                        }
                        if i == selected {
                            d.fill_rect(row, Color::rgb(50, 70, 120), 255, lclip);
                        }
                        d.draw_text(
                            crate::geometry::Point { x: abs.x + 4, y: ry + 4 },
                            item,
                            resolved.text_color,
                            lclip,
                        );
                    }
                }
                WidgetKind::Obj => {}
            }
        }
        for c in self.children(obj) {
            self.draw_node(c, clip, len);
        }
    }

    fn kind_snapshot(&self, obj: ObjRef) -> WidgetKind {
        match &self.arena.get(obj).unwrap().kind {
            WidgetKind::Obj => WidgetKind::Obj,
            WidgetKind::Label { text } => WidgetKind::Label { text: text.clone() },
            WidgetKind::Button { text } => WidgetKind::Button { text: text.clone() },
            WidgetKind::Slider { min, max, value } => WidgetKind::Slider { min: *min, max: *max, value: *value },
            WidgetKind::Switch { on } => WidgetKind::Switch { on: *on },
            WidgetKind::Bar { min, max, value } => WidgetKind::Bar { min: *min, max: *max, value: *value },
            WidgetKind::List { items, selected, scroll } => WidgetKind::List { items: items.clone(), selected: *selected, scroll: *scroll },
        }
    }

    fn node_draw_info(&self, obj: ObjRef) -> Option<(Rect, u8, crate::style::ResolvedStyle)> {
        self.arena.get(obj).map(|n| {
            (self.abs_rect(obj), n.flags, self.resolved_style(obj))
        })
    }

    pub fn create_label(&mut self, parent: ObjRef, text: &str) -> ObjRef {
        let (w, h) = crate::font::text_size(text);
        let r = self.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Label { text: text.into() });
        self.set_style(r, crate::style::theme_label());
        r
    }

    fn insert_node(&mut self, parent: ObjRef, rect: Rect, kind: WidgetKind) -> ObjRef {
        let r = self.arena.insert(Node::new(Some(parent), rect, kind));
        if let Some(p) = self.arena.get_mut(parent) {
            p.children.push(r);
        }
        self.invalidate_obj(r);
        r
    }

    pub fn create_button(&mut self, parent: ObjRef, text: &str) -> ObjRef {
        let (tw, th) = crate::font::text_size(text);
        let r = self.insert_node(parent, Rect::new(0, 0, tw + 24, th + 12),
            WidgetKind::Button { text: text.into() });
        self.set_style(r, crate::style::theme_button());
        self.set_style_pressed(r, crate::style::theme_button_pressed());
        self.set_style_focused(r, crate::style::theme_button_focused());
        if let Some(n) = self.arena.get_mut(r) {
            n.flags |= crate::node::flag::CLICKABLE;
        }
        r
    }

    pub fn create_slider(&mut self, parent: ObjRef, min: i32, max: i32) -> ObjRef {
        let r = self.insert_node(parent, Rect::new(0, 0, 100, 12),
            WidgetKind::Slider { min, max, value: min });
        self.set_style(r, crate::style::theme_slider());
        r
    }

    pub fn create_switch(&mut self, parent: ObjRef) -> ObjRef {
        let r = self.insert_node(parent, Rect::new(0, 0, 40, 20), WidgetKind::Switch { on: false });
        self.set_style(r, crate::style::theme_switch());
        r
    }

    pub fn create_bar(&mut self, parent: ObjRef, min: i32, max: i32) -> ObjRef {
        let r = self.insert_node(parent, Rect::new(0, 0, 100, 8),
            WidgetKind::Bar { min, max, value: min });
        self.set_style(r, crate::style::theme_bar());
        r
    }

    pub fn create_list(&mut self, parent: ObjRef, items: &[&str]) -> ObjRef {
        let rows = items.len().min(5).max(1) as i32;
        let r = self.insert_node(parent, Rect::new(0, 0, 120, rows * 16 + 8),
            WidgetKind::List { items: items.iter().map(|s| (*s).into()).collect(), selected: 0, scroll: 0 });
        self.set_style(r, crate::style::theme_list());
        self.set_style_focused(r, crate::style::theme_list_focused());
        r
    }

    pub fn set_value(&mut self, obj: ObjRef, v: i32) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            match &mut n.kind {
                WidgetKind::Slider { min, max, value } | WidgetKind::Bar { min, max, value } => {
                    *value = v.clamp(*min, *max);
                }
                _ => {}
            }
        }
        self.invalidate_obj(obj);
    }

    pub fn value(&self, obj: ObjRef) -> i32 {
        if let Some(n) = self.arena.get(obj) {
            match &n.kind {
                WidgetKind::Slider { value, .. } | WidgetKind::Bar { value, .. } => *value,
                _ => 0,
            }
        } else {
            0
        }
    }

    pub fn set_range(&mut self, obj: ObjRef, min: i32, max: i32) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            match &mut n.kind {
                WidgetKind::Slider { min: mn, max: mx, value } | WidgetKind::Bar { min: mn, max: mx, value } => {
                    *mn = min;
                    *mx = max;
                    *value = (*value).clamp(min, max);
                }
                _ => {}
            }
        }
        self.invalidate_obj(obj);
    }

    pub fn list_selected(&self, obj: ObjRef) -> usize {
        if let Some(n) = self.arena.get(obj) {
            if let WidgetKind::List { selected, .. } = &n.kind {
                return *selected;
            }
        }
        0
    }

    pub fn list_select(&mut self, obj: ObjRef, idx: usize) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            if let WidgetKind::List { items, selected, scroll } = &mut n.kind {
                if !items.is_empty() {
                    *selected = idx.min(items.len() - 1);
                    // 保证 selected 行可见：行高 16，可见高 = n.rect.h
                    let top = *selected as i32 * 16;
                    let vis_h = n.rect.h;
                    if top < *scroll {
                        *scroll = top;
                    } else if top + 16 > *scroll + vis_h {
                        *scroll = top + 16 - vis_h;
                    }
                }
            }
        }
        self.invalidate_obj(obj);
    }

    pub fn set_text(&mut self, obj: ObjRef, text: &str) {
        self.invalidate_obj(obj);
        let (w, h) = crate::font::text_size(text);
        if let Some(n) = self.arena.get_mut(obj) {
            if let WidgetKind::Label { text: t } = &mut n.kind {
                *t = text.into();
                n.rect.w = w;
                n.rect.h = h;
            }
        }
        self.invalidate_obj(obj);
    }

    pub fn text(&self, obj: ObjRef) -> alloc::string::String {
        if let Some(n) = self.arena.get(obj) {
            if let WidgetKind::Label { text } = &n.kind {
                return text.clone();
            }
        }
        alloc::string::String::new()
    }
}
