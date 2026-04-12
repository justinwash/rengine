use crate::renderer::TextureId;

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub texture: TextureId,

    pub texture_width: u32,

    pub texture_height: u32,

    pub cell_width: u32,

    pub cell_height: u32,
}

impl SpriteSheet {
    pub fn new(
        texture: TextureId,
        texture_width: u32,
        texture_height: u32,
        cell_width: u32,
        cell_height: u32,
    ) -> Self {
        Self {
            texture,
            texture_width,
            texture_height,
            cell_width,
            cell_height,
        }
    }

    pub fn columns(&self) -> u32 {
        self.texture_width / self.cell_width
    }

    pub fn rows(&self) -> u32 {
        self.texture_height / self.cell_height
    }

    pub fn uv_rect(&self, col: u32, row: u32) -> [f32; 4] {
        let tw = self.texture_width as f32;
        let th = self.texture_height as f32;
        let cw = self.cell_width as f32;
        let ch = self.cell_height as f32;
        [col as f32 * cw / tw, row as f32 * ch / th, cw / tw, ch / th]
    }
}

#[derive(Debug, Clone)]
pub struct Animation {

    pub frames: Vec<(u32, u32)>,

    pub frame_time: f32,

    elapsed: f32,

    current: usize,
}

impl Animation {
    pub fn new(frames: Vec<(u32, u32)>, fps: f32) -> Self {
        Self {
            frames,
            frame_time: 1.0 / fps,
            elapsed: 0.0,
            current: 0,
        }
    }

    pub fn update(&mut self, dt: f32) -> (u32, u32) {
        self.elapsed += dt;
        if self.elapsed >= self.frame_time {
            self.elapsed -= self.frame_time;
            self.current = (self.current + 1) % self.frames.len();
        }
        self.frames[self.current]
    }

    pub fn current_frame(&self) -> (u32, u32) {
        self.frames[self.current]
    }

    pub fn reset(&mut self) {
        self.current = 0;
        self.elapsed = 0.0;
    }
}
