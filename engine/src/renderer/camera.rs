use glam::{Mat4, Vec2};


pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
        }
    }


    pub fn projection(&self, viewport_width: f32, viewport_height: f32) -> Mat4 {
        let half_w = viewport_width / 2.0 / self.zoom;
        let half_h = viewport_height / 2.0 / self.zoom;

        Mat4::orthographic_rh(
            self.position.x - half_w,
            self.position.x + half_w,
            self.position.y - half_h,
            self.position.y + half_h,
            -1.0,
            1.0,
        )
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self::new()
    }
}
