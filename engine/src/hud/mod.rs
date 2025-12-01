use crate::assets::Color;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HudVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl HudVertex {
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
            ],
        }
    }
}

pub const MAX_HUD_VERTICES: usize = 4_000;

pub fn create_hud_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("hud_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("hud.wgsl").into()),
    });

    let hud_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("hud_pipeline_layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("hud_pipeline"),
        layout: Some(&hud_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &hud_shader,
            entry_point: Some("vs_main"),
            buffers: &[HudVertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &hud_shader,
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

pub fn create_hud_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hud_vertex_buffer"),
        size: (MAX_HUD_VERTICES * std::mem::size_of::<HudVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub fn push_rect(
    verts: &mut Vec<HudVertex>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    screen_size: (u32, u32),
) {
    let sw = screen_size.0 as f32;
    let sh = screen_size.1 as f32;
    let ndc_x0 = (x / sw) * 2.0 - 1.0;
    let ndc_y0 = 1.0 - (y / sh) * 2.0;
    let ndc_x1 = ((x + w) / sw) * 2.0 - 1.0;
    let ndc_y1 = 1.0 - ((y + h) / sh) * 2.0;

    let c = color.to_array();
    let v0 = HudVertex {
        position: [ndc_x0, ndc_y0],
        color: c,
    };
    let v1 = HudVertex {
        position: [ndc_x1, ndc_y0],
        color: c,
    };
    let v2 = HudVertex {
        position: [ndc_x1, ndc_y1],
        color: c,
    };
    let v3 = HudVertex {
        position: [ndc_x0, ndc_y1],
        color: c,
    };
    verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
}

pub fn push_crosshair(
    verts: &mut Vec<HudVertex>,
    size: f32,
    thickness: f32,
    color: Color,
    screen_size: (u32, u32),
) {
    let cx = screen_size.0 as f32 / 2.0;
    let cy = screen_size.1 as f32 / 2.0;
    let ht = thickness / 2.0;
    push_rect(
        verts,
        cx - size,
        cy - ht,
        size * 2.0,
        thickness,
        color,
        screen_size,
    );
    push_rect(
        verts,
        cx - ht,
        cy - size,
        thickness,
        size * 2.0,
        color,
        screen_size,
    );
}

const DIGIT_BITMAPS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

pub fn push_number(
    verts: &mut Vec<HudVertex>,
    mut x: f32,
    y: f32,
    value: u32,
    scale: f32,
    color: Color,
    screen_size: (u32, u32),
) {
    let text = value.to_string();
    for ch in text.chars() {
        let digit = (ch as u32 - '0' as u32) as usize;
        if digit < 10 {
            let bitmap = &DIGIT_BITMAPS[digit];
            for (row, &bits) in bitmap.iter().enumerate() {
                for col in 0..3 {
                    if (bits >> (2 - col)) & 1 == 1 {
                        push_rect(
                            verts,
                            x + col as f32 * scale,
                            y + row as f32 * scale,
                            scale,
                            scale,
                            color,
                            screen_size,
                        );
                    }
                }
            }
        }
        x += 4.0 * scale;
    }
}

pub fn push_fps(verts: &mut Vec<HudVertex>, fps: f32, screen_size: (u32, u32)) {
    let fps_rounded = fps.round() as u32;

    let digits = if fps_rounded >= 1000 {
        4
    } else if fps_rounded >= 100 {
        3
    } else if fps_rounded >= 10 {
        2
    } else {
        1
    };
    let scale = 3.0;
    let bg_w = (digits as f32 * 4.0 - 1.0) * scale + 8.0;
    let bg_h = 5.0 * scale + 8.0;
    push_rect(
        verts,
        4.0,
        4.0,
        bg_w,
        bg_h,
        Color::from_rgba8(0, 0, 0, 160),
        screen_size,
    );

    push_number(
        verts,
        8.0,
        8.0,
        fps_rounded,
        scale,
        Color::from_rgba8(0, 255, 0, 255),
        screen_size,
    );
}

pub fn render_hud_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
    hud_verts: &[HudVertex],
) {
    if hud_verts.is_empty() {
        return;
    }
    queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(hud_verts));

    let mut hud_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("hud_pass"),
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
    hud_pass.set_pipeline(pipeline);
    hud_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    hud_pass.draw(0..hud_verts.len() as u32, 0..1);
}
