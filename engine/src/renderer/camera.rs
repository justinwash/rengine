use glam::{Mat4, Vec2};

pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,

    pub bounds: Option<CameraBounds>,

    follow_target: Option<Vec2>,
    follow_speed: f32,
    dead_zone: Vec2,

    shake_intensity: f32,
    shake_duration: f32,
    shake_elapsed: f32,
    shake_offset: Vec2,
    shake_seed: u32,
}

pub struct CameraBounds {
    pub min: Vec2,
    pub max: Vec2,
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            bounds: None,
            follow_target: None,
            follow_speed: 5.0,
            dead_zone: Vec2::ZERO,
            shake_intensity: 0.0,
            shake_duration: 0.0,
            shake_elapsed: 0.0,
            shake_offset: Vec2::ZERO,
            shake_seed: 0,
        }
    }

    pub fn follow(&mut self, target: Vec2, speed: f32) {
        self.follow_target = Some(target);
        self.follow_speed = speed;
    }

    pub fn set_dead_zone(&mut self, half_size: Vec2) {
        self.dead_zone = half_size.abs();
    }

    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.shake_intensity = intensity;
        self.shake_duration = duration;
        self.shake_elapsed = 0.0;
        self.shake_seed = self.shake_seed.wrapping_add(1);
    }

    pub fn update(&mut self, dt: f32) {
        if let Some(target) = self.follow_target {
            let diff = target - self.position;
            let clamped = Vec2::new(
                if diff.x.abs() > self.dead_zone.x {
                    diff.x - diff.x.signum() * self.dead_zone.x
                } else {
                    0.0
                },
                if diff.y.abs() > self.dead_zone.y {
                    diff.y - diff.y.signum() * self.dead_zone.y
                } else {
                    0.0
                },
            );
            let t = (self.follow_speed * dt).clamp(0.0, 1.0);
            self.position += clamped * t;
        }

        if let Some(ref b) = self.bounds {
            self.position = self.position.clamp(b.min, b.max);
        }

        if self.shake_elapsed < self.shake_duration {
            self.shake_elapsed += dt;
            let t = 1.0 - (self.shake_elapsed / self.shake_duration).min(1.0);
            let s = self
                .shake_seed
                .wrapping_add((self.shake_elapsed * 1000.0) as u32);
            let hash_x = ((s.wrapping_mul(2654435761)) >> 16) as f32 / 32768.0 - 1.0;
            let hash_y = ((s.wrapping_mul(2246822519)) >> 16) as f32 / 32768.0 - 1.0;
            self.shake_offset = Vec2::new(hash_x, hash_y) * self.shake_intensity * t;
        } else {
            self.shake_offset = Vec2::ZERO;
        }
    }

    pub fn projection(&self, viewport_width: f32, viewport_height: f32) -> Mat4 {
        let half_w = viewport_width / 2.0 / self.zoom;
        let half_h = viewport_height / 2.0 / self.zoom;

        let pos = self.position + self.shake_offset;

        let ortho = Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, -1.0, 1.0);

        let view = Mat4::from_rotation_z(-self.rotation)
            * Mat4::from_translation(glam::Vec3::new(-pos.x, -pos.y, 0.0));

        ortho * view
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self::new()
    }
}
