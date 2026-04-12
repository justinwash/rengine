use crate::assets::Color;
use crate::text::{FontAtlas, ATLAS_SIZE, FONT_SIZE};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CanvasVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl CanvasVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
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
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub const MAX_CANVAS_VERTICES: usize = 8_000;

const WHITE_UV: [f32; 2] = [1.0 / ATLAS_SIZE as f32, 1.0 / ATLAS_SIZE as f32];

pub struct Canvas {
    pub(crate) verts: Vec<CanvasVertex>,
}

impl Canvas {
    pub fn new() -> Self {
        Self { verts: Vec::new() }
    }

    pub fn shape(&mut self, triangles: &[CanvasVertex]) {
        self.verts.extend_from_slice(triangles);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color, screen_size: (u32, u32)) {
        let [x0, y0] = world_to_ndc(x, y, screen_size);
        let [x1, y1] = world_to_ndc(x + w, y + h, screen_size);

        let c = color.to_array();
        let uv = WHITE_UV;
        let v0 = CanvasVertex {
            position: [x0, y0],
            color: c,
            uv,
        };
        let v1 = CanvasVertex {
            position: [x1, y0],
            color: c,
            uv,
        };
        let v2 = CanvasVertex {
            position: [x1, y1],
            color: c,
            uv,
        };
        let v3 = CanvasVertex {
            position: [x0, y1],
            color: c,
            uv,
        };
        self.verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
    }

    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        screen_size: (u32, u32),
        atlas: &FontAtlas,
    ) {
        let scale = size / FONT_SIZE;
        let c = color.to_array();
        let mut cursor_x = x;

        for ch in text.chars() {
            let idx = ch as usize;
            if idx >= 128 {
                continue;
            }
            let entry = match atlas.glyphs[idx] {
                Some(e) => e,
                None => continue,
            };

            if entry.width_px > 0.0 {
                let gx = cursor_x + entry.x_offset * scale;
                let gy = y + (atlas.line_height - entry.y_offset - entry.height_px) * scale;
                let gw = entry.width_px * scale;
                let gh = entry.height_px * scale;

                let [x0, y0] = world_to_ndc(gx, gy, screen_size);
                let [x1, y1] = world_to_ndc(gx + gw, gy + gh, screen_size);

                let v0 = CanvasVertex {
                    position: [x0, y0],
                    color: c,
                    uv: [entry.u0, entry.v1],
                };
                let v1 = CanvasVertex {
                    position: [x1, y0],
                    color: c,
                    uv: [entry.u1, entry.v1],
                };
                let v2 = CanvasVertex {
                    position: [x1, y1],
                    color: c,
                    uv: [entry.u1, entry.v0],
                };
                let v3 = CanvasVertex {
                    position: [x0, y1],
                    color: c,
                    uv: [entry.u0, entry.v0],
                };
                self.verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
            }

            cursor_x += entry.advance * scale;
        }
    }
}

/// Convert world coordinates (center-origin, Y-up) to NDC.
///
/// The world coordinate system has (0, 0) at the centre of the viewport,
/// with *x* increasing rightward and *y* increasing upward – the same
/// convention used by the sprite renderer.
pub fn world_to_ndc(x: f32, y: f32, screen_size: (u32, u32)) -> [f32; 2] {
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    [x / hw, y / hh]
}

pub(crate) fn draw_fps(canvas: &mut Canvas, fps: f32, screen_size: (u32, u32), atlas: &FontAtlas) {
    let text = format!("{}", fps.round() as u32);
    let size = 16.0;
    let scale = size / FONT_SIZE;
    let mut text_w: f32 = 0.0;
    for ch in text.chars() {
        let idx = ch as usize;
        if idx < 128 {
            if let Some(e) = atlas.glyphs[idx] {
                text_w += e.advance * scale;
            }
        }
    }
    let bg_w = text_w + 8.0;
    let bg_h = size + 8.0;
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    canvas.rect(
        -hw + 4.0,
        hh - 4.0 - bg_h,
        bg_w,
        bg_h,
        Color::from_rgba8(0, 0, 0, 160),
        screen_size,
    );
    canvas.text(
        -hw + 8.0,
        hh - 8.0 - size,
        &text,
        size,
        Color::from_rgba8(0, 255, 0, 255),
        screen_size,
        atlas,
    );
}

pub(crate) fn pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    font_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("canvas_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("canvas.wgsl").into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("canvas_pipeline_layout"),
        bind_group_layouts: &[font_bgl],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("canvas_pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[CanvasVertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("canvas_vertex_buffer"),
        size: (MAX_CANVAS_VERTICES * std::mem::size_of::<CanvasVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub(crate) fn render_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
    canvases: &[Canvas],
    font_atlas: &FontAtlas,
) {
    let verts: Vec<CanvasVertex> = canvases
        .iter()
        .flat_map(|c| c.verts.iter().copied())
        .collect();
    if verts.is_empty() {
        return;
    }
    queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&verts));

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("canvas_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &font_atlas.bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    pass.draw(0..verts.len() as u32, 0..1);
}
