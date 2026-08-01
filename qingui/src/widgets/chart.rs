use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::anim::Easing;
use crate::arena::ObjRef;
use crate::draw::DrawBuf;
use crate::event::{EventCb, EventKind};
use crate::geometry::{Color, Point, Rect};
use crate::layout::Sizing;
use crate::style::Style;
use crate::ui::Ui;
use super::{WidgetCtx, WidgetKind};

/// 一条数据线：定容环形缓冲，满时挤掉最旧点
pub struct Series {
    pub color: Color,
    pub capacity: usize, // ≥1（传 0 钳到 1）
    pub points: VecDeque<i32>,
}

impl Series {
    pub fn new(color: Color, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { color, capacity, points: VecDeque::with_capacity(capacity) }
    }
    /// 追加一点（调用方已 clamp 到 range）；满则挤掉最旧
    pub fn push(&mut self, v: i32) {
        if self.points.len() == self.capacity {
            self.points.pop_front();
        }
        self.points.push_back(v);
    }
}

pub struct ChartState {
    pub min: i32, // Y 轴固定范围
    pub max: i32,
    pub series: Vec<Series>,
}

/// 只画数据线（背景/边框由通用 draw_node 处理）：
/// 相邻点连线，单点序列画圆点，空序列跳过。无分配。
pub(crate) fn draw(s: &ChartState, ctx: &WidgetCtx, d: &mut DrawBuf, clip: Rect) {
    let abs = ctx.abs;
    if abs.w < 1 || abs.h < 1 {
        return;
    }
    let (min, max) = (s.min, s.max);
    for ser in &s.series {
        let n = ser.points.len();
        if n == 0 {
            continue;
        }
        let mut prev: Option<Point> = None;
        for (i, &v) in ser.points.iter().enumerate() {
            // X：按容量定位、从左填充；capacity==1 时唯一点画在水平中心
            let x = if ser.capacity > 1 {
                abs.x + i as i32 * (abs.w - 1) / (ser.capacity as i32 - 1)
            } else {
                abs.x + abs.w / 2
            };
            // Y：钳到 [min,max] 后线性映射；min==max 画水平中线（避免除零）
            let y = if max > min {
                let frac = (v.clamp(min, max) - min) as i64 * (abs.h - 1) as i64 / (max - min) as i64;
                abs.y + abs.h - 1 - frac as i32
            } else {
                abs.y + abs.h / 2
            };
            let p = Point { x, y };
            match prev {
                Some(q) => d.draw_line(q, p, 2, ser.color, ctx.ap(255), clip),
                None if n == 1 => d.fill_circle(p, 1, ser.color, ctx.ap(255), clip),
                None => {}
            }
            prev = Some(p);
        }
    }
}

/// Chart 构建器：默认 120x60 + range 0..100
pub struct ChartBuilder {
    min: i32,
    max: i32,
    size: Option<(i32, i32)>,
    style: Option<Style>,
    sizing: Option<(Option<Sizing>, Option<Sizing>)>,
    transition: Option<(u32, Easing)>,
    events: Vec<(EventKind, EventCb)>,
    series: Vec<(Color, usize)>,
}

impl ChartBuilder {
    pub fn new() -> Self {
        Self {
            min: 0, max: 100,
            size: None, style: None, sizing: None,
            transition: None, events: Vec::new(), series: Vec::new(),
        }
    }
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.min = min;
        self.max = max;
        self
    }
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.size = Some((w, h));
        self
    }
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }
    pub fn sizing(mut self, w: Option<Sizing>, h: Option<Sizing>) -> Self {
        self.sizing = Some((w, h));
        self
    }
    pub fn transition(mut self, dur: u32, easing: Easing) -> Self {
        self.transition = Some((dur, easing));
        self
    }
    pub fn on(mut self, kind: EventKind, cb: EventCb) -> Self {
        self.events.push((kind, cb));
        self
    }
    /// 预建一条序列（可多次调用）
    pub fn series(mut self, color: Color, capacity: usize) -> Self {
        self.series.push((color, capacity));
        self
    }

    pub fn build(self, ui: &mut Ui, parent: ObjRef) -> ObjRef {
        let (w, h) = self.size.unwrap_or((120, 60));
        let state = ChartState {
            min: self.min,
            max: self.max,
            series: self.series.into_iter().map(|(c, cap)| Series::new(c, cap)).collect(),
        };
        let r = ui.insert_node(parent, Rect::new(0, 0, w, h), WidgetKind::Chart(state));
        ui.set_style(r, self.style.unwrap_or_default());
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
