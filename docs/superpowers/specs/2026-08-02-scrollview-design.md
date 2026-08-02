# ScrollView 滚动容器设计

日期:2026-08-02
状态:已与用户确认

## 目标

新增独立的 ScrollView widget(滚动容器),并用它修复 demo About 页内容溢出(图片占满后 gif 无位置)的问题。

## 已确认的决策

| 决策点 | 结论 |
|---|---|
| 形态 | 独立 ScrollView widget(视口 + content 两层,与 ItemList 同构) |
| 交互 | 容器聚焦滚动:ScrollView 进焦点组,聚焦时 Up/Down 滚内容;子控件不进焦点组 |
| 方向 | 仅垂直滚动 |

## 结构

```rust
// qingui/src/widgets/scrollview.rs
pub struct ScrollViewState {
    pub(crate) content: ObjRef,
    pub scroll: i32, // ≤0,视口顶到内容顶的偏移
}
```

- 视口节点带 `CLIP_CHILDREN`(子树裁剪到视口内,复用现有 draw 管线)。
- content 是普通 Obj(WidgetKind::Obj),默认 column flex;用户可 set_layout 覆盖。
- 滚动 = `set_translate(content, 0, scroll)`;scroll 钳在 `[-(content_h - view_h).max(0), 0]`,content_h 取子节点最大底边(与 itemlist ensure_visible 同款算法)。
- `WidgetKind::ScrollView(ScrollViewState)` 经 define_widgets! 注册;`WidgetBehavior` 只需 on_key(draw 空,内容由子节点绘制;overflow = 0 因 CLIP)。

## API

```rust
ScrollViewBuilder::new()
    .size(w, h) // 默认 120x100
    .build(&mut ui, parent) // -> 视口 ObjRef

// 扩展 trait(进 prelude)
ui.scrollview_content(sv) -> Option<ObjRef> // 子控件往 content 里加
ui.scrollview_scroll_to(sv, y: i32)         // 程序化滚动(自动 clamp)
```

## 交互

- ScrollView 进焦点组(ui.group_add(sv));聚焦时 Up/Down 按固定步进 20px 滚动,`KeyOutcome::Consumed`,即时生效(无动画)。
- **边界**:scrollview 内的子控件不应进焦点组(容器聚焦模型)。聚焦/失焦样式由现有 FOCUSED 机制自然呈现。

## 明确不做(YAGNI)

- 横向滚动、滚动条指示器、平滑滚动动画、惯性/触摸、子控件聚焦跟随(ensure_visible)

## About 页修复(demo)

- Wide 按钮留在页面上方(scrollview 外,可聚焦);文本与两张图(haizei 117x120 + miao 80x80)放入 ScrollView(GROW 占剩余空间)。
- scrollview 加入焦点组。

## 测试(host 端,tests/scrollview.rs)

- builder:默认尺寸;scrollview_content 返回 content 且为其子节点。
- 按键:聚焦后 Up/Down 改变 scroll(经 translate 断言),Consumed 不发 Clicked;未聚焦不影响。
- clamp:滚到底/顶不再变化;内容不足一屏时 scroll 恒 0。
- 渲染:内容超出部分被 CLIP_CHILDREN 裁掉(像素断言,参照 tests/clip.rs 模式)。
- scrollview_scroll_to 程序化滚动 + clamp。

## 影响面

- `qingui/src/widgets/scrollview.rs`:新建
- `qingui/src/widgets/mod.rs`:mod 声明 + define_widgets! 一行
- `qingui/src/lib.rs`:prelude 加 UiScrollViewExt
- `qingui/tests/scrollview.rs`:新建
- `qingui/examples/demo.rs`:About 页改造
