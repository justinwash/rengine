use crate::assets::Color;
use crate::math::Rng;
use crate::renderer::sprite::DrawParams;
use crate::renderer::texture::TextureId;
use crate::renderer::Frame;
use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmitShape {
    Point,
    Circle(f32),
    Rect(f32, f32),
}

impl Default for EmitShape {
    fn default() -> Self {
        EmitShape::Point
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RangeF32 {
    pub min: f32,
    pub max: f32,
}

impl RangeF32 {
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    pub fn constant(value: f32) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    pub fn sample(&self, rng: &mut Rng) -> f32 {
        rng.f32_range(self.min, self.max)
    }
}

impl Default for RangeF32 {
    fn default() -> Self {
        Self { min: 1.0, max: 1.0 }
    }
}

impl From<f32> for RangeF32 {
    fn from(v: f32) -> Self {
        Self::constant(v)
    }
}

impl From<(f32, f32)> for RangeF32 {
    fn from((min, max): (f32, f32)) -> Self {
        Self::new(min, max)
    }
}

#[derive(Debug, Clone)]
pub struct EmitterConfig {
    pub emit_rate: f32,
    pub burst_count: u32,
    pub lifetime: RangeF32,
    pub speed: RangeF32,
    pub angle: RangeF32,
    pub spin: RangeF32,
    pub size_start: RangeF32,
    pub size_end: RangeF32,
    pub color_start: Color,
    pub color_end: Color,
    pub gravity: Vec2,
    pub damping: f32,
    pub emit_shape: EmitShape,
    pub z_order: i32,
    pub looping: bool,
    pub max_particles: usize,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            emit_rate: 10.0,
            burst_count: 0,
            lifetime: RangeF32::new(0.5, 1.5),
            speed: RangeF32::new(20.0, 80.0),
            angle: RangeF32::new(0.0, std::f32::consts::TAU),
            spin: RangeF32::constant(0.0),
            size_start: RangeF32::new(4.0, 8.0),
            size_end: RangeF32::new(1.0, 2.0),
            color_start: Color::WHITE,
            color_end: Color::new(1.0, 1.0, 1.0, 0.0),
            gravity: Vec2::ZERO,
            damping: 0.0,
            emit_shape: EmitShape::Point,
            z_order: 0,
            looping: true,
            max_particles: 512,
        }
    }
}

impl EmitterConfig {
    pub fn with_emit_rate(mut self, rate: f32) -> Self {
        self.emit_rate = rate;
        self
    }

    pub fn with_burst_count(mut self, count: u32) -> Self {
        self.burst_count = count;
        self
    }

    pub fn with_lifetime(mut self, range: impl Into<RangeF32>) -> Self {
        self.lifetime = range.into();
        self
    }

    pub fn with_speed(mut self, range: impl Into<RangeF32>) -> Self {
        self.speed = range.into();
        self
    }

    pub fn with_angle(mut self, range: impl Into<RangeF32>) -> Self {
        self.angle = range.into();
        self
    }

    pub fn with_spin(mut self, range: impl Into<RangeF32>) -> Self {
        self.spin = range.into();
        self
    }

    pub fn with_size_start(mut self, range: impl Into<RangeF32>) -> Self {
        self.size_start = range.into();
        self
    }

    pub fn with_size_end(mut self, range: impl Into<RangeF32>) -> Self {
        self.size_end = range.into();
        self
    }

    pub fn with_color_start(mut self, color: Color) -> Self {
        self.color_start = color;
        self
    }

    pub fn with_color_end(mut self, color: Color) -> Self {
        self.color_end = color;
        self
    }

    pub fn with_gravity(mut self, gravity: Vec2) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    pub fn with_emit_shape(mut self, shape: EmitShape) -> Self {
        self.emit_shape = shape;
        self
    }

    pub fn with_z_order(mut self, z: i32) -> Self {
        self.z_order = z;
        self
    }

    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn with_max_particles(mut self, max: usize) -> Self {
        self.max_particles = max;
        self
    }
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    rotation: f32,
    spin: f32,
    age: f32,
    lifetime: f32,
    size_start: f32,
    size_end: f32,
    alive: bool,
}

pub struct ParticleEmitter {
    config: EmitterConfig,
    particles: Vec<Particle>,
    position: Vec2,
    emit_accum: f32,
    active: bool,
    first_free: usize,
    alive: usize,
}

impl ParticleEmitter {
    pub fn new(config: EmitterConfig) -> Self {
        let cap = config.max_particles;
        let mut particles = Vec::with_capacity(cap);
        for _ in 0..cap {
            particles.push(Particle {
                pos: Vec2::ZERO,
                vel: Vec2::ZERO,
                rotation: 0.0,
                spin: 0.0,
                age: 0.0,
                lifetime: 1.0,
                size_start: 1.0,
                size_end: 1.0,
                alive: false,
            });
        }
        Self {
            config,
            particles,
            position: Vec2::ZERO,
            emit_accum: 0.0,
            active: true,
            first_free: 0,
            alive: 0,
        }
    }

    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn config(&self) -> &EmitterConfig {
        &self.config
    }

    pub fn alive_count(&self) -> usize {
        self.alive
    }

    pub fn is_finished(&self) -> bool {
        !self.active && self.alive == 0
    }

    pub fn clear(&mut self) {
        for p in self.particles.iter_mut() {
            p.alive = false;
        }
        self.alive = 0;
        self.emit_accum = 0.0;
        self.first_free = 0;
    }

    pub fn burst(&mut self, rng: &mut Rng) {
        let count = self.config.burst_count as usize;
        for _ in 0..count {
            self.spawn_one(rng);
        }
    }

    pub fn update(&mut self, dt: f32, rng: &mut Rng) {
        for p in self.particles.iter_mut() {
            if !p.alive {
                continue;
            }
            p.age += dt;
            if p.age >= p.lifetime {
                p.alive = false;
                self.alive -= 1;
                continue;
            }
            p.vel += self.config.gravity * dt;
            if self.config.damping > 0.0 {
                let factor = 1.0 - (self.config.damping * dt).min(1.0);
                p.vel *= factor;
            }
            p.pos += p.vel * dt;
            p.rotation += p.spin * dt;
        }

        if self.active && self.config.emit_rate > 0.0 {
            self.emit_accum += dt * self.config.emit_rate;
            while self.emit_accum >= 1.0 {
                self.emit_accum -= 1.0;
                self.spawn_one(rng);
            }
        }

        if !self.config.looping && self.active && self.config.emit_rate <= 0.0 && self.alive == 0 {
            self.active = false;
        }
    }

    pub fn draw(&self, frame: &mut Frame, texture: TextureId) {
        let cfg = &self.config;
        for p in &self.particles {
            if !p.alive {
                continue;
            }
            let t = if p.lifetime > 0.0 {
                (p.age / p.lifetime).min(1.0)
            } else {
                1.0
            };
            let size = p.size_start + (p.size_end - p.size_start) * t;
            let color = cfg.color_start.lerp(cfg.color_end, t);
            let half = Vec2::splat(size * 0.5);
            frame.draw_sprite(
                DrawParams::new(texture, p.pos - half, Vec2::splat(size))
                    .with_color(color)
                    .with_rotation(p.rotation)
                    .with_origin(half)
                    .with_z_order(cfg.z_order),
            );
        }
    }

    fn spawn_one(&mut self, rng: &mut Rng) {
        let slot = self.find_free_slot();
        let slot = match slot {
            Some(i) => i,
            None => return,
        };

        let cfg = &self.config;
        let angle = cfg.angle.sample(rng);
        let speed = cfg.speed.sample(rng);
        let offset = match cfg.emit_shape {
            EmitShape::Point => Vec2::ZERO,
            EmitShape::Circle(r) => rng.in_circle(r),
            EmitShape::Rect(w, h) => Vec2::new(
                rng.f32_range(-w * 0.5, w * 0.5),
                rng.f32_range(-h * 0.5, h * 0.5),
            ),
        };

        let p = &mut self.particles[slot];
        p.pos = self.position + offset;
        p.vel = Vec2::new(angle.cos(), angle.sin()) * speed;
        p.rotation = 0.0;
        p.spin = cfg.spin.sample(rng);
        p.age = 0.0;
        p.lifetime = cfg.lifetime.sample(rng).max(0.001);
        p.size_start = cfg.size_start.sample(rng);
        p.size_end = cfg.size_end.sample(rng);
        p.alive = true;
        self.alive += 1;
    }

    fn find_free_slot(&mut self) -> Option<usize> {
        let len = self.particles.len();
        for i in 0..len {
            let idx = (self.first_free + i) % len;
            if !self.particles[idx].alive {
                self.first_free = (idx + 1) % len;
                return Some(idx);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_spawn_and_die() {
        let config = EmitterConfig {
            emit_rate: 0.0,
            burst_count: 5,
            lifetime: RangeF32::constant(0.5),
            speed: RangeF32::constant(10.0),
            looping: false,
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        let mut rng = Rng::new(42);

        assert_eq!(emitter.alive_count(), 0);
        emitter.burst(&mut rng);
        assert_eq!(emitter.alive_count(), 5);

        for _ in 0..60 {
            emitter.update(1.0 / 60.0, &mut rng);
        }
        assert_eq!(emitter.alive_count(), 0);
        assert!(emitter.is_finished());
    }

    #[test]
    fn emit_rate_spawns_over_time() {
        let config = EmitterConfig {
            emit_rate: 60.0,
            lifetime: RangeF32::constant(10.0),
            speed: RangeF32::constant(0.0),
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        let mut rng = Rng::new(42);

        for _ in 0..60 {
            emitter.update(1.0 / 60.0, &mut rng);
        }
        let count = emitter.alive_count();
        assert!(count >= 55 && count <= 65, "expected ~60, got {count}");
    }

    #[test]
    fn gravity_affects_velocity() {
        let config = EmitterConfig {
            emit_rate: 0.0,
            burst_count: 1,
            lifetime: RangeF32::constant(10.0),
            speed: RangeF32::constant(0.0),
            gravity: Vec2::new(0.0, -100.0),
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        let mut rng = Rng::new(42);
        emitter.burst(&mut rng);

        for _ in 0..60 {
            emitter.update(1.0 / 60.0, &mut rng);
        }
        let p = emitter.particles.iter().find(|p| p.alive).unwrap();
        assert!(p.vel.y < -90.0);
        assert!(p.pos.y < -40.0);
    }

    #[test]
    fn max_particles_cap() {
        let config = EmitterConfig {
            emit_rate: 0.0,
            burst_count: 100,
            lifetime: RangeF32::constant(10.0),
            max_particles: 10,
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        let mut rng = Rng::new(42);
        emitter.burst(&mut rng);
        assert_eq!(emitter.alive_count(), 10);
    }

    #[test]
    fn inactive_emitter_stops_spawning() {
        let config = EmitterConfig {
            emit_rate: 100.0,
            lifetime: RangeF32::constant(10.0),
            ..Default::default()
        };
        let mut emitter = ParticleEmitter::new(config);
        let mut rng = Rng::new(42);

        emitter.set_active(false);
        for _ in 0..60 {
            emitter.update(1.0 / 60.0, &mut rng);
        }
        assert_eq!(emitter.alive_count(), 0);
    }

    #[test]
    fn color_lerp() {
        let a = Color::RED;
        let b = Color::BLUE;
        let mid = a.lerp(b, 0.5);
        assert!((mid.r - 0.5).abs() < 0.01);
        assert!((mid.b - 0.5).abs() < 0.01);
    }

    #[test]
    fn range_f32_from_tuple() {
        let r: RangeF32 = (1.0, 5.0).into();
        assert_eq!(r.min, 1.0);
        assert_eq!(r.max, 5.0);
    }

    #[test]
    fn range_f32_from_scalar() {
        let r: RangeF32 = 3.0.into();
        assert_eq!(r.min, 3.0);
        assert_eq!(r.max, 3.0);
    }
}
