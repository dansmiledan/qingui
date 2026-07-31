use core::any::Any;

use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::input::Key;
use crate::ui::Ui;

use super::{TickOut, WidgetCtx};

/// 用户自定义 widget：经 Ui::create_custom 挂载为 WidgetKind::Custom，
/// 与内置控件一样参与绘制/逐帧/按键。
///
/// 注意：on_key 调用期间本节点的 kind 处于"拆出"状态（节点内是占位 Obj），
/// 修改自身状态请直接改 self；对其他节点的操作不受限。
pub trait Widget {
    /// 内容绘制（背景/边框/opa 由 Ui 统一处理）
    fn draw(&self, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect);
    /// 每帧推进：返回活动状态（默认无逐帧行为）
    fn tick(&mut self, _now: u64) -> TickOut {
        TickOut::IDLE
    }
    /// 按键处理：返回 true 表示消费（默认不消费，走默认移焦/Clicked）
    fn on_key(&mut self, _ui: &mut Ui, _obj: ObjRef, _key: Key) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
