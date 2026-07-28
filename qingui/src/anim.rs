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
    TranslateX,
    TranslateY,
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
