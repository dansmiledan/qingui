# Rust LVGL 子集 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 Rust 实现 LVGL 子集：Arena 对象树 + PFB 分块渲染 + 脏矩形 + 动画 + 按键焦点组，附 minifb 桌面模拟器 demo。

**Architecture:** `no_std + alloc` 核心库（Arena + `ObjRef` 句柄、扁平样式、软件光栅化、分块 flush），`std` 模拟器 crate（minifb 窗口 + 键盘映射）。渲染流程：`tick_inc` → `timer_handler`（动画 → 布局 → 脏矩形分块渲染 → flush）。

**Tech Stack:** Rust stable, edition 2021; `font8x8` (no_std 位图字体表); `minifb` 0.29 (仅模拟器); cargo workspace。

**Spec:** `docs/superpowers/specs/2026-07-27-rust-lvgl-subset-design.md`

## Global Constraints

- 核心库 `rust-lvgl`：`#![no_std]` + `extern crate alloc;`；除 `font8x8` 外无外部依赖。
- 模拟器 `rust-lvgl-sim`：仅依赖 `rust-lvgl` + `minifb = "29"`。
- 字体决策：使用 `font8x8 = { version = "0.3", default-features = false, features = ["unicode"] }`（public-domain 8x8 ASCII 字模表，编译期内置，等价于规格 §11 的"编译期生成字模表"；`unicode` feature 提供 `UnicodeFonts::get`）。
- 颜色内部一律 RGB888（`Color { r, g, b }`）；flush 推送 `&[Color]`，由后端转换格式（RGB565 转换函数随核心库提供）。
- 测试一律放在 `rust-lvgl/tests/` 集成测试目录（宿主 std 环境），核心库内部不写 `#[cfg(test)]`。
- 每个 Task 完成后按步骤里的命令提交（conventional commits）。
- 公共 API 命名必须与计划中的签名完全一致（各 Task 的 Interfaces 块是契约）。

## File Structure

```
Cargo.toml                          # workspace: members = ["rust-lvgl", "rust-lvgl-sim"]
rust-lvgl/Cargo.toml
rust-lvgl/src/lib.rs                # #![no_std], extern crate alloc, pub mod 声明 + re-export
rust-lvgl/src/geometry.rs           # Point, Rect, Color
rust-lvgl/src/arena.rs              # Arena<T>, ObjRef（代际句柄）
rust-lvgl/src/node.rs               # Node, WidgetKind, state/flag 常量
rust-lvgl/src/style.rs              # Style, ResolvedStyle, Layout 类型, theme 函数
rust-lvgl/src/dirty.rs              # DirtyQueue
rust-lvgl/src/draw.rs               # DrawBuf：fill_rect / fill_rounded / draw_border / draw_glyph（全部带 clip）
rust-lvgl/src/font.rs               # 字形查询 + 文本测量
rust-lvgl/src/display.rs            # Flush trait；Ui 的 PFB 分块渲染
rust-lvgl/src/anim.rs               # Anim, AnimProp, Easing
rust-lvgl/src/event.rs              # EventKind, 事件分发
rust-lvgl/src/input.rs              # Key, 焦点组, 编辑态
rust-lvgl/src/ui.rs                 # Ui：持有全部状态，timer_handler / tick_inc / 树操作 / 控件 API
rust-lvgl/src/layout.rs             # Flex / Grid 布局计算
rust-lvgl/tests/*.rs                # 每个 Task 一个集成测试文件
rust-lvgl-sim/Cargo.toml
rust-lvgl-sim/src/main.rs           # 窗口 + 按键映射 + FPS/脏矩形可视化
rust-lvgl-sim/src/demo.rs           # 综合 demo 界面
```

---

### Task 1: Workspace 脚手架 + geometry（Point/Rect/Color）

**Files:**
- Create: `Cargo.toml`
- Create: `rust-lvgl/Cargo.toml`
- Create: `rust-lvgl/src/lib.rs`
- Create: `rust-lvgl/src/geometry.rs`
- Test: `rust-lvgl/tests/geometry.rs`

**Interfaces:**
- Produces（后续所有 Task 依赖）:
  - `Point { pub x: i32, pub y: i32 }`
  - `Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }`，方法：`new(x,y,w,h)`, `is_empty() -> bool`, `right() -> i32`, `bottom() -> i32`, `intersect(&self, other: &Rect) -> Option<Rect>`, `union(&self, other: &Rect) -> Rect`, `intersects(&self, other: &Rect) -> bool`, `contains(&self, p: Point) -> bool`, `translate(&self, dx: i32, dy: i32) -> Rect`
  - `Color { pub r: u8, pub g: u8, pub b: u8 }`，常量 `BLACK/WHITE/RED/GREEN/BLUE/GRAY/LIGHT_GRAY/DARK_GRAY`，方法 `rgb(r,g,b) -> Color`, `to_rgb565() -> u16`, `blend(self, over: Color, opa: u8) -> Color`（self 为背景，over 以 opa 0..=255 覆盖）

- [ ] **Step 1: 写 workspace 与 crate 清单**

`Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["rust-lvgl", "rust-lvgl-sim"]
```

`rust-lvgl/Cargo.toml`:
```toml
[package]
name = "rust-lvgl"
version = "0.1.0"
edition = "2021"

[dependencies]
font8x8 = { version = "0.3", default-features = false, features = ["unicode"] }
```

`rust-lvgl/src/lib.rs`:
```rust
#![no_std]
extern crate alloc;

pub mod geometry;
pub use geometry::{Color, Point, Rect};
```

- [ ] **Step 2: 写失败测试**

`rust-lvgl/tests/geometry.rs`:
```rust
use rust_lvgl::{Color, Point, Rect};

#[test]
fn rect_intersect_overlap() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(5, 5, 10, 10);
    assert_eq!(a.intersect(&b), Some(Rect::new(5, 5, 5, 5)));
}

#[test]
fn rect_intersect_disjoint() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(20, 0, 5, 5);
    assert_eq!(a.intersect(&b), None);
}

#[test]
fn rect_intersect_touching_edges_is_none() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(10, 0, 5, 5);
    assert_eq!(a.intersect(&b), None);
}

#[test]
fn rect_union() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(5, 5, 10, 10);
    assert_eq!(a.union(&b), Rect::new(0, 0, 15, 15));
}

#[test]
fn rect_contains_point_and_translate() {
    let r = Rect::new(0, 0, 10, 10);
    assert!(r.contains(Point { x: 9, y: 9 }));
    assert!(!r.contains(Point { x: 10, y: 0 }));
    assert_eq!(r.translate(3, -2), Rect::new(3, -2, 10, 10));
}

#[test]
fn color_rgb565() {
    assert_eq!(Color::rgb(255, 255, 255).to_rgb565(), 0xFFFF);
    assert_eq!(Color::rgb(0, 0, 0).to_rgb565(), 0x0000);
    assert_eq!(Color::rgb(255, 0, 0).to_rgb565(), 0xF800);
}

#[test]
fn color_blend() {
    let bg = Color::BLACK;
    assert_eq!(bg.blend(Color::WHITE, 255), Color::WHITE);
    assert_eq!(bg.blend(Color::WHITE, 0), Color::BLACK);
    let half = bg.blend(Color::rgb(200, 100, 50), 128);
    assert_eq!(half, Color::rgb(100, 50, 25));
}
```

- [ ] **Step 3: 运行确认失败**

Run: `cargo test -p rust-lvgl --test geometry`
Expected: 编译失败（`geometry` 模块不存在 / 方法未定义）

- [ ] **Step 4: 实现 geometry.rs**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }
    pub fn right(&self) -> i32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
    pub fn intersects(&self, other: &Rect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        if r > x && b > y {
            Some(Rect::new(x, y, r - x, b - y))
        } else {
            None
        }
    }
    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let r = self.right().max(other.right());
        let b = self.bottom().max(other.bottom());
        Rect::new(x, y, r - x, b - y)
    }
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }
    pub fn translate(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    pub const LIGHT_GRAY: Color = Color::rgb(200, 200, 200);
    pub const DARK_GRAY: Color = Color::rgb(40, 40, 40);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }
    pub fn to_rgb565(&self) -> u16 {
        (((self.r as u16) & 0xF8) << 8) | (((self.g as u16) & 0xFC) << 3) | ((self.b as u16) >> 3)
    }
    /// self 为背景，over 以 opa (0..=255) 覆盖混合
    pub fn blend(self, over: Color, opa: u8) -> Color {
        let a = opa as u32;
        let inv = 255 - a;
        let m = |s: u8, o: u8| ((s as u32 * inv + o as u32 * a + 127) / 255) as u8;
        Color::rgb(m(self.r, over.r), m(self.g, over.g), m(self.b, over.b))
    }
}
```

- [ ] **Step 5: 运行确认通过**

Run: `cargo test -p rust-lvgl --test geometry`
Expected: 6 tests PASS

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml rust-lvgl
git commit -m "feat: workspace scaffold + geometry primitives"
```

---

### Task 2: Arena + ObjRef + 对象树基础

**Files:**
- Create: `rust-lvgl/src/arena.rs`
- Create: `rust-lvgl/src/node.rs`
- Create: `rust-lvgl/src/ui.rs`
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/tree.rs`

**Interfaces:**
- Consumes: Task 1 的 `Rect`。
- Produces:
  - `ObjRef { pub index: u32, pub generation: u32 }`（Copy/Clone/Eq/Debug）
  - `Arena<T>`：`insert(T) -> ObjRef`, `get(ObjRef) -> Option<&T>`, `get_mut(ObjRef) -> Option<&mut T>`, `remove(ObjRef) -> Option<T>`, `contains(ObjRef) -> bool`
  - `WidgetKind`（本 Task 只有 `Obj` 变体，后续 Task 追加变体）
  - `Ui`：`new(width: i32, height: i32, buf_rows: u32) -> Ui`, `screen() -> ObjRef`, `create_obj(parent: ObjRef) -> ObjRef`, `delete(obj: ObjRef)`, `is_valid(obj: ObjRef) -> bool`, `set_pos(obj, x, y)`, `set_size(obj, w, h)`, `rect(obj) -> Rect`（相对父对象的本地坐标矩形）, `abs_rect(obj) -> Rect`（屏幕绝对坐标）, `set_hidden(obj, bool)`, `children(obj) -> Vec<ObjRef>`
  - 约定：对象的 `rect` 是相对父内容原点的本地坐标；`abs_rect` 在调用时沿父链累加计算（不做缓存）。

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/tree.rs`:
```rust
use rust_lvgl::{Rect, Ui};

#[test]
fn create_and_hierarchy() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ui.create_obj(screen);
    let b = ui.create_obj(a);
    assert_eq!(ui.children(screen), vec![a]);
    assert_eq!(ui.children(a), vec![b]);
}

#[test]
fn delete_invalidates_handle_and_reparents_nothing() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ui.create_obj(screen);
    let b = ui.create_obj(a);
    ui.set_pos(b, 10, 10);
    ui.delete(a);
    assert!(!ui.is_valid(a));
    assert!(!ui.is_valid(b)); // 删除父对象级联删除子树
    // 悬垂句柄操作安全 no-op
    ui.set_pos(a, 5, 5);
    assert_eq!(ui.children(screen).len(), 0);
}

#[test]
fn generation_recycled_slot_is_safe() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ui.create_obj(screen);
    ui.delete(a);
    let b = ui.create_obj(screen); // 复用 slot
    assert_eq!(a.index, b.index);
    assert_ne!(a, b);
    assert!(!ui.is_valid(a));
    assert!(ui.is_valid(b));
}

#[test]
fn abs_rect_accumulates_parents() {
    let mut ui = Ui::new(320, 240, 40);
    let screen = ui.screen();
    let a = ui.create_obj(screen);
    ui.set_pos(a, 10, 20);
    ui.set_size(a, 100, 80);
    let b = ui.create_obj(a);
    ui.set_pos(b, 5, 5);
    ui.set_size(b, 30, 30);
    assert_eq!(ui.rect(b), Rect::new(5, 5, 30, 30));
    assert_eq!(ui.abs_rect(b), Rect::new(15, 25, 30, 30));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test tree`
Expected: 编译失败（`Ui` 不存在）

- [ ] **Step 3: 实现 arena.rs / node.rs / ui.rs**

`rust-lvgl/src/arena.rs`:
```rust
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ObjRef {
    pub index: u32,
    pub generation: u32,
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { slots: Vec::new(), free: Vec::new() }
    }
    pub fn insert(&mut self, v: T) -> ObjRef {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(v);
            ObjRef { index, generation: slot.generation }
        } else {
            self.slots.push(Slot { generation: 0, value: Some(v) });
            ObjRef { index: (self.slots.len() - 1) as u32, generation: 0 }
        }
    }
    pub fn get(&self, r: ObjRef) -> Option<&T> {
        self.slots
            .get(r.index as usize)
            .filter(|s| s.generation == r.generation)
            .and_then(|s| s.value.as_ref())
    }
    pub fn get_mut(&mut self, r: ObjRef) -> Option<&mut T> {
        self.slots
            .get_mut(r.index as usize)
            .filter(|s| s.generation == r.generation)
            .and_then(|s| s.value.as_mut())
    }
    pub fn remove(&mut self, r: ObjRef) -> Option<T> {
        let slot = self.slots.get_mut(r.index as usize)?;
        if slot.generation != r.generation {
            return None;
        }
        let v = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(r.index);
        Some(v)
    }
    pub fn contains(&self, r: ObjRef) -> bool {
        self.get(r).is_some()
    }
}
```

`rust-lvgl/src/node.rs`:
```rust
use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::geometry::Rect;

pub mod state {
    pub const PRESSED: u8 = 1 << 0;
    pub const FOCUSED: u8 = 1 << 1;
    pub const DISABLED: u8 = 1 << 2;
    pub const EDITED: u8 = 1 << 3;
}

pub mod flag {
    pub const HIDDEN: u8 = 1 << 0;
    pub const CLICKABLE: u8 = 1 << 1;
}

pub enum WidgetKind {
    Obj,
}

pub struct Node {
    pub parent: Option<ObjRef>,
    pub children: Vec<ObjRef>,
    pub rect: Rect, // 相对父内容原点的本地坐标
    pub state: u8,
    pub flags: u8,
    pub kind: WidgetKind,
}

impl Node {
    pub fn new(parent: Option<ObjRef>, rect: Rect, kind: WidgetKind) -> Self {
        Self {
            parent,
            children: Vec::new(),
            rect,
            state: 0,
            flags: 0,
            kind,
        }
    }
}
```

`rust-lvgl/src/ui.rs`:
```rust
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
}
```

`lib.rs` 追加：
```rust
pub mod arena;
pub mod node;
pub mod ui;
pub use arena::ObjRef;
pub use ui::Ui;
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl --test tree`
Expected: 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: arena with generational handles + object tree"
```

---

### Task 3: 样式系统（扁平 Style + 状态覆盖 + theme）

**Files:**
- Create: `rust-lvgl/src/style.rs`
- Modify: `rust-lvgl/src/node.rs`（加样式字段）
- Modify: `rust-lvgl/src/ui.rs`（加样式 API）
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/style.rs`

**Interfaces:**
- Consumes: Task 2 的 `Ui`/`Node`。
- Produces:
  - `Style`（全 `Option` 字段，见下）；`Layout`（本 Task 仅 `None` 占位，`Flex`/`Grid` 变体在 Task 13 追加）
  - `ResolvedStyle { bg_color: Color, bg_opa: u8, border_color: Color, border_width: i32, radius: i32, pad_left: i32, pad_right: i32, pad_top: i32, pad_bottom: i32, text_color: Color, layout: Layout }`（全具体值）
  - theme 函数：`theme_screen() -> Style`, `theme_obj() -> Style`, `theme_button() -> Style`, `theme_button_focused() -> Style`, `theme_button_pressed() -> Style`, `theme_label() -> Style`
  - `Ui` API：`set_style(obj, style: Style)`, `set_style_pressed(obj, style: Style)`, `set_style_focused(obj, style: Style)`, `resolved_style(obj) -> ResolvedStyle`
  - 解析规则：逐字段 `状态覆盖（pressed 优先于 focused）→ 基础 style → theme 默认`

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/style.rs`:
```rust
use rust_lvgl::style::{theme_button, theme_button_pressed, Style};
use rust_lvgl::{Color, Ui};

#[test]
fn default_button_resolves_theme() {
    let mut ui = Ui::new(320, 240, 40);
    let b = ui.create_obj(ui.screen());
    ui.set_style(b, theme_button());
    let r = ui.resolved_style(b);
    assert_eq!(r.bg_color, theme_button().bg_color.unwrap());
    assert_eq!(r.bg_opa, 255);
    assert_eq!(r.border_width, theme_button().border_width.unwrap());
}

#[test]
fn base_style_field_fallback() {
    let mut ui = Ui::new(320, 240, 40);
    let o = ui.create_obj(ui.screen());
    let mut s = Style::default();
    s.bg_color = Some(Color::RED);
    ui.set_style(o, s);
    let r = ui.resolved_style(o);
    assert_eq!(r.bg_color, Color::RED);
    assert_eq!(r.bg_opa, 255); // 未设置字段落回默认
}

#[test]
fn state_override_wins_then_falls_back() {
    let mut ui = Ui::new(320, 240, 40);
    let b = ui.create_obj(ui.screen());
    let mut base = theme_button();
    base.bg_color = Some(Color::BLUE);
    ui.set_style(b, base);
    let mut pressed = theme_button_pressed();
    pressed.bg_color = Some(Color::GREEN);
    ui.set_style_pressed(b, pressed);
    assert_eq!(ui.resolved_style(b).bg_color, Color::BLUE);

    ui.set_state(b, rust_lvgl::node::state::PRESSED, true);
    assert_eq!(ui.resolved_style(b).bg_color, Color::GREEN);
    // pressed 未覆盖的字段仍回落到 base
    assert_eq!(ui.resolved_style(b).radius, base.radius.unwrap());
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test style`
Expected: 编译失败（`style` 模块 / `set_state` 不存在）

- [ ] **Step 3: 实现 style.rs 并接线**

`rust-lvgl/src/style.rs`:
```rust
use crate::geometry::Color;

/// 布局描述。Task 13 追加 Flex/Grid 变体与参数类型。
#[derive(Clone, PartialEq, Debug)]
pub enum Layout {
    None,
}

impl Default for Layout {
    fn default() -> Self {
        Layout::None
    }
}

/// 扁平样式：Option 字段，None 表示"不覆盖"。
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Style {
    pub bg_color: Option<Color>,
    pub bg_opa: Option<u8>,
    pub border_color: Option<Color>,
    pub border_width: Option<i32>,
    pub radius: Option<i32>,
    pub pad_left: Option<i32>,
    pub pad_right: Option<i32>,
    pub pad_top: Option<i32>,
    pub pad_bottom: Option<i32>,
    pub text_color: Option<Color>,
    pub layout: Option<Layout>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ResolvedStyle {
    pub bg_color: Color,
    pub bg_opa: u8,
    pub border_color: Color,
    pub border_width: i32,
    pub radius: i32,
    pub pad_left: i32,
    pub pad_right: i32,
    pub pad_top: i32,
    pub pad_bottom: i32,
    pub text_color: Color,
    pub layout: Layout,
}

impl Default for ResolvedStyle {
    fn default() -> Self {
        Self {
            bg_color: Color::BLACK,
            bg_opa: 255,
            border_color: Color::BLACK,
            border_width: 0,
            radius: 0,
            pad_left: 0,
            pad_right: 0,
            pad_top: 0,
            pad_bottom: 0,
            text_color: Color::WHITE,
            layout: Layout::None,
        }
    }
}

/// 逐字段回落：overlay -> base -> ResolvedStyle::default()
pub fn resolve(base: &Style, overlay: Option<&Style>) -> ResolvedStyle {
    let d = ResolvedStyle::default();
    let pick = |o: Option<&Style>, f: fn(&Style) -> Option<Color>| -> Option<Color> {
        o.and_then(f).or_else(|| f(base))
    };
    let pick_i = |o: Option<&Style>, f: fn(&Style) -> Option<i32>| -> Option<i32> {
        o.and_then(f).or_else(|| f(base))
    };
    let pick_u8 = |o: Option<&Style>, f: fn(&Style) -> Option<u8>| -> Option<u8> {
        o.and_then(f).or_else(|| f(base))
    };
    ResolvedStyle {
        bg_color: pick(overlay, |s| s.bg_color).unwrap_or(d.bg_color),
        bg_opa: pick_u8(overlay, |s| s.bg_opa).unwrap_or(d.bg_opa),
        border_color: pick(overlay, |s| s.border_color).unwrap_or(d.border_color),
        border_width: pick_i(overlay, |s| s.border_width).unwrap_or(d.border_width),
        radius: pick_i(overlay, |s| s.radius).unwrap_or(d.radius),
        pad_left: pick_i(overlay, |s| s.pad_left).unwrap_or(d.pad_left),
        pad_right: pick_i(overlay, |s| s.pad_right).unwrap_or(d.pad_right),
        pad_top: pick_i(overlay, |s| s.pad_top).unwrap_or(d.pad_top),
        pad_bottom: pick_i(overlay, |s| s.pad_bottom).unwrap_or(d.pad_bottom),
        text_color: pick(overlay, |s| s.text_color).unwrap_or(d.text_color),
        layout: overlay
            .and_then(|s| s.layout.clone())
            .or_else(|| base.layout.clone())
            .unwrap_or(Layout::None),
    }
}

pub fn theme_screen() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(24, 24, 32));
    s
}

pub fn theme_obj() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(40, 40, 52));
    s.radius = Some(4);
    s
}

pub fn theme_label() -> Style {
    let mut s = Style::default();
    s.text_color = Some(Color::WHITE);
    s.bg_opa = Some(0); // 透明背景
    s
}

pub fn theme_button() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(60, 90, 160));
    s.radius = Some(6);
    s.border_color = Some(Color::rgb(90, 120, 200));
    s.border_width = Some(1);
    s.text_color = Some(Color::WHITE);
    s
}

pub fn theme_button_pressed() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(40, 60, 110));
    s
}

pub fn theme_button_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s.border_width = Some(2);
    s
}
```

`node.rs`：`Node` 结构体追加字段并在 `Node::new` 初始化：
```rust
// 结构体字段追加：
    pub style: crate::style::Style,
    pub style_pressed: crate::style::Style,
    pub style_focused: crate::style::Style,
// Node::new 中初始化：
            style: crate::style::Style::default(),
            style_pressed: crate::style::Style::default(),
            style_focused: crate::style::Style::default(),
```

`ui.rs` 追加方法：
```rust
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
```

`lib.rs` 追加：`pub mod style;`（`node` 模块已在 Task 2 声明为 `pub mod node;`，确认未被改成私有）

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl --test style`
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: flat style system with state overrides and theme"
```

---

### Task 4: 脏矩形队列

**Files:**
- Create: `rust-lvgl/src/dirty.rs`
- Modify: `rust-lvgl/src/ui.rs`（挂接 DirtyQueue；几何/样式变更自动标脏）
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/dirty.rs`

**Interfaces:**
- Consumes: Task 1 `Rect`；Task 2/3 的 `Ui`。
- Produces:
  - `DirtyQueue::new(screen: Rect, cap: usize) -> DirtyQueue`, `add(&mut self, r: Rect)`, `take(&mut self) -> Vec<Rect>`, `is_empty(&self) -> bool`
  - 合并规则：与队列中任一矩形相交（或共边相邻）则 union 合并，迭代至收敛；裁剪到 screen；当不同矩形数量超过 `cap` 时整体坍缩为 screen 全屏
  - `Ui::invalidate_area(rect: Rect)`、`Ui::invalidate_obj(obj)`、`Ui::take_dirty() -> Vec<Rect>`（测试用）、`Ui::dirty_is_empty() -> bool`
  - 约定：`set_pos`/`set_size`/`set_style*`/`set_hidden` 自动标脏（旧 abs_rect ∪ 新 abs_rect）

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/dirty.rs`:
```rust
use rust_lvgl::{Rect, Ui};

#[test]
fn move_obj_marks_old_and_new_area() {
    let mut ui = Ui::new(320, 240, 40);
    ui.take_dirty(); // 清掉建屏时的全屏脏
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 10, 10);
    ui.set_size(o, 20, 20);
    ui.take_dirty();
    ui.set_pos(o, 50, 50);
    let dirty = ui.take_dirty();
    // 旧区域与新区域不相交 → 两个独立脏矩形
    assert_eq!(dirty.len(), 2);
    assert!(dirty.iter().any(|r| r.contains(rust_lvgl::Point { x: 10, y: 10 })));
    assert!(dirty.iter().any(|r| r.contains(rust_lvgl::Point { x: 60, y: 60 })));
}

#[test]
fn disjoint_areas_stay_separate_until_cap() {
    use rust_lvgl::dirty::DirtyQueue;
    let mut q = DirtyQueue::new(Rect::new(0, 0, 320, 240), 2);
    q.add(Rect::new(0, 0, 10, 10));
    q.add(Rect::new(100, 0, 10, 10));
    q.add(Rect::new(200, 0, 10, 10));
    // 超过 cap，坍缩为全屏
    assert_eq!(q.take(), vec![Rect::new(0, 0, 320, 240)]);
}

#[test]
fn area_clipped_to_screen() {
    let mut ui = Ui::new(320, 240, 40);
    ui.take_dirty();
    ui.invalidate_area(Rect::new(-50, -50, 100, 100));
    let dirty = ui.take_dirty();
    assert_eq!(dirty, vec![Rect::new(0, 0, 50, 50)]);
}

#[test]
fn style_change_invalidates_obj() {
    let mut ui = Ui::new(320, 240, 40);
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 10, 10);
    ui.set_size(o, 20, 20);
    ui.take_dirty();
    let mut s = rust_lvgl::style::Style::default();
    s.bg_color = Some(rust_lvgl::Color::RED);
    ui.set_style(o, s);
    assert_eq!(ui.take_dirty(), vec![Rect::new(10, 10, 20, 20)]);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test dirty`
Expected: 编译失败（`invalidate_area` 等不存在）

- [ ] **Step 3: 实现 dirty.rs 并挂接 Ui**

`rust-lvgl/src/dirty.rs`:
```rust
use alloc::vec::Vec;
use crate::geometry::Rect;

pub struct DirtyQueue {
    rects: Vec<Rect>,
    cap: usize,
    screen: Rect,
}

impl DirtyQueue {
    pub fn new(screen: Rect, cap: usize) -> Self {
        Self { rects: Vec::new(), cap: cap.max(1), screen }
    }

    pub fn add(&mut self, r: Rect) {
        let Some(r) = r.intersect(&self.screen) else { return };
        if r.is_empty() {
            return;
        }
        if self.rects.len() == 1 && self.rects[0] == self.screen {
            return; // 已是全屏
        }
        // 与相交或共边相邻的矩形迭代合并（共边：膨胀 1px 后相交）
        let mut cur = r;
        loop {
            let mut merged = false;
            let grown = Rect::new(cur.x - 1, cur.y - 1, cur.w + 2, cur.h + 2);
            let mut i = 0;
            while i < self.rects.len() {
                if grown.intersects(&self.rects[i]) {
                    cur = cur.union(&self.rects.remove(i));
                    merged = true;
                    break;
                }
                i += 1;
            }
            if !merged {
                break;
            }
        }
        self.rects.push(cur);
        if self.rects.len() > self.cap {
            self.rects.clear();
            self.rects.push(self.screen);
        }
    }

    pub fn take(&mut self) -> Vec<Rect> {
        core::mem::take(&mut self.rects)
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}
```

`ui.rs`：
- `Ui` 结构体追加字段：`dirty: crate::dirty::DirtyQueue`
- `Ui::new` 中初始化：`dirty: crate::dirty::DirtyQueue::new(Rect::new(0, 0, width, height), 16)`，且建屏后调用一次全屏标脏（`dirty.add(Rect::new(0,0,width,height))`）。
- 追加方法：
```rust
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
```
- 修改既有方法使其自动标脏：`set_pos` / `set_size` 改为先 `invalidate_obj(obj)`，改完后再次 `invalidate_obj(obj)`（注意借用顺序：先算 abs_rect 入 dirty，再 get_mut 改值，再算新 abs_rect 入 dirty）。`set_style` / `set_style_pressed` / `set_style_focused` / `set_state` / `set_hidden` 改为改完后 `invalidate_obj(obj)`。`delete` 在移除前 `invalidate_obj(obj)`。

`lib.rs` 追加：`pub mod dirty;`

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部测试）
Expected: geometry 6 + tree 4 + style 3 + dirty 4，全部 PASS

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: dirty rectangle queue with merge/clip/cap"
```

---

### Task 5: 绘制原语（DrawBuf，全部带 clip）

**Files:**
- Create: `rust-lvgl/src/draw.rs`
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/draw.rs`

**Interfaces:**
- Consumes: Task 1 `Rect`/`Color`/`Point`。
- Produces:
  - `DrawBuf<'a> { pixels: &'a mut [Color], area: Rect, stride: i32 }`：`area` 是缓冲对应的屏幕坐标矩形，`pixels.len() == (area.w * area.h)`，`stride == area.w`
  - 方法：`clear(c: Color)`, `fill_rect(r: Rect, c: Color, opa: u8, clip: Rect)`, `fill_rounded(r: Rect, radius: i32, c: Color, opa: u8, clip: Rect)`, `draw_border(r: Rect, width: i32, radius: i32, c: Color, opa: u8, clip: Rect)`
  - 本 Task 不含文字（draw_glyph 在 Task 7）

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/draw.rs`:
```rust
use rust_lvgl::draw::DrawBuf;
use rust_lvgl::{Color, Rect};

fn buf(w: i32, h: i32) -> (Vec<Color>, Rect) {
    (vec![Color::BLACK; (w * h) as usize], Rect::new(0, 0, w, h))
}

#[test]
fn fill_rect_basic() {
    let (mut px, area) = buf(10, 10);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    d.fill_rect(Rect::new(2, 2, 3, 3), Color::RED, 255, area);
    let at = |px: &[Color], x: i32, y: i32| px[(y * 10 + x) as usize];
    assert_eq!(at(d.pixels, 2, 2), Color::RED);
    assert_eq!(at(d.pixels, 4, 4), Color::RED);
    assert_eq!(at(d.pixels, 1, 2), Color::BLACK);
    assert_eq!(at(d.pixels, 5, 5), Color::BLACK);
}

#[test]
fn fill_rect_clipped() {
    let (mut px, area) = buf(10, 10);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    let clip = Rect::new(0, 0, 3, 10);
    d.fill_rect(Rect::new(0, 0, 10, 10), Color::RED, 255, clip);
    assert_eq!(d.pixels[(5 * 10 + 2) as usize], Color::RED);  // clip 内
    assert_eq!(d.pixels[(5 * 10 + 3) as usize], Color::BLACK); // clip 外
}

#[test]
fn fill_rect_opa_blends() {
    let (mut px, area) = buf(4, 4);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 4 };
    d.clear(Color::BLACK);
    d.fill_rect(Rect::new(0, 0, 4, 4), Color::WHITE, 128, area);
    assert_eq!(d.pixels[0], Color::rgb(128, 128, 128));
}

#[test]
fn fill_rounded_corners_cut() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.fill_rounded(Rect::new(0, 0, 20, 20), 6, Color::RED, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(0, 0), Color::BLACK);   // 角被切掉
    assert_eq!(at(10, 10), Color::RED);   // 中心保留
    assert_eq!(at(10, 0), Color::RED);    // 顶边中部保留
}

#[test]
fn draw_border_ring() {
    let (mut px, area) = buf(20, 20);
    let mut d = DrawBuf { pixels: &mut px, area, stride: 20 };
    d.draw_border(Rect::new(0, 0, 20, 20), 2, 0, Color::GREEN, 255, area);
    let at = |x: usize, y: usize| d.pixels[y * 20 + x];
    assert_eq!(at(10, 0), Color::GREEN);  // 顶边
    assert_eq!(at(10, 1), Color::GREEN);  // 宽度 2
    assert_eq!(at(10, 2), Color::BLACK);  // 内部不画
    assert_eq!(at(0, 10), Color::GREEN);  // 左边
}

#[test]
fn buffer_offset_area_coords() {
    // area 不是从 (0,0) 开始：模拟 PFB chunk（屏幕坐标 0..10 x 100..110）
    let area = Rect::new(0, 100, 10, 10);
    let mut px = vec![Color::BLACK; 100];
    let mut d = DrawBuf { pixels: &mut px, area, stride: 10 };
    d.fill_rect(Rect::new(0, 105, 10, 5), Color::RED, 255, area);
    assert_eq!(d.pixels[0], Color::BLACK); // 屏幕 y=100 行未画
    assert_eq!(d.pixels[5 * 10], Color::RED); // 屏幕 y=105 行
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test draw`
Expected: 编译失败（`draw` 模块不存在）

- [ ] **Step 3: 实现 draw.rs**

```rust
use crate::geometry::{Color, Rect};

/// 一块屏幕区域的像素缓冲。坐标一律为屏幕绝对坐标，写入时减去 area 原点。
pub struct DrawBuf<'a> {
    pub pixels: &'a mut [Color],
    pub area: Rect,
    pub stride: i32,
}

impl DrawBuf<'_> {
    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(c);
    }

    fn put(&mut self, x: i32, y: i32, c: Color, opa: u8) {
        if !self.area.contains(crate::geometry::Point { x, y }) {
            return;
        }
        let lx = x - self.area.x;
        let ly = y - self.area.y;
        let idx = (ly * self.stride + lx) as usize;
        if opa >= 255 {
            self.pixels[idx] = c;
        } else if opa > 0 {
            self.pixels[idx] = self.pixels[idx].blend(c, opa);
        }
    }

    pub fn fill_rect(&mut self, r: Rect, c: Color, opa: u8, clip: Rect) {
        let Some(r) = r.intersect(&clip).and_then(|r| r.intersect(&self.area)) else {
            return;
        };
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.put(x, y, c, opa);
            }
        }
    }

    /// 圆角实心矩形：角用整数圆判定（无抗锯齿）
    pub fn fill_rounded(&mut self, r: Rect, radius: i32, c: Color, opa: u8, clip: Rect) {
        let radius = radius.min(r.w / 2).min(r.h / 2).max(0);
        if radius == 0 {
            self.fill_rect(r, c, opa, clip);
            return;
        }
        // 中间带（覆盖全高的中央竖带）
        self.fill_rect(Rect::new(r.x + radius, r.y, r.w - 2 * radius, r.h), c, opa, clip);
        // 左右侧带（角区之间的直边段）
        self.fill_rect(Rect::new(r.x, r.y + radius, radius, r.h - 2 * radius), c, opa, clip);
        self.fill_rect(Rect::new(r.right() - radius, r.y + radius, radius, r.h - 2 * radius), c, opa, clip);
        // 四个角：圆心 (cx, cy)，距圆心 (dx, dy) 且 dx²+dy² ≤ r² 的像素在圆盘内；
        // 靠近圆心的像素被填充，外角被切掉。与中间带重叠处重复绘制无害。
        let r2 = radius * radius;
        let corners = [
            (r.x + radius, r.y + radius, -1i32, -1i32),
            (r.right() - radius - 1, r.y + radius, 1, -1),
            (r.x + radius, r.bottom() - radius - 1, -1, 1),
            (r.right() - radius - 1, r.bottom() - radius - 1, 1, 1),
        ];
        for (cx, cy, sx, sy) in corners {
            for dy in 0..=radius {
                for dx in 0..=radius {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    if dx * dx + dy * dy <= r2 {
                        self.put_clipped(cx + sx * dx, cy + sy * dy, c, opa, clip);
                    }
                }
            }
        }
    }

    fn put_clipped(&mut self, x: i32, y: i32, c: Color, opa: u8, clip: Rect) {
        if clip.contains(crate::geometry::Point { x, y }) {
            self.put(x, y, c, opa);
        }
    }

    /// 边框：外接矩形 r 的内侧画 width 宽的一圈，角半径 radius。
    /// 实现为 width 个 1px 圆角矩形描边（逐圈内缩）。
    pub fn draw_border(&mut self, r: Rect, width: i32, radius: i32, c: Color, opa: u8, clip: Rect) {
        for i in 0..width {
            let inner = Rect::new(r.x + i, r.y + i, r.w - 2 * i, r.h - 2 * i);
            if inner.is_empty() {
                break;
            }
            let rad = (radius - i).max(0).min(inner.w / 2).min(inner.h / 2);
            // 四条直边
            self.fill_rect(Rect::new(inner.x + rad, inner.y, inner.w - 2 * rad, 1), c, opa, clip);
            self.fill_rect(Rect::new(inner.x + rad, inner.bottom() - 1, inner.w - 2 * rad, 1), c, opa, clip);
            self.fill_rect(Rect::new(inner.x, inner.y + rad, 1, inner.h - 2 * rad), c, opa, clip);
            self.fill_rect(Rect::new(inner.right() - 1, inner.y + rad, 1, inner.h - 2 * rad), c, opa, clip);
            // 四个角的 1px 圆弧带
            if rad > 0 {
                let r2 = rad * rad;
                let inner2 = (rad - 1) * (rad - 1);
                let corners = [
                    (inner.x + rad, inner.y + rad, -1i32, -1i32),
                    (inner.right() - rad - 1, inner.y + rad, 1, -1),
                    (inner.x + rad, inner.bottom() - rad - 1, -1, 1),
                    (inner.right() - rad - 1, inner.bottom() - rad - 1, 1, 1),
                ];
                for (cx, cy, sx, sy) in corners {
                    for dy in 0..=rad {
                        for dx in 0..=rad {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let d2 = dx * dx + dy * dy;
                            if d2 <= r2 && d2 >= inner2 {
                                self.put_clipped(cx + sx * dx, cy + sy * dy, c, opa, clip);
                            }
                        }
                    }
                }
            }
        }
    }
}
```

> 注：`fill_rounded` 中的角判定以 `(cx, cy)` 为 1/4 圆参考点，角区为 `radius × radius` 方块。实现时保持上面的结构即可，编译警告（如未使用变量）清理干净。测试中 `fill_rounded_corners_cut` 只断言角点/中心/边中三类代表像素。

`lib.rs` 追加：`pub mod draw;`

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl --test draw`
Expected: 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: draw primitives with clip (rect, rounded, border)"
```

---

### Task 6: PFB 分块渲染 + Flush trait

**Files:**
- Create: `rust-lvgl/src/display.rs`
- Modify: `rust-lvgl/src/ui.rs`（持有像素缓冲 + flush 回调；实现 `render`）
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/render.rs`

**Interfaces:**
- Consumes: Task 2 `Ui`/树、Task 3 `resolved_style`、Task 4 脏矩形、Task 5 `DrawBuf`。
- Produces:
  - `pub trait Flush { fn flush(&mut self, area: Rect, pixels: &[Color]); }`
  - `Ui::set_flush(f: alloc::boxed::Box<dyn Flush>)`
  - `Ui::render(&mut self)`：取脏矩形 → 每个脏矩形按 `buf_rows` 行切 chunk → 每 chunk 清背景（screen 的 resolved bg）→ 先序遍历对象树绘制（跳过 hidden 与不相交对象，clip = chunk）→ `flush(chunk, &buf)`
  - 对象绘制规则（本 Task）：`bg_opa > 0` 时画 `fill_rounded(abs_rect, radius, bg_color, bg_opa)`；`border_width > 0` 时画 `draw_border`。Label 文字在 Task 7 追加。
  - chunk 像素缓冲由 `Ui` 持有：`Vec<Color>`，长度 `width * buf_rows`（`Ui::new` 的 `buf_rows` 在此启用）；最后一个 chunk 不满 `buf_rows` 行时只 flush 实际行数（`area.h` 按实际）。

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/render.rs`:
```rust
use rust_lvgl::display::Flush;
use rust_lvgl::style::theme_screen;
use rust_lvgl::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush {
    chunks: Vec<(Rect, Vec<Color>)>,
}
/// Rc 不是 fundamental type，orphan rule 要求包一层本地 newtype
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

#[test]
fn chunked_render_covers_dirty_area() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16); // 缓冲 16 行 → 全屏 48 行 = 3 chunks
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.set_style(ui.screen(), theme_screen());
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 8, 8);
    ui.set_size(o, 16, 16);
    let mut s = rust_lvgl::style::Style::default();
    s.bg_color = Some(Color::RED);
    s.bg_opa = Some(255);
    ui.set_style(o, s);

    ui.render();

    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].0, Rect::new(0, 0, 64, 16));
    assert_eq!(chunks[1].0, Rect::new(0, 16, 64, 16));
    assert_eq!(chunks[2].0, Rect::new(0, 32, 64, 16));
    // 对象在 chunk0 中：屏幕 (8,8) → 缓冲 (8,8)
    assert_eq!(chunks[0].1[8 * 64 + 8], Color::RED);
    // 对象之外是 screen 背景色
    assert_eq!(chunks[0].1[0], theme_screen().bg_color.unwrap());
}

#[test]
fn partial_last_chunk_height() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 50, 16); // 48 + 2 行
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.render();
    let chunks = &rec.borrow().chunks;
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[3].0, Rect::new(0, 48, 64, 2));
    assert_eq!(chunks[3].1.len(), (64 * 2) as usize);
}

#[test]
fn no_dirty_no_flush() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.render();
    assert_eq!(rec.borrow().chunks.len(), 3);
    ui.render(); // 无脏矩形
    assert_eq!(rec.borrow().chunks.len(), 3);
}

#[test]
fn small_dirty_flushes_only_that_area() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 16);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    ui.set_style(ui.screen(), theme_screen());
    ui.render();
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 40, 40);
    ui.set_size(o, 8, 8);
    let mut s = rust_lvgl::style::Style::default();
    s.bg_color = Some(Color::GREEN);
    ui.set_style(o, s);
    ui.render();
    let chunks = &rec.borrow().chunks;
    // 累计 3（首帧全屏）+ 1：最后一个 chunk 恰好覆盖对象脏区
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[3].0, Rect::new(40, 40, 8, 8));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test render`
Expected: 编译失败（`display` 模块 / `set_flush` / `render` 不存在）

- [ ] **Step 3: 实现 display.rs 并接线 Ui**

`rust-lvgl/src/display.rs`:
```rust
use crate::geometry::{Color, Rect};

pub trait Flush {
    /// area 为屏幕绝对坐标矩形；pixels 为 area.w*area.h 个像素（行优先，RGB888）
    fn flush(&mut self, area: Rect, pixels: &[Color]);
}
```

`ui.rs` 修改：
- `Ui` 结构体追加字段：
```rust
    flush: Option<alloc::boxed::Box<dyn crate::display::Flush>>,
    buf: Vec<crate::geometry::Color>,
```
- `Ui::new`：移除 `buf_rows` 的 `#[allow(dead_code)]`，字段改为 `buf_rows: u32`，并初始化 `buf: alloc::vec![crate::geometry::Color::BLACK; width as usize * buf_rows as usize]`、`flush: None`。
- 追加方法：
```rust
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
        // 2) 先序遍历对象树绘制
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
        if let Some(vis) = abs.intersect(&clip) {
            let _ = vis;
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
```

> 说明：`draw_node` 的递归借用安全——`self.buf` 的 `&mut` 借用都在局部作用域内结束，递归调用前已释放。screen 本身不画（背景在 render_chunk 第 1 步处理），从 screen 的子对象开始遍历。

`lib.rs` 追加：`pub mod display;`

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 既有 17 + render 4，全部 PASS

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: PFB chunked rendering with Flush trait"
```

---

### Task 7: 字体 + Label 控件

**Files:**
- Create: `rust-lvgl/src/font.rs`
- Modify: `rust-lvgl/src/draw.rs`（加 `draw_glyph`）
- Modify: `rust-lvgl/src/node.rs`（`WidgetKind` 加 `Label`）
- Modify: `rust-lvgl/src/ui.rs`（`create_label`/`set_text`/`text` + Label 绘制）
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/label.rs`

**Interfaces:**
- Consumes: Task 5 `DrawBuf`、Task 6 渲染链路、`font8x8::BASIC_FONTS`。
- Produces:
  - `font::GLYPH_W: i32 = 8`、`font::GLYPH_H: i32 = 8`、`font::glyph(ch: char) -> [u8; 8]`（非 ASCII → `'?'`）、`font::text_size(s: &str) -> (i32, i32)`（支持 `\n`，返回 宽, 高 = 行数*行高，行高 = GLYPH_H）
  - `DrawBuf::draw_text(pos: Point, s: &str, c: Color, clip: Rect)`（支持 `\n` 换行，逐像素 clip）
  - `WidgetKind::Label { text: alloc::string::String }`
  - `Ui::create_label(parent: ObjRef, text: &str) -> ObjRef`（应用 `theme_label()`，尺寸自动设为文本大小）、`Ui::set_text(obj, text: &str)`（标脏旧区域 + 更新尺寸 + 标脏新区域）、`Ui::text(obj) -> String`
  - Label 绘制：在 `abs_rect` 原点逐行 blit，颜色取 resolved `text_color`

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/label.rs`:
```rust
use rust_lvgl::display::Flush;
use rust_lvgl::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush { chunks: Vec<(Rect, Vec<Color>)> }
/// Rc 不是 fundamental type，orphan rule 要求包一层本地 newtype
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

#[test]
fn text_size_multiline() {
    let (w, h) = rust_lvgl::font::text_size("AB\nABC");
    assert_eq!(w, 3 * 8);
    assert_eq!(h, 2 * 8);
}

#[test]
fn non_ascii_falls_back_to_question_mark() {
    assert_eq!(rust_lvgl::font::glyph('中'), rust_lvgl::font::glyph('?'));
}

#[test]
fn label_renders_glyph_pixels() {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(64, 48, 48); // 单行缓冲：1 个 chunk
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = rust_lvgl::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    ui.set_style(ui.screen(), bg);
    let l = ui.create_label(ui.screen(), "A");
    ui.set_pos(l, 0, 0);
    ui.render();
    let px = &rec.borrow().chunks[0].1;
    // 'A' 的 8x8 字模：第一行 0x0C → 第 2、3 个像素点亮（bit 从低位起）
    let glyph = rust_lvgl::font::glyph('A');
    assert_eq!(glyph[0], 0x0C);
    assert_eq!(px[2], Color::WHITE); // (x=2, y=0)
    assert_eq!(px[3], Color::WHITE);
    assert_eq!(px[0], Color::BLACK);
    assert_eq!(ui.text(l), "A");
    assert_eq!(ui.rect(l).w, 8);
    assert_eq!(ui.rect(l).h, 8);
}

#[test]
fn set_text_invalidates_and_resizes() {
    let mut ui = Ui::new(64, 48, 48);
    let l = ui.create_label(ui.screen(), "A");
    ui.set_pos(l, 10, 10);
    ui.take_dirty();
    ui.set_text(l, "ABCD");
    assert_eq!(ui.rect(l).w, 32);
    let dirty = ui.take_dirty();
    // 旧区域 (10,10,8,8) 与新区域 (10,10,32,8) 共边合并
    assert_eq!(dirty.len(), 1);
    assert!(dirty[0].contains(rust_lvgl::Point { x: 41, y: 10 }));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test label`
Expected: 编译失败（`font` 模块 / `create_label` 不存在）

- [ ] **Step 3: 实现 font.rs、draw_glyph、Label**

`rust-lvgl/src/font.rs`:
```rust
use font8x8::{BASIC_FONTS, UnicodeFonts};

pub const GLYPH_W: i32 = 8;
pub const GLYPH_H: i32 = 8;
pub const LINE_H: i32 = GLYPH_H;

/// 取 8x8 字模（每行 1 字节，bit0 = 最左像素）。非 ASCII 回落 '?'。
pub fn glyph(ch: char) -> [u8; 8] {
    BASIC_FONTS
        .get(ch)
        .or_else(|| BASIC_FONTS.get('?'))
        .unwrap_or([0; 8])
}

/// 返回 (宽, 高)。支持 '\n' 换行；空串为 (0, LINE_H)。
pub fn text_size(s: &str) -> (i32, i32) {
    let mut max_w = 0i32;
    let mut lines = 0i32;
    for line in s.split('\n') {
        max_w = max_w.max(line.chars().count() as i32 * GLYPH_W);
        lines += 1;
    }
    (max_w, lines * LINE_H)
}
```

`draw.rs` 追加：
```rust
    /// 逐行绘制文本，支持 '\n'。glyph bit0 = 最左像素。
    pub fn draw_text(&mut self, pos: crate::geometry::Point, s: &str, c: Color, clip: Rect) {
        let mut y = pos.y;
        for line in s.split('\n') {
            let mut x = pos.x;
            for ch in line.chars() {
                let g = crate::font::glyph(ch);
                for row in 0..8i32 {
                    let bits = g[row as usize];
                    for col in 0..8i32 {
                        if bits & (1 << col) != 0 {
                            self.put_clipped(x + col, y + row, c, 255, clip);
                        }
                    }
                }
                x += crate::font::GLYPH_W;
            }
            y += crate::font::LINE_H;
        }
    }
```

`node.rs`：`WidgetKind` 追加变体：
```rust
pub enum WidgetKind {
    Obj,
    Label { text: alloc::string::String },
}
```
（`node.rs` 顶部需 `use alloc::string::String;` 或用全路径）

`ui.rs` 追加：
```rust
    pub fn create_label(&mut self, parent: ObjRef, text: &str) -> ObjRef {
        let (w, h) = crate::font::text_size(text);
        let r = self.arena.insert(crate::node::Node::new(
            Some(parent),
            Rect::new(0, 0, w, h),
            WidgetKind::Label { text: text.into() },
        ));
        if let Some(p) = self.arena.get_mut(parent) {
            p.children.push(r);
        }
        self.set_style(r, crate::style::theme_label());
        r
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
```

`ui.rs` 的 `draw_node` 中，在 border 绘制之后追加 Label 文字绘制（`node_draw_info` 返回的元组不够用时，直接在 `draw_node` 内对 kind 做 match）：
```rust
            if let WidgetKind::Label { text } = &self.arena.get(obj).unwrap().kind {
                let text = text.clone();
                let mut d = crate::draw::DrawBuf {
                    pixels: &mut self.buf[..len],
                    area: clip,
                    stride: clip.w,
                };
                d.draw_text(
                    crate::geometry::Point { x: abs.x, y: abs.y },
                    &text,
                    resolved.text_color,
                    clip,
                );
            }
```
（`ui.rs` 顶部 `use crate::node::WidgetKind;`）

`lib.rs` 追加：`pub mod font;`

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 全部 PASS（含 label 4 个）

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: 8x8 bitmap font + label widget"
```

---

### Task 8: 控件构造器与绘制分派（Button/Slider/Switch/Bar/List）

**Files:**
- Modify: `rust-lvgl/src/node.rs`（`WidgetKind` 加 5 个变体）
- Modify: `rust-lvgl/src/style.rs`（加控件 theme）
- Modify: `rust-lvgl/src/ui.rs`（create_* 构造器 + 值访问 API + 各控件绘制）
- Test: `rust-lvgl/tests/widgets.rs`

**Interfaces:**
- Consumes: Task 6 渲染、Task 7 文字。
- Produces:
  - `WidgetKind` 新变体：`Button { text: String }`, `Slider { min: i32, max: i32, value: i32 }`, `Switch { on: bool }`, `Bar { min: i32, max: i32, value: i32 }`, `List { items: Vec<String>, selected: usize, scroll: i32 }`（`scroll` 为像素滚动偏移）
  - `Ui` 构造器（均应用对应 theme，返回 ObjRef）：
    - `create_button(parent, text: &str) -> ObjRef`（默认尺寸由文本 + padding 推出，置 `flag::CLICKABLE`）
    - `create_slider(parent, min: i32, max: i32) -> ObjRef`（默认尺寸 100x12）
    - `create_switch(parent) -> ObjRef`（默认尺寸 40x20）
    - `create_bar(parent, min: i32, max: i32) -> ObjRef`（默认尺寸 100x8）
    - `create_list(parent, items: &[&str]) -> ObjRef`（默认宽 120，高 = 行数*行高(16) 上限 5 行）
  - 值 API（对不适用控件安全 no-op / 返回 0）：`set_value(obj, v: i32)`, `value(obj) -> i32`, `set_range(obj, min, max)`, `list_selected(obj) -> usize`, `list_select(obj, idx: usize)`
  - 各控件绘制规则（在 `draw_node` 内按 kind 分派，均先画 resolved 背景/边框）：
    - Button：圆角矩形 + 边框（通用路径已画）+ 居中文字（`text_color`）
    - Slider：轨道=背景圆角矩形；指示条=从左到 value 比例处填 `Color::rgb(80,140,255)`；旋钮=value 处 8px 宽的白色圆角竖块；编辑态（EDITED）旋钮变 `Color::rgb(255,200,60)`
    - Switch：轨道=全圆角矩形，on=`Color::rgb(60,180,90)`、off=`Color::rgb(90,90,90)`；旋钮=白色圆形方块，on 在右、off 在左
    - Bar：同 Slider 但无旋钮
    - List：背景 + 每行文本（行高 16，文本 y 居中）；selected 行填 `Color::rgb(50,70,120)` 底；可见窗口按 `scroll` 偏移裁剪（clip 用对象 abs_rect ∩ chunk）

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/widgets.rs`:
```rust
use rust_lvgl::display::Flush;
use rust_lvgl::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct RecFlush { chunks: Vec<(Rect, Vec<Color>)> }
/// Rc 不是 fundamental type，orphan rule 要求包一层本地 newtype
struct SharedFlush(Rc<RefCell<RecFlush>>);
impl Flush for SharedFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        self.0.borrow_mut().chunks.push((area, pixels.to_vec()));
    }
}

fn setup() -> (Ui, Rc<RefCell<RecFlush>>) {
    let rec = Rc::new(RefCell::new(RecFlush::default()));
    let mut ui = Ui::new(160, 120, 120);
    ui.set_flush(Box::new(SharedFlush(rec.clone())));
    let mut bg = rust_lvgl::style::Style::default();
    bg.bg_color = Some(Color::BLACK);
    ui.set_style(ui.screen(), bg);
    (ui, rec)
}

fn px(rec: &Rc<RefCell<RecFlush>>, x: i32, y: i32) -> Color {
    let chunks = &rec.borrow().chunks;
    for (area, buf) in chunks {
        if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
            return buf[((y - area.y) * area.w + (x - area.x)) as usize];
        }
    }
    panic!("pixel not flushed");
}

#[test]
fn slider_value_and_indicator() {
    let (mut ui, rec) = setup();
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.set_pos(s, 10, 10);
    ui.set_value(s, 50);
    ui.render();
    assert_eq!(ui.value(s), 50);
    // 轨道 y 中心 = 10+6，指示条到 50% ≈ x=10+50
    assert_eq!(px(&rec, 20, 16), Color::rgb(80, 140, 255));
    // 指示条末端之后是轨道色（非指示色）
    assert_ne!(px(&rec, 100, 16), Color::rgb(80, 140, 255));
    // 旋钮在 ~x=10+50-4.. 处是白色
    assert_eq!(px(&rec, 58, 16), Color::WHITE);
}

#[test]
fn slider_value_clamped_to_range() {
    let (mut ui, _) = setup();
    let s = ui.create_slider(ui.screen(), 10, 20);
    ui.set_value(s, 999);
    assert_eq!(ui.value(s), 20);
    ui.set_value(s, -5);
    assert_eq!(ui.value(s), 10);
}

#[test]
fn switch_toggle_visual() {
    let (mut ui, rec) = setup();
    let sw = ui.create_switch(ui.screen());
    ui.set_pos(sw, 10, 10);
    ui.render();
    // off：轨道灰，旋钮在左
    assert_eq!(px(&rec, 12, 20), Color::WHITE); // 旋钮左
    assert_eq!(px(&rec, 44, 20), Color::rgb(90, 90, 90)); // 右端轨道
}

#[test]
fn bar_renders_progress() {
    let (mut ui, rec) = setup();
    let b = ui.create_bar(ui.screen(), 0, 100);
    ui.set_pos(b, 10, 10);
    ui.set_value(b, 25);
    ui.render();
    assert_eq!(px(&rec, 20, 14), Color::rgb(80, 140, 255));
    assert_ne!(px(&rec, 100, 14), Color::rgb(80, 140, 255));
}

#[test]
fn list_selected_row_highlighted() {
    let (mut ui, rec) = setup();
    let l = ui.create_list(ui.screen(), &["alpha", "beta", "gamma"]);
    ui.set_pos(l, 10, 10);
    ui.list_select(l, 1);
    assert_eq!(ui.list_selected(l), 1);
    ui.render();
    // 第 2 行（beta）底色 = 高亮色。行高 16，行 1 中心 y = 10+16+8=34，文本左侧 x=12
    assert_eq!(px(&rec, 12, 34), Color::rgb(50, 70, 120));
}

#[test]
fn button_renders_text_centered() {
    let (mut ui, rec) = setup();
    let b = ui.create_button(ui.screen(), "OK");
    ui.set_pos(b, 10, 10);
    ui.render();
    let r = ui.rect(b);
    // 文字 "OK" 宽 16px，居中：起始 x = 10 + (w-16)/2；'O' 第一行有像素点亮
    assert!(r.w > 16);
    let text_x = 10 + (r.w - 16) / 2;
    let g = rust_lvgl::font::glyph('O');
    assert!(g.iter().any(|&row| row != 0));
    // 文字颜色（白）应出现在文本区域内某处
    let mut found_white = false;
    for y in 10..10 + r.h {
        for x in text_x..text_x + 16 {
            if px(&rec, x, y) == Color::WHITE {
                found_white = true;
            }
        }
    }
    assert!(found_white);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test widgets`
Expected: 编译失败（create_slider 等不存在）

- [ ] **Step 3: 实现**

`node.rs` — `WidgetKind` 扩展为：
```rust
pub enum WidgetKind {
    Obj,
    Label { text: String },
    Button { text: String },
    Slider { min: i32, max: i32, value: i32 },
    Switch { on: bool },
    Bar { min: i32, max: i32, value: i32 },
    List { items: Vec<String>, selected: usize, scroll: i32 },
}
```

`style.rs` 追加 theme：
```rust
pub fn theme_slider() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(70, 70, 80));
    s.radius = Some(6);
    s
}

pub fn theme_switch() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(90, 90, 90));
    s.radius = Some(10); // 高度 20 的全圆角
    s
}

pub fn theme_bar() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(70, 70, 80));
    s.radius = Some(4);
    s
}

pub fn theme_list() -> Style {
    let mut s = Style::default();
    s.bg_color = Some(Color::rgb(34, 34, 44));
    s.radius = Some(4);
    s.border_color = Some(Color::rgb(70, 70, 90));
    s.border_width = Some(1);
    s.text_color = Some(Color::WHITE);
    s
}

pub fn theme_list_focused() -> Style {
    let mut s = Style::default();
    s.border_color = Some(Color::WHITE);
    s
}
```

`ui.rs` 追加构造器与值 API：
```rust
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

    fn insert_node(&mut self, parent: ObjRef, rect: Rect, kind: WidgetKind) -> ObjRef {
        let r = self.arena.insert(crate::node::Node::new(Some(parent), rect, kind));
        if let Some(p) = self.arena.get_mut(parent) {
            p.children.push(r);
        }
        self.invalidate_obj(r);
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
```

`draw_node` 内按 kind 分派（在通用背景/边框之后、递归子对象之前）：
```rust
        if let Some(vis) = abs.intersect(&clip) {
            let kind_snap = self.kind_snapshot(obj);
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
                    let track = Rect::new(abs.x, abs.y, abs.w, abs.h);
                    let _ = track;
                    if iw > 0 {
                        d.fill_rounded(Rect::new(abs.x, abs.y, iw, abs.h), resolved.radius, Color::rgb(80, 140, 255), 255, clip);
                    }
                    let kx = abs.x + iw;
                    let knob = Rect::new(kx - 4, abs.y - 2, 8, abs.h + 4);
                    let edited = self.state(obj) & crate::node::state::EDITED != 0;
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
            let _ = vis;
        }
```

`kind_snapshot` 辅助（克隆 kind 以避免绘制时持有 arena 借用）：
```rust
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
```
（`WidgetKind` 无法 derive Clone 时手写此函数；上面即为手写版。同时删掉 Task 7 里 `draw_node` 中单独处理 Label 的那段代码——现在统一走 kind 分派。）

> 注意：Slider/Switch/Bar 也走通用背景路径（先画轨道色），指示条/旋钮叠加其上；Switch 的轨道色由 kind 分支里的 `tc` 覆盖（通用背景用的是 theme_switch 的灰，与 off 态一致，on 态由分支覆盖为绿——视觉正确即可）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 全部 PASS（含 widgets 6 个）

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: button/slider/switch/bar/list widgets with rendering"
```

---

### Task 9: 动画引擎

**Files:**
- Create: `rust-lvgl/src/anim.rs`
- Modify: `rust-lvgl/src/node.rs`（`Node` 加 `opa: u8`，默认 255）
- Modify: `rust-lvgl/src/ui.rs`（`tick_inc`、`timer_handler`、动画推进、opa 参与绘制）
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/anim.rs`

**Interfaces:**
- Consumes: Task 2 树、Task 4 标脏、Task 6 渲染、Task 8 `set_value`。
- Produces:
  - `AnimProp { X, Y, W, H, Opa, Value }`
  - `Easing { Linear, EaseInQuad, EaseOutQuad, EaseInOutQuad, Bounce, Overshoot }`，`Easing::eval(t: f32) -> f32`（t∈[0,1]）
  - `Anim { target: ObjRef, prop: AnimProp, start: i32, end: i32, duration_ms: u32, delay_ms: u32, repeat: i32 /* -1 = 无限 */, playback: bool, easing: Easing, on_done: Option<Box<dyn FnMut(&mut Ui)>> }`，构造辅助 `Anim::new(target, prop, start, end, duration_ms)`（其余字段取默认：delay 0, repeat 1, playback false, easing Linear, on_done None）
  - `Ui::anim_start(a: Anim)`, `Ui::anim_stop(target, prop)`（删除该目标该属性的所有动画）, `Ui::anim_running() -> bool`
  - `Ui::tick_inc(ms: u32)`（推进内部时钟）, `Ui::time() -> u64`
  - `Ui::timer_handler() -> u32`：推进动画（每帧应用值并标脏）→ 渲染 → 返回距下次需唤醒的毫秒（有动画运行返回 0，无动画返回 `u32::MAX`）
  - 属性应用：X/Y/W/H 改 `rect`；Opa 改 `node.opa`（绘制时作为乘数作用于 bg/border/文字：最终 opa = style_opa * node_opa / 255）；Value 走 `set_value` 语义（Slider/Bar）
  - 动画时序：基于 `time()` 的绝对时间轴；`elapsed = time - start_time - delay`；`t = elapsed / duration`；repeat 与 playback（往返）按 LVGL 语义：playback 时奇数轮反向

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/anim.rs`:
```rust
use rust_lvgl::anim::{Anim, AnimProp, Easing};
use rust_lvgl::Ui;

fn anim_to(target: rust_lvgl::ObjRef, prop: AnimProp, end: i32, dur: u32) -> Anim {
    Anim { target, prop, start: 0, end, duration_ms: dur, delay_ms: 0,
           repeat: 1, playback: false, easing: Easing::Linear, on_done: None }
}

#[test]
fn easing_bounds() {
    for e in [Easing::Linear, Easing::EaseInQuad, Easing::EaseOutQuad, Easing::EaseInOutQuad] {
        assert_eq!(e.eval(0.0), 0.0);
        assert!((e.eval(1.0) - 1.0).abs() < 1e-6);
    }
    assert!((Easing::Bounce.eval(1.0) - 1.0).abs() < 1e-6);
    assert!(Easing::Overshoot.eval(0.7) > 1.0); // overshoot 中后段冲过终点
}

#[test]
fn linear_anim_progresses_with_tick() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    ui.set_pos(o, 0, 0);
    ui.anim_start(anim_to(o, AnimProp::X, 100, 100));
    assert!(ui.anim_running());
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 50);
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 100);
    assert!(!ui.anim_running());
    // 结束后 timer_handler 返回 u32::MAX（无待唤醒任务）
    assert_eq!(ui.timer_handler(), u32::MAX);
}

#[test]
fn anim_with_delay() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    let mut a = anim_to(o, AnimProp::X, 100, 100);
    a.delay_ms = 100;
    ui.anim_start(a);
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 0); // delay 期间不动
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 100);
}

#[test]
fn playback_reverses() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    let mut a = anim_to(o, AnimProp::X, 100, 100);
    a.repeat = 2;
    a.playback = true;
    ui.anim_start(a);
    ui.tick_inc(100);
    ui.timer_handler(); // 第 1 轮结束 x=100
    assert_eq!(ui.rect(o).x, 100);
    ui.tick_inc(50);
    ui.timer_handler(); // 第 2 轮反向中点
    assert_eq!(ui.rect(o).x, 50);
    ui.tick_inc(50);
    ui.timer_handler();
    assert_eq!(ui.rect(o).x, 0);
    assert!(!ui.anim_running());
}

#[test]
fn anim_stop_removes() {
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    ui.anim_start(anim_to(o, AnimProp::X, 100, 1000));
    ui.anim_stop(o, AnimProp::X);
    assert!(!ui.anim_running());
}

#[test]
fn on_done_callback_fires() {
    use std::cell::Cell;
    use std::rc::Rc;
    let fired = Rc::new(Cell::new(false));
    let fired2 = fired.clone();
    let mut ui = Ui::new(64, 48, 48);
    let o = ui.create_obj(ui.screen());
    let mut a = anim_to(o, AnimProp::X, 10, 10);
    a.on_done = Some(Box::new(move |_ui: &mut Ui| fired2.set(true)));
    ui.anim_start(a);
    ui.tick_inc(10);
    ui.timer_handler();
    assert!(fired.get());
}

#[test]
fn anim_value_updates_widget_and_dirty() {
    let mut ui = Ui::new(64, 48, 48);
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.take_dirty();
    ui.anim_start(anim_to(s, AnimProp::Value, 100, 100));
    // anim_start 立即应用起始值 → 标脏（动画与脏矩形联动）
    assert!(!ui.dirty_is_empty());
    ui.tick_inc(100);
    ui.timer_handler();
    assert_eq!(ui.value(s), 100);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test anim`
Expected: 编译失败（`anim` 模块不存在）

- [ ] **Step 3: 实现 anim.rs 并接线**

`rust-lvgl/src/anim.rs`:
```rust
use alloc::boxed::Box;
use crate::arena::ObjRef;
use crate::ui::Ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnimProp {
    X,
    Y,
    W,
    H,
    Opa,
    Value,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Easing {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    Bounce,
    Overshoot,
}

impl Easing {
    pub fn eval(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => t * (2.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 { 2.0 * t * t } else { -1.0 + (4.0 - 2.0 * t) * t }
            }
            Easing::Overshoot => {
                // 先冲过 1 再回落（s=1.70158）
                let s = 1.70158f32;
                let t = t - 1.0;
                t * t * ((s + 1.0) * t + s) + 1.0
            }
            Easing::Bounce => {
                // ease-out bounce
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t = t - 1.5 / 2.75;
                    7.5625 * t * t + 0.75
                } else if t < 2.5 / 2.75 {
                    let t = t - 2.25 / 2.75;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / 2.75;
                    7.5625 * t * t + 0.984375
                }
            }
        }
    }
}

pub struct Anim {
    pub target: ObjRef,
    pub prop: AnimProp,
    pub start: i32,
    pub end: i32,
    pub duration_ms: u32,
    pub delay_ms: u32,
    pub repeat: i32, // -1 = 无限
    pub playback: bool,
    pub easing: Easing,
    pub on_done: Option<Box<dyn FnMut(&mut Ui)>>,
}

impl Anim {
    pub fn new(target: ObjRef, prop: AnimProp, start: i32, end: i32, duration_ms: u32) -> Self {
        Self {
            target, prop, start, end, duration_ms,
            delay_ms: 0, repeat: 1, playback: false,
            easing: Easing::Linear, on_done: None,
        }
    }
}

/// 运行中的动画实例（内部）
pub(crate) struct RunningAnim {
    pub anim: Anim,
    pub start_time: u64,
}
```

`node.rs`：`Node` 加字段 `pub opa: u8`，`Node::new` 初始化为 `255`。

`ui.rs`：
- `Ui` 追加字段：`time_ms: u64`、`anims: Vec<crate::anim::RunningAnim>`（`Ui::new` 初始化 `0` / 空）。
- 追加方法：
```rust
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
```
- opa 参与绘制：`draw_node` 中读出 `node.opa`（Task 9 新增字段），所有绘制调用（通用背景/边框/文字及各控件分支的固定色）的 opa 参数先经 `let apply = |base: u8| (base as u32 * node_opa as u32 / 255) as u8;` 合成。

`lib.rs` 追加：`pub mod anim;`

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 全部 PASS（含 anim 7 个）

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: animation engine with easing, repeat, playback"
```

---

### Task 10: 事件系统 + 按键输入 + 焦点组

**Files:**
- Create: `rust-lvgl/src/event.rs`
- Create: `rust-lvgl/src/input.rs`
- Modify: `rust-lvgl/src/node.rs`（`Node` 加 `events` 字段）
- Modify: `rust-lvgl/src/ui.rs`（`send_event`、`keypad_input`、焦点组状态）
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/input.rs`

**Interfaces:**
- Consumes: Task 2 树、Task 3 `set_state`、Task 8 控件。
- Produces:
  - `EventKind { Clicked, ValueChanged, Focused, Defocused, Key(Key) }`（Clone/Eq/Debug；`Key` 见下）
  - `pub type EventCb = alloc::boxed::Box<dyn FnMut(&mut Ui, ObjRef, EventKind)>`
  - `Ui::add_event_cb(obj, kind: EventKind, cb: EventCb)`（`Key(_)` 注册时匹配任意 Key）、`Ui::send_event(obj, kind: EventKind)`
  - `Key { Prev, Next, Up, Down, Left, Right, Enter, Esc }`（Copy/Eq/Debug）
  - 焦点组：`Ui::group_add(obj)`, `Ui::group_remove(obj)`, `Ui::focused() -> Option<ObjRef>`, `Ui::group_focus(obj)`, `Ui::group_focus_next()`, `Ui::group_focus_prev()`
  - `Ui::keypad_input(key: Key)` 行为：
    - 无焦点对象 → 忽略
    - 焦点对象处于 EDITED 态：Left/Right 调 Slider 值（±1）并发 ValueChanged；Enter/Esc 退出编辑态
    - 非编辑态：Next/Right/Down = 焦点前移，Prev/Left/Up = 焦点后移；Enter → 若焦点是 Slider 则进入 EDITED，若 Switch 则切换 on 并发 ValueChanged，若 List 则由 Task 12 处理（本 Task 先按 Clicked 处理），否则发 Clicked；Esc 无操作
    - 焦点移动：发 Defocused/Focused 事件 + 更新 FOCUSED 状态位（触发样式切换与标脏）
  - `set_value` 追加语义：值变化时自动 `send_event(obj, ValueChanged)`（动画驱动的 Value 属性变化同样触发）

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/input.rs`:
```rust
use rust_lvgl::input::Key;
use rust_lvgl::{EventKind, Ui};
use std::cell::RefCell;
use std::rc::Rc;

type Log = Rc<RefCell<Vec<EventKind>>>;

fn logger(log: &Log) -> impl FnMut(&mut Ui, rust_lvgl::ObjRef, EventKind) + 'static {
    let l = log.clone();
    move |_ui, _t, k| l.borrow_mut().push(k)
}

#[test]
fn focus_cycles_with_next_prev() {
    let mut ui = Ui::new(160, 120, 120);
    let a = ui.create_button(ui.screen(), "A");
    let b = ui.create_button(ui.screen(), "B");
    ui.group_add(a);
    ui.group_add(b);
    assert_eq!(ui.focused(), Some(a)); // 首个入组自动聚焦
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(b));
    ui.keypad_input(Key::Next);
    assert_eq!(ui.focused(), Some(a)); // 循环
    ui.keypad_input(Key::Prev);
    assert_eq!(ui.focused(), Some(b));
}

#[test]
fn focus_events_and_state_flag() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let a = ui.create_button(ui.screen(), "A");
    let b = ui.create_button(ui.screen(), "B");
    ui.add_event_cb(a, EventKind::Defocused, logger(&log));
    ui.add_event_cb(b, EventKind::Focused, logger(&log));
    ui.group_add(a);
    ui.group_add(b);
    ui.keypad_input(Key::Next);
    assert_eq!(*log.borrow(), vec![EventKind::Defocused, EventKind::Focused]);
    assert_eq!(ui.state(b) & rust_lvgl::node::state::FOCUSED, rust_lvgl::node::state::FOCUSED);
}

#[test]
fn enter_clicks_button() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let a = ui.create_button(ui.screen(), "A");
    ui.add_event_cb(a, EventKind::Clicked, logger(&log));
    ui.group_add(a);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
}

#[test]
fn slider_edit_mode() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let s = ui.create_slider(ui.screen(), 0, 100);
    ui.add_event_cb(s, EventKind::ValueChanged, logger(&log));
    ui.group_add(s);
    ui.keypad_input(Key::Right);
    assert_eq!(ui.value(s), 0); // 非编辑态：Right 是焦点移动（组内仅一个对象，值不变）
    ui.keypad_input(Key::Enter); // 进入编辑态
    assert_ne!(ui.state(s) & rust_lvgl::node::state::EDITED, 0);
    ui.keypad_input(Key::Right);
    assert_eq!(ui.value(s), 1);
    ui.keypad_input(Key::Right);
    ui.keypad_input(Key::Left);
    assert_eq!(ui.value(s), 1);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged, EventKind::ValueChanged, EventKind::ValueChanged]);
    ui.keypad_input(Key::Esc); // 退出编辑态
    assert_eq!(ui.state(s) & rust_lvgl::node::state::EDITED, 0);
}

#[test]
fn switch_toggles_on_enter() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let sw = ui.create_switch(ui.screen());
    ui.add_event_cb(sw, EventKind::ValueChanged, logger(&log));
    ui.group_add(sw);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
    assert_eq!(ui.value(sw), 1); // Switch 的 value：on=1 off=0
    ui.keypad_input(Key::Enter);
    assert_eq!(ui.value(sw), 0);
}

#[test]
fn set_value_fires_value_changed() {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Ui::new(160, 120, 120);
    let b = ui.create_bar(ui.screen(), 0, 100);
    ui.add_event_cb(b, EventKind::ValueChanged, logger(&log));
    ui.set_value(b, 42);
    assert_eq!(*log.borrow(), vec![EventKind::ValueChanged]);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test input`
Expected: 编译失败（`input` 模块 / `add_event_cb` 等不存在）

- [ ] **Step 3: 实现 event.rs / input.rs 并接线**

`rust-lvgl/src/event.rs`:
```rust
use alloc::boxed::Box;
use crate::arena::ObjRef;
use crate::input::Key;
use crate::ui::Ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    Clicked,
    ValueChanged,
    Focused,
    Defocused,
    Key(Key),
}

pub type EventCb = Box<dyn FnMut(&mut Ui, ObjRef, EventKind)>;
```

`rust-lvgl/src/input.rs`:
```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Prev,
    Next,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
}
```

`node.rs`：`Node` 加字段 `pub events: Vec<(crate::event::EventKind, crate::event::EventCb)>`，`Node::new` 初始化 `Vec::new()`。

`ui.rs`：
- `Ui` 追加字段：`group: Vec<ObjRef>`、`focused_idx: Option<usize>`（`Ui::new` 初始化空）。
- 事件：
```rust
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
```
其中辅助函数（`ui.rs` 内自由函数）：
```rust
/// Key 事件统一存储为占位值，匹配按类别通配（见 send_event）
fn stored_label(kind: crate::event::EventKind) -> crate::event::EventKind {
    match kind {
        crate::event::EventKind::Key(_) => crate::event::EventKind::Key(crate::input::Key::Enter),
        k => k,
    }
}
```

- 焦点组与按键：
```rust
    pub fn group_add(&mut self, obj: ObjRef) {
        if self.is_valid(obj) && !self.group.contains(&obj) {
            self.group.push(obj);
            if self.focused_idx.is_none() {
                self.focused_idx = Some(self.group.len() - 1);
                self.set_state(obj, crate::node::state::FOCUSED, true);
                self.send_event(obj, crate::event::EventKind::Focused);
            }
        }
    }
    pub fn group_remove(&mut self, obj: ObjRef) {
        if let Some(pos) = self.group.iter().position(|&o| o == obj) {
            self.group.remove(pos);
            if self.focused_idx == Some(pos) {
                self.focused_idx = None;
                self.set_state(obj, crate::node::state::FOCUSED, false);
                if !self.group.is_empty() {
                    let ni = pos.min(self.group.len() - 1);
                    self.focused_idx = Some(ni);
                    let f = self.group[ni];
                    self.set_state(f, crate::node::state::FOCUSED, true);
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
        if !self.group.is_empty() {
            let cur = self.focused_idx.unwrap_or(0);
            self.focus_to((cur + 1) % self.group.len());
        }
    }
    pub fn group_focus_prev(&mut self) {
        if !self.group.is_empty() {
            let cur = self.focused_idx.unwrap_or(0);
            self.focus_to((cur + self.group.len() - 1) % self.group.len());
        }
    }
    fn focus_to(&mut self, idx: usize) {
        if self.focused_idx == Some(idx) {
            return;
        }
        if let Some(old) = self.focused() {
            self.set_state(old, crate::node::state::FOCUSED, false);
            self.set_state(old, crate::node::state::EDITED, false);
            self.send_event(old, crate::event::EventKind::Defocused);
        }
        self.focused_idx = Some(idx);
        if let Some(new) = self.focused() {
            self.set_state(new, crate::node::state::FOCUSED, true);
            self.send_event(new, crate::event::EventKind::Focused);
        }
    }

    pub fn keypad_input(&mut self, key: crate::input::Key) {
        use crate::input::Key;
        let Some(f) = self.focused() else { return };
        if !self.is_valid(f) {
            return;
        }
        let edited = self.state(f) & crate::node::state::EDITED != 0;
        self.send_event(f, crate::event::EventKind::Key(key));
        if edited {
            match key {
                Key::Left => { let v = self.value(f); self.set_value(f, v - 1); }
                Key::Right => { let v = self.value(f); self.set_value(f, v + 1); }
                Key::Enter | Key::Esc => self.set_state(f, crate::node::state::EDITED, false),
                _ => {}
            }
            return;
        }
        match key {
            Key::Next | Key::Right | Key::Down => self.group_focus_next(),
            Key::Prev | Key::Left | Key::Up => self.group_focus_prev(),
            Key::Enter => self.activate(f),
            Key::Esc => {}
        }
    }

    fn activate(&mut self, obj: ObjRef) {
        // 按控件类型分派；List 的行为在 Task 12 扩展
        let is_slider = matches!(self.arena.get(obj).map(|n| &n.kind), Some(WidgetKind::Slider { .. }));
        let is_switch = matches!(self.arena.get(obj).map(|n| &n.kind), Some(WidgetKind::Switch { .. }));
        if is_slider {
            self.set_state(obj, crate::node::state::EDITED, true);
        } else if is_switch {
            self.toggle_switch(obj);
        } else {
            self.send_event(obj, crate::event::EventKind::Clicked);
        }
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
```
- `value()` 扩展：Switch on=1/off=0（`value()` 的 match 加 `WidgetKind::Switch { on } => *on as i32`）。
- `set_value` 追加：值实际变化后 `self.send_event(obj, EventKind::ValueChanged)`（仅当新值 != 旧值）。
- `delete()` 追加：级联删除的对象若在本 `group` 中，同步 `group_remove`（逐个处理，保证 focused_idx 正确）。

`lib.rs` 追加：`pub mod event; pub mod input; pub use event::EventKind;`

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 全部 PASS（含 input 6 个）

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: event system, keypad input, focus group with editing"
```

---

### Task 11: List 键盘导航

**Files:**
- Modify: `rust-lvgl/src/ui.rs`（`keypad_input` 中 List 截获 Up/Down；`activate` 中 List 分支）
- Test: `rust-lvgl/tests/list_nav.rs`

**Interfaces:**
- Consumes: Task 8 `list_select`/`list_selected`、Task 10 `keypad_input`。
- Produces:
  - 行为：List 获得焦点且非编辑态时，Up/Down 在列表项间移动选中（不发焦点移动，越界环绕）；Enter 发 `Clicked`（外部用 `list_selected()` 取选中项）；Next/Prev/Left/Right 仍移动焦点

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/list_nav.rs`:
```rust
use rust_lvgl::input::Key;
use rust_lvgl::{EventKind, Ui};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn up_down_navigates_items_not_focus() {
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["a", "b", "c"]);
    let btn = ui.create_button(ui.screen(), "x");
    ui.group_add(l);
    ui.group_add(btn);
    assert_eq!(ui.focused(), Some(l));
    ui.keypad_input(Key::Down);
    assert_eq!(ui.list_selected(l), 1);
    assert_eq!(ui.focused(), Some(l)); // 焦点不动
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Down); // 越界环绕
    assert_eq!(ui.list_selected(l), 0);
    ui.keypad_input(Key::Up);
    assert_eq!(ui.list_selected(l), 2);
    ui.keypad_input(Key::Next); // Next 仍移动焦点
    assert_eq!(ui.focused(), Some(btn));
}

#[test]
fn enter_on_list_fires_clicked() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let mut ui = Ui::new(160, 120, 120);
    let l = ui.create_list(ui.screen(), &["a", "b", "c"]);
    ui.add_event_cb(l, EventKind::Clicked, move |_ui, _t, k| l2.borrow_mut().push(k));
    ui.group_add(l);
    ui.keypad_input(Key::Down);
    ui.keypad_input(Key::Enter);
    assert_eq!(*log.borrow(), vec![EventKind::Clicked]);
    assert_eq!(ui.list_selected(l), 1);
}

#[test]
fn selection_keeps_visible_with_scroll() {
    let mut ui = Ui::new(160, 120, 120);
    // 8 项，可见 5 行（create_list 高度上限 5 行 = 88px）
    let l = ui.create_list(ui.screen(), &["0", "1", "2", "3", "4", "5", "6", "7"]);
    ui.group_add(l);
    for _ in 0..7 {
        ui.keypad_input(Key::Down);
    }
    assert_eq!(ui.list_selected(l), 7);
    // scroll 已下滚保证第 7 行可见：scroll > 0
    let scroll = match &ui.debug_kind(l) {
        rust_lvgl::node::WidgetKind::List { scroll, .. } => *scroll,
        _ => panic!(),
    };
    assert!(scroll > 0);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test list_nav`
Expected: 编译失败（`debug_kind` 不存在）+ 行为断言失败

- [ ] **Step 3: 实现**

`ui.rs`：
- 追加测试辅助（仅集成测试可读 kind，正式 API 不暴露）：
```rust
    /// 测试/调试用：返回对象 kind 的引用。不稳定 API。
    pub fn debug_kind(&self, obj: ObjRef) -> &WidgetKind {
        &self.arena.get(obj).expect("invalid ObjRef").kind
    }
```
- `keypad_input` 非编辑态分支改为：
```rust
        let is_list = matches!(self.arena.get(f).map(|n| &n.kind), Some(WidgetKind::List { .. }));
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
```
- 追加 `list_len`：
```rust
    pub fn list_len(&self, obj: ObjRef) -> usize {
        if let Some(n) = self.arena.get(obj) {
            if let WidgetKind::List { items, .. } = &n.kind {
                return items.len();
            }
        }
        0
    }
```
- `activate`：List 走默认 `Clicked` 分支（无需改动，确认 `is_slider`/`is_switch` 判断不影响 List 即可）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 全部 PASS（含 list_nav 3 个）

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: list item navigation via up/down keys"
```

---

### Task 12: Flex 布局

**Files:**
- Create: `rust-lvgl/src/layout.rs`
- Modify: `rust-lvgl/src/style.rs`（`Layout` 加 `Flex` 变体）
- Modify: `rust-lvgl/src/ui.rs`（`set_layout`、布局标脏、`timer_handler` 中布局 pass）
- Modify: `rust-lvgl/src/lib.rs`
- Test: `rust-lvgl/tests/flex.rs`

**Interfaces:**
- Consumes: Task 3 `Style.layout`、Task 9 `timer_handler`。
- Produces:
  - `FlexDir { Row, Column, RowReverse, ColumnReverse }`
  - `Align { Start, Center, End, SpaceBetween, SpaceAround, SpaceEvenly }`（Copy/Clone/Eq/Debug）
  - `Flex { dir: FlexDir, wrap: bool, main: Align, cross: Align, track: Align, gap: i32 }`（Clone/Debug）
  - `Layout::Flex(Flex)`
  - `Ui::set_layout(obj, layout: Layout)`（写入 `style.layout` 并标布局脏）
  - 布局时机：`timer_handler` 在渲染前，若 `layout_dirty` 则对全树执行布局 pass，然后清标志；布局本身会 `set_pos`（触发脏矩形）
  - Flex 算法（对齐 LVGL 语义，简化点见下）：
    - 子对象基准尺寸 = 其当前 `rect.w/h`（即 content size，无 grow/shrink）
    - 沿主轴累加 `gap`；`wrap=true` 且超出容器内容区主轴长度时换行/列
    - 行内按 `main` 对齐分配主轴剩余空间；按 `cross` 对齐在行内交叉轴定位；行间按 `track` 对齐分配交叉轴剩余空间（SpaceBetween/Around/Evenly 只对多行生效）
    - 容器内容原点 = `pad_left/pad_top`，内容区 = 尺寸减去 padding
    - Reverse 方向：子对象顺序反转参与布局
  - 简化（不做）：flex grow/shrink/basis 百分比尺寸

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/flex.rs`:
```rust
use rust_lvgl::layout::{Align, Flex, FlexDir};
use rust_lvgl::style::Layout;
use rust_lvgl::Ui;

fn flex(dir: FlexDir, main: Align, cross: Align, gap: i32) -> Layout {
    Layout::Flex(Flex { dir, wrap: false, main, cross, track: Align::Start, gap })
}

fn row_of(ui: &mut Ui, n: usize, w: i32, h: i32) -> Vec<rust_lvgl::ObjRef> {
    let c = ui.create_obj(ui.screen());
    ui.set_pos(c, 0, 0);
    ui.set_size(c, 200, 100);
    (0..n)
        .map(|_| {
            let ch = ui.create_obj(c);
            ui.set_size(ch, w, h);
            ch
        })
        .collect()
}

#[test]
fn row_start_gap() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let c = ui.children(ui.screen())[0];
    ui.set_layout(c, flex(FlexDir::Row, Align::Start, Align::Start, 5));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).x, 0);
    assert_eq!(ui.rect(kids[1]).x, 25);
    assert_eq!(ui.rect(kids[2]).x, 50);
}

#[test]
fn row_space_between() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 3, 20, 10);
    let c = ui.children(ui.screen())[0];
    ui.set_layout(c, flex(FlexDir::Row, Align::SpaceBetween, Align::Start, 0));
    ui.timer_handler();
    // 容器宽 200，子宽 20×3=60，剩余 140 分两间隙 = 70
    assert_eq!(ui.rect(kids[0]).x, 0);
    assert_eq!(ui.rect(kids[1]).x, 90);
    assert_eq!(ui.rect(kids[2]).x, 180);
}

#[test]
fn row_center_cross_center() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 1, 20, 10);
    let c = ui.children(ui.screen())[0];
    ui.set_layout(c, Layout::Flex(Flex {
        dir: FlexDir::Row, wrap: false,
        main: Align::Center, cross: Align::Center, track: Align::Center, gap: 0,
    }));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).x, 90); // (200-20)/2
    assert_eq!(ui.rect(kids[0]).y, 45); // (100-10)/2，track Center 把行整体居中
}

#[test]
fn column_wrap() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 4, 20, 40); // 容器高 100 → 每列 2 个
    let c = ui.children(ui.screen())[0];
    let mut f = flex(FlexDir::Column, Align::Start, Align::Start, 0);
    if let Layout::Flex(ref mut fl) = f {
        fl.wrap = true;
    }
    ui.set_layout(c, f);
    ui.timer_handler();
    assert_eq!(ui.rect(kids[0]).y, 0);
    assert_eq!(ui.rect(kids[1]).y, 40);
    assert_eq!(ui.rect(kids[2]).y, 0);  // 换列
    assert_eq!(ui.rect(kids[2]).x, 20); // 第二列 x = 列宽 20
}

#[test]
fn layout_reruns_on_size_change() {
    let mut ui = Ui::new(320, 240, 240);
    let kids = row_of(&mut ui, 2, 20, 10);
    let c = ui.children(ui.screen())[0];
    ui.set_layout(c, flex(FlexDir::Row, Align::End, Align::Start, 0));
    ui.timer_handler();
    assert_eq!(ui.rect(kids[1]).x, 180);
    ui.set_size(c, 100, 100); // 容器变小 → 布局标脏 → 下一帧重算
    ui.timer_handler();
    assert_eq!(ui.rect(kids[1]).x, 80);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test flex`
Expected: 编译失败（`layout` 模块不存在）

- [ ] **Step 3: 实现 layout.rs 并接线**

`rust-lvgl/src/layout.rs`:
```rust
use alloc::vec::Vec;
use crate::arena::ObjRef;
use crate::ui::Ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlexDir {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Flex {
    pub dir: FlexDir,
    pub wrap: bool,
    pub main: Align,
    pub cross: Align,
    pub track: Align,
    pub gap: i32,
}

/// 对容器 container 执行一次 flex 布局（直接修改子对象 rect 的 x/y）
pub fn layout_flex(ui: &mut Ui, container: ObjRef, f: &Flex) {
    let kids: Vec<ObjRef> = ui
        .children(container)
        .into_iter()
        .filter(|&k| !ui.is_hidden(k))
        .collect();
    if kids.is_empty() {
        return;
    }
    let style = ui.resolved_style(container);
    let origin_x = style.pad_left;
    let origin_y = style.pad_top;
    let area_w = ui.rect(container).w - style.pad_left - style.pad_right;
    let area_h = ui.rect(container).h - style.pad_top - style.pad_bottom;

    let is_row = matches!(f.dir, FlexDir::Row | FlexDir::RowReverse);
    let reverse = matches!(f.dir, FlexDir::RowReverse | FlexDir::ColumnReverse);
    let mut order = kids.clone();
    if reverse {
        order.reverse();
    }

    let main_of = |k: ObjRef| if is_row { ui.rect(k).w } else { ui.rect(k).h };
    let cross_of = |k: ObjRef| if is_row { ui.rect(k).h } else { ui.rect(k).w };
    let area_main = if is_row { area_w } else { area_h };

    // 分行
    let mut lines: Vec<Vec<ObjRef>> = Vec::new();
    let mut cur: Vec<ObjRef> = Vec::new();
    let mut cur_main = 0i32;
    for &k in &order {
        let m = main_of(k);
        let need = if cur.is_empty() { m } else { cur_main + f.gap + m };
        if f.wrap && !cur.is_empty() && need > area_main {
            lines.push(core::mem::take(&mut cur));
            cur_main = 0;
        }
        cur_main = if cur.is_empty() { m } else { cur_main + f.gap + m };
        cur.push(k);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }

    // 行高（交叉轴尺寸）
    let line_cross: Vec<i32> = lines
        .iter()
        .map(|l| l.iter().map(|&k| cross_of(k)).max().unwrap_or(0))
        .collect();
    let area_cross_total = if is_row { area_h } else { area_w };
    let total_cross: i32 = line_cross.iter().sum::<i32>() + f.gap * (lines.len() as i32 - 1).max(0);

    // track 对齐：行间交叉轴分布
    let (mut cross_pos, track_gap) = distribute(total_cross, area_cross_total, f.track, lines.len() as i32, f.gap);

    for (li, line) in lines.iter().enumerate() {
        let line_main: i32 = {
            let sum: i32 = line.iter().map(|&k| main_of(k)).sum();
            sum + f.gap * (line.len() as i32 - 1).max(0)
        };
        let (mut main_pos, item_gap) = distribute(line_main, area_main, f.main, line.len() as i32, f.gap);
        for &k in line {
            let m = main_of(k);
            let c = cross_of(k);
            let lc = line_cross[li];
            let cross_off = align_offset(c, lc, f.cross);
            let (x, y) = if is_row {
                (origin_x + main_pos, origin_y + cross_pos + cross_off)
            } else {
                (origin_x + cross_pos + cross_off, origin_y + main_pos)
            };
            ui.set_pos(k, x, y);
            main_pos += m + item_gap;
        }
        cross_pos += line_cross[li] + track_gap;
    }
}

/// 计算起始位置与项间距：把 content 总长按 align 放入 area
fn distribute(content: i32, area: i32, align: Align, count: i32, gap: i32) -> (i32, i32) {
    let free = (area - content).max(0);
    match align {
        Align::Start => (0, gap),
        Align::Center => (free / 2, gap),
        Align::End => (free, gap),
        Align::SpaceBetween => {
            if count > 1 {
                (0, gap + free / (count - 1))
            } else {
                (0, gap)
            }
        }
        Align::SpaceAround => {
            let g = free / count.max(1);
            (g / 2, gap + g)
        }
        Align::SpaceEvenly => {
            let g = free / (count + 1);
            (g, gap + g)
        }
    }
}

fn align_offset(item: i32, line: i32, align: Align) -> i32 {
    match align {
        Align::Start => 0,
        Align::Center => (line - item) / 2,
        Align::End => line - item,
        _ => 0, // Space* 对单item无意义
    }
}
```

`style.rs`：`Layout` 改为：
```rust
#[derive(Clone, PartialEq, Debug)]
pub enum Layout {
    None,
    Flex(crate::layout::Flex),
    // Grid 变体在 Task 13 追加
}
```

`ui.rs`：
- `Ui` 追加字段 `layout_dirty: bool`（`Ui::new` 初始化 `false`）。
- 追加：
```rust
    pub fn set_layout(&mut self, obj: ObjRef, layout: crate::style::Layout) {
        if let Some(n) = self.arena.get_mut(obj) {
            n.style.layout = Some(layout);
        }
        self.layout_dirty = true;
    }
    pub fn is_hidden(&self, obj: ObjRef) -> bool {
        self.arena.get(obj).map(|n| n.flags & crate::node::flag::HIDDEN != 0).unwrap_or(false)
    }
```
- `set_pos`/`set_size`/`set_style*`/`create_*`/`delete` 均置 `self.layout_dirty = true`（在方法末尾；注意 `insert_node` 里也要置）。
- `timer_handler` 改为：
```rust
    pub fn timer_handler(&mut self) -> u32 {
        self.step_anims();
        if self.layout_dirty {
            self.layout_pass();
            self.layout_dirty = false;
        }
        self.render();
        if self.anim_running() { 0 } else { u32::MAX }
    }
    fn layout_pass(&mut self) {
        let screen = self.screen;
        self.layout_subtree(screen);
    }
    fn layout_subtree(&mut self, obj: ObjRef) {
        let layout = self.arena.get(obj).and_then(|n| n.style.layout.clone());
        if let Some(crate::style::Layout::Flex(f)) = layout {
            crate::layout::layout_flex(self, obj, &f);
        }
        for c in self.children(obj) {
            self.layout_subtree(c);
        }
    }
```

`lib.rs` 追加：`pub mod layout;`

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 全部 PASS（含 flex 5 个）

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: flex layout (row/column, wrap, alignment)"
```

---

### Task 13: Grid 布局

**Files:**
- Modify: `rust-lvgl/src/layout.rs`（加 `Grid`/`Track` 类型与 `layout_grid`）
- Modify: `rust-lvgl/src/style.rs`（`Layout` 加 `Grid` 变体）
- Modify: `rust-lvgl/src/node.rs`（`Node` 加 `grid_col`/`grid_row`）
- Modify: `rust-lvgl/src/ui.rs`（`set_grid_cell`、layout pass 接 Grid）
- Test: `rust-lvgl/tests/grid.rs`

**Interfaces:**
- Consumes: Task 12 布局 pass 机制。
- Produces:
  - `Track { Px(i32), Fr(u8), Content }`（Copy/Clone/Eq/Debug）
  - `Grid { cols: Vec<Track>, rows: Vec<Track>, col_gap: i32, row_gap: i32 }`（Clone/Debug）
  - `Layout::Grid(Grid)`
  - `Ui::set_grid_cell(obj, col: (u8, u8), row: (u8, u8))`：(起始索引, 跨度)，跨度 ≥1
  - 算法：轨道尺寸 = Px 固定；Content = 该轨道内 span=1 子对象的最大对应尺寸（无则为 0）；剩余空间按 Fr 权重分配（剩余 = 内容区 − 固定/内容轨道 − gap，负数按 0）。子对象放在单元格原点，保持自身尺寸（不做 stretch，简化点）
  - 未设置 `grid_cell` 的子对象默认 `(0,1)/(0,1)` 之后的自动放置**不做**（简化：未设置即 `(0,1),(0,1)`，实现时文档注明）

- [ ] **Step 1: 写失败测试**

`rust-lvgl/tests/grid.rs`:
```rust
use rust_lvgl::layout::{Grid, Track};
use rust_lvgl::style::Layout;
use rust_lvgl::Ui;

fn grid(cols: Vec<Track>, rows: Vec<Track>, gap: i32) -> Layout {
    Layout::Grid(Grid { cols, rows, col_gap: gap, row_gap: gap })
}

#[test]
fn px_tracks_place_children() {
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_pos(c, 0, 0);
    ui.set_size(c, 300, 200);
    ui.set_layout(c, grid(vec![Track::Px(100), Track::Px(100)], vec![Track::Px(50), Track::Px(50)], 10));
    let a = ui.create_obj(c);
    ui.set_size(a, 10, 10);
    ui.set_grid_cell(a, (0, 1), (0, 1));
    let b = ui.create_obj(c);
    ui.set_size(b, 10, 10);
    ui.set_grid_cell(b, (1, 1), (1, 1));
    ui.timer_handler();
    assert_eq!((ui.rect(a).x, ui.rect(a).y), (0, 0));
    assert_eq!((ui.rect(b).x, ui.rect(b).y), (110, 60)); // 100+gap, 50+gap
}

#[test]
fn fr_shares_remaining_space() {
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, 300, 100);
    ui.set_layout(c, grid(vec![Track::Px(100), Track::Fr(1), Track::Fr(2)], vec![Track::Px(50)], 0));
    let a = ui.create_obj(c);
    ui.set_grid_cell(a, (1, 1), (0, 1));
    let b = ui.create_obj(c);
    ui.set_grid_cell(b, (2, 1), (0, 1));
    ui.timer_handler();
    // 剩余 200，fr1=66（200/3 取整），fr2=134
    assert_eq!(ui.rect(a).x, 100);
    let fr1 = ui.rect(b).x - 100;
    assert!((fr1 - 66).abs() <= 1);
}

#[test]
fn content_track_sizes_to_child() {
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, 300, 100);
    ui.set_layout(c, grid(vec![Track::Content, Track::Px(10)], vec![Track::Px(50)], 0));
    let a = ui.create_obj(c);
    ui.set_size(a, 42, 10);
    ui.set_grid_cell(a, (0, 1), (0, 1));
    let b = ui.create_obj(c);
    ui.set_grid_cell(b, (1, 1), (0, 1));
    ui.timer_handler();
    assert_eq!(ui.rect(b).x, 42); // content 轨道 = 最宽子对象 42
}

#[test]
fn span_places_across_tracks() {
    let mut ui = Ui::new(320, 240, 240);
    let c = ui.create_obj(ui.screen());
    ui.set_size(c, 300, 100);
    ui.set_layout(c, grid(vec![Track::Px(50), Track::Px(50)], vec![Track::Px(50)], 10));
    let a = ui.create_obj(c);
    ui.set_size(a, 10, 10);
    ui.set_grid_cell(a, (0, 2), (0, 1)); // 跨 2 列
    let b = ui.create_obj(c);
    ui.set_grid_cell(b, (1, 1), (0, 1));
    ui.timer_handler();
    assert_eq!(ui.rect(a).x, 0);
    assert_eq!(ui.rect(b).x, 60);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p rust-lvgl --test grid`
Expected: 编译失败（`Grid`/`Track` 不存在）

- [ ] **Step 3: 实现**

`layout.rs` 追加：
```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    Px(i32),
    Fr(u8),
    Content,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Grid {
    pub cols: Vec<Track>,
    pub rows: Vec<Track>,
    pub col_gap: i32,
    pub row_gap: i32,
}

/// 轨道尺寸求解：返回每条轨道的像素尺寸
fn solve_tracks(
    tracks: &[Track],
    child_sizes: &[(u8, u8, i32)], // (起始, 跨度, 尺寸)，仅 span=1 参与 Content
    gap: i32,
    area: i32,
) -> Vec<i32> {
    let mut sizes: Vec<i32> = tracks
        .iter()
        .map(|t| match t {
            Track::Px(p) => *p,
            Track::Fr(_) | Track::Content => 0,
        })
        .collect();
    // Content：取该轨道 span=1 子对象最大尺寸
    for (start, span, size) in child_sizes {
        if *span == 1 {
            if let Some(Track::Content) = tracks.get(*start as usize) {
                sizes[*start as usize] = sizes[*start as usize].max(*size);
            }
        }
    }
    let fixed: i32 = sizes.iter().sum::<i32>() + gap * (tracks.len() as i32 - 1).max(0);
    let remaining = (area - fixed).max(0);
    let fr_total: u32 = tracks
        .iter()
        .filter_map(|t| if let Track::Fr(w) = t { Some(*w as u32) } else { None })
        .sum();
    if fr_total > 0 {
        let mut used = 0i32;
        let last_fr = tracks.iter().rposition(|t| matches!(t, Track::Fr(_)));
        for (i, t) in tracks.iter().enumerate() {
            if let Track::Fr(w) = t {
                if Some(i) == last_fr {
                    sizes[i] = remaining - used; // 最后一条吃掉取整误差
                } else {
                    sizes[i] = remaining * *w as i32 / fr_total as i32;
                    used += sizes[i];
                }
            }
        }
    }
    sizes
}

fn track_offset(sizes: &[i32], idx: u8, gap: i32) -> i32 {
    sizes[..idx as usize].iter().sum::<i32>() + gap * idx as i32
}

pub fn layout_grid(ui: &mut Ui, container: ObjRef, g: &Grid) {
    let style = ui.resolved_style(container);
    let area_w = ui.rect(container).w - style.pad_left - style.pad_right;
    let area_h = ui.rect(container).h - style.pad_top - style.pad_bottom;
    let kids: Vec<ObjRef> = ui.children(container).into_iter().filter(|&k| !ui.is_hidden(k)).collect();

    let col_sizes_in: Vec<(u8, u8, i32)> = kids
        .iter()
        .map(|&k| {
            let (c, s) = ui.grid_cell(k).0;
            (c, s, ui.rect(k).w)
        })
        .collect();
    let row_sizes_in: Vec<(u8, u8, i32)> = kids
        .iter()
        .map(|&k| {
            let (r, s) = ui.grid_cell(k).1;
            (r, s, ui.rect(k).h)
        })
        .collect();

    let col_px = solve_tracks(&g.cols, &col_sizes_in, g.col_gap, area_w);
    let row_px = solve_tracks(&g.rows, &row_sizes_in, g.row_gap, area_h);

    for &k in &kids {
        let ((ci, _), (ri, _)) = ui.grid_cell(k);
        let x = style.pad_left + track_offset(&col_px, ci, g.col_gap);
        let y = style.pad_top + track_offset(&row_px, ri, g.row_gap);
        ui.set_pos(k, x, y);
    }
}
```

`node.rs`：`Node` 加字段 `pub grid_col: (u8, u8)`、`pub grid_row: (u8, u8)`，`Node::new` 初始化 `(0, 1)`、`(0, 1)`。

`style.rs`：`Layout` 加变体 `Grid(crate::layout::Grid)`。

`ui.rs`：
```rust
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
```
`layout_subtree` 的 match 加：
```rust
        if let Some(crate::style::Layout::Grid(g)) = layout {
            crate::layout::layout_grid(self, obj, &g);
        }
```
（把 Task 12 的单 `if let Flex` 改为对 Flex/Grid 两个分支都处理）

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p rust-lvgl`（全部）
Expected: 全部 PASS（含 grid 4 个）

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl
git commit -m "feat: grid layout with px/fr/content tracks"
```

---

### Task 14: 桌面模拟器（minifb）

**Files:**
- Create: `rust-lvgl-sim/Cargo.toml`
- Create: `rust-lvgl-sim/src/main.rs`
- Test: 无自动化测试（人工验收）

**Interfaces:**
- Consumes: 核心库全部公共 API。
- Produces:
  - `SimFlush`：实现 `rust_lvgl::display::Flush`，持有全屏 `Vec<u32>`（0x00RRGGBB）+ minifb `Window`；`flush` 把 chunk 像素写入对应区域，`debug_dirty` 开启时给 chunk 画 1px 绿色边框（脏矩形可视化）
  - 按键映射：方向键 → Up/Down/Left/Right；Enter → Enter；Esc → Esc；Tab → Next；Backspace → Prev
  - 主循环：每帧统计 elapsed ms → `ui.tick_inc` → 投递按键 → `ui.timer_handler()` → `window.update_with_buffer`；窗口标题显示 FPS（每秒刷新一次）；ESC+关闭按钮退出（Esc 键同时作为 UI 按键，按住 Shift+Esc 或直接关窗退出程序）
  - 常量：`WIDTH = 320, HEIGHT = 240, BUF_ROWS = 24`（1/10 屏，验证 PFB 分块）

- [ ] **Step 1: 写 Cargo.toml**

`rust-lvgl-sim/Cargo.toml`:
```toml
[package]
name = "rust-lvgl-sim"
version = "0.1.0"
edition = "2021"

[dependencies]
rust-lvgl = { path = "../rust-lvgl" }
minifb = "0.29"
```

- [ ] **Step 2: 实现 main.rs**

```rust
use minifb::{Key as MKey, Scale, Window, WindowOptions};
use rust_lvgl::display::Flush;
use rust_lvgl::input::Key;
use rust_lvgl::{Color, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

mod demo;

const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const BUF_ROWS: u32 = 24; // 1/10 屏，验证 PFB 分块

/// flush 写入共享的全屏 u32 缓冲（0x00RRGGBB）；debug_dirty 时给 chunk 画绿色 1px 边框
struct SimFlush {
    fb: Rc<RefCell<Vec<u32>>>,
    debug_dirty: bool,
}

impl Flush for SimFlush {
    fn flush(&mut self, area: Rect, pixels: &[Color]) {
        let mut fb = self.fb.borrow_mut();
        for y in 0..area.h {
            for x in 0..area.w {
                let sx = area.x + x;
                let sy = area.y + y;
                if sx >= 0 && sx < WIDTH as i32 && sy >= 0 && sy < HEIGHT as i32 {
                    let c = pixels[(y * area.w + x) as usize];
                    fb[sy as usize * WIDTH + sx as usize] =
                        ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32;
                }
            }
        }
        if self.debug_dirty {
            for x in area.x..area.right() {
                for y in [area.y, area.bottom() - 1] {
                    if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                        fb[y as usize * WIDTH + x as usize] = 0x00FF00;
                    }
                }
            }
            for y in area.y..area.bottom() {
                for x in [area.x, area.right() - 1] {
                    if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                        fb[y as usize * WIDTH + x as usize] = 0x00FF00;
                    }
                }
            }
        }
    }
}

fn map_key(k: MKey) -> Option<Key> {
    Some(match k {
        MKey::Up => Key::Up,
        MKey::Down => Key::Down,
        MKey::Left => Key::Left,
        MKey::Right => Key::Right,
        MKey::Enter => Key::Enter,
        MKey::Escape => Key::Esc,
        MKey::Tab => Key::Next,
        MKey::Backspace => Key::Prev,
        _ => return None,
    })
}

const KEYS: [MKey; 8] = [
    MKey::Up, MKey::Down, MKey::Left, MKey::Right,
    MKey::Enter, MKey::Escape, MKey::Tab, MKey::Backspace,
];

fn main() {
    let mut window = Window::new(
        "rust-lvgl sim",
        WIDTH,
        HEIGHT,
        WindowOptions { scale: Scale::X2, ..Default::default() },
    )
    .expect("open window");
    window.set_target_fps(60);

    // 共享全屏缓冲：SimFlush 写 chunk，主循环整块交给 minifb
    let fb = Rc::new(RefCell::new(vec![0u32; WIDTH * HEIGHT]));
    let mut ui = Ui::new(WIDTH as i32, HEIGHT as i32, BUF_ROWS);
    ui.set_flush(Box::new(SimFlush { fb: fb.clone(), debug_dirty: true }));
    demo::build(&mut ui);

    let mut last = Instant::now();
    let mut frames = 0u32;
    let mut fps_ts = Instant::now();

    while window.is_open() && !window.is_key_down(MKey::Q) {
        let now = Instant::now();
        ui.tick_inc(now.duration_since(last).as_millis().max(1) as u32);
        last = now;

        for &k in &KEYS {
            if window.is_key_pressed(k, minifb::KeyRepeat::No) {
                if let Some(key) = map_key(k) {
                    ui.keypad_input(key);
                }
            }
        }

        ui.timer_handler();
        window
            .update_with_buffer(&fb.borrow(), WIDTH, HEIGHT)
            .unwrap();

        frames += 1;
        if fps_ts.elapsed().as_secs() >= 1 {
            window.set_title(&format!("rust-lvgl sim — {} fps", frames));
            frames = 0;
            fps_ts = Instant::now();
        }
    }
}
```

> 说明：绿色调试边框会留在 fb 上，直到该区域再次被 flush 覆盖——视觉残留正好指示"最近刷新过的区域"，符合 demo 目的。按 Q 退出（Esc 键留给 UI 导航）。

- [ ] **Step 3: 编译验证（demo 模块先放空壳）**

`rust-lvgl-sim/src/demo.rs`（临时空壳，Task 15 填实）：
```rust
use rust_lvgl::Ui;

pub fn build(_ui: &mut Ui) {}
```

Run: `cargo build -p rust-lvgl-sim`
Expected: 编译通过；`cargo run -p rust-lvgl-sim` 能打开黑窗口，按 Q 退出

- [ ] **Step 4: Commit**

```bash
git add rust-lvgl-sim Cargo.toml Cargo.lock
git commit -m "feat: minifb simulator backend with key mapping and dirty overlay"
```

---

### Task 15: 综合 demo + 最终验证

**Files:**
- Modify: `rust-lvgl-sim/src/demo.rs`（完整 demo）
- Test: 无新增自动化测试（复用全部既有测试 + 人工验收）

**Interfaces:**
- Consumes: 全部。
- Produces: `demo::build(ui: &mut Ui)`：构建 demo 界面并注册事件/动画

- [ ] **Step 1: 实现 demo.rs**

界面结构（320×240）：
- 顶部标题 Label "rust-lvgl demo"（Grid 布局 header 行）
- 左侧 List（菜单："Settings"、"About"、"Animate"），焦点组第一项
- 右侧内容面板（Obj 容器）：
  - Settings 页：Label "Brightness" + Slider(0..100)；Label "Enabled" + Switch
  - About 页：Label 多行文本
  - Animate 页：Bar + 往返动画演示
- 页面切换：List Clicked → 隐藏/显示对应页 + 面板 X 滑入动画（`Anim` X 从 320 滑到面板 x，EaseOutQuad，200ms）
- Slider ValueChanged → 用动画把 Bar 的值同步（`AnimProp::Value`，演示动画驱动控件值）
- 焦点切换 Focused → 小动画（如焦点对象 W 轻微脉动可省略，保留 Opacity 淡入即可——简化：页面切换动画已足够展示）

完整实现（所有 API 均已在前面 Task 定义）：
```rust
use rust_lvgl::anim::{Anim, AnimProp, Easing};
use rust_lvgl::layout::{Align, Flex, FlexDir};
use rust_lvgl::style::Layout;
use rust_lvgl::{EventKind, Ui};

fn column() -> Layout {
    Layout::Flex(Flex {
        dir: FlexDir::Column, wrap: false,
        main: Align::Start, cross: Align::Start, track: Align::Start, gap: 8,
    })
}

pub fn build(ui: &mut Ui) {
    let screen = ui.screen();

    let title = ui.create_label(screen, "rust-lvgl demo");
    ui.set_pos(title, 8, 8);

    let menu = ui.create_list(screen, &["Settings", "About", "Animate"]);
    ui.set_pos(menu, 8, 32);
    ui.set_size(menu, 100, 200);

    let panel = ui.create_obj(screen);
    ui.set_pos(panel, 116, 32);
    ui.set_size(panel, 196, 200);
    ui.set_layout(panel, column());

    // ---- Settings 页：Slider + Switch + preview Bar ----
    let page_settings = ui.create_obj(panel);
    ui.set_size(page_settings, 188, 192);
    ui.set_layout(page_settings, column());
    let l1 = ui.create_label(page_settings, "Brightness");
    let _ = l1;
    let slider = ui.create_slider(page_settings, 0, 100);
    ui.set_size(slider, 160, 12);
    ui.set_value(slider, 30);
    let l2 = ui.create_label(page_settings, "Enabled");
    let _ = l2;
    let sw = ui.create_switch(page_settings);
    let l3 = ui.create_label(page_settings, "Preview");
    let _ = l3;
    let preview = ui.create_bar(page_settings, 0, 100);
    ui.set_size(preview, 160, 10);
    ui.set_value(preview, 30);
    // Slider 调值 → 动画驱动 preview Bar（演示动画与控件值联动）
    ui.add_event_cb(slider, EventKind::ValueChanged, move |ui, s, _| {
        let v = ui.value(s);
        let cur = ui.value(preview);
        ui.anim_start(Anim::new(preview, AnimProp::Value, cur, v, 300));
    });

    // ---- About 页：多行文本 ----
    let page_about = ui.create_obj(panel);
    ui.set_size(page_about, 188, 192);
    ui.set_layout(page_about, column());
    let la = ui.create_label(
        page_about,
        "rust-lvgl subset\nPFB + dirty rect\nanim + keypad\n\narrows/tab: move\nenter: select/edit\nesc: exit edit",
    );
    let _ = la;

    // ---- Animate 页：无限往返动画的 Bar ----
    let page_animate = ui.create_obj(panel);
    ui.set_size(page_animate, 188, 192);
    ui.set_layout(page_animate, column());
    let bar = ui.create_bar(page_animate, 0, 100);
    ui.set_size(bar, 160, 10);
    let mut a = Anim::new(bar, AnimProp::Value, 0, 100, 1200);
    a.easing = Easing::EaseInOutQuad;
    a.repeat = -1;
    a.playback = true;
    ui.anim_start(a);

    ui.set_hidden(page_about, true);
    ui.set_hidden(page_animate, true);

    // 菜单点击 → 切页 + 面板滑入动画
    ui.add_event_cb(menu, EventKind::Clicked, move |ui, m, _| {
        let idx = ui.list_selected(m);
        ui.set_hidden(page_settings, idx != 0);
        ui.set_hidden(page_about, idx != 1);
        ui.set_hidden(page_animate, idx != 2);
        ui.set_pos(panel, 320, 32);
        let mut a = Anim::new(panel, AnimProp::X, 320, 116, 200);
        a.easing = Easing::EaseOutQuad;
        ui.anim_start(a);
    });

    // 焦点组：菜单 → slider → switch
    ui.group_add(menu);
    ui.group_add(slider);
    ui.group_add(sw);
}
```

> 已知限制（接受，不修）：焦点组不跳过 hidden 对象——切到 About/Animate 页后 Tab 仍会聚焦隐藏的 slider/switch。规格第一阶段不含此行为。

- [ ] **Step 2: 全量自动化验证**

Run: `cargo test`（workspace 全量）
Expected: 全部 PASS

- [ ] **Step 3: no_std 目标验证**

```bash
rustup target add thumbv7em-none-eabihf
cargo build -p rust-lvgl --target thumbv7em-none-eabihf
```
Expected: 编译通过（验证核心库不依赖 std）。若因 `getrandom`/`font8x8` feature 出问题，修正 feature 配置直至通过。

- [ ] **Step 4: 人工验收（执行者在报告中列出结果，由用户确认）**

Run: `cargo run -p rust-lvgl-sim`
检查项：
- 窗口显示标题、菜单、Settings 页；脏矩形绿框只在变化区域出现
- Tab/方向键移动焦点（焦点样式：白色加粗边框）；Up/Down 在菜单项间移动
- Enter 切页，面板滑入动画平滑；切页后只有面板区域重绘
- 焦点到 Slider 后 Enter 进入编辑（旋钮变黄），Left/Right 调值，Bar 跟随动画，Esc 退出
- Switch Enter 切换开关
- Animate 页 Bar 往返动画；窗口标题 FPS 接近 60

- [ ] **Step 5: Commit**

```bash
git add rust-lvgl-sim
git commit -m "feat: comprehensive demo (menu, pages, animation, keypad nav)"
```

---

## 附录：验收清单（对应规格 §1）

- [ ] `cargo run -p rust-lvgl-sim` 综合 demo 可运行（List 菜单 + Slider/Switch + 页面切换动画）
- [ ] 键盘导航焦点组工作（方向键/Tab/Enter/Esc，Slider 编辑态，List 项导航）
- [ ] 脏矩形可视化：只有变化区域出现绿框
- [ ] PFB：模拟器以 1/10 屏缓冲运行（`BUF_ROWS = 24`），渲染正确
- [ ] `cargo test` 全绿
- [ ] `cargo build -p rust-lvgl --target thumbv7em-none-eabihf` 通过
