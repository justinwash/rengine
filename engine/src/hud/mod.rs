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

/// Convert a screen-space pixel coordinate to NDC.
pub fn screen_to_ndc(x: f32, y: f32, screen_size: (u32, u32)) -> [f32; 2] {
    let sw = screen_size.0 as f32;
    let sh = screen_size.1 as f32;
    [(x / sw) * 2.0 - 1.0, 1.0 - (y / sh) * 2.0]
}

/// Push pre-built triangles directly into the HUD vertex buffer.
/// The caller provides vertices already in NDC with colors set.
pub fn push_shape(verts: &mut Vec<HudVertex>, triangles: &[HudVertex]) {
    verts.extend_from_slice(triangles);
}

/// Push an axis-aligned rectangle in screen-pixel coordinates.
pub fn push_rect(
    verts: &mut Vec<HudVertex>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    screen_size: (u32, u32),
) {
    let [x0, y0] = screen_to_ndc(x, y, screen_size);
    let [x1, y1] = screen_to_ndc(x + w, y + h, screen_size);

    let c = color.to_array();
    let v0 = HudVertex { position: [x0, y0], color: c };
    let v1 = HudVertex { position: [x1, y0], color: c };
    let v2 = HudVertex { position: [x1, y1], color: c };
    let v3 = HudVertex { position: [x0, y1], color: c };
    verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
}

const GLYPH_BITMAPS: [[u8; 5]; 96] = {
    let mut table = [[0u8; 5]; 96];
    // digits 0-9 (ASCII 48-57, index 16-25)
    table[16] = [0b111, 0b101, 0b101, 0b101, 0b111]; // 0
    table[17] = [0b010, 0b110, 0b010, 0b010, 0b111]; // 1
    table[18] = [0b111, 0b001, 0b111, 0b100, 0b111]; // 2
    table[19] = [0b111, 0b001, 0b111, 0b001, 0b111]; // 3
    table[20] = [0b101, 0b101, 0b111, 0b001, 0b001]; // 4
    table[21] = [0b111, 0b100, 0b111, 0b001, 0b111]; // 5
    table[22] = [0b111, 0b100, 0b111, 0b101, 0b111]; // 6
    table[23] = [0b111, 0b001, 0b010, 0b010, 0b010]; // 7
    table[24] = [0b111, 0b101, 0b111, 0b101, 0b111]; // 8
    table[25] = [0b111, 0b101, 0b111, 0b001, 0b111]; // 9
    // uppercase A-Z (ASCII 65-90, index 33-58)
    table[33] = [0b010, 0b101, 0b111, 0b101, 0b101]; // A
    table[34] = [0b110, 0b101, 0b110, 0b101, 0b110]; // B
    table[35] = [0b111, 0b100, 0b100, 0b100, 0b111]; // C
    table[36] = [0b110, 0b101, 0b101, 0b101, 0b110]; // D
    table[37] = [0b111, 0b100, 0b110, 0b100, 0b111]; // E
    table[38] = [0b111, 0b100, 0b110, 0b100, 0b100]; // F
    table[39] = [0b111, 0b100, 0b101, 0b101, 0b111]; // G
    table[40] = [0b101, 0b101, 0b111, 0b101, 0b101]; // H
    table[41] = [0b111, 0b010, 0b010, 0b010, 0b111]; // I
    table[42] = [0b001, 0b001, 0b001, 0b101, 0b010]; // J
    table[43] = [0b101, 0b110, 0b100, 0b110, 0b101]; // K
    table[44] = [0b100, 0b100, 0b100, 0b100, 0b111]; // L
    table[45] = [0b101, 0b111, 0b111, 0b101, 0b101]; // M
    table[46] = [0b101, 0b111, 0b111, 0b111, 0b101]; // N
    table[47] = [0b010, 0b101, 0b101, 0b101, 0b010]; // O
    table[48] = [0b110, 0b101, 0b110, 0b100, 0b100]; // P
    table[49] = [0b010, 0b101, 0b101, 0b110, 0b011]; // Q
    table[50] = [0b110, 0b101, 0b110, 0b101, 0b101]; // R
    table[51] = [0b111, 0b100, 0b111, 0b001, 0b111]; // S
    table[52] = [0b111, 0b010, 0b010, 0b010, 0b010]; // T
    table[53] = [0b101, 0b101, 0b101, 0b101, 0b111]; // U
    table[54] = [0b101, 0b101, 0b101, 0b101, 0b010]; // V
    table[55] = [0b101, 0b101, 0b111, 0b111, 0b101]; // W
    table[56] = [0b101, 0b101, 0b010, 0b101, 0b101]; // X
    table[57] = [0b101, 0b101, 0b010, 0b010, 0b010]; // Y
    table[58] = [0b111, 0b001, 0b010, 0b100, 0b111]; // Z
    // lowercase maps to uppercase
    // (handled at lookup time by converting to uppercase)
    // punctuation
    table[14] = [0b000, 0b000, 0b000, 0b000, 0b010]; // . (ASCII 46)
    table[13] = [0b000, 0b000, 0b010, 0b000, 0b010]; // : (ASCII 58 → we store at index 26 below)
    table[11] = [0b000, 0b000, 0b111, 0b000, 0b000]; // - (ASCII 45)
    table[26] = [0b000, 0b000, 0b010, 0b000, 0b010]; // : (ASCII 58)
    table[15] = [0b001, 0b010, 0b100, 0b010, 0b001]; // / (ASCII 47)
    table[1]  = [0b010, 0b010, 0b010, 0b000, 0b010]; // ! (ASCII 33)
    table
};

fn glyph_index(ch: char) -> Option<usize> {
    let c = ch as u32;
    if c < 32 || c > 127 { return None; }
    let idx = (c - 32) as usize;
    // map lowercase to uppercase
    if ch.is_ascii_lowercase() {
        return Some((ch.to_ascii_uppercase() as u32 - 32) as usize);
    }
    Some(idx)
}

/// Render a text string using the built-in 3×5 bitmap font.
/// `x`, `y` are screen-pixel coordinates for the top-left of the first character.
/// `scale` is the pixel size of each dot in the glyph.
pub fn push_text(
    verts: &mut Vec<HudVertex>,
    mut x: f32,
    y: f32,
    text: &str,
    scale: f32,
    color: Color,
    screen_size: (u32, u32),
) {
    for ch in text.chars() {
        if ch == ' ' {
            x += 4.0 * scale;
            continue;
        }
        if let Some(idx) = glyph_index(ch) {
            let bitmap = &GLYPH_BITMAPS[idx];
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

/// Convenience: render the FPS counter in the top-left corner.
pub(crate) fn push_fps(verts: &mut Vec<HudVertex>, fps: f32, screen_size: (u32, u32)) {
    let text = format!("{}", fps.round() as u32);
    let scale = 3.0;
    let char_count = text.len() as f32;
    let bg_w = (char_count * 4.0 - 1.0) * scale + 8.0;
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
    push_text(
        verts,
        8.0,
        8.0,
        &text,
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
