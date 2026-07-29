use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::arena::{Arena, ObjRef};
use crate::geometry::Rect;
use crate::node::{Flag, Node, State, WidgetKind};

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
    group: Vec<ObjRef>,
    focused_idx: Option<usize>,
    pub(crate) layout_dirty: bool,
    canvas_cbs: Vec<Option<crate::widgets::canvas::CanvasCb>>,
    modal: Option<ObjRef>,
}

impl Ui {
    pub fn new(width: i32, height: i32, buf_rows: u32) -> Ui {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, width, height), WidgetKind::Obj));
        let mut dirty = crate::dirty::DirtyQueue::new(Rect::new(0, 0, width, height), 16);
        dirty.add(Rect::new(0, 0, width, height)); // 建屏全屏标脏
        let buf = alloc::vec![crate::geometry::Color::BLACK; (width * buf_rows as i32).max(0) as usize];
        Ui { arena, screen, width, height, dirty, flush: None, buf, time_ms: 0, anims: Vec::new(), group: Vec::new(), focused_idx: None, layout_dirty: false, canvas_cbs: Vec::new(), modal: None }
    }

    pub(crate) fn register_canvas_cb(&mut self, cb: crate::widgets::canvas::CanvasCb) -> usize {
        self.canvas_cbs.push(Some(cb));
        self.canvas_cbs.len() - 1
    }

    /// 自定义绘制控件：回调签名为 (画板, 控件绝对矩形, 裁剪矩形, 当前时间 ms)
    pub fn create_canvas(&mut self, parent: ObjRef, w: i32, h: i32, cb: crate::widgets::canvas::CanvasCb) -> ObjRef {
        crate::widgets::canvas::create(self, parent, w, h, cb)
    }

    pub fn screen(&self) -> ObjRef {
        self.screen
    }

    /// 链式配置包装：`ui.widget(obj).pos(10, 10).size(80, 30)`
    pub fn widget(&mut self, obj: ObjRef) -> crate::widget::WidgetMut<'_> {
        crate::widget::WidgetMut::new(self, obj)
    }

    pub fn is_valid(&self, obj: ObjRef) -> bool {
        self.arena.contains(obj)
    }

    pub fn create_obj(&mut self, parent: ObjRef) -> ObjRef {
        self.insert_node(parent, Rect::default(), WidgetKind::Obj)
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
        for r in all.clone() {
            if self.modal == Some(r) {
                self.modal = None; // modal 子树被删除：解除模态锁定
            }
            self.arena.remove(r);
        }
        // 焦点组同步移除
        for r in all {
            self.group_remove(r);
        }
        self.layout_dirty = true;
    }

    pub fn children(&self, obj: ObjRef) -> Vec<ObjRef> {
        self.arena.get(obj).map(|n| n.children.clone()).unwrap_or_default()
    }

    pub fn rect(&self, obj: ObjRef) -> Rect {
        self.arena.get(obj).map(|n| n.rect).unwrap_or_default()
    }

    pub fn abs_rect(&self, obj: ObjRef) -> Rect {
        let mut r = self.rect(obj);
        // 沿父链累加：祖先的本地坐标与 translate 都作用于子树（translate 是子树级视觉偏移）
        let mut cur = self.arena.get(obj).and_then(|n| n.parent);
        while let Some(p) = cur {
            let n = self.arena.get(p).unwrap();
            r = r.translate(n.rect.x + n.translate.x, n.rect.y + n.translate.y);
            cur = n.parent;
        }
        // 自身视觉平移
        if let Some(n) = self.arena.get(obj) {
            r = r.translate(n.translate.x, n.translate.y);
        }
        r
    }

    /// 设置视觉平移偏移（对齐 LVGL translate_x/y）：子树整体偏移，只影响渲染，不参与布局
    pub fn set_translate(&mut self, obj: ObjRef, x: i32, y: i32) {
        self.invalidate_subtree(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.translate = crate::geometry::Point { x, y };
        }
        self.invalidate_subtree(obj);
    }

    /// 标脏整棵子树的渲染区域（translate 变化时子元素也会移动）
    fn invalidate_subtree(&mut self, obj: ObjRef) {
        if !self.is_valid(obj) {
            return;
        }
        let mut area = self.abs_rect(obj);
        let mut stack = alloc::vec![obj];
        while let Some(r) = stack.pop() {
            for c in self.children(r) {
                area = area.union(&self.abs_rect(c));
                stack.push(c);
            }
        }
        self.invalidate_area(area);
    }

    pub fn translate(&self, obj: ObjRef) -> crate::geometry::Point {
        self.arena.get(obj).map(|n| n.translate).unwrap_or_default()
    }

    /// 设置对象位置（本地坐标）。注意：不触发布局重算——位置对布局是输出而非输入，
    /// 被 Flex/Grid 管理的子对象位置归布局所有（下次布局重算时会被覆盖），
    /// 需要视觉位移请用 set_translate。
    /// 标脏整棵子树：子元素的屏幕坐标随父移动。
    pub fn set_pos(&mut self, obj: ObjRef, x: i32, y: i32) {
        self.invalidate_subtree(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.x = x;
            n.rect.y = y;
        }
        self.invalidate_subtree(obj);
    }

    /// 设置对象尺寸。标脏整棵子树（子元素坐标/裁剪可能随父变化）。
    pub fn set_size(&mut self, obj: ObjRef, w: i32, h: i32) {
        self.invalidate_subtree(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.w = w;
            n.rect.h = h;
        }
        self.invalidate_subtree(obj);
        self.layout_dirty = true;
    }

    pub fn invalidate_area(&mut self, rect: Rect) {
        self.dirty.add(rect);
    }
    pub fn invalidate_obj(&mut self, obj: ObjRef) {
        if self.is_valid(obj) {
            let ext = self.arena.get(obj).map(|n| crate::widgets::overflow_of(&n.kind)).unwrap_or(0);
            let r = self.abs_rect(obj);
            // 控件绘制可能超出自身矩形（旋钮等），标脏外扩
            self.dirty.add(Rect::new(r.x - ext, r.y - ext, r.w + 2 * ext, r.h + 2 * ext));
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
            n.flags.set(Flag::HIDDEN, hidden);
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }

    pub fn set_style(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style = style;
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    pub fn set_style_pressed(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_pressed = style;
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    pub fn set_style_focused(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_focused = style;
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    pub fn set_state(&mut self, obj: ObjRef, state: State, on: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.state.set(state, on);
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    pub fn state(&self, obj: ObjRef) -> State {
        self.arena.get(obj).map(|n| n.state).unwrap_or_default()
    }
    pub fn resolved_style(&self, obj: ObjRef) -> crate::style::ResolvedStyle {
        let Some(n) = self.arena.get(obj) else {
            return crate::style::ResolvedStyle::default();
        };
        // pressed 优先于 focused
        let overlay = if n.state.contains(State::PRESSED) {
            Some(&n.style_pressed)
        } else if n.state.contains(State::FOCUSED) {
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
        if self.layout_dirty {
            self.layout_pass();
            self.layout_dirty = false;
        }
        // 浮层定位每帧执行（跟随目标移动/动画；位置未变时无开销）
        self.layout_floating(self.screen);
        let list_fx_active = self.tick_list_fx();
        self.render();
        if self.anim_running() || list_fx_active { 0 } else { u32::MAX }
    }

    /// 遍历对象树：驱动控件自动画（List 效果、Spinner 旋转）。
    /// 活动中的标脏（驱动逐帧重绘），返回是否有活动效果。
    fn tick_list_fx(&mut self) -> bool {
        let now = self.time_ms;
        let mut any = false;
        let mut stack = alloc::vec![self.screen];
        while let Some(r) = stack.pop() {
            let (children, redraw, active) = match self.arena.get_mut(r) {
                Some(n) => {
                    let (redraw, active) = match &mut n.kind {
                        WidgetKind::List { fx, .. } => {
                            let was_active = fx.active(now);
                            let removed = fx.prune(now);
                            // 活动中逐帧重绘；清理掉效果的这一帧也补一次重绘（清掉 ghost 残影）
                            (was_active || removed, fx.active(now))
                        }
                        WidgetKind::Roller { sel_from, .. } => {
                            let had_fx = sel_from.is_some();
                            let active = crate::widgets::roller::fx_active(*sel_from, now);
                            if !active {
                                *sel_from = None;
                            }
                            // 有 fx（含本帧过期）就重绘：完成帧必须补最后一定格
                            (had_fx, active)
                        }
                        // Spinner 永远自转
                        WidgetKind::Spinner => (true, true),
                        _ => (false, false),
                    };
                    (n.children.clone(), redraw, active)
                }
                None => (Vec::new(), false, false),
            };
            if redraw {
                self.invalidate_obj(r);
            }
            if active {
                any = true;
            }
            stack.extend_from_slice(&children);
        }
        any
    }

    fn layout_pass(&mut self) {
        let screen = self.screen;
        self.layout_subtree(screen);
    }
    /// 浮层定位（先序遍历保证锚定链按树序解析；位置未变化时不标脏）
    fn layout_floating(&mut self, obj: ObjRef) {
        let fl = self.arena.get(obj).and_then(|n| n.floating);
        if let Some((target, attach)) = fl {
            use crate::layout::Attach;
            let t = self.abs_rect(target);
            let r = self.rect(obj);
            let (dx, dy) = match attach {
                Attach::Center => (t.x + (t.w - r.w) / 2, t.y + (t.h - r.h) / 2),
                Attach::Top => (t.x + (t.w - r.w) / 2, t.y - r.h),
                Attach::Bottom => (t.x + (t.w - r.w) / 2, t.bottom()),
                Attach::Left => (t.x - r.w, t.y + (t.h - r.h) / 2),
                Attach::Right => (t.right(), t.y + (t.h - r.h) / 2),
            };
            // 转为父对象本地坐标（相对父 abs 原点）
            let pabs = self.arena.get(obj).and_then(|n| n.parent).map(|p| self.abs_rect(p));
            let (px, py) = pabs.map(|p| (p.x, p.y)).unwrap_or((0, 0));
            let (nx, ny) = (dx - px, dy - py);
            let cur = self.rect(obj);
            if cur.x != nx || cur.y != ny {
                self.set_pos(obj, nx, ny);
            }
        }
        for c in self.children(obj) {
            self.layout_floating(c);
        }
    }
    fn layout_subtree(&mut self, obj: ObjRef) {
        let layout = self.arena.get(obj).and_then(|n| n.style.layout.clone());
        match layout {
            Some(crate::style::Layout::Flex(f)) => crate::layout::layout_flex(self, obj, &f),
            Some(crate::style::Layout::Grid(g)) => crate::layout::layout_grid(self, obj, &g),
            _ => {}
        }
        for c in self.children(obj) {
            self.layout_subtree(c);
        }
    }

    pub fn grid_cell(&self, obj: ObjRef) -> ((u8, u8), (u8, u8)) {
        self.arena.get(obj).map(|n| (n.grid_col, n.grid_row)).unwrap_or(((0, 1), (0, 1)))
    }
    pub fn set_grid_cell(&mut self, obj: ObjRef, col: (u8, u8), row: (u8, u8)) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.grid_col = (col.0, col.1.max(1));
            n.grid_row = (row.0, row.1.max(1));
        }
        self.layout_dirty = true;
    }

    pub fn set_layout(&mut self, obj: ObjRef, layout: crate::style::Layout) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style.layout = Some(layout);
        }
        self.layout_dirty = true;
    }

    /// 设置宽/高尺寸策略（None = 内容尺寸）
    pub fn set_sizing(&mut self, obj: ObjRef, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style.sizing_w = w;
            n.style.sizing_h = h;
        }
        self.layout_dirty = true;
    }

    /// 设置宽高比（千分比：1000 = 1:1，1778 ≈ 16:9；None 取消）
    pub fn set_aspect(&mut self, obj: ObjRef, ratio: Option<u32>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style.aspect_ratio = ratio;
        }
        self.layout_dirty = true;
    }

    /// 设置布局过渡：(时长 ms, 缓动)。布局改变位置/尺寸时自动动画过渡；None 关闭
    pub fn set_transition(&mut self, obj: ObjRef, transition: Option<(u32, crate::anim::Easing)>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style.transition = transition;
        }
        self.layout_dirty = true;
    }

    /// 已有奔向相同目标值的同属性动画？（避免布局重算反复重启过渡动画）
    fn anim_end_for(&self, target: ObjRef, prop: crate::anim::AnimProp) -> Option<i32> {
        self.anims
            .iter()
            .find(|r| r.anim.target == target && r.anim.prop == prop)
            .map(|r| r.anim.end)
    }

    /// 布局写位置：开启过渡且非首次布局时自动动画到目标，否则瞬移
    pub(crate) fn layout_move(&mut self, obj: ObjRef, x: i32, y: i32) {
        let Some(n) = self.arena.get(obj) else { return };
        let laid = n.laid_out;
        let cur = n.rect;
        let tr = self.resolved_style(obj).transition;
        let mut animated = false;
        if laid && (cur.x != x || cur.y != y) {
            if let Some((dur, easing)) = tr {
                if dur > 0 {
                    use crate::anim::AnimProp;
                    if cur.x != x && self.anim_end_for(obj, AnimProp::X) != Some(x) {
                        let mut a = crate::anim::Anim::new(obj, AnimProp::X, cur.x, x, dur);
                        a.easing = easing;
                        self.anim_start(a);
                    }
                    if cur.y != y && self.anim_end_for(obj, AnimProp::Y) != Some(y) {
                        let mut a = crate::anim::Anim::new(obj, AnimProp::Y, cur.y, y, dur);
                        a.easing = easing;
                        self.anim_start(a);
                    }
                    animated = true;
                }
            }
        }
        if !animated && (cur.x != x || cur.y != y) {
            self.set_pos(obj, x, y);
        }
        if let Some(n) = self.arena.get_mut(obj) {
            n.laid_out = true;
        }
    }

    /// 布局写尺寸：开启过渡且非首次布局时自动动画到目标，否则瞬移
    pub(crate) fn layout_resize(&mut self, obj: ObjRef, w: i32, h: i32) {
        let Some(n) = self.arena.get(obj) else { return };
        let laid = n.laid_out;
        let cur = n.rect;
        let tr = self.resolved_style(obj).transition;
        let mut animated = false;
        if laid && (cur.w != w || cur.h != h) {
            if let Some((dur, easing)) = tr {
                if dur > 0 {
                    use crate::anim::AnimProp;
                    if cur.w != w && self.anim_end_for(obj, AnimProp::W) != Some(w) {
                        let mut a = crate::anim::Anim::new(obj, AnimProp::W, cur.w, w, dur);
                        a.easing = easing;
                        self.anim_start(a);
                    }
                    if cur.h != h && self.anim_end_for(obj, AnimProp::H) != Some(h) {
                        let mut a = crate::anim::Anim::new(obj, AnimProp::H, cur.h, h, dur);
                        a.easing = easing;
                        self.anim_start(a);
                    }
                    animated = true;
                }
            }
        }
        if !animated && (cur.w != w || cur.h != h) {
            self.set_size(obj, w, h);
        }
        // laid_out 由 layout_move 统一标记（两者总是成对调用）
    }
    pub fn is_hidden(&self, obj: ObjRef) -> bool {
        self.arena.get(obj).map(|n| n.flags.contains(Flag::HIDDEN)).unwrap_or(false)
    }

    /// 设置浮层锚定：对象变为浮动（IGNORE_LAYOUT），位置由锚点自动计算并跟随目标
    pub fn set_floating(&mut self, obj: ObjRef, target: ObjRef, attach: crate::layout::Attach) {
        if !self.is_valid(obj) || !self.is_valid(target) {
            return;
        }
        if let Some(n) = self.arena.get_mut(obj) {
            n.floating = Some((target, attach));
            n.flags |= Flag::IGNORE_LAYOUT;
        }
        self.layout_dirty = true;
    }

    /// 取消浮层锚定（IGNORE_LAYOUT 标志保留，可手动清除）
    pub fn clear_floating(&mut self, obj: ObjRef) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.floating = None;
        }
        self.layout_dirty = true;
    }

    /// 设置叠放次序（渲染时兄弟节点按 z_index 稳定排序，大者在上）
    pub fn set_z_index(&mut self, obj: ObjRef, z: i16) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.z_index = z;
        }
        self.invalidate_obj(obj);
    }

    /// 设置/查询浮动标志：浮动对象不参与父容器布局（弹窗/悬浮层用）
    pub fn set_ignore_layout(&mut self, obj: ObjRef, ignore: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            if ignore {
                n.flags |= Flag::IGNORE_LAYOUT;
            } else {
                n.flags &= !Flag::IGNORE_LAYOUT;
            }
        }
        self.layout_dirty = true;
    }
    pub fn is_ignore_layout(&self, obj: ObjRef) -> bool {
        self.arena.get(obj).map(|n| n.flags.contains(Flag::IGNORE_LAYOUT)).unwrap_or(false)
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
            AnimProp::TranslateX => {
                let y = self.translate(target).y;
                self.set_translate(target, v, y);
            }
            AnimProp::TranslateY => {
                let x = self.translate(target).x;
                self.set_translate(target, x, v);
            }
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
        let roots = self.children_z_sorted(self.screen);
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
        if flags.contains(Flag::HIDDEN) {
            return;
        }
        // 节点 opa 作为乘数作用于本对象的所有绘制
        let ap = |base: u8| (base as u32 * node_opa as u32 / 255) as u8;
        if abs.intersect(&clip).is_some() {
            let kind_snap = self.arena.get(obj).unwrap().kind.clone();
            let edited = self.state(obj).contains(State::EDITED);
            let now = self.time_ms;
            let mut d = crate::draw::DrawBuf {
                pixels: &mut self.buf[..len],
                area: clip,
                stride: clip.w,
            };
            if resolved.bg_opa > 0 && ap(resolved.bg_opa) > 0 {
                d.fill_rounded(abs, resolved.radius, resolved.bg_color, ap(resolved.bg_opa), clip);
            }
            let ctx = crate::widgets::WidgetCtx { abs, resolved: &resolved, edited, opa: node_opa, now };
            crate::widgets::draw(&kind_snap, &ctx, &mut d, clip);
            // Canvas：调用注册表中的用户回调
            if let WidgetKind::Canvas { cb } = &kind_snap {
                if let Some(f) = self.canvas_cbs.get_mut(*cb).and_then(|c| c.as_mut()) {
                    f(&mut d, abs, clip, now);
                }
            }
            // 边框最后画（对齐 LVGL：border 在内容之上），避免被控件内容覆盖
            if resolved.border_width > 0 {
                d.draw_border(abs, resolved.border_width, resolved.radius, resolved.border_color, ap(255), clip);
            }
        }
        for c in self.children_z_sorted(obj) {
            self.draw_node(c, clip, len);
        }
    }

    /// 子对象按 z_index 稳定排序（小者先画，大者在上）
    fn children_z_sorted(&self, obj: ObjRef) -> Vec<ObjRef> {
        let mut kids = self.children(obj);
        kids.sort_by_key(|&c| self.arena.get(c).map(|n| n.z_index).unwrap_or(0));
        kids
    }

    fn node_draw_info(&self, obj: ObjRef) -> Option<(Rect, Flag, u8, crate::style::ResolvedStyle)> {
        self.arena.get(obj).map(|n| {
            (self.abs_rect(obj), n.flags, n.opa, self.resolved_style(obj))
        })
    }

    pub fn create_label(&mut self, parent: ObjRef, text: &str) -> ObjRef {
        crate::widgets::label::create(self, parent, text)
    }

    pub(crate) fn insert_node(&mut self, parent: ObjRef, rect: Rect, kind: WidgetKind) -> ObjRef {
        let r = self.arena.insert(Node::new(Some(parent), rect, kind));
        if let Some(p) = self.arena.get_mut(parent) {
            p.children.push(r);
        }
        self.invalidate_obj(r);
        self.layout_dirty = true;
        r
    }

    pub fn create_button(&mut self, parent: ObjRef, text: &str) -> ObjRef {
        crate::widgets::button::create(self, parent, text)
    }

    pub fn create_slider(&mut self, parent: ObjRef, min: i32, max: i32) -> ObjRef {
        crate::widgets::slider::create(self, parent, min, max)
    }

    pub fn create_switch(&mut self, parent: ObjRef) -> ObjRef {
        crate::widgets::switch::create(self, parent)
    }

    pub fn create_bar(&mut self, parent: ObjRef, min: i32, max: i32) -> ObjRef {
        crate::widgets::bar::create(self, parent, min, max)
    }

    pub fn create_list(&mut self, parent: ObjRef, items: &[&str]) -> ObjRef {
        crate::widgets::list::create(self, parent, items)
    }

    pub fn create_arc(&mut self, parent: ObjRef, min: i32, max: i32) -> ObjRef {
        crate::widgets::arc::create(self, parent, min, max)
    }

    pub fn create_checkbox(&mut self, parent: ObjRef, text: &str) -> ObjRef {
        crate::widgets::checkbox::create(self, parent, text)
    }

    pub fn create_spinner(&mut self, parent: ObjRef) -> ObjRef {
        crate::widgets::spinner::create(self, parent)
    }

    /// 模态消息框：标题 + 文本 + 按钮行。点击按钮关闭并读 msgbox_selected（Esc = -1）
    pub fn create_msgbox(&mut self, parent: ObjRef, title: &str, text: &str, buttons: &[&str]) -> ObjRef {
        crate::widgets::msgbox::create(self, parent, title, text, buttons)
    }
    pub fn msgbox_selected(&self, obj: ObjRef) -> i32 {
        if let Some(n) = self.arena.get(obj) {
            if let WidgetKind::Msgbox { selected } = &n.kind {
                return *selected;
            }
        }
        -1
    }

    pub fn create_led(&mut self, parent: ObjRef, color: crate::geometry::Color) -> ObjRef {
        crate::widgets::led::create(self, parent, color)
    }

    pub fn create_table(&mut self, parent: ObjRef, cols: u8, rows: u8) -> ObjRef {
        crate::widgets::table::create(self, parent, cols, rows)
    }

    pub fn table_set_cell(&mut self, obj: ObjRef, row: u8, col: u8, text: &str) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            if let WidgetKind::Table { cols, rows, cells } = &mut n.kind {
                if row < *rows && col < *cols {
                    cells[row as usize * *cols as usize + col as usize] = text.into();
                }
            }
        }
        self.invalidate_obj(obj);
    }

    pub fn create_spinbox(&mut self, parent: ObjRef, min: i32, max: i32, digits: u8) -> ObjRef {
        crate::widgets::spinbox::create(self, parent, min, max, digits)
    }

    pub fn create_roller(&mut self, parent: ObjRef, items: &[&str]) -> ObjRef {
        crate::widgets::roller::create(self, parent, items)
    }

    pub fn roller_selected(&self, obj: ObjRef) -> usize {
        if let Some(n) = self.arena.get(obj) {
            if let WidgetKind::Roller { selected, .. } = &n.kind {
                return *selected;
            }
        }
        0
    }

    pub fn create_dropdown(&mut self, parent: ObjRef, items: &[&str]) -> ObjRef {
        crate::widgets::dropdown::create(self, parent, items)
    }

    pub fn set_value(&mut self, obj: ObjRef, v: i32) {
        self.invalidate_value_area(obj);
        let changed = match self.arena.get_mut(obj) {
            Some(n) => crate::widgets::set_value_of(&mut n.kind, v),
            None => false,
        };
        self.invalidate_value_area(obj);
        if changed {
            self.send_event(obj, crate::event::EventKind::ValueChanged);
        }
    }

    /// 值变化时的标脏区域（invalidate_obj 已按控件类型外扩）
    fn invalidate_value_area(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
    }

    pub fn value(&self, obj: ObjRef) -> i32 {
        self.arena.get(obj).map(|n| crate::widgets::value_of(&n.kind)).unwrap_or(0)
    }

    pub fn set_range(&mut self, obj: ObjRef, min: i32, max: i32) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            crate::widgets::set_range_of(&mut n.kind, min, max);
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
        let now = self.time_ms;
        if let Some(n) = self.arena.get_mut(obj) {
            let vis_h = n.rect.h;
            if let WidgetKind::List { items, selected, scroll, fx } = &mut n.kind {
                crate::widgets::list::select(items, selected, scroll, fx, idx, vis_h, now);
            }
        }
        self.invalidate_obj(obj);
    }

    /// 在 idx 处插入一项（下方 item 下滑让位，新项淡入）。
    /// 容量上限由调用方控制（可用 list_len 判断）。
    pub fn list_insert(&mut self, obj: ObjRef, idx: usize, text: &str) {
        self.invalidate_obj(obj);
        let now = self.time_ms;
        if let Some(n) = self.arena.get_mut(obj) {
            if let WidgetKind::List { items, selected, fx, .. } = &mut n.kind {
                let idx = idx.min(items.len());
                // 插入位置在选中项之上时，选中索引顺延
                if !items.is_empty() && *selected >= idx {
                    *selected += 1;
                }
                crate::widgets::list::insert(items, fx, idx, text, now);
            }
        }
        self.invalidate_obj(obj);
    }

    /// 删除当前选中项（渐隐 + 下方 item 上移），返回是否成功
    pub fn list_remove(&mut self, obj: ObjRef) -> bool {
        self.invalidate_obj(obj);
        let now = self.time_ms;
        let ok = match self.arena.get_mut(obj) {
            Some(n) => {
                let vis_h = n.rect.h;
                if let WidgetKind::List { items, selected, scroll, fx } = &mut n.kind {
                    let ok = crate::widgets::list::remove(items, fx, selected, now);
                    // 删除后尾部空窗时自动上滚填满窗口
                    crate::widgets::list::ensure_visible(*selected, items.len(), scroll, fx, vis_h, now);
                    ok
                } else {
                    false
                }
            }
            None => false,
        };
        self.invalidate_obj(obj);
        ok
    }

    pub fn set_text(&mut self, obj: ObjRef, text: &str) {
        crate::widgets::label::set_text(self, obj, text);
    }

    pub fn text(&self, obj: ObjRef) -> alloc::string::String {
        crate::widgets::label::text(self, obj)
    }

    pub fn add_event_cb(&mut self, obj: ObjRef, kind: crate::event::EventKind, cb: crate::event::EventCb) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.events.push((kind, cb));
        }
    }

    pub fn send_event(&mut self, obj: ObjRef, kind: crate::event::EventKind) {
        use crate::event::EventKind;
        let mut cursor = 0usize;
        loop {
            // 找到下一个匹配的回调并取出（先移出 arena，避免回调内 &mut Ui 冲突）
            let taken = {
                let Some(n) = self.arena.get_mut(obj) else { return };
                let mut found = None;
                let mut i = cursor;
                while i < n.events.len() {
                    let matches = match (&n.events[i].0, &kind) {
                        (EventKind::Key(_), EventKind::Key(_)) => true, // Key 按类别通配
                        (a, b) => a == b,
                    };
                    if matches {
                        found = Some(n.events.remove(i).1);
                        cursor = i;
                        break;
                    }
                    i += 1;
                }
                found
            };
            let Some(mut cb) = taken else { return };
            cb(self, obj, kind);
            // 放回（对象可能已被回调删除；回调内新注册的回调本轮不触发）
            if let Some(n) = self.arena.get_mut(obj) {
                let idx = cursor.min(n.events.len());
                n.events.insert(idx, (stored_label(kind), cb));
            } else {
                return;
            }
            cursor += 1;
        }
    }

    pub fn group_add(&mut self, obj: ObjRef) {
        if self.is_valid(obj) && !self.group.contains(&obj) {
            self.group.push(obj);
            if self.focused_idx.is_none() {
                self.focused_idx = Some(self.group.len() - 1);
                self.set_state(obj, State::FOCUSED, true);
                self.send_event(obj, crate::event::EventKind::Focused);
            }
        }
    }
    pub fn group_remove(&mut self, obj: ObjRef) {
        if let Some(pos) = self.group.iter().position(|&o| o == obj) {
            self.group.remove(pos);
            if self.focused_idx == Some(pos) {
                self.focused_idx = None;
                self.set_state(obj, State::FOCUSED, false);
                if !self.group.is_empty() {
                    let ni = pos.min(self.group.len() - 1);
                    self.focused_idx = Some(ni);
                    let f = self.group[ni];
                    self.set_state(f, State::FOCUSED, true);
                }
            } else if let Some(fi) = self.focused_idx {
                if pos < fi {
                    self.focused_idx = Some(fi - 1);
                }
            }
        }
    }
    pub fn focused(&self) -> Option<ObjRef> {
        self.focused_idx.and_then(|i| self.group.get(i).copied())
    }
    pub fn group_focus(&mut self, obj: ObjRef) {
        if let Some(pos) = self.group.iter().position(|&o| o == obj) {
            self.focus_to(pos);
        }
    }
    pub fn group_focus_next(&mut self) {
        if self.group.is_empty() {
            return;
        }
        let cur = self.focused_idx.unwrap_or(0);
        for step in 1..=self.group.len() {
            let idx = (cur + step) % self.group.len();
            if self.focusable(self.group[idx]) {
                self.focus_to(idx);
                return;
            }
        }
    }
    pub fn group_focus_prev(&mut self) {
        if self.group.is_empty() {
            return;
        }
        let cur = self.focused_idx.unwrap_or(0);
        for step in 1..=self.group.len() {
            let idx = (cur + self.group.len() - step) % self.group.len();
            if self.focusable(self.group[idx]) {
                self.focus_to(idx);
                return;
            }
        }
    }
    /// 可被聚焦：未有效隐藏，且在 modal 子树内（modal 未设置时全局）
    fn focusable(&self, obj: ObjRef) -> bool {
        if self.is_hidden_eff(obj) {
            return false;
        }
        let Some(m) = self.modal else { return true };
        // obj == modal 或 obj 是 modal 的后代
        let mut cur = Some(obj);
        while let Some(o) = cur {
            if o == m {
                return true;
            }
            cur = self.arena.get(o).and_then(|n| n.parent);
        }
        false
    }

    /// 设置模态对象：焦点导航锁定在其子树内，并把焦点移入
    pub fn set_modal(&mut self, obj: ObjRef) {
        if !self.is_valid(obj) {
            return;
        }
        self.modal = Some(obj);
        let cur = self.focused();
        let cur_in = cur.is_some_and(|f| self.focusable(f));
        if !cur_in {
            if let Some(idx) = self.group.iter().position(|&o| self.focusable(o)) {
                self.focus_to(idx);
            }
        }
    }

    /// 清除模态：恢复全局焦点导航（焦点保持当前对象）
    pub fn clear_modal(&mut self) {
        self.modal = None;
    }

    /// 有效隐藏：自身或任一祖先 HIDDEN
    fn is_hidden_eff(&self, obj: ObjRef) -> bool {
        let mut cur = Some(obj);
        while let Some(o) = cur {
            let Some(n) = self.arena.get(o) else { return false };
            if n.flags.contains(Flag::HIDDEN) {
                return true;
            }
            cur = n.parent;
        }
        false
    }
    fn focus_to(&mut self, idx: usize) {
        if self.focused_idx == Some(idx) {
            return;
        }
        if let Some(old) = self.focused() {
            self.set_state(old, State::FOCUSED, false);
            self.set_state(old, State::EDITED, false);
            self.send_event(old, crate::event::EventKind::Defocused);
        }
        self.focused_idx = Some(idx);
        if let Some(new) = self.focused() {
            self.set_state(new, State::FOCUSED, true);
            self.send_event(new, crate::event::EventKind::Focused);
        }
    }

    pub fn keypad_input(&mut self, key: crate::input::Key) {
        use crate::input::Key;
        let Some(f) = self.focused() else { return };
        if !self.is_valid(f) {
            return;
        }
        let edited = self.state(f).contains(State::EDITED);
        self.send_event(f, crate::event::EventKind::Key(key));
        if edited {
            let is_spinbox = matches!(self.arena.get(f).map(|n| &n.kind), Some(WidgetKind::Spinbox { .. }));
            if is_spinbox {
                // Spinbox 编辑态：Left/Right 选位，Up/Down 增减，Enter/Esc 退出
                enum Act {
                    Idle,
                    Set(i32),
                    Exit,
                }
                let act = if let Some(n) = self.arena.get_mut(f) {
                    if let WidgetKind::Spinbox { min, max, value, digits, cursor } = &mut n.kind {
                        match key {
                            Key::Left => {
                                crate::widgets::spinbox::move_cursor(*digits, cursor, -1);
                                Act::Idle
                            }
                            Key::Right => {
                                crate::widgets::spinbox::move_cursor(*digits, cursor, 1);
                                Act::Idle
                            }
                            Key::Up => {
                                let mut nv = *value;
                                crate::widgets::spinbox::step_digit(*min, *max, &mut nv, *digits, *cursor, 1);
                                Act::Set(nv)
                            }
                            Key::Down => {
                                let mut nv = *value;
                                crate::widgets::spinbox::step_digit(*min, *max, &mut nv, *digits, *cursor, -1);
                                Act::Set(nv)
                            }
                            Key::Enter | Key::Esc => Act::Exit,
                            _ => Act::Idle,
                        }
                    } else {
                        Act::Idle
                    }
                } else {
                    Act::Idle
                };
                match act {
                    Act::Set(v) => {
                        self.invalidate_obj(f);
                        self.set_value(f, v);
                    }
                    Act::Exit => self.set_state(f, State::EDITED, false),
                    Act::Idle => self.invalidate_obj(f),
                }
                return;
            }
            match key {
                Key::Left => { let v = self.value(f); self.set_value(f, v - 1); }
                Key::Right => { let v = self.value(f); self.set_value(f, v + 1); }
                Key::Enter | Key::Esc => self.set_state(f, State::EDITED, false),
                _ => {}
            }
            return;
        }
        let is_list = matches!(self.arena.get(f).map(|n| &n.kind), Some(WidgetKind::List { .. }));
        let is_roller = matches!(self.arena.get(f).map(|n| &n.kind), Some(WidgetKind::Roller { .. }));
        if is_roller {
            // Roller：Up/Down 滚轮选择（首尾停止）
            match key {
                Key::Up => { self.roller_step(f, -1); return; }
                Key::Down => { self.roller_step(f, 1); return; }
                _ => {}
            }
        }
        if is_list {
            match key {
                Key::Up => {
                    let cur = self.list_selected(f);
                    let n = self.list_len(f);
                    if n > 0 {
                        self.list_select(f, (cur + n - 1) % n);
                    }
                    return;
                }
                Key::Down => {
                    let cur = self.list_selected(f);
                    let n = self.list_len(f);
                    if n > 0 {
                        self.list_select(f, (cur + 1) % n);
                    }
                    return;
                }
                _ => {}
            }
        }
        match key {
            Key::Next | Key::Right | Key::Down => self.group_focus_next(),
            Key::Prev | Key::Left | Key::Up => self.group_focus_prev(),
            Key::Enter => self.activate(f),
            Key::Esc => {}
        }
    }

    pub fn list_len(&self, obj: ObjRef) -> usize {
        if let Some(n) = self.arena.get(obj) {
            if let WidgetKind::List { items, .. } = &n.kind {
                return items.len();
            }
        }
        0
    }

    /// 测试/调试用：返回对象 kind 的引用。不稳定 API。
    pub fn debug_kind(&self, obj: ObjRef) -> &WidgetKind {
        &self.arena.get(obj).expect("invalid ObjRef").kind
    }

    fn activate(&mut self, obj: ObjRef) {
        // 按控件类型分派
        let is_slider = matches!(self.arena.get(obj).map(|n| &n.kind), Some(WidgetKind::Slider { .. }));
        let is_switch = matches!(self.arena.get(obj).map(|n| &n.kind), Some(WidgetKind::Switch { .. }));
        let is_checkbox = matches!(self.arena.get(obj).map(|n| &n.kind), Some(WidgetKind::Checkbox { .. }));
        let is_dropdown = matches!(self.arena.get(obj).map(|n| &n.kind), Some(WidgetKind::Dropdown { .. }));
        let is_spinbox = matches!(self.arena.get(obj).map(|n| &n.kind), Some(WidgetKind::Spinbox { .. }));
        if is_slider || is_spinbox {
            self.set_state(obj, State::EDITED, true);
        } else if is_switch {
            self.toggle_switch(obj);
        } else if is_checkbox {
            self.toggle_checkbox(obj);
        } else if is_dropdown {
            self.open_dropdown(obj);
        } else {
            self.send_event(obj, crate::event::EventKind::Clicked);
        }
    }

    fn roller_step(&mut self, obj: ObjRef, dir: i32) {
        let now = self.time_ms;
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            if let WidgetKind::Roller { items, selected, sel_from } = &mut n.kind {
                let next = (*selected as i32 + dir).clamp(0, items.len().saturating_sub(1) as i32);
                crate::widgets::roller::select(items, selected, sel_from, next as usize, now);
            }
        }
        self.invalidate_obj(obj);
    }

    /// 打开 Dropdown 的浮层列表（Attach::Bottom 锚定，模态锁定）
    fn open_dropdown(&mut self, obj: ObjRef) {
        let Some((items, sel, w)) = self.arena.get(obj).map(|n| match &n.kind {
            WidgetKind::Dropdown { items, selected } => (items.clone(), *selected, n.rect.w),
            _ => (Vec::new(), 0, 0),
        }) else { return };
        if items.is_empty() {
            return;
        }
        let prev = self.focused();
        let screen = self.screen;
        let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
        let lst = self.create_list(screen, &refs);
        self.set_size(lst, w.max(80), (items.len().min(5) * 16 + 2) as i32);
        self.list_select(lst, sel);
        self.set_floating(lst, obj, crate::layout::Attach::Bottom);
        self.group_add(lst);
        self.set_modal(lst);
        // 选中：写回 dropdown 并发 ValueChanged，关闭浮层，还原焦点
        self.add_event_cb(lst, crate::event::EventKind::Clicked, Box::new(move |ui, l, _| {
            let idx = ui.list_selected(l);
            if let Some(n) = ui.arena.get_mut(obj) {
                if let WidgetKind::Dropdown { selected, .. } = &mut n.kind {
                    *selected = idx;
                }
            }
            ui.invalidate_obj(obj);
            ui.send_event(obj, crate::event::EventKind::ValueChanged);
            ui.clear_modal();
            ui.delete(l);
            if let Some(p) = prev {
                ui.group_focus(p);
            }
        }));
        // Esc：不改值，直接关闭
        self.add_event_cb(lst, crate::event::EventKind::Key(crate::input::Key::Esc), Box::new(move |ui, l, k| {
            if k == crate::event::EventKind::Key(crate::input::Key::Esc) {
                ui.clear_modal();
                ui.delete(l);
                if let Some(p) = prev {
                    ui.group_focus(p);
                }
            }
        }));
    }

    pub fn toggle_checkbox(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            if let WidgetKind::Checkbox { checked, .. } = &mut n.kind {
                *checked = !*checked;
            }
        }
        self.invalidate_obj(obj);
        self.send_event(obj, crate::event::EventKind::ValueChanged);
    }

    pub fn toggle_switch(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            if let WidgetKind::Switch { on } = &mut n.kind {
                *on = !*on;
            }
        }
        self.invalidate_obj(obj);
        self.send_event(obj, crate::event::EventKind::ValueChanged);
    }
}

/// Key 事件统一存储为占位值，匹配按类别通配（见 send_event）
fn stored_label(kind: crate::event::EventKind) -> crate::event::EventKind {
    match kind {
        crate::event::EventKind::Key(_) => crate::event::EventKind::Key(crate::input::Key::Enter),
        k => k,
    }
}
