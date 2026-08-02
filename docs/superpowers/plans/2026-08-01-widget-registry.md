# Widget 注册与 API 简化(宏 + update + 扩展 trait)实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按 `docs/superpowers/specs/2026-08-01-widget-registry-design.md` 重构:define_widgets! 宏生成 WidgetKind 分发,WidgetBehavior trait 统一行为接口,Ui::update 唯一受控 mutation 入口,widget 专属 API 以扩展 trait 搬回各自文件。

**Architecture:** 纯内部重构,对外方法名与语义不变,现有 171 个测试即行为契约。宏不用外部依赖(不用 paste),访问器名显式传入。

**Tech Stack:** Rust no_std + alloc(core::any::TypeId/Any);host 端 `cargo test -p qingui`,嵌入式 `cargo build -p qingui --target thumbv7em-none-eabihf`,`cargo check -p qingui --examples`。

## Global Constraints

- 不改任何对外行为/语义;现有测试全部保持绿(只允许因 use 路径调整的机械改动)。
- no_std + alloc;禁止新增外部依赖(宏纯手工,不用 paste)。
- 热路径无每帧分配。
- 无效 ObjRef / 类型不符:静默 no-op。
- 中文注释,风格与现有代码一致;commit message 用中文。
- 每个 Task 结束:`cargo test -p qingui` 全绿 + thumbv7em 编译通过 + `cargo check -p qingui --examples` 通过。
- Custom widget 的 on_key 特殊路径(take-call-putback,在 ui.rs 而非分发中)不在本次重构范围,保持原样。

---

### Task 1: WidgetBehavior + define_widgets! 宏 + 全 widget 行为迁移

**Files:**
- Modify: `qingui/src/widgets/mod.rs`(删手写 enum 与 impl,加 trait + 宏 + clamp helpers)
- Modify: `qingui/src/widgets/obj.rs`(加 ObjState)
- Modify: `qingui/src/widgets/spinner.rs`(加 SpinnerState)
- Modify: `qingui/src/widgets/custom.rs`(加 CustomState)
- Modify: `qingui/src/widgets/{itemlist,label,button,slider,switch,bar,list,arc,checkbox,msgbox,led,table,spinbox,roller,dropdown}.rs`(各加 impl WidgetBehavior)
- Modify: `qingui/src/ui.rs`(`WidgetKind::Custom(w)` 匹配点、`WidgetKind::Obj` 构造点适配)
- Modify: 各 builder 中 `WidgetKind::Obj` / `WidgetKind::Custom(...)` 构造点(msgbox.rs、itemlist.rs、canvas.rs、spinner.rs 等,编译器会指出全部位置)

**Interfaces:**
- Consumes: 现有 `WidgetCtx/TickOut/KeyCtx/KeyOutcome`(mod.rs 保留不动);各 widget 现有 draw 自由函数与 state 方法。
- Produces(后续 Task 依赖):
  - `pub(crate) trait WidgetBehavior`(签名见下)
  - `pub enum WidgetKind`(变体全部带 payload,`Custom(custom::CustomState)`、`Obj(obj::ObjState)`、`Spinner(spinner::SpinnerState)`)
  - `impl WidgetKind`: `draw/tick/on_key/value/set_value/set_range/overflow`(pub(crate),签名与现状逐一相同)+ `as_xxx/as_xxx_mut`(pub,与现状同名同签名)+ `pub(crate) fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T>`
  - `custom::CustomState(pub alloc::boxed::Box<dyn custom::Widget>)`
  - `obj::ObjState`(单元结构)、`spinner::SpinnerState`(单元结构)

- [ ] **Step 1: mod.rs 写入 trait + helpers + 宏定义**

在 `qingui/src/widgets/mod.rs` 中,删除手写 `pub enum WidgetKind` 与整个 `impl WidgetKind`(保留 WidgetCtx/TickOut/KeyCtx/KeyOutcome 与 mod 声明),替换为:

```rust
/// 控件行为接口:draw 必须实现(新 widget 忘了画会编译错),
/// 其余行为大多数控件没有,给默认空实现。
pub(crate) trait WidgetBehavior {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect);
    fn tick(&mut self, _now: u64) -> TickOut { TickOut::IDLE }
    fn on_key(&mut self, _key: Key, _ctx: KeyCtx) -> KeyOutcome { KeyOutcome::Pass }
    fn value(&self) -> i32 { 0 }
    fn set_value(&mut self, _v: i32) -> bool { false }
    fn set_range(&mut self, _min: i32, _max: i32) {}
    fn overflow(&self) -> i32 { 0 }
}

/// set_value 共用:clamp 到 [min,max],返回是否有变化
pub(crate) fn clamp_val(min: i32, max: i32, value: &mut i32, v: i32) -> bool {
    let nv = v.clamp(min, max);
    let changed = nv != *value;
    *value = nv;
    changed
}

/// 选择型控件共用:clamp 到 [0,len),返回是否有变化
pub(crate) fn select_clamp(len: usize, selected: &mut usize, v: i32) -> bool {
    if len == 0 { return false; }
    let nv = (v.max(0) as usize).min(len - 1);
    let changed = nv != *selected;
    *selected = nv;
    changed
}

/// 声明式注册 widget:生成 enum、行为分发、as_xxx 访问器、downcast。
/// 每加一个 widget 只需在此处加一行。
macro_rules! define_widgets {
    ($($variant:ident($state:ty, $as:ident, $as_mut:ident)),+ $(,)?) => {
        pub enum WidgetKind {
            $( $variant($state), )+
        }

        impl WidgetKind {
            pub(crate) fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::draw(s, ctx, d, clip), )+ }
            }
            pub(crate) fn overflow(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::overflow(s), )+ }
            }
            pub(crate) fn value(&self) -> i32 {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::value(s), )+ }
            }
            pub(crate) fn set_value(&mut self, v: i32) -> bool {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_value(s, v), )+ }
            }
            pub(crate) fn set_range(&mut self, min: i32, max: i32) {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::set_range(s, min, max), )+ }
            }
            pub(crate) fn tick(&mut self, now: u64) -> TickOut {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::tick(s, now), )+ }
            }
            pub(crate) fn on_key(&mut self, key: Key, ctx: KeyCtx) -> KeyOutcome {
                match self { $( WidgetKind::$variant(s) => WidgetBehavior::on_key(s, key, ctx), )+ }
            }
            $(
                pub fn $as(&self) -> Option<&$state> {
                    match self { WidgetKind::$variant(s) => Some(s), _ => None }
                }
                pub fn $as_mut(&mut self) -> Option<&mut $state> {
                    match self { WidgetKind::$variant(s) => Some(s), _ => None }
                }
            )+
            /// 按类型下发 &mut 状态(Ui::update 用);TypeId 比对 + Any downcast
            pub(crate) fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
                $(
                    if core::any::TypeId::of::<T>() == core::any::TypeId::of::<$state>() {
                        if let WidgetKind::$variant(s) = self {
                            return (s as &mut dyn core::any::Any).downcast_mut::<T>();
                        }
                    }
                )+
                None
            }
        }
    };
}

define_widgets! {
    Obj(obj::ObjState, as_obj, as_obj_mut),
    ItemList(itemlist::ItemListState, as_itemlist, as_itemlist_mut),
    Label(label::LabelState, as_label, as_label_mut),
    Button(button::ButtonState, as_button, as_button_mut),
    Slider(slider::SliderState, as_slider, as_slider_mut),
    Switch(switch::SwitchState, as_switch, as_switch_mut),
    Bar(bar::BarState, as_bar, as_bar_mut),
    List(list::ListState, as_list, as_list_mut),
    Arc(arc::ArcState, as_arc, as_arc_mut),
    Checkbox(checkbox::CheckboxState, as_checkbox, as_checkbox_mut),
    Spinner(spinner::SpinnerState, as_spinner, as_spinner_mut),
    Msgbox(msgbox::MsgboxState, as_msgbox, as_msgbox_mut),
    Led(led::LedState, as_led, as_led_mut),
    Table(table::TableState, as_table, as_table_mut),
    Spinbox(spinbox::SpinboxState, as_spinbox, as_spinbox_mut),
    Roller(roller::RollerState, as_roller, as_roller_mut),
    Dropdown(dropdown::DropdownState, as_dropdown, as_dropdown_mut),
    Custom(custom::CustomState, as_custom_state, as_custom_state_mut),
}
```

注意:原 `as_custom`/`as_custom_mut`(返回 `&dyn custom::Widget`)是 pub(crate) 且在 ui.rs 有调用点,宏生成的是 `as_custom_state`(返回 &CustomState)。保留一个手写适配:

```rust
impl WidgetKind {
    pub(crate) fn as_custom(&self) -> Option<&dyn custom::Widget> {
        match self { WidgetKind::Custom(s) => Some(s.0.as_ref()), _ => None }
    }
    pub(crate) fn as_custom_mut(&mut self) -> Option<&mut dyn custom::Widget> {
        match self { WidgetKind::Custom(s) => Some(s.0.as_mut()), _ => None }
    }
}
```

- [ ] **Step 2: 新增 ObjState / SpinnerState / CustomState**

obj.rs 末尾加:

```rust
/// 占位状态:Obj 无数据,仅为让宏对所有变体一视同仁
pub struct ObjState;

impl super::WidgetBehavior for ObjState {
    fn draw(&self, _ctx: &super::WidgetCtx, _d: &mut DrawBuf, _clip: Rect) {}
}
```

spinner.rs 加:

```rust
pub struct SpinnerState;

impl super::WidgetBehavior for SpinnerState {
    fn draw(&self, ctx: &super::WidgetCtx, d: &mut DrawBuf, clip: Rect) { draw(ctx, d, clip) }
    fn tick(&mut self, _now: u64) -> super::TickOut { super::TickOut::ACTIVE }
}
```

custom.rs 加:

```rust
/// Custom 变体的状态包装:把 trait object 委托收进 WidgetBehavior,宏即可一视同仁
pub struct CustomState(pub alloc::boxed::Box<dyn Widget>);

impl super::WidgetBehavior for CustomState {
    fn draw(&self, ctx: &super::WidgetCtx, d: &mut DrawBuf, clip: Rect) { self.0.draw(ctx, d, clip) }
    fn tick(&mut self, now: u64) -> super::TickOut { self.0.tick(now) }
}
```

- [ ] **Step 3: 每个 widget 文件加 impl WidgetBehavior(逐字按下表)**

行为与现有 match 臂**逐一对应,不得增减**。下表"draw 委托"列为 impl 的方法体(ctx/d/clip 为参数名);未列的行为=用 trait 默认,不写。

| 文件 | impl 内容 |
|---|---|
| itemlist.rs | draw: `{}`(空);on_key: `self.on_key(key, ctx)`(调现有同名方法,加前缀区分:把现有 `pub(crate) fn on_key` 改名 `on_key_impl` 后委托,或直接内联 `match key { Key::Up => KeyOutcome::NavSelect(-1), Key::Down => KeyOutcome::NavSelect(1), _ => KeyOutcome::Pass }`);value: `self.selected as i32` |
| label.rs | draw: `super::label::draw(&self.text, ctx, d, clip)` —— 注意在文件内即 `draw(&self.text, ctx, d, clip)`(自由函数与 impl 方法同名不冲突,自由函数直接写 `draw(&self.text, ctx, d, clip)`) |
| button.rs | draw: `draw(&self.text, ctx, d, clip)` |
| slider.rs | draw: `draw(self.min, self.max, self.value, ctx, d, clip)`;on_key: 现有 `s.on_key(key, ctx)` 同款委托(改名 on_key_impl 或内联);value: `self.value`;set_value: `super::clamp_val(self.min, self.max, &mut self.value, v)`;set_range: `self.min = min; self.max = max; self.value = self.value.clamp(min, max);`;overflow: `4` |
| switch.rs | draw: `draw(self.on, ctx, d, clip)`;on_key: 委托现有;value: `self.on as i32` |
| bar.rs | draw: `draw(self.min, self.max, self.value, ctx, d, clip)`;value: `self.value`;set_value: clamp_val 同款;set_range: 同 slider |
| list.rs | draw: `draw(&self.items, self.selected, self.scroll, &self.fx, ctx, d, clip)`;tick: `self.tick(now)`(现有 ListState::tick 改名 tick_impl 委托或保持同名——inherent 方法优先于 trait 方法,`self.tick(now)` 在 trait impl 内会调到 inherent 版,无需改名);on_key: 委托现有; |
| arc.rs | draw: `draw(self.min, self.max, self.value, ctx, d, clip)`;value;set_value clamp_val;overflow: `4` |
| checkbox.rs | draw: `draw(&self.text, self.checked, ctx, d, clip)`;on_key 委托;value: `self.checked as i32`;set_value: `let nv = v != 0; let c = nv != self.checked; self.checked = nv; c` |
| msgbox.rs | draw: `{}`(空) |
| led.rs | draw: `draw(self.color, self.bright, ctx, d, clip)`;value: `self.bright as i32`;set_value: `let nv = v.clamp(0, 255) as u8; let c = nv != self.bright; self.bright = nv; c` |
| table.rs | draw: `draw(self.cols, self.rows, &self.cells, ctx, d, clip)` |
| spinbox.rs | draw: `draw(self.min, self.max, self.value, self.digits, self.cursor, ctx, d, clip)`;on_key 委托;value;set_value clamp_val |
| roller.rs | draw: `draw(&self.items, self.selected, self.sel_from, ctx, d, clip)`;tick 委托(inherent 优先);on_key 委托;value: `self.selected as i32`;set_value: `super::select_clamp(self.items.len(), &mut self.selected, v)` |
| dropdown.rs | draw: `draw(&self.items, self.selected, ctx, d, clip)`;on_key 委托;value: `self.selected as i32`;set_value: `super::select_clamp(self.items.len(), &mut self.selected, v)` |

关键提示:on_key/tick 与各 State 现有 inherent 方法同名时,**inherent 方法优先**,trait impl 里 `self.on_key(key, ctx)`/`self.tick(now)` 会正确调到 inherent 版,不会递归(trait 方法须用全限定语法才调得到)。value() 无 inherent 冲突。

- [ ] **Step 4: 修构造点与 Custom 匹配点**

- 全部 `WidgetKind::Obj` 构造 → `WidgetKind::Obj(obj::ObjState)`(msgbox.rs 的 row、itemlist.rs 的 viewport/content、canvas.rs 等;builders 在各自文件,用 `super::obj::ObjState` 或 `crate::widgets::obj::ObjState` 路径)。
- spinner.rs build 中 `WidgetKind::Spinner` → `WidgetKind::Spinner(SpinnerState)`。
- `WidgetKind::Custom(boxed)` 构造点(ui.rs create_custom、canvas 等)→ `WidgetKind::Custom(custom::CustomState(boxed))`。
- ui.rs 中匹配 `WidgetKind::Custom(w)` 处改匹配 `WidgetKind::Custom(_)` 或经 as_custom/as_custom_mut。
- 依赖编译器找全:`cargo check -p qingui 2>&1 | grep "^error"` 迭代到零 error。

- [ ] **Step 5: 全量验证**

Run: `cargo test -p qingui 2>&1 | grep -oE "[0-9]+ passed" | awk '{s+=$1} END {print s}'`(应 171)
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(应为空)
Run: `cargo build -p qingui --target thumbv7em-none-eabihf` 与 `cargo check -p qingui --examples`
Expected: 全绿

- [ ] **Step 6: Commit**

```bash
git add -A qingui/src
git commit -m "refactor(widgets): define_widgets! 宏 + WidgetBehavior 统一行为分发

声明式注册替代手写 enum/match:每加一个 widget 从'改全部 match
臂'降为'宏里加一行'。draw 无默认实现保留编译期安全网,其余行为
走 trait 默认。Custom 包装为 CustomState 与内置控件一视同仁,其
on_key 特殊路径(take-call-putback)保持不变。纯重构,行为不变。"
```

---

### Task 2: Ui::update + kind/kind_mut 受控入口

**Files:**
- Modify: `qingui/src/ui.rs`
- Test: `qingui/tests/registry.rs`(新建)

**Interfaces:**
- Consumes: Task 1 的 `WidgetKind::downcast_mut`、`as_chart`。
- Produces(后续 Task 依赖):
  - `pub fn update<T: 'static, R>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> R) -> Option<R>` —— downcast 成功则执行 f、标脏并返回 Some(R);否则 None 不标脏。
  - `pub(crate) fn kind(&self, obj: ObjRef) -> Option<&WidgetKind>`
  - `pub(crate) fn kind_mut(&mut self, obj: ObjRef) -> Option<&mut WidgetKind>`

- [ ] **Step 1: 写失败测试**

新建 `qingui/tests/registry.rs`:

```rust
use qingui::widgets::bar::BarBuilder;
use qingui::widgets::chart::{ChartBuilder, ChartState};
use qingui::{Color, Ui};

#[test]
fn update_mutates_and_invalidates() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let c = ChartBuilder::new().series(Color::BLUE, 4).build(&mut ui, s);
    ui.take_dirty();
    let r = ui.update::<ChartState, _>(c, |st| {
        st.series[0].push(7);
        st.series.len()
    });
    assert_eq!(r, Some(1));
    assert!(!ui.dirty_is_empty()); // 执行过 f → 标脏
}

#[test]
fn update_wrong_type_is_noop() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let b = BarBuilder::new(0, 100).build(&mut ui, s); // BarState,不是 ChartState
    ui.take_dirty();
    let r = ui.update::<ChartState, _>(b, |st| st.series.len());
    assert_eq!(r, None);
    assert!(ui.dirty_is_empty());
}

#[test]
fn update_deleted_obj_is_noop() {
    let mut ui = Ui::new(64, 64, 16);
    let s = ui.screen();
    let c = ChartBuilder::new().series(Color::BLUE, 4).build(&mut ui, s);
    ui.delete(c);
    ui.take_dirty();
    let r = ui.update::<ChartState, _>(c, |st| st.series.len());
    assert_eq!(r, None);
    assert!(ui.dirty_is_empty());
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p qingui --test registry 2>&1 | tail -3`
Expected: 编译失败(Ui 没有 update 方法)

- [ ] **Step 3: 实现(ui.rs)**

```rust
    pub(crate) fn kind(&self, obj: ObjRef) -> Option<&crate::widgets::WidgetKind> {
        self.arena.get(obj).map(|n| &n.kind)
    }
    pub(crate) fn kind_mut(&mut self, obj: ObjRef) -> Option<&mut crate::widgets::WidgetKind> {
        self.arena.get_mut(obj).map(|n| &mut n.kind)
    }

    /// 唯一下发 &mut widget 状态的入口:类型匹配则执行 f 并标脏,
    /// 返回 f 的返回值;无效对象/类型不符静默返回 None。
    pub fn update<T: 'static, R>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let r = match self.arena.get_mut(obj) {
            Some(n) => n.kind.downcast_mut::<T>().map(f),
            None => None,
        };
        if r.is_some() {
            self.invalidate_obj(obj);
        }
        r
    }
```

注意:与原逐个方法"仅实际变更才标脏"相比,update 是"执行即标脏"——多一次重绘,无害,语义更简单。

- [ ] **Step 4: 跑测试确认通过 + 全量回归**

Run: `cargo test -p qingui --test registry 2>&1 | tail -3`(3 个 PASS)
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(应为空)
Run: `cargo build -p qingui --target thumbv7em-none-eabihf`

- [ ] **Step 5: Commit**

```bash
git add qingui/src/ui.rs qingui/tests/registry.rs
git commit -m "feat(ui): update<T> 唯一受控 mutation 入口 + kind/kind_mut

widget 专属 API 不再需要手写'查 arena → as_mut → 改 → invalidate'
四步:downcast 成功则执行闭包并自动标脏,无效目标静默 None。"
```

---

### Task 3: chart_* 迁移为 UiChartExt(立 pattern)

**Files:**
- Modify: `qingui/src/widgets/chart.rs`(加 UiChartExt)
- Modify: `qingui/src/ui.rs`(删 chart_add_series/chart_push/chart_set_points/chart_clear/chart_point_count/chart_point,约 ui.rs:934-1000)
- Modify: `qingui/src/lib.rs`(加 prelude)
- Modify: `qingui/tests/chart.rs`、`qingui/examples/demo.rs`(use 调整)

**Interfaces:**
- Consumes: Task 2 的 `update`/`kind`。
- Produces: `qingui::widgets::chart::UiChartExt`(6 个方法,签名与现 ui.rs 完全一致);`qingui::prelude`(本 Task 先含 UiChartExt,后续 Task 追加)。

- [ ] **Step 1: chart.rs 加扩展 trait(方法体从 ui.rs 原样搬移,仅改状态访问方式)**

```rust
/// chart 数据 API(经 prelude 或显式 use 引入)
pub trait UiChartExt {
    fn chart_add_series(&mut self, c: ObjRef, color: Color, capacity: usize) -> usize;
    fn chart_push(&mut self, c: ObjRef, series: usize, v: i32);
    fn chart_set_points(&mut self, c: ObjRef, series: usize, points: &[i32]);
    fn chart_clear(&mut self, c: ObjRef, series: usize);
    fn chart_point_count(&self, c: ObjRef, series: usize) -> usize;
    fn chart_point(&self, c: ObjRef, series: usize, idx: usize) -> Option<i32>;
}

impl UiChartExt for Ui {
    fn chart_add_series(&mut self, c: ObjRef, color: Color, capacity: usize) -> usize {
        self.update::<ChartState, _>(c, move |s| {
            s.series.push(Series::new(color, capacity));
            s.series.len() - 1
        })
        .unwrap_or(0)
    }

    fn chart_push(&mut self, c: ObjRef, series: usize, v: i32) {
        self.update::<ChartState, _>(c, |s| {
            let (min, max) = (s.min, s.max);
            if let Some(ser) = s.series.get_mut(series) {
                ser.push(v.clamp(min, max));
            }
        });
    }

    fn chart_set_points(&mut self, c: ObjRef, series: usize, points: &[i32]) {
        self.update::<ChartState, _>(c, |s| {
            let (min, max) = (s.min, s.max);
            if let Some(ser) = s.series.get_mut(series) {
                let start = points.len().saturating_sub(ser.capacity);
                ser.points.clear();
                ser.points.extend(points[start..].iter().map(|&v| v.clamp(min, max)));
            }
        });
    }

    fn chart_clear(&mut self, c: ObjRef, series: usize) {
        self.update::<ChartState, _>(c, |s| {
            if let Some(ser) = s.series.get_mut(series) {
                ser.points.clear();
            }
        });
    }

    fn chart_point_count(&self, c: ObjRef, series: usize) -> usize {
        self.kind(c)
            .and_then(|k| k.as_chart())
            .and_then(|s| s.series.get(series))
            .map(|ser| ser.points.len())
            .unwrap_or(0)
    }

    fn chart_point(&self, c: ObjRef, series: usize, idx: usize) -> Option<i32> {
        self.kind(c)
            .and_then(|k| k.as_chart())
            .and_then(|s| s.series.get(series))
            .and_then(|ser| ser.points.get(idx).copied())
    }
}
```

- [ ] **Step 2: ui.rs 删除 6 个 chart_* 方法**

- [ ] **Step 3: lib.rs 加 prelude**

```rust
/// 各 widget 扩展 trait 汇总:一行引入全部 widget 专属 API
pub mod prelude {
    pub use crate::widgets::chart::UiChartExt;
}
```

- [ ] **Step 4: 修调用点**

tests/chart.rs 顶部加 `use qingui::prelude::*;`;examples/demo.rs 同样加。`cargo test -p qingui` 与 `cargo check -p qingui --examples` 迭代到零 error。

- [ ] **Step 5: 全量验证 + Commit**

Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(空)+ thumbv7em + examples check

```bash
git add -A qingui
git commit -m "refactor(chart): chart_* API 迁移为 UiChartExt,新增 prelude

扩展 trait 立在 widget 自己的文件里,基于 Ui::update;ui.rs 不再
随 widget 增长。方法名与语义不变,调用方经 qingui::prelude 引入。"
```

---

### Task 4: list/label/roller/table/msgbox 专属 API 迁移

**Files:**
- Modify: `qingui/src/widgets/list.rs`(UiListExt:list_select/list_selected/list_insert/list_remove/list_len)
- Modify: `qingui/src/widgets/label.rs`(UiTextExt:set_text/text)
- Modify: `qingui/src/widgets/roller.rs`(UiRollerExt:roller_selected)
- Modify: `qingui/src/widgets/table.rs`(UiTableExt:table_set_cell)
- Modify: `qingui/src/widgets/msgbox.rs`(UiMsgboxExt:msgbox_selected)
- Modify: `qingui/src/ui.rs`(删对应方法,约 ui.rs:819-836、874-932、1002-1010、1274-1281)
- Modify: `qingui/src/lib.rs`(prelude 追加)
- Modify: 相关 tests/examples 的 use

**Interfaces:**
- Consumes: `update`/`kind`/`kind_mut`;list.rs 现有 `select` 自由函数(list_select/list_insert 的方法体原样用它);Ui 公开方法 `invalidate_obj`、`time`、`send_event`、`children`、`rect`。
- Produces: `UiListExt`/`UiTextExt`/`UiRollerExt`/`UiTableExt`/`UiMsgboxExt`,方法名签名与现 ui.rs 一致;prelude 追加这 5 个 trait。

- [ ] **Step 1: 各文件加扩展 trait(方法体从 ui.rs 原样搬移)**

规则:
- 只读 getter(msgbox_selected/roller_selected/list_selected/list_len/text):`self.kind(obj).and_then(|k| k.as_xxx())...` 链,默认值与原代码一致(msgbox_selected 默认 -1,roller/list_selected/list_len 默认 0,text 默认空 String)。
- 写操作:
  - `list_select`/`list_insert`/`list_remove`:原方法体中 `self.invalidate_obj(obj)`(首尾)与 `self.time_ms` 保留(invalidate_obj 与 time 都是 Ui 公开方法,trait impl 里直接 `self.invalidate_obj(obj)`、`let now = self.time();`);`self.arena.get_mut(obj)` 改为 `self.kind_mut(obj)`。
  - `table_set_cell`:改为 `self.update::<TableState, _>(obj, |s| { ... 原单元格写入逻辑 ... });`。
  - `set_text`:原为 Label/Button 两分支匹配;改为 `self.kind_mut(obj)` 后按 `as_label_mut()`/`as_button_mut()` 依次尝试(任一成功即写 text 并标脏,与原语义一致)。

- [ ] **Step 2: ui.rs 删除已迁移方法;lib.rs prelude 追加**

```rust
pub mod prelude {
    pub use crate::widgets::chart::UiChartExt;
    pub use crate::widgets::label::UiTextExt;
    pub use crate::widgets::list::UiListExt;
    pub use crate::widgets::msgbox::UiMsgboxExt;
    pub use crate::widgets::roller::UiRollerExt;
    pub use crate::widgets::table::UiTableExt;
}
```

- [ ] **Step 3: 修调用点并全量验证**

tests 与 examples 中用到这些方法(list_nav.rs、list_fx.rs、label.rs、p0_widgets.rs、p1_widgets.rs、demo.rs、gallery.rs 等,编译器会指出)统一在顶部加 `use qingui::prelude::*;`。
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(空)+ thumbv7em + examples check

- [ ] **Step 4: Commit**

```bash
git add -A qingui
git commit -m "refactor(widgets): list/label/roller/table/msgbox 专属 API 迁移为扩展 trait"
```

---

### Task 5: itemlist_* 迁移为 UiItemListExt

**Files:**
- Modify: `qingui/src/widgets/itemlist.rs`(UiItemListExt:itemlist_add_item/itemlist_remove_selected/itemlist_select/itemlist_selected/itemlist_len;私有 helper itemlist_ensure_visible 随行)
- Modify: `qingui/src/ui.rs`(删对应方法,约 ui.rs:1263-1375;apply_key_outcome 的内部调用点 `use crate::widgets::itemlist::UiItemListExt`)
- Modify: `qingui/src/lib.rs`(prelude 追加 UiItemListExt)
- Modify: `qingui/tests/itemlist.rs`、`qingui/examples/demo.rs` 等 use

**Interfaces:**
- Consumes: `update`/`kind`/`kind_mut`;Ui 公开方法 `insert_node`(pub(crate))、`children`、`set_style`、`set_style_selected`、`set_state`、`send_event`、`rect`、`translate`、`set_translate`、`invalidate_obj`、`time`。
- Produces: `UiItemListExt`(5 个方法,签名与现 ui.rs 一致);prelude 追加。

- [ ] **Step 1: itemlist.rs 加扩展 trait**

规则:5 个方法体从 ui.rs 原样搬移。`self.arena.get(il)` → `self.kind(il)`(只读)、`self.arena.get_mut(il)` → `self.kind_mut(il)`;其余 Ui 公开方法调用原样保留。`itemlist_ensure_visible` 是 ui.rs 的私有方法,一并搬到 trait impl 块内作为关联私有函数(`fn ensure_visible(ui: &mut Ui, il: ObjRef)` 自由函数放 itemlist.rs,pub(crate) 不需要)。

- [ ] **Step 2: ui.rs 适配内部调用点**

ui.rs 顶部加 `use crate::widgets::itemlist::UiItemListExt;`,apply_key_outcome 中 `self.itemlist_len/selected/select` 调用原样编译(trait 方法)。删除 ui.rs 中已迁移的方法与 itemlist_ensure_visible。

- [ ] **Step 3: prelude 追加 + 修调用点 + 全量验证**

prelude 加 `pub use crate::widgets::itemlist::UiItemListExt;`;tests/itemlist.rs、examples/demo.rs 加 `use qingui::prelude::*;`。
Run: `cargo test -p qingui 2>&1 | grep -E "[1-9][0-9]* failed"`(空)+ thumbv7em + examples check

- [ ] **Step 4: Commit**

```bash
git add -A qingui
git commit -m "refactor(itemlist): itemlist_* API 迁移为 UiItemListExt

至此全部 widget 专属 API 搬离 ui.rs:新 widget 的完整成本 =
一个新文件 + define_widgets! 一行 + prelude 一行。"
```

---

## Self-Review 记录

- Spec 覆盖:WidgetBehavior(Task 1 Step 1)、宏(Task 1 Step 1,含 as_xxx 与 downcast)、CustomState/ObjState/SpinnerState(Task 1 Step 2)、逐 widget 行为不增不减(Task 1 Step 3 表格)、update/kind/kind_mut(Task 2)、chart 扩展 trait(Task 3)、list/label/roller/table/msgbox(Task 4)、itemlist(Task 5)、prelude(Task 3 起)、验证命令(每 Task)。spec 第 5 节"完成后全成本"在 Task 5 commit message 中验收。
- 占位符:无;Task 4 的"原方法体搬移"配合具体替换规则(kind/kind_mut/update/time/invalidate_obj),方法清单逐一列出。
- 类型一致性:update 签名全文一致(`<T: 'static, R> -> Option<R>`);prelude 内容三个 Task 间累加一致;trait 名 UiXxxExt 全文一致。
- 已知行为微差(已在 Task 2 注明):update"执行即标脏"替代"实际变更才标脏",多一次重绘,无害。
- Custom 的 on_key 特殊路径与 `as_custom` 适配器在 Task 1 Step 1 显式保留。
