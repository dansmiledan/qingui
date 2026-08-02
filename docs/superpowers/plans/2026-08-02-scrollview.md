# ScrollView 滚动容器实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按 `docs/superpowers/specs/2026-08-02-scrollview-design.md` 新增 ScrollView 滚动容器(视口 CLIP + content translate),并修复 demo About 页溢出。

**Architecture:** 与 ItemList 同构两层结构;按键经新 `KeyOutcome::ScrollBy(i32)` 由 Ui 统一执行(clamp + set_translate);API 为扩展 trait `UiScrollViewExt` 进 prelude。

**Tech Stack:** Rust no_std + alloc;host `cargo test -p qingui`;嵌入式 `cargo build -p qingui --target thumbv7em-none-eabihf`;`cargo check -p qingui --examples`。

## Global Constraints

- no_std + alloc;draw 热路径无分配;无效 ObjRef 静默 no-op。
- 仅垂直滚动;scroll ≤ 0,clamp 到 `[-(content_h - view_h).max(0), 0]`;内容不足一屏时 scroll 恒 0。
- 步进固定 20px,即时生效无动画;容器聚焦模型,子控件不进焦点组。
- 中文注释风格;commit message 中文。
- 每个 Task 结束:`cargo test -p qingui` 全绿 + thumbv7em + examples check。

---

### Task 1: scrollview widget + KeyOutcome::ScrollBy + UiScrollViewExt

**Files:**
- Create: `qingui/src/widgets/scrollview.rs`
- Modify: `qingui/src/widgets/mod.rs`(`pub mod scrollview;` 在 roller 之后 slider 之前;define_widgets! 加一行在 Roller 之后;`KeyOutcome` 加 `ScrollBy(i32)` 变体)
- Modify: `qingui/src/ui.rs`(apply_key_outcome 加 ScrollBy 臂;顶部 use UiScrollViewExt)
- Modify: `qingui/src/lib.rs`(prelude 加 UiScrollViewExt,字母序)
- Test: `qingui/tests/scrollview.rs`(新建)

**Interfaces:**
- Consumes: `WidgetBehavior`(draw 必实现,on_key 默认 Pass);`KeyOutcome`/`KeyCtx`(widgets/mod.rs);itemlist 的视口结构模式(itemlist.rs:84-96:占位 Obj → set_clip_children → content → 换真身);`Ui::set_translate/translate/children/rect`(公开);`apply_key_outcome`(ui.rs:1120 附近 NavSelect 臂为参照)。
- Produces:
  - `qingui::widgets::scrollview::{ScrollViewState, ScrollViewBuilder, UiScrollViewExt, STEP}`
  - `ScrollViewState { content: ObjRef, scroll: i32 }`(content pub(crate),scroll pub)
  - `UiScrollViewExt::scrollview_content(sv) -> Option<ObjRef>`、`scrollview_scroll_to(sv, y: i32)`、`scrollview_scroll_by(sv, delta: i32)`
  - `KeyOutcome::ScrollBy(i32)`(正值向下滚/内容向上移)

- [ ] **Step 1: 写失败测试(新建 tests/scrollview.rs)**

```rust
use qingui::input::Key;
use qingui::prelude::*;
use qingui::widgets::label::LabelBuilder;
use qingui::widgets::obj::ObjBuilder;
use qingui::widgets::scrollview::{ScrollViewBuilder, STEP};
use qingui::{Rect, Ui};

/// 建一个 60px 视口 + 3 个 40px item(content 120px)
fn build() -> (Ui, qingui::ObjRef, qingui::ObjRef) {
    let mut ui = Ui::new(160, 120, 24);
    let s = ui.screen();
    let sv = ScrollViewBuilder::new().size(80, 60).build(&mut ui, s);
    let content = ui.scrollview_content(sv).unwrap();
    for _ in 0..3 {
        let item = ObjBuilder::new().build(&mut ui, content);
        ui.set_size(item, 60, 40);
    }
    (ui, sv, content)
}

#[test]
fn builder_and_content_accessor() {
    let (ui, sv, content) = build();
    assert_eq!(ui.rect(sv), Rect::new(0, 0, 80, 60));
    assert_eq!(ui.children(sv), vec![content]);
    assert_eq!(ui.children(content).len(), 3);
    assert_eq!(ui.translate(content).y, 0);
    // 无效目标
    assert!(ui.scrollview_content(content).is_none());
}

#[test]
fn focused_up_down_scrolls_and_clamps() {
    let (mut ui, sv, content) = build();
    ui.group_add(sv);
    ui.group_focus(sv);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.translate(content).y, -STEP);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down); // 4 步 = 80,但 clamp 到 -(120-60) = -60
    assert_eq!(ui.translate(content).y, -60);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.translate(content).y, -60 + STEP);
    for _ in 0..10 {
        ui.keypad_input(Key::Up); // 顶到 0 不再动
    }
    assert_eq!(ui.translate(content).y, 0);
}

#[test]
fn short_content_never_scrolls() {
    let mut ui = Ui::new(160, 120, 24);
    let s = ui.screen();
    let sv = ScrollViewBuilder::new().size(80, 60).build(&mut ui, s);
    let content = ui.scrollview_content(sv).unwrap();
    let item = ObjBuilder::new().build(&mut ui, content);
    ui.set_size(item, 60, 30); // 内容 30 < 视口 60
    ui.group_add(sv);
    ui.group_focus(sv);
    ui.keypad_input(Key::Down);
    assert_eq!(ui.translate(content).y, 0);
}

#[test]
fn scroll_to_programmatic() {
    let (mut ui, sv, content) = build();
    ui.scrollview_scroll_to(sv, -30);
    assert_eq!(ui.translate(content).y, -30);
    ui.scrollview_scroll_to(sv, -999); // clamp 到 -60
    assert_eq!(ui.translate(content).y, -60);
    ui.scrollview_scroll_to(sv, 50); // clamp 到 0
    assert_eq!(ui.translate(content).y, 0);
    ui.scrollview_scroll_to(content, -10); // 非 scrollview:静默 no-op
    assert_eq!(ui.translate(content).y, 0);
}
```

- [ ] **Step 2: 跑测试确认失败**(编译失败,scrollview 模块不存在)

- [ ] **Step 3: mod.rs 加 KeyOutcome::ScrollBy**

`KeyOutcome` 枚举(`NavSelect(i32)` 之后)加:

```rust
    /// 滚动容器滚动(步进 ±px),由 Ui 执行(clamp + translate)
    ScrollBy(i32),
```

- [ ] **Step 4: 实现 qingui/src/widgets/scrollview.rs**

```rust
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::Rect;
use crate::input::Key;
use crate::layout::{Align, Flex, FlexDir, Sizing};
use crate::style::{Layout, Style};
use crate::ui::Ui;
use super::{KeyCtx, KeyOutcome, WidgetBehavior, WidgetCtx, WidgetKind};

/// 单次按键滚动步进(px)
pub const STEP: i32 = 20;

/// 滚动容器状态:视口 CLIP_CHILDREN,content 经 translate 移动
pub struct ScrollViewState {
    pub(crate) content: ObjRef,
    pub scroll: i32, // ≤0
}

impl ScrollViewState {
    pub(crate) fn on_key(&mut self, key: Key, _ctx: KeyCtx) -> KeyOutcome {
        match key {
            Key::Up => KeyOutcome::ScrollBy(-STEP),
            Key::Down => KeyOutcome::ScrollBy(STEP),
            _ => KeyOutcome::Pass,
        }
    }
}

impl WidgetBehavior for ScrollViewState {
    // 容器:内容由子节点绘制(视口 CLIP 已由通用管线处理)
    fn draw(&self, _ctx: &WidgetCtx, _d: &mut DrawBuf, _clip: Rect) {}
    fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
        self.on_key(key, ctx)
    }
}

/// ScrollView 构建器:默认 120x100,视口透明 + content column flex
pub struct ScrollViewBuilder {
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
}

impl ScrollViewBuilder {
    pub fn new() -> Self {
        Self { size: None, style: None, sizing: None, transition: None, events: Vec::new() }
    }
    pub fn size(mut self, w: i32, h: i32) -> Self { self.size = Some((w, h)); self }
    pub fn style(mut self, s: Style) -> Self { self.style = Some(s); self }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h)); self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing)); self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb)); self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((120, 100));
        // 视口先以 Obj 占位(content 引用需要自指后的句柄)
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Obj(super::obj::ObjState));
        ui.set_clip_children(r, true);
        // content:column flex,宽 GROW,透明
        let content = ui.insert_node(r, Rect::new(0, 0, w, 0), WidgetKind::Obj(super::obj::ObjState));
        let mut cs = Style::default();
        cs.bg_opa = Some(0);
        ui.set_style(content, cs);
        ui.set_sizing(content, Some(Sizing::GROW), None);
        ui.set_layout(content, Layout::Flex(Flex {
            dir: FlexDir::Column, wrap: false,
            main: Align::Start, cross: Align::Start, track: Align::Start, gap: 0,
        }));
        // 占位 kind 换真身
        if let Some(n) = ui.kind_mut(r) {
            *n = WidgetKind::ScrollView(ScrollViewState { content, scroll: 0 });
        }
        // 视口样式:默认透明;聚焦样式给默认边框高亮
        let mut vs = self.style.unwrap_or_default();
        if vs.bg_opa.is_none() { vs.bg_opa = Some(0); }
        ui.set_style(r, vs);
        ui.set_style_focused(r, crate::style::theme_list_focused());
        if let Some((sw, sh)) = self.sizing {
            ui.set_sizing(r, sw, sh);
        }
        if let Some(t) = self.transition {
            ui.set_transition(r, Some(t));
        }
        for (k, cb) in self.events {
            ui.add_event_cb(r, k, cb);
        }
        r
    }
}

/// ScrollView API(经 prelude 引入)
pub trait UiScrollViewExt {
    fn scrollview_content(&self, sv: ObjRef) -> Option<ObjRef>;
    fn scrollview_scroll_to(&mut self, sv: ObjRef, y: i32);
    fn scrollview_scroll_by(&mut self, sv: ObjRef, delta: i32);
}

impl UiScrollViewExt for Ui {
    fn scrollview_content(&self, sv: ObjRef) -> Option<ObjRef> {
        self.kind(sv).and_then(|k| k.as_scrollview()).map(|s| s.content)
    }

    fn scrollview_scroll_to(&mut self, sv: ObjRef, y: i32) {
        let Some(content) = self.scrollview_content(sv) else { return };
        // content_h = 子节点最大底边;视口高 = sv 高度
        let content_h = self.children(content).iter()
            .map(|&c| self.rect(c).y + self.rect(c).h)
            .max()
            .unwrap_or(0);
        let view_h = self.rect(sv).h;
        let min = -(content_h - view_h).max(0);
        let ny = y.clamp(min, 0);
        if let Some(s) = self.kind_mut(sv).and_then(|k| k.as_scrollview_mut()) {
            s.scroll = ny;
        }
        self.set_translate(content, 0, ny);
    }

    fn scrollview_scroll_by(&mut self, sv: ObjRef, delta: i32) {
        let cur = self.kind(sv).and_then(|k| k.as_scrollview()).map(|s| s.scroll);
        if let Some(cur) = cur {
            self.scrollview_scroll_to(sv, cur + delta);
        }
    }
}
```

注意:`ui.kind_mut(r)`/`ui.kind(r)` 是 pub(crate)(registry 重构已加),同 crate 可用。`theme_list_focused` 若不存在于 style.rs,用 style.rs 中现有的聚焦主题函数(参照 itemlist.rs 的用法)。

- [ ] **Step 5: 注册与接线**

- mod.rs:`pub mod scrollview;`(roller 之后 slider 之前);define_widgets! 在 Roller 行之后加:
  `ScrollView(scrollview::ScrollViewState, as_scrollview, as_scrollview_mut),`
- ui.rs apply_key_outcome(`NavSelect` 臂之后)加:

```rust
            KeyOutcome::ScrollBy(d) => {
                self.scrollview_scroll_by(obj, d);
                true
            }
```

  ui.rs 顶部加 `use crate::widgets::scrollview::UiScrollViewExt;`。
- lib.rs prelude 字母序加 `pub use crate::widgets::scrollview::UiScrollViewExt;`。

- [ ] **Step 6: 跑测试确认通过 + 全量回归 + thumbv7em + examples check**

- [ ] **Step 7: Commit**

```bash
git add qingui/src/widgets/scrollview.rs qingui/src/widgets/mod.rs qingui/src/ui.rs qingui/src/lib.rs qingui/tests/scrollview.rs
git commit -m "feat(scrollview): ScrollView 滚动容器(视口 CLIP + content translate)

容器聚焦模型:聚焦时 Up/Down 按 20px 步进滚动,clamp 到内容范围,
内容不足一屏不滚。KeyOutcome::ScrollBy 由 Ui 统一执行。不做横向/
滚动条/动画/聚焦跟随(YAGNI)。"
```

---

### Task 2: demo About 页改造

**Files:**
- Modify: `qingui/examples/demo.rs`

**Interfaces:**
- Consumes: Task 1 的 `ScrollViewBuilder`/`UiScrollViewExt`;demo.rs 现有 page_about(column flex 容器,含 Wide 按钮与两张图)。
- Produces: 无新接口。

- [ ] **Step 1: 改造 About 页**

读 demo.rs 找到 page_about 现有内容(label、wide_btn、images::HAIZEI、images::MIAO)。改造:

- Wide 按钮与其余可聚焦内容保持直接挂在 page_about(scrollview 外);
- 新增 `let sv = ScrollViewBuilder::new().build(ui, page_about);`,`ui.set_sizing(sv, Some(Sizing::GROW), Some(Sizing::GROW));`;
- 两张图(及 About 文本 label,若视觉上属于滚动内容)改挂到 `ui.scrollview_content(sv).unwrap()` 下;
- `ui.group_add(sv)` 加入焦点组(wide_btn 附近的位置);
- imports 加 `use qingui::widgets::scrollview::ScrollViewBuilder;`(字母序)。

- [ ] **Step 2: 验证**

Run: `cargo check -p qingui --examples`(零 error)
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(空)
手动:sim About 页,Tab 到 ScrollView(边框高亮),Up/Down 滚动,gif 滚出来可见。

- [ ] **Step 3: Commit**

```bash
git add qingui/examples/demo.rs
git commit -m "feat(demo): About 页图片移入 ScrollView,修复内容溢出 gif 不可见"
```

---

## Self-Review 记录

- Spec 覆盖:两层结构/CLIP/translate/clamp(Task 1 Step 4)、容器聚焦 Up/Down 20px(Step 4 on_key + Step 5)、scrollview_content/scroll_to API(Step 4)、builder(Step 4)、About 修复(Task 2)、测试清单(Task 1 Step 1)。
- 占位符:无;theme_list_focused 给了 fallback 核查指令(参照 itemlist.rs 实际用法)。
- 类型一致性:ScrollBy(i32) 正值向下、scrollview_scroll_to/scroll_by 签名、STEP 常量名全文一致;测试断言与实现(-STEP/-60/clamp 0)自洽(60 视口、120 内容 → min=-60)。
- 交互推论已在 spec 声明:scrollview 内子控件不进焦点组;Task 2 据此让 Wide 按钮留在 sv 外。
