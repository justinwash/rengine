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
