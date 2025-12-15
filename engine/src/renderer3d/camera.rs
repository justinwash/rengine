use glam::{Mat4, Vec3};


pub struct Camera3D {
    pub position: Vec3,

    pub yaw: f32,

    pub pitch: f32,

    pub fov_y: f32,

    pub z_near: f32,

    pub z_far: f32,
}

impl Camera3D {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fov_y: std::f32::consts::FRAC_PI_3,
            z_near: 0.1,
            z_far: 500.0,
        }
    }


    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }


    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }


    pub fn view_matrix(&self) -> Mat4 {
        let target = self.position + self.forward();
        Mat4::look_at_rh(self.position, target, Vec3::Y)
    }


    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect_ratio, self.z_near, self.z_far)
    }


    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }


    pub fn mouse_look(&mut self, dx: f64, dy: f64, sensitivity: f32) {
        self.yaw += dx as f32 * sensitivity;
        self.pitch -= dy as f32 * sensitivity;

        let max_pitch = 89.0f32.to_radians();
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);
    }
}

impl Default for Camera3D {
    fn default() -> Self {
        Self::new()
    }
}
