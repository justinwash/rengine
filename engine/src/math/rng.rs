/// Seeded pseudo-random number generator based on xoshiro256**.
///
/// Fast, high-quality, and deterministic for a given seed.
/// No external dependencies — suitable for game logic, card draws,
/// event rolls, procedural generation, and anything that needs
/// reproducible randomness.
///
/// # Example
/// ```
/// use rengine::Rng;
///
/// let mut rng = Rng::new(42);
/// let roll = rng.range(1, 6);        // 1..=6 (dice roll)
/// let pct = rng.f32();               // 0.0..1.0
/// let coin = rng.chance(0.5);        // true/false
/// let pick = rng.pick(&["a", "b", "c"]);
/// ```
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    /// Create a new RNG with the given seed.
    /// Different seeds produce completely different sequences.
    pub fn new(seed: u64) -> Self {
        // Use splitmix64 to expand a single seed into 4 state words.
        let mut s = seed;
        let mut state = [0u64; 4];
        for word in &mut state {
            *word = splitmix64(&mut s);
        }
        // Ensure state is not all-zero (degenerate for xoshiro).
        if state == [0; 4] {
            state[0] = 1;
        }
        Self { state }
    }

    /// Create a new RNG seeded from system time (nanoseconds).
    /// Convenient for non-deterministic use; for reproducible runs, prefer `new(seed)`.
    pub fn from_time() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self::new(nanos)
    }

    /// Fork a new independent RNG from this one.
    /// Useful for giving subsystems their own stream without affecting the parent.
    pub fn fork(&mut self) -> Self {
        Self::new(self.next_u64())
    }

    /// Raw u64 in the full range.
    pub fn next_u64(&mut self) -> u64 {
        let result = (self.state[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;

        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];

        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);

        result
    }

    /// Random `f32` in `[0.0, 1.0)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Random `f64` in `[0.0, 1.0)`.
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Random `f32` in `[min, max)`.
    /// Returns `min` if `min >= max`.
    pub fn f32_range(&mut self, min: f32, max: f32) -> f32 {
        if min >= max {
            return min;
        }
        min + self.f32() * (max - min)
    }

    /// Random `bool`.
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Returns `true` with probability `p` (0.0 = never, 1.0 = always).
    pub fn chance(&mut self, p: f32) -> bool {
        debug_assert!(
            p.is_finite() && (0.0..=1.0).contains(&p),
            "chance(p) requires p to be finite and within [0.0, 1.0], got {p}"
        );
        self.f32() < p
    }

    /// Unbiased random `u64` in `[0, n)` using rejection sampling.
    fn below(&mut self, n: u64) -> u64 {
        if n <= 1 {
            return 0;
        }
        let threshold = n.wrapping_neg() % n; // (2^64 - n) % n
        loop {
            let r = self.next_u64();
            if r >= threshold {
                return r % n;
            }
        }
    }

    /// Random integer in `[min, max]` (inclusive on both ends).
    pub fn range(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let span = (max - min + 1) as u64;
        min + self.below(span) as i32
    }

    /// Random `u32` in `[0, n)`.
    pub fn u32(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.below(n as u64) as u32
    }

    /// Random `usize` in `[0, n)`.
    pub fn usize(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        self.below(n as u64) as usize
    }

    /// Pick a random element from a slice. Panics if empty.
    pub fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[self.usize(slice.len())]
    }

    /// Shuffle a slice in-place (Fisher–Yates).
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let len = slice.len();
        for i in (1..len).rev() {
            let j = self.usize(i + 1);
            slice.swap(i, j);
        }
    }

    /// Randomly select `n` distinct indices from `[0, len)`.
    /// Returns up to `min(n, len)` indices in random order.
    pub fn sample_indices(&mut self, len: usize, n: usize) -> Vec<usize> {
        let n = n.min(len);
        let mut indices: Vec<usize> = (0..len).collect();
        for i in 0..n {
            let j = i + self.usize(len - i);
            indices.swap(i, j);
        }
        indices.truncate(n);
        indices
    }

    /// Roll a weighted index. `weights` are relative (don't need to sum to 1).
    /// Returns the index of the chosen weight. Panics if `weights` is empty
    /// or contains a negative or non-finite weight.
    pub fn weighted(&mut self, weights: &[f32]) -> usize {
        assert!(!weights.is_empty(), "weighted() called with empty weights");
        for &w in weights {
            assert!(
                w.is_finite() && w >= 0.0,
                "weighted() called with negative or non-finite weight"
            );
        }
        let total: f32 = weights.iter().sum();
        if total <= 0.0 {
            return self.usize(weights.len());
        }
        let mut roll = self.f32() * total;
        for (i, &w) in weights.iter().enumerate() {
            if w == 0.0 {
                continue;
            }
            if roll < w {
                return i;
            }
            roll -= w;
        }
        weights.len() - 1
    }

    /// Normal distribution sample (Box–Muller transform).
    /// Returns a value centered on `mean` with standard deviation `std_dev`.
    pub fn normal(&mut self, mean: f32, std_dev: f32) -> f32 {
        let u1 = self.f32().max(f32::EPSILON); // avoid log(0)
        let u2 = self.f32();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
        mean + z * std_dev
    }

    /// Random Vec2 with each component in `[0.0, 1.0)`.
    pub fn vec2(&mut self) -> glam::Vec2 {
        glam::Vec2::new(self.f32(), self.f32())
    }

    /// Random point inside a circle of the given radius (uniform distribution).
    pub fn in_circle(&mut self, radius: f32) -> glam::Vec2 {
        let angle = self.f32() * 2.0 * std::f32::consts::PI;
        let r = radius * self.f32().sqrt();
        glam::Vec2::new(r * angle.cos(), r * angle.sin())
    }

    /// Random unit direction vector (angle uniformly distributed).
    pub fn direction(&mut self) -> glam::Vec2 {
        let angle = self.f32() * 2.0 * std::f32::consts::PI;
        glam::Vec2::new(angle.cos(), angle.sin())
    }
}

/// splitmix64 — used to seed xoshiro state from a single u64.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(12345);
        let mut b = Rng::new(12345);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let vals_a: Vec<_> = (0..10).map(|_| a.next_u64()).collect();
        let vals_b: Vec<_> = (0..10).map(|_| b.next_u64()).collect();
        assert_ne!(vals_a, vals_b);
    }

    #[test]
    fn range_bounds() {
        let mut rng = Rng::new(42);
        for _ in 0..10_000 {
            let v = rng.range(1, 6);
            assert!((1..=6).contains(&v));
        }
    }

    #[test]
    fn f32_bounds() {
        let mut rng = Rng::new(99);
        for _ in 0..10_000 {
            let v = rng.f32();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn chance_never_and_always() {
        let mut rng = Rng::new(7);
        for _ in 0..100 {
            assert!(!rng.chance(0.0));
            assert!(rng.chance(1.0));
        }
    }

    #[test]
    fn shuffle_preserves_elements() {
        let mut rng = Rng::new(1);
        let mut v = vec![1, 2, 3, 4, 5];
        rng.shuffle(&mut v);
        v.sort();
        assert_eq!(v, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn weighted_respects_weights() {
        let mut rng = Rng::new(42);
        let weights = [0.0, 0.0, 1.0];
        for _ in 0..100 {
            assert_eq!(rng.weighted(&weights), 2);
        }
    }

    #[test]
    fn fork_is_independent() {
        let mut parent = Rng::new(42);
        let _ = parent.next_u64(); // advance parent
        let mut child = parent.fork();
        let parent_val = parent.next_u64();
        let child_val = child.next_u64();
        assert_ne!(parent_val, child_val);
    }

    #[test]
    fn sample_indices_correct_count() {
        let mut rng = Rng::new(42);
        let indices = rng.sample_indices(10, 3);
        assert_eq!(indices.len(), 3);
        // All unique
        let mut sorted = indices.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn in_circle_within_radius() {
        let mut rng = Rng::new(42);
        for _ in 0..10_000 {
            let p = rng.in_circle(5.0);
            assert!(p.length() <= 5.001); // small epsilon for float
        }
    }
}
