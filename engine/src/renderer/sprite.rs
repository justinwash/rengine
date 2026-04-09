use super::texture::TextureId;
use crate::assets::Color;
use glam::Vec2;

#[derive(Debug, Clone)]
pub struct DrawParams {
    pub texture: TextureId,
    pub position: Vec2,
    pub size: Vec2,
    pub color: Color,

    pub uv_rect: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,

    pub rotation: f32,
    pub origin: Vec2,
    pub z_order: i32,
}

impl DrawParams {
    pub fn new(texture: TextureId, position: Vec2, size: Vec2) -> Self {
        Self {
            texture,
            position,
            size,
            color: Color::WHITE,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
            rotation: 0.0,
            origin: Vec2::ZERO,
            z_order: 0,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_uv_rect(mut self, rect: [f32; 4]) -> Self {
        self.uv_rect = rect;
        self
    }

    pub fn with_flip_x(mut self, flip: bool) -> Self {
        self.flip_x = flip;
        self
    }

    pub fn with_flip_y(mut self, flip: bool) -> Self {
        self.flip_y = flip;
        self
    }

    pub fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    pub fn with_origin(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    pub fn with_z_order(mut self, z: i32) -> Self {
        self.z_order = z;
        self
    }

    pub fn with_centered_origin(mut self) -> Self {
        self.origin = self.size * 0.5;
        self
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
