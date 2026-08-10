use alloc::vec::Vec;
use crate::arena::{Arena, ObjRef};
use crate::geometry::Rect;
use crate::node::{Flag, Node, State};

/// The UI state: owns the widget tree (arena), dirty tracking, animation, focus, and rendering.
pub struct Ui {
    pub(crate) arena: Arena<Node>,
    screen: ObjRef,
    width: i32,
    height: i32,
    dirty: crate::dirty::DirtyQueue,
    flush: Option<alloc::boxed::Box<dyn crate::display::Flush>>,
    buf: Vec<crate::geometry::Color>,
    time_ms: u64,
    anims: Vec<crate::anim::RunningAnim>,
    group: Vec<ObjRef>,
    focused_idx: Option<usize>,
    pub(crate) layout_dirty: bool,
    modal: Option<ObjRef>,
    default_font: &'static embedded_graphics::mono_font::MonoFont<'static>,
}

impl Ui {
    /// Sets padding (l, r, t, b).
    pub fn set_pad(&mut self, obj: ObjRef, pad: (i32, i32, i32, i32)) {
        if let Some(n) = self.arena.get_mut(obj) { n.pad = pad; }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    /// Returns padding (l, r, t, b).
    pub fn pad(&self, obj: ObjRef) -> (i32, i32, i32, i32) {
        self.arena.get(obj).map(|n| n.pad).unwrap_or((0, 0, 0, 0))
    }

    /// Creates a UI for a `width` x `height` screen with a pixel buffer holding `buf_rows`
    /// scanlines (used for chunked rendering).
    pub fn new(width: i32, height: i32, buf_rows: u32) -> Ui {
        let mut arena = Arena::new();
        let screen = arena.insert(Node::new(None, Rect::new(0, 0, width, height), alloc::boxed::Box::new(crate::widgets::obj::Manual)));
        let mut dirty = crate::dirty::DirtyQueue::new(Rect::new(0, 0, width, height), 16);
        dirty.add(Rect::new(0, 0, width, height)); // build screen: dirty the full screen
        let buf = alloc::vec![crate::geometry::Color::BLACK; (width * buf_rows as i32).max(0) as usize];
        Ui { arena, screen, width, height, dirty, flush: None, buf, time_ms: 0, anims: Vec::new(), group: Vec::new(), focused_idx: None, layout_dirty: false, modal: None, default_font: crate::font::DEFAULT_FONT }
    }

    /// Sets the global default font (used by widgets that do not specify a `font` in style);
    /// dirties the whole screen.
    pub fn set_default_font(&mut self, font: &'static embedded_graphics::mono_font::MonoFont<'static>) {
        self.default_font = font;
        self.invalidate_area(Rect::new(0, 0, self.width, self.height));
        self.layout_dirty = true;
    }

    /// The current global default font.
    pub fn default_font(&self) -> &'static embedded_graphics::mono_font::MonoFont<'static> {
        self.default_font
    }

    /// Sets the overlay draw hook (`None` clears it). Drawn on top of the widget's own content.
    pub fn set_draw_hook(&mut self, obj: ObjRef, hook: Option<crate::node::DrawHook>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.draw_hook = hook;
        }
        self.invalidate_obj(obj);
    }

    /// Sets the per-frame hook (`None` clears it). Frames that return `true` dirty the object
    /// and keep the timer handler awake.
    pub fn set_tick_hook(&mut self, obj: ObjRef, hook: Option<crate::node::TickHook>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.tick_hook = hook;
        }
    }

    /// Returns the handle of the screen root object.
    pub fn screen(&self) -> ObjRef {
        self.screen
    }

    /// Returns `true` if `obj` is a live object in the tree.
    pub fn is_valid(&self, obj: ObjRef) -> bool {
        self.arena.contains(obj)
    }

    /// Mounts a user-defined widget (implementing `widgets::Widget`). This is the
    /// same insertion path built-in widgets use: user widgets are first-class.
    pub fn create_widget(&mut self, parent: ObjRef, w: i32, h: i32, widget: alloc::boxed::Box<dyn crate::widgets::Widget>) -> ObjRef {
        self.insert_node(parent, Rect::new(0, 0, w, h), widget)
    }

    /// Read-only access to widget state by type (returns `None` on type mismatch).
    pub fn widget<T: 'static>(&self, obj: ObjRef) -> Option<&T> {
        self.arena.get(obj)?.kind.as_any().downcast_ref::<T>()
    }

    /// Read-only access to the List state (returns `None` for non-List objects).
    pub fn as_list(&self, obj: ObjRef) -> Option<&crate::widgets::list::ListState> {
        self.widget::<crate::widgets::list::ListState>(obj)
    }
    /// Read-only access to the Roller state (returns `None` for non-Roller objects).
    pub fn as_roller(&self, obj: ObjRef) -> Option<&crate::widgets::roller::RollerState> {
        self.widget::<crate::widgets::roller::RollerState>(obj)
    }

    pub(crate) fn kind_mut(&mut self, obj: ObjRef) -> Option<&mut alloc::boxed::Box<dyn crate::widgets::Widget>> {
        self.arena.get_mut(obj).map(|n| &mut n.kind)
    }

    /// Unified entry point for the built-in widget extension APIs: runs `f` when the type
    /// matches and dirties the object,
    /// returning `f`'s result; invalid objects or type mismatches silently return `None`.
    pub fn update<T: 'static, R>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let r = match self.arena.get_mut(obj) {
            Some(n) => n.kind.as_any_mut().downcast_mut::<T>().map(f),
            None => None,
        };
        if r.is_some() {
            self.invalidate_obj(obj);
        }
        r
    }

    /// Deletes `obj` and its whole subtree, releasing focus-group membership and the modal
    /// lock if the modal subtree is removed.
    pub fn delete(&mut self, obj: ObjRef) {
        if obj == self.screen || !self.is_valid(obj) {
            return;
        }
        self.invalidate_obj(obj);
        // Collect the subtree bottom-up first
        let mut stack = alloc::vec![obj];
        let mut all = Vec::new();
        while let Some(r) = stack.pop() {
            if let Some(n) = self.arena.get(r) {
                stack.extend_from_slice(&n.children);
                all.push(r);
            }
        }
        // Unlink from the parent
        if let Some(n) = self.arena.get(obj)
            && let Some(p) = n.parent
            && let Some(pn) = self.arena.get_mut(p)
        {
            pn.children.retain(|&c| c != obj);
        }
        for r in all.clone() {
            if self.modal == Some(r) {
                self.modal = None; // modal subtree deleted: release the modal lock
            }
            self.arena.remove(r);
        }
        // Remove from the focus group too
        for r in all {
            self.group_remove(r);
        }
        self.layout_dirty = true;
    }

    /// Returns the direct children of `obj`.
    pub fn children(&self, obj: ObjRef) -> Vec<ObjRef> {
        self.arena.get(obj).map(|n| n.children.clone()).unwrap_or_default()
    }

    /// Moves a child within its parent's order (triggers a layout pass; pairs well with
    /// `transition` for smooth reordering).
    pub fn move_child_to_index(&mut self, obj: ObjRef, index: usize) {
        let Some(parent) = self.arena.get(obj).and_then(|n| n.parent) else { return };
        if let Some(p) = self.arena.get_mut(parent)
            && let Some(pos) = p.children.iter().position(|&c| c == obj)
        {
            let c = p.children.remove(pos);
            let idx = index.min(p.children.len());
            p.children.insert(idx, c);
        }
        self.layout_dirty = true;
    }

    /// Returns the local rect of `obj`.
    pub fn rect(&self, obj: ObjRef) -> Rect {
        self.arena.get(obj).map(|n| n.rect).unwrap_or_default()
    }

    /// Returns the absolute screen rect of `obj`.
    pub fn abs_rect(&self, obj: ObjRef) -> Rect {
        crate::render::abs_rect(&self.arena, obj)
    }

    /// Sets the visual translation offset (mirrors LVGL translate_x/y): shifts the whole
    /// subtree visually; affects rendering only, not layout.
    pub fn set_translate(&mut self, obj: ObjRef, x: i32, y: i32) {
        self.invalidate_subtree(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.translate = crate::geometry::Point { x, y };
        }
        self.invalidate_subtree(obj);
    }

    /// Dirties the render area of the whole subtree (children move when the translate changes).
    /// Each node expands by its widget type's draw overflow (knobs, etc.).
    /// Effectively hidden subtrees produce no dirty area (re-shown via `set_hidden`, which dirties).
    fn invalidate_subtree(&mut self, obj: ObjRef) {
        if !self.is_valid(obj) || self.is_hidden_eff(obj) {
            return;
        }
        let mut stack = alloc::vec![obj];
        let mut area: Option<Rect> = None;
        while let Some(r) = stack.pop() {
            let ext = self.arena.get(r).map(|n| n.kind.overflow()).unwrap_or(0);
            let a = self.abs_rect(r);
            let a = Rect::new(a.x - ext, a.y - ext, a.w + 2 * ext, a.h + 2 * ext);
            area = Some(match area {
                None => a,
                Some(u) => u.union(&a),
            });
            for c in self.children(r) {
                stack.push(c);
            }
        }
        if let Some(a) = area {
            self.invalidate_area(a);
        }
    }

    /// Returns the visual translation offset of `obj`.
    pub fn translate(&self, obj: ObjRef) -> crate::geometry::Point {
        self.arena.get(obj).map(|n| n.translate).unwrap_or_default()
    }

    /// Sets the object position (local coordinates). Note: this does not trigger a layout
    /// pass —position is layout output, not input;
    /// children managed by Flex/Grid own their positions (the next layout pass overwrites them),
    /// so use `set_translate` for visual displacement.
    /// Dirties the whole subtree: children's screen coordinates follow the parent's move.
    pub fn set_pos(&mut self, obj: ObjRef, x: i32, y: i32) {
        self.invalidate_subtree(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.x = x;
            n.rect.y = y;
        }
        self.invalidate_subtree(obj);
    }

    /// Sets the object size. Dirties the whole subtree (child coordinates/clipping may
    /// change with the parent).
    pub fn set_size(&mut self, obj: ObjRef, w: i32, h: i32) {
        self.invalidate_subtree(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.rect.w = w;
            n.rect.h = h;
        }
        self.invalidate_subtree(obj);
        self.layout_dirty = true;
    }

    /// Marks `rect` as needing a repaint.
    pub fn invalidate_area(&mut self, rect: Rect) {
        self.dirty.add(rect);
    }
    /// Dirties the object's area (expanded by the widget's draw overflow).
    /// Effectively hidden objects produce no dirty area (re-shown via `set_hidden`, which dirties).
    pub fn invalidate_obj(&mut self, obj: ObjRef) {
        if self.is_valid(obj) && !self.is_hidden_eff(obj) {
            let ext = self.arena.get(obj).map(|n| n.kind.overflow()).unwrap_or(0);
            let r = self.abs_rect(obj);
            // A widget may draw outside its own rect (knobs, etc.), so expand the dirty area
            self.dirty.add(Rect::new(r.x - ext, r.y - ext, r.w + 2 * ext, r.h + 2 * ext));
        }
    }
    /// Takes all pending dirty rects.
    pub fn take_dirty(&mut self) -> Vec<Rect> {
        self.dirty.take()
    }
    /// Returns `true` if there are no pending dirty rects.
    pub fn dirty_is_empty(&self) -> bool {
        self.dirty.is_empty()
    }

    /// Shows or hides `obj` (dirties the old area when hiding and the new area when showing).
    pub fn set_hidden(&mut self, obj: ObjRef, hidden: bool) {
        if hidden {
            self.invalidate_obj(obj); // dirty before setting: erase the object's old area
        }
        if let Some(n) = self.arena.get_mut(obj) {
            n.flags.set(Flag::HIDDEN, hidden);
        }
        if !hidden {
            self.invalidate_obj(obj); // dirty after showing: repaint the object
        }
        self.layout_dirty = true;
    }

    /// Replaces the base style of `obj`.
    pub fn set_style(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style = style;
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    /// Replaces the pressed-state overlay style of `obj`.
    pub fn set_style_pressed(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_pressed = Some(alloc::boxed::Box::new(style));
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    /// Replaces the focused-state overlay style of `obj`.
    pub fn set_style_focused(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_focused = Some(alloc::boxed::Box::new(style));
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    /// Replaces the selected-state overlay style of `obj`.
    pub fn set_style_selected(&mut self, obj: ObjRef, style: crate::style::Style) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style_selected = Some(alloc::boxed::Box::new(style));
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    /// Sets (`on` = true) or clears a state flag on `obj`.
    pub fn set_state(&mut self, obj: ObjRef, state: State, on: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.state.set(state, on);
        }
        self.invalidate_obj(obj);
        self.layout_dirty = true;
    }
    /// Returns the current state flags of `obj`.
    pub fn state(&self, obj: ObjRef) -> State {
        self.arena.get(obj).map(|n| n.state).unwrap_or_default()
    }
    /// Returns the fully resolved style of `obj`.
    pub fn resolved_style(&self, obj: ObjRef) -> crate::style::ResolvedStyle {
        crate::render::resolved_style(&self.arena, obj, self.default_font)
    }

    /// Sets the display flush callback.
    pub fn set_flush(&mut self, f: alloc::boxed::Box<dyn crate::display::Flush>) {
        self.flush = Some(f);
    }

    /// Advances the internal clock by `ms` milliseconds.
    pub fn tick_inc(&mut self, ms: u32) {
        self.time_ms += ms as u64;
    }
    /// Returns the current internal time in milliseconds.
    pub fn time(&self) -> u64 {
        self.time_ms
    }

    /// Starts an animation (an existing animation on the same target/property is replaced).
    pub fn anim_start(&mut self, a: crate::anim::Anim) {
        // The old animation on the same target/property is replaced (mirrors LVGL semantics)
        self.anim_stop(a.target, a.prop);
        // Apply the start value immediately to avoid a jump
        self.apply_anim_value(a.target, a.prop, a.start);
        self.anims.push(crate::anim::RunningAnim { anim: a, start_time: self.time_ms });
    }
    /// Stops any animation running on `target`'s `prop`.
    pub fn anim_stop(&mut self, target: ObjRef, prop: crate::anim::AnimProp) {
        self.anims.retain(|r| !(r.anim.target == target && r.anim.prop == prop));
    }
    /// Returns `true` while any animation is in flight.
    pub fn anim_running(&self) -> bool {
        !self.anims.is_empty()
    }

    /// Advances the whole UI by one frame: steps animations, lays out, ticks widgets, and
    /// renders. Returns the suggested next timer delay in ms (0 = keep waking, `u32::MAX` = idle).
    pub fn timer_handler(&mut self) -> u32 {
        self.step_anims();
        if self.layout_dirty {
            self.layout_pass();
            self.layout_dirty = false;
        }
        // Floating layers are repositioned every frame (following the target's movement/animation;
        // no cost when the position is unchanged)
        self.layout_floating(self.screen);
        let fx_active = self.tick_widgets();
        self.render();
        if self.anim_running() || fx_active { 0 } else { u32::MAX }
    }

    /// Walks the object tree advancing per-frame effects (fx/Spinner/tick_hook), dirtying
    /// active nodes.
    /// Returns whether any effect is still active (whether `timer_handler` keeps waking).
    fn tick_widgets(&mut self) -> bool {
        let now = self.time_ms;
        let mut any = false;
        let mut stack = alloc::vec![self.screen];
        while let Some(r) = stack.pop() {
            // HIDDEN subtrees are skipped wholesale: since pruned ancestors guarantee every
            // visited node's ancestors are visible,
            // only the node's own flag needs checking. Hidden nodes are not ticked, not
            // dirtied, and count as inactive.
            let hidden = self.arena.get(r).map(|n| n.flags.contains(Flag::HIDDEN)).unwrap_or(false);
            if hidden {
                continue;
            }
            let mut taken = match self.arena.get_mut(r) {
                Some(n) => core::mem::replace(&mut n.kind, alloc::boxed::Box::new(crate::widgets::NoopWidget)),
                None => continue,
            };
            let out = taken.tick(self, r, now);
            let children = match self.arena.get_mut(r) {
                Some(n) => {
                    let c = n.children.clone();
                    n.kind = taken;
                    c
                }
                None => continue, // node deleted during tick
            };
            let has_hook = self.arena.get(r).map(|n| n.tick_hook.is_some()).unwrap_or(false);
            if out.redraw {
                self.invalidate_obj(r);
            }
            if out.active {
                any = true;
            }
            if has_hook {
                // take-call-put-back: the hook signature includes `&mut Ui`
                let mut hook = self.arena.get_mut(r).and_then(|n| n.tick_hook.take());
                if let Some(h) = hook.as_mut()
                    && h(self, r, now)
                {
                    any = true;
                    self.invalidate_obj(r);
                }
                if let Some(n) = self.arena.get_mut(r) {
                    n.tick_hook = hook;
                }
            }
            stack.extend_from_slice(&children);
        }
        any
    }

    pub(crate) fn layout_pass(&mut self) {
        let screen = self.screen;
        self.layout_subtree(screen);
    }

    /// Forces a full layout pass, bypassing the `layout_dirty` flag.
    ///
    /// Intended as a benchmark hook so tools can time the layout phase in
    /// isolation (see `docs/superpowers/specs/2026-08-07-runtime-bench-design.md`).
    #[doc(hidden)]
    pub fn layout(&mut self) {
        self.layout_pass();
    }

    /// Floating-layer positioning (pre-order traversal so anchor chains resolve in tree order;
    /// no dirtying when the position is unchanged).
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
            // Convert to the parent's local coordinates (relative to the parent's abs origin)
            let pabs = self.arena.get(obj).and_then(|n| n.parent).map(|p| self.abs_rect(p));
            let (px, py) = pabs.map(|p| (p.x, p.y)).unwrap_or((0, 0));
            let (nx, ny) = (dx - px, dy - py);
            let cur = self.rect(obj);
            if cur.x != nx || cur.y != ny {
                self.set_pos(obj, nx, ny);
            }
        }
        let nkids = self.arena.get(obj).map(|n| n.children.len()).unwrap_or(0);
        for i in 0..nkids {
            let Some(c) = self.arena.get(obj).and_then(|n| n.children.get(i).copied()) else { break };
            self.layout_floating(c);
        }
    }
    fn layout_subtree(&mut self, obj: ObjRef) {
        // Content box passed to the layout algorithm. Node rects are parent-LOCAL,
        // and layout algorithms place children in that same local space, so the
        // content origin is just the pad offsets (the node's own x/y must NOT be
        // added). w/h may go negative on pad overflow, exactly as the per-algorithm
        // padding math did before the hoist.
        let content = {
            let (r, pad) = match self.arena.get(obj) {
                Some(n) => (n.rect, n.pad),
                None => return,
            };
            crate::geometry::Rect::new(pad.0, pad.2, r.w - pad.0 - pad.1, r.h - pad.2 - pad.3)
        };
        let mut kind = match self.arena.get_mut(obj) {
            Some(n) => core::mem::replace(&mut n.kind, alloc::boxed::Box::new(crate::widgets::NoopWidget)),
            None => return,
        };
        kind.layout(self, obj, content);
        if let Some(n) = self.arena.get_mut(obj) {
            n.kind = kind;
        } else {
            return; // node deleted during layout
        }
        // Iterate children by index: cloning the child Vec per node per pass
        // showed up in the layout profile.
        let nkids = self.arena.get(obj).map(|n| n.children.len()).unwrap_or(0);
        for i in 0..nkids {
            let Some(c) = self.arena.get(obj).and_then(|n| n.children.get(i).copied()) else { break };
            self.layout_subtree(c);
        }
    }

    pub fn grid_cell(&self, obj: ObjRef) -> ((u8, u8), (u8, u8)) {
        match self.arena.get(obj).map(|n| &n.item_props.specific) {
            Some(crate::node::ItemSpecific::Grid { col, row }) => (*col, *row),
            _ => ((0, 1), (0, 1)),
        }
    }
    pub fn set_grid_cell(&mut self, obj: ObjRef, col: (u8, u8), row: (u8, u8)) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.item_props.specific = crate::node::ItemSpecific::Grid { col: (col.0, col.1.max(1)), row: (row.0, row.1.max(1)) };
        }
        self.layout_dirty = true;
    }

    /// Replaces the node's widget kind with a flex layout (runtime layout change).
    /// WARNING: this discards the existing widget kind/state (e.g. calling it on a Label destroys the LabelState).
    pub fn set_flex(&mut self, obj: ObjRef, flex: crate::layout::Flex) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.kind = alloc::boxed::Box::new(crate::widgets::flexbox::FlexLayout { flex });
        }
        self.layout_dirty = true;
    }
    /// Replaces the node's widget kind with a grid layout.
    /// WARNING: this discards the existing widget kind/state (e.g. calling it on a Label destroys the LabelState).
    pub fn set_grid(&mut self, obj: ObjRef, grid: crate::layout::Grid) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.kind = alloc::boxed::Box::new(crate::widgets::gridbox::GridLayout { grid });
        }
        self.layout_dirty = true;
    }

    /// Sets the width/height sizing strategies (None = content size).
    pub fn set_sizing(&mut self, obj: ObjRef, w: Option<crate::layout::Sizing>, h: Option<crate::layout::Sizing>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.item_props.sizing_w = w;
            n.item_props.sizing_h = h;
        }
        self.layout_dirty = true;
    }

    /// Sets the aspect ratio (per-mille: 1000 = 1:1, 1778 ≈16:9; None clears it).
    pub fn set_aspect(&mut self, obj: ObjRef, ratio: Option<u32>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.item_props.aspect_ratio = ratio;
        }
        self.layout_dirty = true;
    }

    /// Attaches third-party layout constraints to `obj` (replaces any existing `specific`).
    pub fn set_item_custom(&mut self, obj: ObjRef, props: alloc::boxed::Box<dyn core::any::Any>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.item_props.specific = crate::node::ItemSpecific::Custom(props);
        }
        self.layout_dirty = true;
    }
    /// Read-only access to a child's third-party layout constraints by type.
    pub fn item_custom<T: 'static>(&self, obj: ObjRef) -> Option<&T> {
        match &self.arena.get(obj)?.item_props.specific {
            crate::node::ItemSpecific::Custom(p) => p.downcast_ref::<T>(),
            _ => None,
        }
    }
    /// Mutates a child's third-party layout constraints (dirties layout on success).
    pub fn update_item_custom<T: 'static, R>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let r = match self.arena.get_mut(obj) {
            Some(n) => match &mut n.item_props.specific {
                crate::node::ItemSpecific::Custom(p) => p.downcast_mut::<T>().map(f),
                _ => None,
            },
            None => None,
        };
        if r.is_some() { self.layout_dirty = true; }
        r
    }

    /// Sets the layout transition: (duration ms, easing). Position/size changes from layout
    /// animate automatically; None disables it.
    pub fn set_transition(&mut self, obj: ObjRef, transition: Option<(u32, crate::anim::Easing)>) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.transition = transition;
        }
        self.layout_dirty = true;
    }

    /// Is there already an animation heading to the same target value for this property?
    /// (Avoids restarting transition animations on repeated layout passes.)
    fn anim_end_for(&self, target: ObjRef, prop: crate::anim::AnimProp) -> Option<i32> {
        self.anims
            .iter()
            .find(|r| r.anim.target == target && r.anim.prop == prop)
            .map(|r| r.anim.end)
    }

    /// Layout write for position: animates to the target when transitions are on and it is not
    /// the first layout, otherwise moves instantly.
    pub(crate) fn layout_move(&mut self, obj: ObjRef, x: i32, y: i32) {
        let Some(n) = self.arena.get(obj) else { return };
        let laid = n.laid_out;
        let cur = n.rect;
        let tr = self.arena.get(obj).and_then(|n| n.transition);
        let mut animated = false;
        if laid
            && (cur.x != x || cur.y != y)
            && let Some((dur, easing)) = tr
            && dur > 0
        {
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
        if !animated && (cur.x != x || cur.y != y) {
            self.set_pos(obj, x, y);
        }
        if let Some(n) = self.arena.get_mut(obj) {
            n.laid_out = true;
        }
    }

    /// Layout write for size: animates to the target when transitions are on and it is not
    /// the first layout, otherwise resizes instantly.
    pub(crate) fn layout_resize(&mut self, obj: ObjRef, w: i32, h: i32) {
        let Some(n) = self.arena.get(obj) else { return };
        let laid = n.laid_out;
        let cur = n.rect;
        let tr = self.arena.get(obj).and_then(|n| n.transition);
        let mut animated = false;
        if laid
            && (cur.w != w || cur.h != h)
            && let Some((dur, easing)) = tr
            && dur > 0
        {
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
        if !animated && (cur.w != w || cur.h != h) {
            self.set_size(obj, w, h);
        }
        // laid_out is set uniformly by layout_move (the two are always called in pairs)
    }
    /// Returns `true` if `obj` has the HIDDEN flag.
    pub fn is_hidden(&self, obj: ObjRef) -> bool {
        self.arena.get(obj).map(|n| n.flags.contains(Flag::HIDDEN)).unwrap_or(false)
    }

    /// Anchors `obj` as a floating layer: it becomes floating (IGNORE_LAYOUT), and its
    /// position is computed automatically and follows the target.
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

    /// Clears the floating anchor (the IGNORE_LAYOUT flag is kept; clear it manually if desired).
    pub fn clear_floating(&mut self, obj: ObjRef) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.floating = None;
        }
        self.layout_dirty = true;
    }

    /// Moves `obj` to the end of its parent's children (drawn last = on top).
    pub fn move_to_front(&mut self, obj: ObjRef) {
        let Some(parent) = self.arena.get(obj).and_then(|n| n.parent) else { return };
        if let Some(p) = self.arena.get_mut(parent)
            && let Some(pos) = p.children.iter().position(|&c| c == obj)
        {
            let c = p.children.remove(pos);
            p.children.push(c);
        }
        self.invalidate_obj(obj);
    }

    /// Moves `obj` to the start of its parent's children (drawn first = bottom).
    pub fn move_to_back(&mut self, obj: ObjRef) {
        let Some(parent) = self.arena.get(obj).and_then(|n| n.parent) else { return };
        if let Some(p) = self.arena.get_mut(parent)
            && let Some(pos) = p.children.iter().position(|&c| c == obj)
        {
            let c = p.children.remove(pos);
            p.children.insert(0, c);
        }
        self.invalidate_obj(obj);
    }

    /// Sets the node opacity multiplier (0..=255) via the base style.
    pub fn set_opa(&mut self, obj: ObjRef, opa: u8) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) { n.style.opa = Some(opa); }
        self.invalidate_obj(obj);
    }

    /// Sets viewport clipping: the subtree is drawn clipped to this object's rect (mirrors
    /// LVGL's clip content, for scroll containers).
    pub fn set_clip_children(&mut self, obj: ObjRef, clip: bool) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.flags.set(Flag::CLIP_CHILDREN, clip);
        }
        self.invalidate_obj(obj);
    }

    /// Sets/queries the ignore-layout flag: floating objects are excluded from the parent
    /// container's layout (for popups/overlays).
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
    /// Returns `true` if `obj` has the IGNORE_LAYOUT flag.
    pub fn is_ignore_layout(&self, obj: ObjRef) -> bool {
        self.arena.get(obj).map(|n| n.flags.contains(Flag::IGNORE_LAYOUT)).unwrap_or(false)
    }

    fn step_anims(&mut self) {
        let now = self.time_ms;
        let mut i = 0;
        while i < self.anims.len() {
            let target = self.anims[i].anim.target;
            if !self.is_valid(target) {
                self.anims.remove(i); // target deleted: clean up the animation
                continue;
            }
            let ev = { let r = &self.anims[i]; crate::anim::eval(&r.anim, r.start_time, now) };
            match ev {
                crate::anim::AnimEval::Delay => i += 1,
                crate::anim::AnimEval::Keep(v) => {
                    let prop = self.anims[i].anim.prop;
                    self.apply_anim_value(target, prop, v);
                    i += 1;
                }
                crate::anim::AnimEval::Done(v) => {
                    let mut r = self.anims.remove(i);
                    self.apply_anim_value(r.anim.target, r.anim.prop, v);
                    if let Some(mut cb) = r.anim.on_done.take() {
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
                    n.style.opa = Some(v.clamp(0, 255) as u8);
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

    /// Renders all pending dirty areas to the flush callback.
    pub fn render(&mut self) {
        crate::render::render(
            self.screen,
            &mut self.arena,
            &mut self.buf,
            &mut self.dirty,
            &mut self.flush,
            self.default_font,
            self.time_ms,
        );
    }

    pub(crate) fn insert_node(&mut self, parent: ObjRef, rect: Rect, kind: alloc::boxed::Box<dyn crate::widgets::Widget>) -> ObjRef {
        let r = self.arena.insert(Node::new(Some(parent), rect, kind));
        if let Some(p) = self.arena.get_mut(parent) {
            p.children.push(r);
        }
        self.invalidate_obj(r);
        self.layout_dirty = true;
        r
    }

    /// Sets the widget's value (sends `ValueChanged` if it actually changed).
    pub fn set_value(&mut self, obj: ObjRef, v: i32) {
        self.invalidate_value_area(obj);
        let changed = match self.arena.get_mut(obj) {
            Some(n) => n.kind.set_value(v),
            None => false,
        };
        self.invalidate_value_area(obj);
        if changed {
            self.send_event(obj, crate::event::EventKind::ValueChanged);
        }
    }

    /// Area dirtied when the value changes (`invalidate_obj` already expands per widget type).
    fn invalidate_value_area(&mut self, obj: ObjRef) {
        self.invalidate_obj(obj);
    }

    /// Returns the widget's current value.
    pub fn value(&self, obj: ObjRef) -> i32 {
        self.arena.get(obj).map(|n| n.kind.value()).unwrap_or(0)
    }

    /// Sets the widget's value range.
    pub fn set_range(&mut self, obj: ObjRef, min: i32, max: i32) {
        self.invalidate_obj(obj);
        if let Some(n) = self.arena.get_mut(obj) {
            n.kind.set_range(min, max);
        }
        self.invalidate_obj(obj);
    }

    /// Registers an event callback on `obj`.
    pub fn add_event_cb(&mut self, obj: ObjRef, kind: crate::event::EventKind, cb: crate::event::EventCb) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.events.push((kind, cb));
        }
    }

    /// Delivers `kind` to every matching callback registered on `obj`.
    pub fn send_event(&mut self, obj: ObjRef, kind: crate::event::EventKind) {
        use crate::event::EventKind;
        let mut cursor = 0usize;
        loop {
            // Find and take the next matching callback (removed from the arena first so a
            // callback's `&mut Ui` does not conflict)
            let taken = {
                let Some(n) = self.arena.get_mut(obj) else { return };
                let mut found = None;
                let mut i = cursor;
                while i < n.events.len() {
                    let matches = match (&n.events[i].0, &kind) {
                        (EventKind::Key(_), EventKind::Key(_)) => true, // Key wildcards by category
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
            // Put it back (the object may have been deleted by the callback; callbacks
            // registered during a callback are not triggered this round)
            if let Some(n) = self.arena.get_mut(obj) {
                let idx = cursor.min(n.events.len());
                n.events.insert(idx, (stored_label(kind), cb));
            } else {
                return;
            }
            cursor += 1;
        }
    }

    /// Adds `obj` to the focus group (focusing it if the group was empty).
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
    /// Removes `obj` from the focus group, moving focus to a neighbor when needed.
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
            } else if let Some(fi) = self.focused_idx
                && pos < fi
            {
                self.focused_idx = Some(fi - 1);
            }
        }
    }
    /// Returns the currently focused object, if any.
    pub fn focused(&self) -> Option<ObjRef> {
        self.focused_idx.and_then(|i| self.group.get(i).copied())
    }
    /// Focuses `obj` (it must be in the focus group).
    pub fn group_focus(&mut self, obj: ObjRef) {
        if let Some(pos) = self.group.iter().position(|&o| o == obj) {
            self.focus_to(pos);
        }
    }
    /// Moves focus to the next focusable object in the group.
    pub fn group_focus_next(&mut self) {
        if let Some(i) = crate::focus::step(&self.group, self.focused_idx, 1, |o| self.focusable(o)) {
            self.focus_to(i);
        }
    }
    /// Moves focus to the previous focusable object in the group.
    pub fn group_focus_prev(&mut self) {
        if let Some(i) = crate::focus::step(&self.group, self.focused_idx, -1, |o| self.focusable(o)) {
            self.focus_to(i);
        }
    }
    /// Focusable: not effectively hidden, and inside the modal subtree (anywhere when no modal
    /// is set).
    fn focusable(&self, obj: ObjRef) -> bool {
        if self.is_hidden_eff(obj) {
            return false;
        }
        let Some(m) = self.modal else { return true };
        // obj == modal, or obj is a descendant of modal
        let mut cur = Some(obj);
        while let Some(o) = cur {
            if o == m {
                return true;
            }
            cur = self.arena.get(o).and_then(|n| n.parent);
        }
        false
    }

    /// Sets a modal object: focus navigation locks to its subtree, and focus moves inside it.
    pub fn set_modal(&mut self, obj: ObjRef) {
        if !self.is_valid(obj) {
            return;
        }
        self.modal = Some(obj);
        let cur = self.focused();
        let cur_in = cur.is_some_and(|f| self.focusable(f));
        if !cur_in
            && let Some(idx) = self.group.iter().position(|&o| self.focusable(o))
        {
            self.focus_to(idx);
        }
    }

    /// Clears the modal: restores global focus navigation (focus stays on the current object).
    pub fn clear_modal(&mut self) {
        self.modal = None;
    }

    /// Effectively hidden: self or any ancestor has HIDDEN.
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

    /// Routes a key to the focused widget: sends the Key event, then the widget's `on_key`;
    /// unconsumed keys fall through to focus navigation / Clicked.
    pub fn keypad_input(&mut self, key: crate::input::Key) {
        use crate::input::Key;
        let Some(f) = self.focused() else { return };
        if !self.is_valid(f) {
            return;
        }
        self.send_event(f, crate::event::EventKind::Key(key));
        if !self.is_valid(f) {
            return; // the Key callback may have deleted the focused object
        }
        if self.call_on_key(f, key) {
            return;
        }
        // Default: keys not consumed by a widget drive focus navigation / Clicked
        match key {
            Key::Next | Key::Right | Key::Down => self.group_focus_next(),
            Key::Prev | Key::Left | Key::Up => self.group_focus_prev(),
            Key::Enter => self.send_event(f, crate::event::EventKind::Clicked),
            Key::Esc => {}
        }
    }

    /// Widget key handling: takes the kind out, calls its `on_key` with `&mut Ui`,
    /// puts it back, then runs the common side effects.
    fn call_on_key(&mut self, obj: ObjRef, key: crate::input::Key) -> bool {
        let mut kind = match self.arena.get_mut(obj) {
            Some(n) => core::mem::replace(&mut n.kind, alloc::boxed::Box::new(crate::widgets::NoopWidget)),
            None => return false,
        };
        let out = kind.on_key(self, obj, key);
        if let Some(n) = self.arena.get_mut(obj) {
            n.kind = kind;
        } else {
            return true; // the node was deleted during handling: treat as consumed
        }
        self.apply_key_outcome(obj, out)
    }

    fn apply_key_outcome(&mut self, obj: ObjRef, out: crate::widgets::KeyOutcome) -> bool {
        use crate::widgets::KeyOutcome;
        match out {
            KeyOutcome::Pass => false,
            KeyOutcome::Consumed => {
                self.invalidate_obj(obj);
                true
            }
            KeyOutcome::ValueChanged => {
                self.invalidate_obj(obj);
                self.send_event(obj, crate::event::EventKind::ValueChanged);
                true
            }
            KeyOutcome::EnterEdit => {
                self.set_state(obj, State::EDITED, true);
                true
            }
            KeyOutcome::ExitEdit => {
                self.set_state(obj, State::EDITED, false);
                self.invalidate_obj(obj);
                true
            }
        }
    }

}

/// Key events are stored as a placeholder value; matching wildcards by category (see `send_event`).
fn stored_label(kind: crate::event::EventKind) -> crate::event::EventKind {
    match kind {
        crate::event::EventKind::Key(_) => crate::event::EventKind::Key(crate::input::Key::Enter),
        k => k,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_runs_flex_pass() {
        use crate::layout::{Align, Flex, FlexDir};
        use crate::widgets::label::LabelCfg;
        use crate::widgets::obj::ObjCfg;
        use crate::widgets::Layout;
        let mut ui = Ui::new(320, 240, 24);
        let scr = ui.screen();
        let container = ObjCfg::new()
            .size(320, 240)
            .layout(Layout::Flex(Flex {
                dir: FlexDir::Column, wrap: false,
                main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
            }))
            .build(&mut ui, scr);
        let a = LabelCfg::new("A").size(10, 10).build(&mut ui, container);
        let b = LabelCfg::new("B").size(10, 10).build(&mut ui, container);
        ui.layout();
        let (ra, rb) = (ui.rect(a), ui.rect(b));
        assert!(rb.y > ra.y, "B should be below A in a column flex (a.y={} b.y={})", ra.y, rb.y);
    }
}
