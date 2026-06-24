use std::time::Instant;

pub struct TimeState {
    start_time: Instant,
    last_frame: Instant,
    dt: f32,
    total_time: f32,
    frame_count: u64,
    fixed_dt: f32,
    accumulator: f32,
    /// When true, `tick()` advances by `fixed_dt` instead of reading the
    /// wall clock. Used for headless/replay runs so frame output is fully
    /// reproducible regardless of how long each frame takes to compute.
    deterministic: bool,
}

impl TimeState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame: now,
            dt: 1.0 / 60.0,
            total_time: 0.0,
            frame_count: 0,
            fixed_dt: 1.0 / 60.0,
            accumulator: 0.0,
            deterministic: false,
        }
    }

    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub fn total_time(&self) -> f32 {
        self.total_time
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn fps(&self) -> f32 {
        if self.dt > 0.0 {
            1.0 / self.dt
        } else {
            0.0
        }
    }

    pub fn fixed_dt(&self) -> f32 {
        self.fixed_dt
    }

    pub(crate) fn set_fixed_dt(&mut self, fixed_dt: f32) {
        assert!(
            fixed_dt.is_finite() && fixed_dt > 0.0,
            "fixed_dt must be finite and > 0.0"
        );
        self.fixed_dt = fixed_dt;
    }

    /// Switch between wall-clock timing (default) and deterministic fixed-step
    /// timing. Headless runs enable this so animation/accumulators are
    /// reproducible frame-for-frame (e.g. visual regression captures).
    pub(crate) fn set_deterministic(&mut self, deterministic: bool) {
        self.deterministic = deterministic;
    }

    pub(crate) fn tick(&mut self) {
        if self.deterministic {
            // Advance by exactly one fixed step; never touch the wall clock so
            // total_time/dt depend only on the frame number.
            self.dt = self.fixed_dt;
            self.total_time += self.fixed_dt;
        } else {
            let now = Instant::now();
            self.dt = now.duration_since(self.last_frame).as_secs_f32();

            if self.dt > 0.1 {
                self.dt = 0.1;
            }
            self.total_time = now.duration_since(self.start_time).as_secs_f32();
            self.last_frame = now;
        }
        self.frame_count += 1;
        self.accumulator += self.dt;
        let max_accumulator = self.fixed_dt * 10.0;
        if self.accumulator > max_accumulator {
            self.accumulator = max_accumulator;
        }
    }

    pub(crate) fn consume_fixed_step(&mut self) -> bool {
        if self.accumulator >= self.fixed_dt {
            self.accumulator -= self.fixed_dt;
            true
        } else {
            false
        }
    }
}
