# Widget 注册与 API 简化设计（宏 + update + 扩展 trait）

日期:2026-08-01
状态:已与用户确认方案(宏 + update + 扩展 trait)

## 目标

把"每加一个 widget"的成本从「改 mod.rs 全部 match + ui.rs 加方法」降为「一个新文件 + 宏里加一行」。对外 API 语义与名称不变,纯内部结构重构。

## 现状问题

- `WidgetKind` 的行为分发(draw/tick/on_key/value/set_value/set_range/overflow)是手写 match,每加一个 widget 要碰所有方法。
- widget 专属 API(list_*、chart_*、itemlist_* 等)全部堆在 ui.rs,且每个都要手写"查 arena → as_xxx_mut → 改 → invalidate"四步。

## 设计

### 1. WidgetBehavior trait(行为接口统一)

```rust
// widgets/mod.rs
pub(crate) trait WidgetBehavior {
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect); // 无默认:必须实现
    fn tick(&mut self, _now: u64) -> TickOut { TickOut::IDLE }
    fn on_key(&mut self, _key: Key, _ctx: KeyCtx) -> KeyOutcome { KeyOutcome::Pass }
    fn value(&self) -> i32 { 0 }
    fn set_value(&mut self, _v: i32) -> bool { false }
    fn set_range(&mut self, _min: i32, _max: i32) {}
    fn overflow(&self) -> i32 { 0 }
}
```

- 每个 `XxxState` 实现该 trait,只覆盖自己有的行为(空行为的 widget 如 Msgbox/ItemList/Obj 只需一个空 draw)。
- **draw 无默认**是有意的取舍:保留"新 widget 忘了画就编译错"的安全网;其余行为本就大多数 widget 没有,给默认。
- Custom 逃生舱:新增 `CustomState(pub Box<dyn custom::Widget>)`,把 trait object 的委托收进 WidgetBehavior 实现,宏即可对它一视同仁。

### 2. define_widgets! 宏(消灭手写 match)

```rust
define_widgets! {
    Obj(obj::ObjState),
    Label(label::LabelState),
    Chart(chart::ChartState),
    Custom(custom::CustomState),
    ...
}
```

宏生成:
- `pub enum WidgetKind` 全部变体;
- `impl WidgetKind` 的 7 个行为方法(各自一行 match 委托到 WidgetBehavior);
- `as_xxx`/`as_xxx_mut` 访问器;
- `downcast_mut<T: 'static>`(TypeId 比对 + Any downcast,供 update 用;core::any,no_std 可用)。

WidgetKind 保持 pub(现有状态类型已 pub)。Obj 目前无状态,补一个单元结构 `ObjState` 让宏对所有变体一视同仁。

### 3. Ui::update(唯一受控 mutation 入口)

```rust
// ui.rs
impl Ui {
    /// 唯一下发 &mut widget 状态的入口:f 返回 true 表示有变更 → 标脏
    pub fn update<T: 'static>(&mut self, obj: ObjRef, f: impl FnOnce(&mut T) -> bool) {
        let mut changed = false;
        if let Some(n) = self.arena.get_mut(obj) {
            if let Some(s) = n.kind.downcast_mut::<T>() {
                changed = f(s);
            }
        }
        if changed {
            self.invalidate_obj(obj);
        }
    }
    pub(crate) fn kind(&self, obj: ObjRef) -> Option<&WidgetKind>; // 供扩展 trait 做只读 getter
}
```

无效 ObjRef / 类型不符:静默 no-op(与现风格一致)。

### 4. 扩展 trait(widget 专属 API 搬回各自文件)

```rust
// widgets/chart.rs
pub trait UiChartExt {
    fn chart_push(&mut self, c: ObjRef, series: usize, v: i32);
    fn chart_add_series(&mut self, c: ObjRef, color: Color, capacity: usize) -> usize;
    /* ...全部 chart_* 方法... */
}
impl UiChartExt for Ui { /* 写操作走 update,只读走 kind() */ }
```

- 迁移范围:全部 widget 专属 API(list_*、chart_*、itemlist_*、roller_*、dropdown_*、spinbox_*、table_*、msgbox_*、label 的 set_text 等),逐个搬到对应 widget 文件的扩展 trait,**方法名与语义不变**。
- 留在 Ui 上的:通用 API(set_value/set_range/set_style*/set_hidden/几何/布局/焦点/动画/事件/渲染)。
- 调用方兼容:`lib.rs` 增加 `pub mod prelude`(pub use 全部扩展 trait),demo/tests 一行 `use qingui::prelude::*;` 即可;crate 内 Ui 自身使用处(ui.rs 内调用 list_select 等)改走 trait 或保留内部私有辅助。

### 5. 完成后加 widget 的全成本

1. 新建 `widgets/foo.rs`:FooState + `impl WidgetBehavior` + FooBuilder + `UiFooExt`;
2. `define_widgets!` 里加一行;
3. prelude 加一行。

ui.rs 与 mod.rs 的手写代码零改动。

## 明确不做

- 不改任何对外行为/语义(纯重构,全部测试应保持绿);
- 不把 kind 换成 Box<dyn Widget>(封闭集合 + 嵌入式,enum 是正确选择);
- 不为扩展 trait 做分 feature 裁剪(YAGNI)。

## 影响面

- `qingui/src/widgets/mod.rs`:WidgetBehavior + define_widgets! 宏,删手写 impl
- `qingui/src/widgets/*.rs`:每个文件加 impl WidgetBehavior + 对应 UiXxxExt(方法从 ui.rs 搬入)
- `qingui/src/widgets/custom.rs`:CustomState 包装
- `qingui/src/ui.rs`:加 update/kind/downcast 支持,删已迁移的 widget 专属方法
- `qingui/src/lib.rs`:prelude 模块
- `qingui/tests/*.rs`、`qingui/examples/*.rs`:use 路径调整

## 验证

- `cargo test -p qingui` 全绿(现有 171 测试即行为契约,不新增功能测试;为 update/downcast 补 1-2 个单测)
- `cargo build -p qingui --target thumbv7em-none-eabihf` 通过
- `cargo check --examples` 通过
