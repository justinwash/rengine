use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    OutCubic,
    InOutCubic,
    InQuart,
    OutQuart,
    InOutQuart,
    InSine,
    OutSine,
    InOutSine,
    InExpo,
    OutExpo,
    InOutExpo,
    InBack,
    OutBack,
    InOutBack,
    InElastic,
    OutElastic,
    InOutElastic,
    InBounce,
    OutBounce,
    InOutBounce,
}

impl Easing {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,

            Easing::InQuad => t * t,
            Easing::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }

            Easing::InCubic => t * t * t,
            Easing::OutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }

            Easing::InQuart => t * t * t * t,
            Easing::OutQuart => 1.0 - (1.0 - t).powi(4),
            Easing::InOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }

            Easing::InSine => 1.0 - (t * PI / 2.0).cos(),
            Easing::OutSine => (t * PI / 2.0).sin(),
            Easing::InOutSine => -(((t * PI).cos() - 1.0) / 2.0),

            Easing::InExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    (2.0f32).powf(10.0 * t - 10.0)
                }
            }
            Easing::OutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - (2.0f32).powf(-10.0 * t)
                }
            }
            Easing::InOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    (2.0f32).powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - (2.0f32).powf(-20.0 * t + 10.0)) / 2.0
                }
            }

            Easing::InBack => {
                let c = 1.70158;
                (c + 1.0) * t * t * t - c * t * t
            }
            Easing::OutBack => {
                let c = 1.70158;
                let u = t - 1.0;
                1.0 + (c + 1.0) * u * u * u + c * u * u
            }
            Easing::InOutBack => {
                let c = 1.70158 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c + 1.0) * 2.0 * t - c)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c + 1.0) * (2.0 * t - 2.0) + c) + 2.0) / 2.0
                }
            }

            Easing::InElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let c = 2.0 * PI / 3.0;
                    -(2.0f32).powf(10.0 * t - 10.0) * ((10.0 * t - 10.75) * c).sin()
                }
            }
            Easing::OutElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let c = 2.0 * PI / 3.0;
                    (2.0f32).powf(-10.0 * t) * ((10.0 * t - 0.75) * c).sin() + 1.0
                }
            }
            Easing::InOutElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let c = 2.0 * PI / 4.5;
                    if t < 0.5 {
                        -((2.0f32).powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c).sin()) / 2.0
                    } else {
                        ((2.0f32).powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c).sin()) / 2.0
                            + 1.0
                    }
                }
            }

            Easing::OutBounce => bounce_out(t),
            Easing::InBounce => 1.0 - bounce_out(1.0 - t),
            Easing::InOutBounce => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }
        }
    }
}

fn bounce_out(t: f32) -> f32 {
    let n = 7.5625;
    let d = 2.75;
    if t < 1.0 / d {
        n * t * t
    } else if t < 2.0 / d {
        let t = t - 1.5 / d;
        n * t * t + 0.75
    } else if t < 2.5 / d {
        let t = t - 2.25 / d;
        n * t * t + 0.9375
    } else {
        let t = t - 2.625 / d;
        n * t * t + 0.984375
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoopMode {
    Once,
    Loop,
    PingPong,
}

#[derive(Clone, Debug)]
pub struct Tween {
    from: f32,
    to: f32,
    duration: f32,
    elapsed: f32,
    easing: Easing,
    loop_mode: LoopMode,
    finished: bool,
}

impl Tween {
    pub fn new(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        assert!(duration > 0.0, "tween duration must be > 0");
        Self {
            from,
            to,
            duration,
            elapsed: 0.0,
            easing,
            loop_mode: LoopMode::Once,
            finished: false,
        }
    }

    pub fn looping(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    pub fn update(&mut self, dt: f32) {
        if self.finished {
            return;
        }
        self.elapsed += dt;
        match self.loop_mode {
            LoopMode::Once => {
                if self.elapsed >= self.duration {
                    self.elapsed = self.duration;
                    self.finished = true;
                }
            }
            LoopMode::Loop => {
                if self.elapsed >= self.duration {
                    self.elapsed = self.elapsed.rem_euclid(self.duration);
                }
            }
            LoopMode::PingPong => {
                let cycle = 2.0 * self.duration;
                if self.elapsed >= cycle {
                    self.elapsed = self.elapsed.rem_euclid(cycle);
                }
            }
        }
    }

    pub fn value(&self) -> f32 {
        let t = match self.loop_mode {
            LoopMode::Once | LoopMode::Loop => self.elapsed / self.duration,
            LoopMode::PingPong => {
                let cycle = self.elapsed / self.duration;
                let phase = cycle % 2.0;
                if phase <= 1.0 {
                    phase
                } else {
                    2.0 - phase
                }
            }
        };
        let eased = self.easing.apply(t);
        self.from + (self.to - self.from) * eased
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.finished = false;
    }

    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn ease(from: f32, to: f32, t: f32, easing: Easing) -> f32 {
    lerp(from, to, easing.apply(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_tween_interpolates() {
        let mut tw = Tween::new(0.0, 100.0, 1.0, Easing::Linear);
        assert!((tw.value() - 0.0).abs() < 0.001);
        tw.update(0.5);
        assert!((tw.value() - 50.0).abs() < 0.001);
        tw.update(0.5);
        assert!((tw.value() - 100.0).abs() < 0.001);
        assert!(tw.is_finished());
    }

    #[test]
    fn tween_clamps_at_end() {
        let mut tw = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        tw.update(5.0);
        assert!((tw.value() - 10.0).abs() < 0.001);
        assert!(tw.is_finished());
    }

    #[test]
    fn loop_mode_wraps() {
        let mut tw = Tween::new(0.0, 100.0, 1.0, Easing::Linear).looping(LoopMode::Loop);
        tw.update(1.5);
        assert!((tw.value() - 50.0).abs() < 0.001);
        assert!(!tw.is_finished());
    }

    #[test]
    fn pingpong_reverses() {
        let mut tw = Tween::new(0.0, 100.0, 1.0, Easing::Linear).looping(LoopMode::PingPong);
        tw.update(0.5);
        assert!((tw.value() - 50.0).abs() < 0.001);
        tw.update(0.75);
        assert!((tw.value() - 75.0).abs() < 0.5);
        tw.update(0.5);
        assert!((tw.value() - 25.0).abs() < 0.5);
    }

    #[test]
    fn reset_restarts() {
        let mut tw = Tween::new(0.0, 100.0, 1.0, Easing::Linear);
        tw.update(1.0);
        assert!(tw.is_finished());
        tw.reset();
        assert!(!tw.is_finished());
        assert!((tw.value() - 0.0).abs() < 0.001);
    }

    #[test]
    fn easing_boundaries() {
        for easing in [
            Easing::Linear,
            Easing::InQuad,
            Easing::OutQuad,
            Easing::InOutQuad,
            Easing::InCubic,
            Easing::OutCubic,
            Easing::InOutCubic,
            Easing::InQuart,
            Easing::OutQuart,
            Easing::InOutQuart,
            Easing::InSine,
            Easing::OutSine,
            Easing::InOutSine,
            Easing::InExpo,
            Easing::OutExpo,
            Easing::InOutExpo,
            Easing::InBack,
            Easing::OutBack,
            Easing::InOutBack,
            Easing::InElastic,
            Easing::OutElastic,
            Easing::InOutElastic,
            Easing::InBounce,
            Easing::OutBounce,
            Easing::InOutBounce,
        ] {
            let v0 = easing.apply(0.0);
            let v1 = easing.apply(1.0);
            assert!(
                (v0 - 0.0).abs() < 0.001,
                "{easing:?} at t=0 was {v0}"
            );
            assert!(
                (v1 - 1.0).abs() < 0.001,
                "{easing:?} at t=1 was {v1}"
            );
        }
    }

    #[test]
    fn ease_helper() {
        let v = ease(10.0, 20.0, 0.5, Easing::Linear);
        assert!((v - 15.0).abs() < 0.001);
    }
}
