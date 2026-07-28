use alloc::string::String;
use alloc::vec::Vec;

use crate::draw::DrawBuf;
use crate::geometry::Rect;
use crate::style::ResolvedStyle;

pub mod bar;
pub mod button;
pub mod label;
pub mod list;
pub mod slider;
pub mod switch;

#[derive(Clone)]
pub enum WidgetKind {
    Obj,
    Label { text: String },
    Button { text: String },
    Slider { min: i32, max: i32, value: i32 },
    Switch { on: bool },
    Bar { min: i32, max: i32, value: i32 },
    List { items: Vec<String>, selected: usize, scroll: i32 },
}

/// 控件绘制上下文：通用部分（背景/边框）由 Ui::draw_node 处理，
/// 各控件 draw 只画自己的内容。
pub struct WidgetCtx<'a> {
    pub abs: Rect,
    pub resolved: &'a ResolvedStyle,
    pub edited: bool,
    pub opa: u8, // node opa 0..=255
}

impl WidgetCtx<'_> {
    /// 与节点 opa 合成后的透明度
    pub fn ap(&self, base: u8) -> u8 {
        (base as u32 * self.opa as u32 / 255) as u8
    }
}

pub(crate) fn draw(kind: &WidgetKind, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    match kind {
        WidgetKind::Obj => {}
        WidgetKind::Label { text } => label::draw(text, ctx, d, clip),
        WidgetKind::Button { text } => button::draw(text, ctx, d, clip),
        WidgetKind::Slider { min, max, value } => slider::draw(*min, *max, *value, ctx, d, clip),
        WidgetKind::Switch { on } => switch::draw(*on, ctx, d, clip),
        WidgetKind::Bar { min, max, value } => bar::draw(*min, *max, *value, ctx, d, clip),
        WidgetKind::List { items, selected, scroll } => list::draw(items, *selected, *scroll, ctx, d, clip),
    }
}
