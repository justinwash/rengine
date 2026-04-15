pub struct Timer {
    remaining: f32,
    repeating: bool,
    interval: f32,
    finished: bool,
}

impl Timer {
    pub fn once(duration: f32) -> Self {
        Self {
            remaining: duration,
            repeating: false,
            interval: duration,
            finished: false,
        }
    }

    pub fn repeating(interval: f32) -> Self {
        Self {
            remaining: interval,
            repeating: true,
            interval,
            finished: false,
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        if self.finished {
            return false;
        }
        self.remaining -= dt;
        if self.remaining <= 0.0 {
            if self.repeating {
                self.remaining += self.interval;
                if self.remaining < 0.0 {
                    self.remaining = 0.0;
                }
            } else {
                self.remaining = 0.0;
                self.finished = true;
            }
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.remaining = self.interval;
        self.finished = false;
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn remaining(&self) -> f32 {
        self.remaining
    }

    pub fn fraction(&self) -> f32 {
        if self.interval <= 0.0 {
            return 1.0;
        }
        1.0 - (self.remaining / self.interval).clamp(0.0, 1.0)
    }
}

pub struct EventQueue<E> {
    events: Vec<(f32, E)>,
}

impl<E> EventQueue<E> {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn schedule(&mut self, delay: f32, event: E) {
        self.events.push((delay, event));
    }

    pub fn tick(&mut self, dt: f32) -> Vec<E> {
        let mut fired = Vec::new();
        let mut i = 0;
        while i < self.events.len() {
            self.events[i].0 -= dt;
            if self.events[i].0 <= 0.0 {
                let (_, event) = self.events.swap_remove(i);
                fired.push(event);
            } else {
                i += 1;
            }
        }
        fired
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl<E> Default for EventQueue<E> {
    fn default() -> Self {
        Self::new()
    }
}
