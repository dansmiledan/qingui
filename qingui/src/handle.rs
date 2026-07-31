//! ObjRef 句柄方法：节点操作的对外主 API。
//! 每个方法都是对 Ui 内部实现的薄封装；`ui` 参数是显式的"世界"借用，
//! 使节点操作天然带无效化与布局标记。
use alloc::string::String;
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Point, Rect};
use crate::layout::{Attach, Sizing};
use crate::node::{DrawHook, State, TickHook};
use crate::style::{Layout, Style};
use crate::ui::Ui;

impl ObjRef {
    /// 设置对象位置（本地坐标；布局管理的对象位置归布局所有）
    pub fn set_pos(self, ui: &mut Ui, x: i32, y: i32) {
        ui.set_pos(self, x, y);
    }
    /// 设置对象尺寸
    pub fn set_size(self, ui: &mut Ui, w: i32, h: i32) {
        ui.set_size(self, w, h);
    }
    /// 本地矩形（相对父对象）
    pub fn rect(self, ui: &Ui) -> Rect {
        ui.rect(self)
    }
    /// 绝对矩形（屏幕坐标，含祖先与自身 translate）
    pub fn abs_rect(self, ui: &Ui) -> Rect {
        ui.abs_rect(self)
    }
    /// 设置视觉平移偏移：子树整体偏移，只影响渲染，不参与布局
    pub fn set_translate(self, ui: &mut Ui, x: i32, y: i32) {
        ui.set_translate(self, x, y);
    }
    /// 当前视觉平移偏移
    pub fn translate(self, ui: &Ui) -> Point {
        ui.translate(self)
    }
    /// 设置隐藏标志
    pub fn set_hidden(self, ui: &mut Ui, hidden: bool) {
        ui.set_hidden(self, hidden);
    }
    /// 是否隐藏（只看自身标志，不含祖先）
    pub fn is_hidden(self, ui: &Ui) -> bool {
        ui.is_hidden(self)
    }
    /// 设置基础样式
    pub fn set_style(self, ui: &mut Ui, style: Style) {
        ui.set_style(self, style);
    }
    /// 设置按下态叠加样式
    pub fn set_style_pressed(self, ui: &mut Ui, style: Style) {
        ui.set_style_pressed(self, style);
    }
    /// 设置聚焦态叠加样式
    pub fn set_style_focused(self, ui: &mut Ui, style: Style) {
        ui.set_style_focused(self, style);
    }
    /// 设置/清除状态位
    pub fn set_state(self, ui: &mut Ui, state: State, on: bool) {
        ui.set_state(self, state, on);
    }
    /// 当前状态位
    pub fn state(self, ui: &Ui) -> State {
        ui.state(self)
    }
    /// 设置宽/高尺寸策略（None = 内容尺寸）
    pub fn set_sizing(self, ui: &mut Ui, w: Option<Sizing>, h: Option<Sizing>) {
        ui.set_sizing(self, w, h);
    }
    /// 设置宽高比（千分比：1000 = 1:1；None 取消）
    pub fn set_aspect(self, ui: &mut Ui, ratio: Option<u32>) {
        ui.set_aspect(self, ratio);
    }
    /// 设置布局过渡：(时长 ms, 缓动)；None 关闭
    pub fn set_transition(self, ui: &mut Ui, transition: Option<(u32, Easing)>) {
        ui.set_transition(self, transition);
    }
    /// 设置布局（Flex/Grid）
    pub fn set_layout(self, ui: &mut Ui, layout: Layout) {
        ui.set_layout(self, layout);
    }
    /// 设置 Grid 单元格（列/行的 (起始, 跨度)）
    pub fn set_grid_cell(self, ui: &mut Ui, col: (u8, u8), row: (u8, u8)) {
        ui.set_grid_cell(self, col, row);
    }
    /// 当前 Grid 单元格
    pub fn grid_cell(self, ui: &Ui) -> ((u8, u8), (u8, u8)) {
        ui.grid_cell(self)
    }
    /// 设置叠放次序（兄弟节点按 z_index 稳定排序，大者在上）
    pub fn set_z_index(self, ui: &mut Ui, z: i16) {
        ui.set_z_index(self, z);
    }
    /// 设置浮层锚定：对象变为浮动，位置由锚点自动计算并跟随目标
    pub fn set_floating(self, ui: &mut Ui, target: ObjRef, attach: Attach) {
        ui.set_floating(self, target, attach);
    }
    /// 取消浮层锚定
    pub fn clear_floating(self, ui: &mut Ui) {
        ui.clear_floating(self);
    }
    /// 设置浮动标志：浮动对象不参与父容器布局
    pub fn set_ignore_layout(self, ui: &mut Ui, ignore: bool) {
        ui.set_ignore_layout(self, ignore);
    }
    /// 是否浮动（不参与父容器布局）
    pub fn is_ignore_layout(self, ui: &Ui) -> bool {
        ui.is_ignore_layout(self)
    }
    /// 调整自身在父对象中的顺序（触发布局重算）
    pub fn move_child_to_index(self, ui: &mut Ui, index: usize) {
        ui.move_child_to_index(self, index);
    }
    /// 设置控件值（clamp 到 range；变化时发 ValueChanged）
    pub fn set_value(self, ui: &mut Ui, v: i32) {
        ui.set_value(self, v);
    }
    /// 控件当前值
    pub fn value(self, ui: &Ui) -> i32 {
        ui.value(self)
    }
    /// 设置控件 range（值随之 clamp）
    pub fn set_range(self, ui: &mut Ui, min: i32, max: i32) {
        ui.set_range(self, min, max);
    }
    /// 标脏自身渲染区域（按控件类型外扩溢出区）
    pub fn invalidate(self, ui: &mut Ui) {
        ui.invalidate_obj(self);
    }
    /// 删除对象及其整棵子树
    pub fn delete(self, ui: &mut Ui) {
        ui.delete(self);
    }
    /// 子对象列表
    pub fn children(self, ui: &Ui) -> Vec<ObjRef> {
        ui.children(self)
    }
    /// 注册事件回调
    pub fn on(self, ui: &mut Ui, kind: EventKind, cb: EventCb) {
        ui.add_event_cb(self, kind, cb);
    }
    /// 触发事件（同步调用匹配的回调）
    pub fn send_event(self, ui: &mut Ui, kind: EventKind) {
        ui.send_event(self, kind);
    }
    /// 加入焦点组
    pub fn group_add(self, ui: &mut Ui) {
        ui.group_add(self);
    }
    /// 移出焦点组
    pub fn group_remove(self, ui: &mut Ui) {
        ui.group_remove(self);
    }
    /// 设置文本（Label/Button/Checkbox 等含文本控件）
    pub fn set_text(self, ui: &mut Ui, text: &str) {
        ui.set_text(self, text);
    }
    /// 当前文本
    pub fn text(self, ui: &Ui) -> String {
        ui.text(self)
    }
    /// List：选中 idx 项（滚动保证可见）
    pub fn list_select(self, ui: &mut Ui, idx: usize) {
        ui.list_select(self, idx);
    }
    /// List：当前选中索引
    pub fn list_selected(self, ui: &Ui) -> usize {
        ui.list_selected(self)
    }
    /// List：在 idx 处插入一项
    pub fn list_insert(self, ui: &mut Ui, idx: usize, text: &str) {
        ui.list_insert(self, idx, text);
    }
    /// List：删除当前选中项，返回是否成功
    pub fn list_remove(self, ui: &mut Ui) -> bool {
        ui.list_remove(self)
    }
    /// List：项数
    pub fn list_len(self, ui: &Ui) -> usize {
        ui.list_len(self)
    }
    /// Roller：当前选中索引
    pub fn roller_selected(self, ui: &Ui) -> usize {
        ui.roller_selected(self)
    }
    /// Msgbox：被点击的按钮索引（未点击/Esc = -1）
    pub fn msgbox_selected(self, ui: &Ui) -> i32 {
        ui.msgbox_selected(self)
    }
    /// Table：设置单元格文本
    pub fn table_set_cell(self, ui: &mut Ui, row: u8, col: u8, text: &str) {
        ui.table_set_cell(self, row, col, text);
    }
    /// Checkbox：切换勾选并发 ValueChanged
    pub fn toggle_checkbox(self, ui: &mut Ui) {
        ui.toggle_checkbox(self);
    }
    /// Switch：切换开关并发 ValueChanged
    pub fn toggle_switch(self, ui: &mut Ui) {
        ui.toggle_switch(self);
    }
    /// 叠加绘制钩子：在控件自带内容之后调用
    pub fn on_draw(self, ui: &mut Ui, hook: DrawHook) {
        ui.set_draw_hook(self, Some(hook));
    }
    /// 清除叠加绘制钩子
    pub fn clear_draw_hook(self, ui: &mut Ui) {
        ui.set_draw_hook(self, None);
    }
    /// 每帧钩子：返回 true 的帧标脏并保持唤醒
    pub fn on_tick(self, ui: &mut Ui, hook: TickHook) {
        ui.set_tick_hook(self, Some(hook));
    }
    /// 清除每帧钩子
    pub fn clear_tick_hook(self, ui: &mut Ui) {
        ui.set_tick_hook(self, None);
    }
    /// 只读访问 List 状态（非 List 返回 None）
    pub fn as_list(self, ui: &Ui) -> Option<&crate::widgets::list::ListState> {
        ui.arena.get(self).and_then(|n| n.kind.as_list())
    }
    /// 只读访问 Roller 状态（非 Roller 返回 None）
    pub fn as_roller(self, ui: &Ui) -> Option<&crate::widgets::roller::RollerState> {
        ui.arena.get(self).and_then(|n| n.kind.as_roller())
    }
    /// 只读查询自定义 widget 状态（类型不匹配或对象非 Custom 返回 None）
    pub fn custom<T: 'static>(self, ui: &Ui) -> Option<&T> {
        ui.custom(self)
    }
    /// 可变更新自定义 widget 状态（前后自动标脏）
    pub fn custom_mut<T: 'static, R>(self, ui: &mut Ui, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        ui.custom_mut(self, f)
    }
}
