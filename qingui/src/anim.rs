use alloc::boxed::Box;
use crate::arena::ObjRef;
use crate::ui::Ui;

/// Properties of an object that an animation can drive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnimProp {
    /// Position X.
    X,
    /// Position Y.
    Y,
    /// Width.
    W,
    /// Height.
    H,
    /// Opacity.
    Opa,
    /// The widget's value.
    Value,
    /// Visual translation along X.
    TranslateX,
    /// Visual translation along Y.
    TranslateY,
}

/// Easing curves used to interpolate an animation over time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Easing {
    /// Linear interpolation.
    Linear,
    /// Quadratic ease-in.
    EaseInQuad,
    /// Quadratic ease-out.
    EaseOutQuad,
    /// Quadratic ease-in-out.
    EaseInOutQuad,
    /// Bouncing ease-out.
    Bounce,
    /// Overshoots past 1 then settles back.
    Overshoot,
}

impl Easing {
    /// Evaluates the easing curve at normalized time `t` (clamped to 0.0..=1.0).
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
                // Overshoots past 1 then settles back (s=1.70158)
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

/// A single animation tweening one property of one object from a start to an end value.
pub struct Anim {
    /// The animated object.
    pub target: ObjRef,
    /// The property being animated.
    pub prop: AnimProp,
    /// Start value.
    pub start: i32,
    /// End value.
    pub end: i32,
    /// Duration of one cycle in milliseconds.
    pub duration_ms: u32,
    /// Delay before the animation starts, in milliseconds.
    pub delay_ms: u32,
    /// Number of cycles; -1 = infinite.
    pub repeat: i32,
    /// When true, alternate direction on each cycle (odd cycles run in reverse).
    pub playback: bool,
    /// The easing curve applied per cycle.
    pub easing: Easing,
    /// Optional callback invoked once when the animation completes.
    pub on_done: Option<Box<dyn FnMut(&mut Ui)>>,
}

impl Anim {
    /// Creates an animation from `start` to `end` over `duration_ms`, targeting `prop` of `target`.
    pub fn new(target: ObjRef, prop: AnimProp, start: i32, end: i32, duration_ms: u32) -> Self {
        Self {
            target, prop, start, end, duration_ms,
            delay_ms: 0, repeat: 1, playback: false,
            easing: Easing::Linear, on_done: None,
        }
    }

    /// Sets the easing curve for this animation.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
    /// Delays the start of the animation by `delay_ms` milliseconds.
    pub fn delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }
    /// Number of cycles, -1 = infinite.
    pub fn repeat(mut self, repeat: i32) -> Self {
        self.repeat = repeat;
        self
    }
    /// Enables ping-pong playback (odd cycles run in reverse).
    pub fn playback(mut self, playback: bool) -> Self {
        self.playback = playback;
        self
    }
    /// Sets a callback invoked once when the animation finishes.
    pub fn on_done(mut self, cb: impl FnMut(&mut Ui) + 'static) -> Self {
        self.on_done = Some(Box::new(cb));
        self
    }
}

/// An animation currently in flight (internal).
pub(crate) struct RunningAnim {
    pub anim: Anim,
    pub start_time: u64,
}

/// Result of a single animation evaluation (the pure `eval` output).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AnimEval {
    /// Still in the delay phase.
    Delay,
    /// In progress: the value to apply.
    Keep(i32),
    /// Finished: final value (the `on_done` callback is handled by the caller after receiving `Done`).
    Done(i32),
}

/// Interpolation evaluation (pure): only computes "which value to take"; never touches `on_done` or the tree.
/// Semantics are byte-for-byte identical to the inline evaluation in the old `Ui::step_anims` (delay/repeat/playback).
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
        assert_eq!(eval(&a, 0, 50), AnimEval::Keep(50));  // round0 forward midpoint
        assert_eq!(eval(&a, 0, 150), AnimEval::Keep(50)); // round1 reverse midpoint
        assert_eq!(eval(&a, 0, 200), AnimEval::Done(0));  // odd final round reverses → start
    }

    #[test]
    fn infinite_repeat_never_done() {
        let a = lin(0, 100, 100).repeat(-1);
        assert!(matches!(eval(&a, 0, 999_999), AnimEval::Keep(_)));
    }
}
