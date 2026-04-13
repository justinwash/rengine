use std::time::Instant;

pub struct TimeState {
    start_time: Instant,
    last_frame: Instant,
    dt: f32,
    total_time: f32,
    frame_count: u64,
    fixed_dt: f32,
    accumulator: f32,
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

    pub(crate) fn tick(&mut self) {
        let now = Instant::now();
        self.dt = now.duration_since(self.last_frame).as_secs_f32();

        if self.dt > 0.1 {
            self.dt = 0.1;
        }
        self.total_time = now.duration_since(self.start_time).as_secs_f32();
        self.last_frame = now;
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
