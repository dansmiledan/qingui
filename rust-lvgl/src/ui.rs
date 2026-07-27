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
    time_ms: u64,
    anims: Vec<crate::anim::RunningAnim>,
}

impl Ui {
    pub fn new(width: i32, height: i32, buf_rows: u32) -> Ui {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, width, height), WidgetKind::Obj));
        let mut dirty = crate::dirty::DirtyQueue::new(Rect::new(0, 0, width, height), 16);
        dirty.add(Rect::new(0, 0, width, height)); // 建屏全屏标脏
        let buf = alloc::vec![crate::geometry::Color::BLACK; (width * buf_rows as i32).max(0) as usize];
        Ui { arena, screen, width, height, dirty, flush: None, buf, time_ms: 0, anims: Vec::new() }
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

    pub fn tick_inc(&mut self, ms: u32) {
        self.time_ms += ms as u64;
    }
    pub fn time(&self) -> u64 {
        self.time_ms
    }

    pub fn anim_start(&mut self, a: crate::anim::Anim) {
        // 同目标同属性的旧动画被替换（对齐 LVGL 语义）
        self.anim_stop(a.target, a.prop);
        // 立即应用起始值，避免跳变
        self.apply_anim_value(a.target, a.prop, a.start);
        self.anims.push(crate::anim::RunningAnim { anim: a, start_time: self.time_ms });
    }
    pub fn anim_stop(&mut self, target: ObjRef, prop: crate::anim::AnimProp) {
        self.anims.retain(|r| !(r.anim.target == target && r.anim.prop == prop));
    }
    pub fn anim_running(&self) -> bool {
        !self.anims.is_empty()
    }

    pub fn timer_handler(&mut self) -> u32 {
        self.step_anims();
        self.render();
        if self.anim_running() { 0 } else { u32::MAX }
    }

    fn step_anims(&mut self) {
        let now = self.time_ms;
        let mut i = 0;
        while i < self.anims.len() {
            let target = self.anims[i].anim.target;
            if !self.is_valid(target) {
                self.anims.remove(i); // 目标已删除：清理动画
                continue;
            }
            enum Out {
                Delay,
                Keep(i32),
                Done(i32, Option<alloc::boxed::Box<dyn FnMut(&mut Ui)>>),
            }
            let out = {
                let r = &mut self.anims[i];
                let a = &mut r.anim;
                let elapsed = now.saturating_sub(r.start_time);
                if elapsed < a.delay_ms as u64 {
                    Out::Delay
                } else {
                    let t_ms = elapsed - a.delay_ms as u64;
                    let dur = a.duration_ms.max(1) as u64;
                    let total: i32 = if a.repeat < 0 { i32::MAX } else { a.repeat.max(1) };
                    if t_ms >= dur * total as u64 {
                        let last = total - 1;
                        let rev = a.playback && last % 2 == 1;
                        let v = if rev { a.start } else { a.end };
                        Out::Done(v, a.on_done.take())
                    } else {
                        let round = (t_ms / dur) as i32;
                        let in_round = t_ms % dur;
                        let rev = a.playback && round % 2 == 1;
                        let mut t = in_round as f32 / dur as f32;
                        if rev {
                            t = 1.0 - t;
                        }
                        let k = a.easing.eval(t);
                        Out::Keep(a.start + ((a.end - a.start) as f32 * k) as i32)
                    }
                }
            };
            match out {
                Out::Delay => i += 1,
                Out::Keep(v) => {
                    let prop = self.anims[i].anim.prop;
                    self.apply_anim_value(target, prop, v);
                    i += 1;
                }
                Out::Done(v, cb) => {
                    let r = self.anims.remove(i);
                    self.apply_anim_value(r.anim.target, r.anim.prop, v);
                    if let Some(mut cb) = cb {
                        cb(self);
                    }
                }
            }
        }
    }

    fn apply_anim_value(&mut self, target: ObjRef, prop: crate::anim::AnimProp, v: i32) {
        use crate::anim::AnimProp;
        if !self.is_valid(target) {
            return;
        }
        match prop {
            AnimProp::X => {
                let y = self.rect(target).y;
                self.set_pos(target, v, y);
            }
            AnimProp::Y => {
                let x = self.rect(target).x;
                self.set_pos(target, x, v);
            }
            AnimProp::W => {
                let h = self.rect(target).h;
                self.set_size(target, v, h);
            }
            AnimProp::H => {
                let w = self.rect(target).w;
                self.set_size(target, w, v);
            }
            AnimProp::Opa => {
                self.invalidate_obj(target);
                if let Some(n) = self.arena.get_mut(target) {
                    n.opa = v.clamp(0, 255) as u8;
                }
                self.invalidate_obj(target);
            }
            AnimProp::Value => self.set_value(target, v),
        }
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
        let Some((abs, flags, node_opa, resolved)) = self.node_draw_info(obj) else {
            return;
        };
        if flags & crate::node::flag::HIDDEN != 0 {
            return;
        }
        // 节点 opa 作为乘数作用于本对象的所有绘制
        let ap = |base: u8| (base as u32 * node_opa as u32 / 255) as u8;
        if abs.intersect(&clip).is_some() {
            let kind_snap = self.kind_snapshot(obj);
            let edited = self.state(obj) & crate::node::state::EDITED != 0;
            let mut d = crate::draw::DrawBuf {
                pixels: &mut self.buf[..len],
                area: clip,
                stride: clip.w,
            };
            if resolved.bg_opa > 0 && ap(resolved.bg_opa) > 0 {
                d.fill_rounded(abs, resolved.radius, resolved.bg_color, ap(resolved.bg_opa), clip);
            }
            if resolved.border_width > 0 {
                d.draw_border(abs, resolved.border_width, resolved.radius, resolved.border_color, ap(255), clip);
            }
            match kind_snap {
                WidgetKind::Label { text } => {
                    d.draw_text_opa(crate::geometry::Point { x: abs.x, y: abs.y }, &text, resolved.text_color, ap(255), clip);
                }
                WidgetKind::Button { text } => {
                    let (tw, th) = crate::font::text_size(&text);
                    let p = crate::geometry::Point {
                        x: abs.x + (abs.w - tw) / 2,
                        y: abs.y + (abs.h - th) / 2,
                    };
                    d.draw_text_opa(p, &text, resolved.text_color, ap(255), clip);
                }
                WidgetKind::Slider { min, max, value } => {
                    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
                    let iw = (abs.w as f32 * frac) as i32;
                    if iw > 0 {
                        d.fill_rounded(Rect::new(abs.x, abs.y, iw, abs.h), resolved.radius, Color::rgb(80, 140, 255), ap(255), clip);
                    }
                    let kx = abs.x + iw;
                    let knob = Rect::new(kx - 4, abs.y - 2, 8, abs.h + 4);
                    let kc = if edited { Color::rgb(255, 200, 60) } else { Color::WHITE };
                    d.fill_rounded(knob, 3, kc, ap(255), clip);
                }
                WidgetKind::Switch { on } => {
                    let tc = if on { Color::rgb(60, 180, 90) } else { Color::rgb(90, 90, 90) };
                    d.fill_rounded(abs, abs.h / 2, tc, ap(255), clip);
                    let k = abs.h - 4;
                    let kx = if on { abs.right() - k - 2 } else { abs.x + 2 };
                    d.fill_rounded(Rect::new(kx, abs.y + 2, k, k), k / 2, Color::WHITE, ap(255), clip);
                }
                WidgetKind::Bar { min, max, value } => {
                    let frac = if max > min { (value - min) as f32 / (max - min) as f32 } else { 0.0 };
                    let iw = (abs.w as f32 * frac) as i32;
                    if iw > 0 {
                        d.fill_rounded(Rect::new(abs.x, abs.y, iw, abs.h), resolved.radius, Color::rgb(80, 140, 255), ap(255), clip);
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
                            d.fill_rect(row, Color::rgb(50, 70, 120), ap(255), lclip);
                        }
                        d.draw_text_opa(
                            crate::geometry::Point { x: abs.x + 4, y: ry + 4 },
                            item,
                            resolved.text_color,
                            ap(255),
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

    fn node_draw_info(&self, obj: ObjRef) -> Option<(Rect, u8, u8, crate::style::ResolvedStyle)> {
        self.arena.get(obj).map(|n| {
            (self.abs_rect(obj), n.flags, n.opa, self.resolved_style(obj))
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
