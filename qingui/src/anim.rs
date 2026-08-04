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

    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
    pub fn delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }
    /// 重复次数，-1 = 无限
    pub fn repeat(mut self, repeat: i32) -> Self {
        self.repeat = repeat;
        self
    }
    /// 往返播放（奇数轮反向）
    pub fn playback(mut self, playback: bool) -> Self {
        self.playback = playback;
        self
    }
    pub fn on_done(mut self, cb: impl FnMut(&mut Ui) + 'static) -> Self {
        self.on_done = Some(Box::new(cb));
        self
    }
}

/// 运行中的动画实例（内部）
pub(crate) struct RunningAnim {
    pub anim: Anim,
    pub start_time: u64,
}

/// 单次动画求值结果（纯函数 eval 的输出）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AnimEval {
    /// 仍在延迟期
    Delay,
    /// 推进中：应应用的值
    Keep(i32),
    /// 已结束：最终值（on_done 回调由调用方在拿到 Done 后处理）
    Done(i32),
}

/// 插值求值（纯）：只算"该拿什么值"，不碰 on_done、不触树。
/// 语义与旧 Ui::step_anims 的内联求值逐字节一致（delay/重复/往返）。
pub(crate) fn eval(a: &Anim, start_time: u64, now: u64) -> AnimEval {
    let elapsed = now.saturating_sub(start_time);
    if elapsed < a.delay_ms as u64 {
        return AnimEval::Delay;
    }
    let t_ms = elapsed - a.delay_ms as u64;
    let dur = a.duration_ms.max(1) as u64;
    let total: i32 = if a.repeat < 0 { i32::MAX } else { a.repeat.max(1) };
    if t_ms >= dur * total as u64 {
        let last = total - 1;
        let rev = a.playback && last % 2 == 1;
        return AnimEval::Done(if rev { a.start } else { a.end });
    }
    let round = (t_ms / dur) as i32;
    let in_round = t_ms % dur;
    let rev = a.playback && round % 2 == 1;
    let t = in_round as f32 / dur as f32;
    let k = if rev { 1.0 - t } else { t };
    AnimEval::Keep(a.start + ((a.end - a.start) as f32 * a.easing.eval(k)) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(i: u32) -> ObjRef { ObjRef { index: i, generation: 0 } }
    fn lin(start: i32, end: i32, dur: u32) -> Anim {
        Anim::new(obj(0), AnimProp::X, start, end, dur)
    }

    #[test]
    fn delay_window() {
        let a = lin(0, 100, 1000).delay(500);
        assert_eq!(eval(&a, 1000, 1499), AnimEval::Delay);
        assert_eq!(eval(&a, 1000, 1500), AnimEval::Keep(0));
    }

    #[test]
    fn linear_midpoint() {
        let a = lin(0, 100, 1000);
        assert_eq!(eval(&a, 0, 500), AnimEval::Keep(50));
    }

    #[test]
    fn done_at_end() {
        let a = lin(0, 100, 1000);
        assert_eq!(eval(&a, 0, 1000), AnimEval::Done(100));
    }

    #[test]
    fn repeat_three_cycles() {
        let a = lin(0, 100, 100).repeat(3);
        assert_eq!(eval(&a, 0, 99), AnimEval::Keep(99));
        assert_eq!(eval(&a, 0, 300), AnimEval::Done(100));
    }

    #[test]
    fn playback_reverses_on_odd_round() {
        let a = lin(0, 100, 100).repeat(2).playback(true);
        assert_eq!(eval(&a, 0, 50), AnimEval::Keep(50));  // round0 正向中点
        assert_eq!(eval(&a, 0, 150), AnimEval::Keep(50)); // round1 反向中点
        assert_eq!(eval(&a, 0, 200), AnimEval::Done(0));  // 奇数末轮反向 → start
    }

    #[test]
    fn infinite_repeat_never_done() {
        let a = lin(0, 100, 100).repeat(-1);
        assert!(matches!(eval(&a, 0, 999_999), AnimEval::Keep(_)));
    }
}
