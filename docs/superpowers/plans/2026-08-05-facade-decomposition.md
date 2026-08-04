# Ui Facade 分解（纯子系统）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 qingui 的 renderer/anim 插值/focus 簿记抽为纯自由函数（脱离 Ui 用 fixture arena 单测），Ui 变薄成协调者。

**Architecture:** 纯子系统以自由函数 + 显式不相交参数的形式抽到 `src/render.rs`、`src/anim.rs`、`src/focus.rs`；Ui 方法用 split borrow 委托（`&mut self.arena` 与 `&mut self.buf` 等不相交字段一次调用共存）。layout 引擎保持 `&mut Ui` 不动，只给已为纯函数的私有数学补单测。凡派发用户回调的路径（`send_event`/`tick_widgets`/`step_anims` 驱动器/`call_on_key`）保持现状。

**Tech Stack:** Rust (no_std + alloc), `cargo test`（集成测试在 `qingui/tests/`，新增模块内 `#[cfg(test)]` 单元测试）。

## Global Constraints

- **行为保持重构**：不改变任何公开 API 与可观察行为。现有 187 集成测试即回归契约，任务完成后必须全绿。
- **公开签名零变化**：`render`/`abs_rect`/`resolved_style`/`group_focus_next`/`group_focus_prev` 的签名与语义不变（Ui 方法保留，仅内部实现委托）。
- **纯子系统函数一律 `pub(crate)`**；单元测试用 `#[cfg(test)] mod tests`（模块内可访问私有函数），**测试代码不构造 `Ui`**，直接 `Arena<Node>` + `Node` fixture。`ObjRef { index, generation }` 字段 pub，测试可直接构造假句柄。
- **不引入 ctx struct**；参数用显式不相交引用。若 split borrow 在某处编译不过，退回"impl Ui 分文件"而非 ctx（YAGNI）。
- **签名修正（相对 spec）**：`render` 需要额外 `screen: ObjRef` 参数（`render_chunk` 要画 screen 的子对象）；`render_area` 需要 `flush` 参数（传给 `render_chunk`）。
- `no_std + alloc` 保持。测试代码用 `alloc::cell::RefCell`/`alloc::rc::Rc`（无需 std）。
- **git**：只本地 commit，不 push。每个 commit 只暂存本任务文件。
- **验证命令**：`cargo test -p qingui`；`cargo build -p qingui --target thumbv7em-none-eabihf`；`cargo check -p qingui --examples`。
- **行号会漂移**：所有"删除/替换 ui.rs 第 N 行"按**内容**定位（本任务会改动 ui.rs 多处，后续任务的基准行号失效）。

---

### Task 1: focus 簿记纯化（`src/focus.rs` + Ui 委托）

**Files:**
- Create: `qingui/src/focus.rs`
- Modify: `qingui/src/lib.rs`（`pub mod focus;`）
- Modify: `qingui/src/ui.rs`（`group_focus_next`/`group_focus_prev` 委托）
- Test: `qingui/src/focus.rs`（`#[cfg(test)] mod tests`）+ `qingui/tests/input.rs`（契约测试）

**Interfaces:**
- Consumes: `crate::arena::ObjRef`。
- Produces: `focus::step(group: &[ObjRef], focused: Option<usize>, dir: i32, valid: impl Fn(ObjRef) -> bool) -> Option<usize>`（pub(crate)）——Task 2-5 不依赖它，但它是 focus 子系统唯一入口。

- [ ] **Step 1: 新建 `focus.rs` 并写失败测试**

创建 `qingui/src/focus.rs`，先只写测试（`step` 尚不存在，编译会失败——这就是 RED）：

```rust
use alloc::vec::Vec;
use crate::arena::ObjRef;

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(i: u32) -> ObjRef { ObjRef { index: i, generation: 0 } }

    #[test]
    fn empty_group_returns_none() {
        assert_eq!(step(&[], Some(0), 1, |_| true), None);
    }

    #[test]
    fn next_wraps_around() {
        let g = vec![obj(0), obj(1), obj(2)];
        // focused=2，Next(+1) → 环绕到 0
        assert_eq!(step(&g, Some(2), 1, |_| true), Some(0));
        // focused=0，Prev(-1) → 环绕到 2
        assert_eq!(step(&g, Some(0), -1, |_| true), Some(2));
    }

    #[test]
    fn skips_invalid() {
        let g = vec![obj(0), obj(1), obj(2)];
        // focused=0，Next(+1)：obj1 不可选 → 跳过到 obj2
        let valid = |o: ObjRef| o.index != 1;
        assert_eq!(step(&g, Some(0), 1, valid), Some(2));
        // 全不可选 → None
        assert_eq!(step(&g, Some(0), 1, |_| false), None);
    }

    #[test]
    fn none_focused_starts_at_zero() {
        let g = vec![obj(0), obj(1)];
        assert_eq!(step(&g, None, 1, |_| true), Some(1));
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p qingui --lib`
Expected: 编译错误，`step` 未定义（RED 确认）。

- [ ] **Step 3: 实现 `step`**

在 `focus.rs` 测试上方写入（函数体逐字节复刻旧循环语义）：

```rust
/// 焦点簿记：计算焦点应移动到的目标索引（纯函数，副作用由 Ui 执行）。
/// 语义与旧 Ui::group_focus_next/prev 完全一致：
/// 空组 → None；base = focused.unwrap_or(0)；从 base 沿 dir（±1）步进
/// 1..=len，跳过 !valid，环绕取模（rem_euclid）；全不可选 → None。
pub(crate) fn step(
    group: &[ObjRef],
    focused: Option<usize>,
    dir: i32,
    valid: impl Fn(ObjRef) -> bool,
) -> Option<usize> {
    if group.is_empty() {
        return None;
    }
    let base = focused.unwrap_or(0);
    let len = group.len();
    for k in 1..=len {
        let idx = (base as i32 + dir * k as i32).rem_euclid(len as i32) as usize;
        if valid(group[idx]) {
            return Some(idx);
        }
    }
    None
}
```

- [ ] **Step 4: 运行单元测试确认通过**

Run: `cargo test -p qingui --lib`
Expected: 4 个 `focus::tests::*` 全部 PASS。

- [ ] **Step 5: Ui 委托 + lib.rs 注册**

在 `qingui/src/ui.rs` 把 `group_focus_next`（现 ~960-972）与 `group_focus_prev`（现 ~973-985）的整体替换为：

```rust
    pub fn group_focus_next(&mut self) {
        if let Some(i) = crate::focus::step(&self.group, self.focused_idx, 1, |o| self.focusable(o)) {
            self.focus_to(i);
        }
    }
    pub fn group_focus_prev(&mut self) {
        if let Some(i) = crate::focus::step(&self.group, self.focused_idx, -1, |o| self.focusable(o)) {
            self.focus_to(i);
        }
    }
```

（`focusable`、`focus_to`、`group`、`focused_idx` 均保持现状。删除被替换的旧循环体。）

在 `qingui/src/lib.rs` 模块列表（`pub mod font;` 与 `pub mod geometry;` 之间）加一行：`pub mod focus;`

- [ ] **Step 6: 运行契约测试**

Run: `cargo test -p qingui --test input`
Expected: 全部 PASS。重点：`focus_cycles_with_next_prev`、`focus_events_and_state_flag`、`focus_skips_hidden_objects`、`modal_restricts_focus_navigation`。

- [ ] **Step 7: Commit**

```bash
git add qingui/src/focus.rs qingui/src/lib.rs qingui/src/ui.rs
git commit -m "refactor(focus): 焦点簿记抽为 focus::step 纯函数 + 单测"
```

---

### Task 2: anim 插值求值纯化（`anim::eval` + step_anims 改造）

**Files:**
- Modify: `qingui/src/anim.rs`（追加 `AnimEval` + `eval` + `#[cfg(test)] mod tests`）
- Modify: `qingui/src/ui.rs`（`step_anims` 用 `eval`）
- Test: `qingui/src/anim.rs`（单测）+ 全仓动画契约（`cargo test -p qingui`）

**Interfaces:**
- Consumes: `crate::anim::{Anim, AnimProp, Easing}`（现有）。`crate::ui::Ui` 仍被 `Anim::on_done` 类型使用（数据层，不动）。
- Produces: `anim::AnimEval`（pub(crate) enum：`Delay`/`Keep(i32)`/`Done(i32)`，Copy）+ `anim::eval(a: &Anim, start_time: u64, now: u64) -> AnimEval`（pub(crate)）——Task 3 的 `step_anims` 依赖。

- [ ] **Step 1: 追加 `eval` 单测（RED）**

在 `qingui/src/anim.rs` 末尾追加（`AnimEval`/`eval` 尚不存在，编译失败即 RED）：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn obj(i: u32) -> ObjRef { ObjRef { index: i, generation: 0 } }
    fn lin(start: i32, end: i32, dur: u32) -> Anim {
        Anim::new(obj(0), AnimProp::X, start, end, dur)
    }

    #[test]
    fn delay_window() {
        let a = lin(0, 100, 1000).delay(500);
        assert_eq!(eval(&a, 1000, 1499), AnimEval::Delay);
        assert_eq!(eval(&a, 1000, 1500), AnimEval::Keep(0));
    }

    #[test]
    fn linear_midpoint() {
        let a = lin(0, 100, 1000);
        assert_eq!(eval(&a, 0, 500), AnimEval::Keep(50));
    }

    #[test]
    fn done_at_end() {
        let a = lin(0, 100, 1000);
        assert_eq!(eval(&a, 0, 1000), AnimEval::Done(100));
    }

    #[test]
    fn repeat_three_cycles() {
        let a = lin(0, 100, 100).repeat(3);
        assert_eq!(eval(&a, 0, 99), AnimEval::Keep(99));
        assert_eq!(eval(&a, 0, 300), AnimEval::Done(100));
    }

    #[test]
    fn playback_reverses_on_odd_round() {
        let a = lin(0, 100, 100).repeat(2).playback(true);
        assert_eq!(eval(&a, 0, 50), AnimEval::Keep(50));  // round0 正向中点
        assert_eq!(eval(&a, 0, 150), AnimEval::Keep(50)); // round1 反向中点
        assert_eq!(eval(&a, 0, 200), AnimEval::Done(0));  // 奇数末轮反向 → start
    }

    #[test]
    fn infinite_repeat_never_done() {
        let a = lin(0, 100, 100).repeat(-1);
        assert!(matches!(eval(&a, 0, 999_999), AnimEval::Keep(_)));
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p qingui --lib`
Expected: 编译错误，`AnimEval`/`eval` 未定义（RED 确认）。

- [ ] **Step 3: 实现 `AnimEval` + `eval`**

在 `qingui/src/anim.rs` 末尾（tests 之上）写入——函数体与旧 `Ui::step_anims` 内联求值（ui.rs ~647-678）逐字节等价：

```rust
/// 单次动画求值结果（纯函数 eval 的输出）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AnimEval {
    /// 仍在延迟期
    Delay,
    /// 推进中：应应用的值
    Keep(i32),
    /// 已结束：最终值（on_done 回调由调用方在拿到 Done 后处理）
    Done(i32),
}

/// 插值求值（纯）：只算"该拿什么值"，不碰 on_done、不触树。
/// 语义与旧 Ui::step_anims 的内联求值逐字节一致（delay/重复/往返）。
pub(crate) fn eval(a: &Anim, start_time: u64, now: u64) -> AnimEval {
    let elapsed = now.saturating_sub(start_time);
    if elapsed < a.delay_ms as u64 {
        return AnimEval::Delay;
    }
    let t_ms = elapsed - a.delay_ms as u64;
    let dur = a.duration_ms.max(1) as u64;
    let total: i32 = if a.repeat < 0 { i32::MAX } else { a.repeat.max(1) };
    if t_ms >= dur * total as u64 {
        let last = total - 1;
        let rev = a.playback && last % 2 == 1;
        return AnimEval::Done(if rev { a.start } else { a.end });
    }
    let round = (t_ms / dur) as i32;
    let in_round = t_ms % dur;
    let rev = a.playback && round % 2 == 1;
    let t = in_round as f32 / dur as f32;
    let k = if rev { 1.0 - t } else { t };
    AnimEval::Keep(a.start + ((a.end - a.start) as f32 * a.easing.eval(k)) as i32)
}
```

- [ ] **Step 4: 运行单元测试确认通过**

Run: `cargo test -p qingui --lib`
Expected: 6 个 `anim::tests::*` 全部 PASS。

- [ ] **Step 5: 改造 `Ui::step_anims`**

把 `qingui/src/ui.rs` 的 `step_anims`（现 ~638-696，含内联 `enum Out`）整体替换为：

```rust
    fn step_anims(&mut self) {
        let now = self.time_ms;
        let mut i = 0;
        while i < self.anims.len() {
            let target = self.anims[i].anim.target;
            if !self.is_valid(target) {
                self.anims.remove(i); // 目标已删除：清理动画
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
                    let r = self.anims.remove(i);
                    self.apply_anim_value(r.anim.target, r.anim.prop, v);
                    if let Some(mut cb) = r.anim.on_done.take() {
                        cb(self);
                    }
                }
            }
        }
    }
```

（`apply_anim_value`、`is_valid` 保持现状；内联 `enum Out` 与旧求值代码删除。）

- [ ] **Step 6: 全仓测试确认动画契约不破**

Run: `cargo test -p qingui`
Expected: 全部 PASS（含 `tests/anim.rs`、`tests/transition_ghost.rs`、`tests/layout_transition.rs`、`tests/roller_ghost.rs`）。

- [ ] **Step 7: Commit**

```bash
git add qingui/src/anim.rs qingui/src/ui.rs
git commit -m "refactor(anim): 插值求值抽为 anim::eval 纯函数 + 单测"
```

---

### Task 3: renderer 纯化（`src/render.rs` 自由函数）

**Files:**
- Create: `qingui/src/render.rs`
- Modify: `qingui/src/lib.rs`（`pub mod render;`）
- Modify: `qingui/src/ui.rs`（删 render 块、`render`/`abs_rect`/`resolved_style` 变委托）
- Test: `qingui/src/render.rs`（像素单测）+ 全仓像素契约（`cargo test -p qingui`）

**Interfaces:**
- Consumes: `crate::arena::{Arena, ObjRef}`、`crate::dirty::DirtyQueue`、`crate::display::Flush`、`crate::node::{Flag, Node, State}`、`crate::style::ResolvedStyle`、`crate::widgets::{WidgetCtx, WidgetKind}`、`crate::font::DEFAULT_FONT`。`WidgetKind::draw` 为 pub(crate)，同 crate 可调。
- Produces: `render::render(screen: ObjRef, arena: &mut Arena<Node>, buf: &mut [Color], dirty: &mut DirtyQueue, flush: &mut Option<Box<dyn Flush>>, font: &'static MonoFont<'static>, time_ms: u64)` + `render::abs_rect(arena: &Arena<Node>, obj: ObjRef) -> Rect` + `render::resolved_style(arena: &Arena<Node>, obj: ObjRef, font) -> ResolvedStyle`（三者 pub(crate)）。Ui 的 `render`/`abs_rect`/`resolved_style` 变一行委托。

- [ ] **Step 1: 新建 `render.rs`（搬移 + 签名适配）**

创建 `qingui/src/render.rs`，整文件写入（行为与旧 Ui 方法逐字节等价；`self.screen`→入参 `screen`、`self.time_ms`→入参 `time_ms`、`self.arena`→入参 `arena`、`self.buf`→入参 `buf`、`self.flush`→入参 `flush`）：

```rust
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
```

（文件顶部需要 `use alloc::vec::Vec;`。）

- [ ] **Step 2: Ui 删除 render 块并委托**

在 `qingui/src/ui.rs` 删除：`render`（~739-744）、`render_area`（~746-756）、`render_chunk`（~758-779）、`draw_node`（~783-827）、`children_z_sorted`（~830-834）、`node_draw_info`（~836-840）——全部按内容定位。

把 `pub fn render` 替换为新委托（位置不变，`render_area` 等直接删除不留壳）：

```rust
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
```

把 `abs_rect`（~180-194）整体替换为委托：

```rust
    pub fn abs_rect(&self, obj: ObjRef) -> Rect {
        crate::render::abs_rect(&self.arena, obj)
    }
```

把 `resolved_style`（~330-345）整体替换为委托：

```rust
    pub fn resolved_style(&self, obj: ObjRef) -> crate::style::ResolvedStyle {
        crate::render::resolved_style(&self.arena, obj, self.default_font)
    }
```

在 `qingui/src/lib.rs` 模块列表（`pub mod input;` 与 `pub mod layout;` 之间）加一行：`pub mod render;`

（注：`rect`/`translate`/`state`/`children`/`is_valid` 等读方法保持现状，render.rs 用自己的私有 `kids`/`node_state`，不重复搬移公开方法。）

- [ ] **Step 3: 运行像素契约测试**

Run: `cargo test -p qingui`
Expected: 全部 PASS。重点覆盖像素断言的文件：`tests/render.rs`、`tests/draw.rs`、`tests/itemlist.rs`（`viewport_clips_scrolled_items`）、`tests/image.rs`、`tests/button` 相关、`tests/scrollview.rs`、`tests/clip.rs`、`tests/focus_visual.rs`。

- [ ] **Step 4: 追加 render 像素单测**

在 `qingui/src/render.rs` 末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Node;
    use crate::widgets::obj::ObjState;
    use crate::widgets::WidgetKind;
    use alloc::boxed::Box;
    use alloc::cell::RefCell;
    use alloc::rc::Rc;
    use alloc::vec::Vec;

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
        let (arena, rec) = {
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
}
```

- [ ] **Step 5: 运行单元测试确认通过**

Run: `cargo test -p qingui --lib`
Expected: 4 个 `render::tests::*` 全部 PASS。

- [ ] **Step 6: Commit**

```bash
git add qingui/src/render.rs qingui/src/lib.rs qingui/src/ui.rs
git commit -m "refactor(render): renderer 抽为 render.rs 自由函数 + 像素单测"
```

---

### Task 4: layout 纯数学单测

**Files:**
- Modify: `qingui/src/layout.rs`（仅追加 `#[cfg(test)] mod tests`）
- Test: `qingui/src/layout.rs`（单测）+ `qingui/tests/flex.rs`、`qingui/tests/grid.rs`（契约）

**Interfaces:**
- Consumes: layout.rs 现有私有纯函数 `axis_basis`、`axis_in_cell`、`distribute`、`align_offset`、`solve_tracks`、`track_offset`（本任务不改它们）。
- Produces: 无新接口（只加测试）。

- [ ] **Step 1: 追加 layout 单测**

在 `qingui/src/layout.rs` 末尾追加（`#[cfg(test)]` 模块内可访问私有函数）：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_basis_variants() {
        assert_eq!(axis_basis(None, 5, 100), 5);
        assert_eq!(axis_basis(Some(Sizing::Fixed(10)), 5, 100), 10);
        assert_eq!(axis_basis(Some(Sizing::Fit { min: 3, max: 8 }), 10, 100), 8);
        assert_eq!(axis_basis(Some(Sizing::Fit { min: 3, max: 8 }), 1, 100), 3);
        assert_eq!(axis_basis(Some(Sizing::Grow { min: 4, max: 100 }), 5, 100), 4);
        assert_eq!(axis_basis(Some(Sizing::Percent(50)), 0, 200), 100);
    }

    #[test]
    fn axis_in_cell_grow_fills() {
        assert_eq!(axis_in_cell(Some(Sizing::Grow { min: 0, max: 100 }), 5, 50), 50);
        assert_eq!(axis_in_cell(Some(Sizing::Grow { min: 80, max: 100 }), 5, 50), 80);
    }

    #[test]
    fn distribute_alignments() {
        assert_eq!(distribute(100, 200, Align::Start, 2, 4), (0, 4));
        assert_eq!(distribute(100, 200, Align::Center, 2, 4), (50, 4));
        assert_eq!(distribute(100, 200, Align::End, 2, 4), (100, 4));
        assert_eq!(distribute(100, 200, Align::SpaceBetween, 2, 4), (0, 104));
        assert_eq!(distribute(100, 200, Align::SpaceEvenly, 2, 4), (33, 37)); // g=33 → (33, 37)
    }

    #[test]
    fn align_offset_cases() {
        assert_eq!(align_offset(10, 100, Align::Start), 0);
        assert_eq!(align_offset(10, 100, Align::Center), 45);
        assert_eq!(align_offset(10, 100, Align::End), 90);
        assert_eq!(align_offset(10, 100, Align::SpaceBetween), 0);
    }

    #[test]
    fn track_offset_accumulates() {
        assert_eq!(track_offset(&[10, 20, 30], 2, 4), 34);
        assert_eq!(track_offset(&[10], 0, 4), 0);
    }

    #[test]
    fn solve_tracks_fr_consumes_remaining() {
        let tracks = [Track::Px(10), Track::Fr(1), Track::Fr(2)];
        assert_eq!(solve_tracks(&tracks, &[], 0, 100), vec![10, 30, 60]);
    }

    #[test]
    fn solve_tracks_content_sizes() {
        let tracks = [Track::Content, Track::Fr(1)];
        let child_sizes = [(0u8, 1u8, 25i32)];
        assert_eq!(solve_tracks(&tracks, &child_sizes, 0, 100), vec![25, 75]);
    }
}
```

验证预期值：
- `distribute(100, 200, Align::SpaceEvenly, 2, 4)`：`free = 100`，`g = 100/3 = 33` → `(33, 37)`。✓
- `solve_tracks([Px(10), Fr(1), Fr(2)], 0 gap, 100)`：fixed=10，remaining=90，fr_total=3；idx1=90*1/3=30，last idx2=90-30=60 → `[10,30,60]`。✓
- `solve_tracks([Content, Fr(1)], child(0,span1,size25), 100)`：Content 轨道取 25，remaining=75，fr idx1=75 → `[25,75]`。✓

- [ ] **Step 2: 运行测试确认通过**

Run: `cargo test -p qingui --lib`
Expected: 7 个 `layout::tests::*` 全部 PASS。

- [ ] **Step 3: 契约测试确认 layout 未动**

Run: `cargo test -p qingui --test flex; cargo test -p qingui --test grid`
Expected: 全部 PASS（本任务未改任何行为代码）。

- [ ] **Step 4: Commit**

```bash
git add qingui/src/layout.rs
git commit -m "test(layout): 纯布局数学单测（axis_basis/distribute/align_offset/solve_tracks/track_offset）"
```

---

### Task 5: 全量验证与收尾

**Files:**
- Test: 全仓库验证（无需改代码）

**Interfaces:**
- Consumes: 前四任务成果。只做验收。

- [ ] **Step 1: 全量测试**

Run: `cargo test -p qingui`
Expected: 全部 PASS（187 集成 + 新增 ~18 单测，一个不差）。确认 `tests/render.rs`、`tests/anim.rs`、`tests/input.rs`、`tests/flex.rs`、`tests/grid.rs` 均绿。

- [ ] **Step 2: no_std 目标编译**

Run: `cargo build -p qingui --target thumbv7em-none-eabihf`
Expected: 编译成功。（target 未装则 `rustup target add thumbv7em-none-eabihf` 后重试。）

- [ ] **Step 3: examples 编译**

Run: `cargo check -p qingui --examples`
Expected: 无错误。

- [ ] **Step 4: 确认纯子系统无 Ui 依赖**

用 grep/Select-String 在 `qingui/src/render.rs` 与 `qingui/src/focus.rs` 中搜 `Ui`：
Expected: 零匹配（两个文件完全不含 Ui 引用）。`anim.rs` 允许 `use crate::ui::Ui`（`Anim::on_done` 数据层需要）。

- [ ] **Step 5: 确认 ui.rs 减重**

Run: `Select-String -Pattern '^    pub fn render|^    fn render|^    fn render_area|^    fn render_chunk|^    fn draw_node|^    fn children_z_sorted|^    fn node_draw_info' qingui/src/ui.rs`
Expected: 只命中 `pub fn render`（委托版），其余全部消失。ui.rs 行数较任务前减 ~150 行。

- [ ] **Step 6: git 状态确认**

Run: `git status --short`
Expected: 工作区干净（四任务已提交，无遗留改动）。

---

## Self-Review

**Spec 覆盖：**
- renderer 全拆 → Task 3（含 `screen`/`flush` 签名修正）。
- anim 插值 → Task 2；focus 簿记 → Task 1；layout 只测数学 → Task 4。
- 不拆事件/tick/anim 驱动器/键盘 → 各任务均未触碰 `send_event`/`tick_widgets`/`call_on_key`/`apply_key_outcome`。
- 单测不依赖 Ui → Task 5 Step 4 用 grep 验收；Task 3 Step 4 修正了隐藏测试的 screen 句柄 hack。
- `ui.rs` 减重 → Task 3 + Task 5 Step 5。

**占位符扫描：** 无 TBD/TODO；每个代码步骤给出完整可粘贴代码；测试的预期值均手算验证过（在步骤内注明）。

**类型一致性：**
- `render::render` 七参签名在 Task 3 Step 1（定义）与 Step 2（Ui 委托）完全一致；`abs_rect`/`resolved_style` 三参一致。
- `focus::step` 五参签名在 Task 1 Step 3（定义）与 Step 5（委托）一致。
- `anim::eval` 三参 + `AnimEval` 三变体在 Task 2 Step 3 与 Step 5（step_anims match）一致。
- 单元测试 fixture 统一用 `Arena::new()` + `Node::new` + 手动 `children.push`；`ObjRef { index, generation }` 直接构造假句柄。
