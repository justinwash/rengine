use crate::assets::Color;
use crate::renderer::TextureId;
use crate::text::{FontAtlas, ATLAS_SIZE, FONT_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DrawTexture {
    Font(usize),
    Texture(usize),
}

pub(crate) struct DrawSegment {
    pub start: usize,
    pub count: usize,
    pub scissor: Option<[u32; 4]>,
    pub texture: DrawTexture,
}

pub struct Canvas {
    pub(crate) verts: Vec<CanvasVertex>,
    pub(crate) segments: Vec<DrawSegment>,
    screen_size: (u32, u32),
    clip_stack: Vec<[u32; 4]>,
    segment_start: usize,
    current_texture: DrawTexture,
    atlas: *const FontAtlas,
}

impl Canvas {
    pub(crate) fn new(screen_size: (u32, u32), atlas: *const FontAtlas) -> Self {
        Self {
            verts: Vec::new(),
            segments: Vec::new(),
            screen_size,
            clip_stack: Vec::new(),
            segment_start: 0,
            current_texture: DrawTexture::Font(0),
            atlas,
        }
    }

    fn atlas(&self) -> &FontAtlas {
        // SAFETY: `Canvas::new` stores a raw pointer to a `FontAtlas`.
        // The pointer is validated non-null below. The atlas lives inside
        // Engine for the entire program lifetime, so it always outlives
        // any Canvas instance.
        let ptr = self.atlas;
        assert!(
            !ptr.is_null(),
            "Canvas font atlas not initialized; call Frame::begin() before drawing text"
        );
        unsafe { &*ptr }
    }

    pub fn screen_size(&self) -> (u32, u32) {
        self.screen_size
    }

    pub fn push_clip(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.close_segment();

        let (sw, sh) = self.screen_size;
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;

        let px = ((x + hw).max(0.0)) as u32;
        let py = ((hh - y - h).max(0.0)) as u32;
        let pw = (w as u32).min(sw.saturating_sub(px));
        let ph = (h as u32).min(sh.saturating_sub(py));

        let mut rect = [px, py, pw, ph];

        if let Some(parent) = self.clip_stack.last() {
            let l = rect[0].max(parent[0]);
            let t = rect[1].max(parent[1]);
            let r = (rect[0] + rect[2]).min(parent[0] + parent[2]);
            let b = (rect[1] + rect[3]).min(parent[1] + parent[3]);
            if l >= r || t >= b {
                rect = [0, 0, 0, 0];
            } else {
                rect = [l, t, r - l, b - t];
            }
        }

        self.clip_stack.push(rect);
    }

    pub fn pop_clip(&mut self) {
        self.close_segment();
        self.clip_stack.pop();
    }

    fn close_segment(&mut self) {
        let count = self.verts.len() - self.segment_start;
        if count > 0 {
            self.segments.push(DrawSegment {
                start: self.segment_start,
                count,
                scissor: self.clip_stack.last().copied(),
                texture: self.current_texture,
            });
        }
        self.segment_start = self.verts.len();
    }

    fn set_font(&mut self, font_id: usize) {
        self.set_texture(DrawTexture::Font(font_id));
    }

    fn set_texture(&mut self, texture: DrawTexture) {
        if texture != self.current_texture {
            self.close_segment();
            self.current_texture = texture;
        }
    }

    pub(crate) fn finalize(&mut self) {
        self.close_segment();
    }

    pub fn shape(&mut self, triangles: &[CanvasVertex]) {
        self.set_font(0);
        self.verts.extend_from_slice(triangles);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.set_font(0);
        let [x0, y0] = screen_to_ndc(x, y, self.screen_size);
        let [x1, y1] = screen_to_ndc(x + w, y + h, self.screen_size);

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

    /// Fill a rect with a smooth vertical gradient (`bottom` at `y`, `top` at
    /// `y + h`), interpolated per-vertex by the GPU.
    pub fn rect_gradient(&mut self, x: f32, y: f32, w: f32, h: f32, bottom: Color, top: Color) {
        self.set_font(0);
        let [x0, y0] = screen_to_ndc(x, y, self.screen_size);
        let [x1, y1] = screen_to_ndc(x + w, y + h, self.screen_size);
        let cb = bottom.to_array();
        let ct = top.to_array();
        let uv = WHITE_UV;
        let v0 = CanvasVertex { position: [x0, y0], color: cb, uv };
        let v1 = CanvasVertex { position: [x1, y0], color: cb, uv };
        let v2 = CanvasVertex { position: [x1, y1], color: ct, uv };
        let v3 = CanvasVertex { position: [x0, y1], color: ct, uv };
        self.verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
    }

    /// Draw a raised-bevel outline around a rect: `highlight` on the top/left
    /// edges, `shadow` on the bottom/right (assuming a y-up coordinate space,
    /// i.e. `y + h` is the top edge).
    pub fn bevel_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        highlight: Color,
        shadow: Color,
        thickness: f32,
    ) {
        self.line(x, y + h, x + w, y + h, thickness, highlight);
        self.line(x, y, x, y + h, thickness, highlight);
        self.line(x, y, x + w, y, thickness, shadow);
        self.line(x + w, y, x + w, y + h, thickness, shadow);
    }

    /// Fill a rect with rounded corners of the given `radius`.
    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
        let r = radius.max(0.0).min(w * 0.5).min(h * 0.5);
        if r <= 0.5 {
            self.rect(x, y, w, h, color);
            return;
        }
        self.rect(x + r, y, w - 2.0 * r, h, color);
        self.rect(x, y + r, r, h - 2.0 * r, color);
        self.rect(x + w - r, y + r, r, h - 2.0 * r, color);
        self.circle_filled(x + r, y + r, r, 14, color);
        self.circle_filled(x + w - r, y + r, r, 14, color);
        self.circle_filled(x + r, y + h - r, r, 14, color);
        self.circle_filled(x + w - r, y + h - r, r, 14, color);
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: Color) {
        self.set_font(0);
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.0001 {
            return;
        }
        let nx = -dy / len * thickness * 0.5;
        let ny = dx / len * thickness * 0.5;

        let c = color.to_array();
        let uv = WHITE_UV;
        let a = screen_to_ndc(x0 + nx, y0 + ny, self.screen_size);
        let b = screen_to_ndc(x0 - nx, y0 - ny, self.screen_size);
        let cc = screen_to_ndc(x1 - nx, y1 - ny, self.screen_size);
        let d = screen_to_ndc(x1 + nx, y1 + ny, self.screen_size);

        let va = CanvasVertex {
            position: a,
            color: c,
            uv,
        };
        let vb = CanvasVertex {
            position: b,
            color: c,
            uv,
        };
        let vc = CanvasVertex {
            position: cc,
            color: c,
            uv,
        };
        let vd = CanvasVertex {
            position: d,
            color: c,
            uv,
        };
        self.verts.extend_from_slice(&[va, vc, vd, va, vb, vc]);
    }

    pub fn polyline(&mut self, points: &[(f32, f32)], thickness: f32, color: Color) {
        for pair in points.windows(2) {
            self.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, thickness, color);
        }
    }

    pub fn circle(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        thickness: f32,
        segments: u32,
        color: Color,
    ) {
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a0 = step * i as f32;
            let a1 = step * (i + 1) as f32;
            self.line(
                cx + a0.cos() * radius,
                cy + a0.sin() * radius,
                cx + a1.cos() * radius,
                cy + a1.sin() * radius,
                thickness,
                color,
            );
        }
    }

    pub fn circle_filled(&mut self, cx: f32, cy: f32, radius: f32, segments: u32, color: Color) {
        self.set_font(0);
        let c = color.to_array();
        let uv = WHITE_UV;
        let center = screen_to_ndc(cx, cy, self.screen_size);
        let vc = CanvasVertex {
            position: center,
            color: c,
            uv,
        };
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a0 = step * i as f32;
            let a1 = step * (i + 1) as f32;
            let p0 = screen_to_ndc(
                cx + a0.cos() * radius,
                cy + a0.sin() * radius,
                self.screen_size,
            );
            let p1 = screen_to_ndc(
                cx + a1.cos() * radius,
                cy + a1.sin() * radius,
                self.screen_size,
            );
            let v0 = CanvasVertex {
                position: p0,
                color: c,
                uv,
            };
            let v1 = CanvasVertex {
                position: p1,
                color: c,
                uv,
            };
            self.verts.extend_from_slice(&[vc, v0, v1]);
        }
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
        let ptr = self.atlas;
        assert!(!ptr.is_null(), "Canvas font atlas not initialized");
        let atlas = unsafe { &*ptr };
        self.text_with_font(x, y, text, size, color, atlas);
    }

    pub fn text_with_font(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        atlas: &FontAtlas,
    ) {
        self.set_font(atlas.id().0);
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
                let gy = y - (atlas.line_height - entry.y_offset) * scale;
                let gw = entry.width_px * scale;
                let gh = entry.height_px * scale;

                let [x0, y0] = screen_to_ndc(gx, gy, self.screen_size);
                let [x1, y1] = screen_to_ndc(gx + gw, gy + gh, self.screen_size);

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

    pub fn text_aligned(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        let atlas = self.atlas();
        let offset = if align == TextAlign::Left {
            0.0
        } else {
            let (w, _) = atlas.measure_text(text, size);
            match align {
                TextAlign::Center => -w / 2.0,
                TextAlign::Right => -w,
                TextAlign::Left => unreachable!(),
            }
        };
        self.text(x + offset, y, text, size, color);
    }

    pub fn text_spans(&mut self, x: f32, y: f32, spans: &[(&str, Color)], size: f32) {
        let ptr = self.atlas;
        assert!(!ptr.is_null(), "Canvas font atlas not initialized");
        let atlas = unsafe { &*ptr };
        self.text_spans_with_font(x, y, spans, size, atlas);
    }

    pub fn text_spans_with_font(
        &mut self,
        x: f32,
        y: f32,
        spans: &[(&str, Color)],
        size: f32,
        atlas: &FontAtlas,
    ) {
        self.set_font(atlas.id().0);
        let scale = size / FONT_SIZE;
        let mut cursor_x = x;

        for &(span_text, span_color) in spans {
            let c = span_color.to_array();
            for ch in span_text.chars() {
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
                    let gy = y - (atlas.line_height - entry.y_offset) * scale;
                    let gw = entry.width_px * scale;
                    let gh = entry.height_px * scale;

                    let [x0, y0] = screen_to_ndc(gx, gy, self.screen_size);
                    let [x1, y1] = screen_to_ndc(gx + gw, gy + gh, self.screen_size);

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

    pub fn image(&mut self, texture: TextureId, x: f32, y: f32, w: f32, h: f32) {
        self.image_colored(texture, x, y, w, h, Color::WHITE);
    }

    pub fn image_colored(
        &mut self,
        texture: TextureId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Color,
    ) {
        self.image_region(texture, x, y, w, h, [0.0, 0.0, 1.0, 1.0], color);
    }

    pub fn image_region(
        &mut self,
        texture: TextureId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv_rect: [f32; 4],
        color: Color,
    ) {
        self.set_texture(DrawTexture::Texture(texture.0));

        let [x0, y0] = screen_to_ndc(x, y, self.screen_size);
        let [x1, y1] = screen_to_ndc(x + w, y + h, self.screen_size);

        let c = color.to_array();
        let [u0, v0, uw, vh] = uv_rect;
        let u1 = u0 + uw;
        let v_bottom = v0 + vh;

        let bottom_left = CanvasVertex {
            position: [x0, y0],
            color: c,
            uv: [u0, v_bottom],
        };
        let bottom_right = CanvasVertex {
            position: [x1, y0],
            color: c,
            uv: [u1, v_bottom],
        };
        let top_right = CanvasVertex {
            position: [x1, y1],
            color: c,
            uv: [u1, v0],
        };
        let top_left = CanvasVertex {
            position: [x0, y1],
            color: c,
            uv: [u0, v0],
        };
        self.verts.extend_from_slice(&[
            bottom_left,
            top_right,
            bottom_right,
            bottom_left,
            top_left,
            top_right,
        ]);
    }

    pub fn text_spans_aligned(
        &mut self,
        x: f32,
        y: f32,
        spans: &[(&str, Color)],
        size: f32,
        align: TextAlign,
    ) {
        let atlas = self.atlas();
        let offset = if align == TextAlign::Left {
            0.0
        } else {
            let total_w: f32 = spans
                .iter()
                .map(|(s, _)| atlas.measure_text(s, size).0)
                .sum();
            match align {
                TextAlign::Center => -total_w / 2.0,
                TextAlign::Right => -total_w,
                TextAlign::Left => unreachable!(),
            }
        };
        self.text_spans(x + offset, y, spans, size);
    }

    pub fn text_block(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        max_width: f32,
        align: TextAlign,
    ) {
        let lines = {
            let atlas = self.atlas();
            wrap_text(text, size, max_width, atlas)
        };
        self.text_block_lines(x, y, &lines, size, color, align);
    }

    pub(crate) fn text_block_lines(
        &mut self,
        x: f32,
        y: f32,
        lines: &[String],
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        let atlas = self.atlas();
        let lh = atlas.line_height(size);
        for (i, line) in lines.iter().enumerate() {
            let ly = y - (i as f32) * lh;
            self.text_aligned(x, ly, line, size, color, align);
        }
    }

    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        self.atlas().measure_text(text, size)
    }

    pub fn line_height(&self, size: f32) -> f32 {
        self.atlas().line_height(size)
    }
}

pub fn screen_to_ndc(x: f32, y: f32, screen_size: (u32, u32)) -> [f32; 2] {
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    [x / hw, y / hh]
}

pub fn wrap_text(text: &str, size: f32, max_width: f32, atlas: &FontAtlas) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let words: Vec<&str> = raw_line.split(' ').collect();
        let mut current = String::new();
        let mut current_w: f32 = 0.0;
        let space_w = atlas.measure_text(" ", size).0;
        for word in &words {
            let word_w = atlas.measure_text(word, size).0;
            if current.is_empty() {
                if word_w > max_width {
                    lines.push((*word).to_string());
                } else {
                    current = (*word).to_string();
                    current_w = word_w;
                }
            } else if current_w + space_w + word_w <= max_width {
                current.push(' ');
                current.push_str(word);
                current_w += space_w + word_w;
            } else {
                lines.push(current);
                if word_w > max_width {
                    lines.push((*word).to_string());
                    current = String::new();
                    current_w = 0.0;
                } else {
                    current = (*word).to_string();
                    current_w = word_w;
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(crate) fn draw_fps(canvas: &mut Canvas, fps: f32) {
    let screen_size = canvas.screen_size();
    let text = format!("{}", fps.round() as u32);
    let size = 16.0;
    let (text_w, _) = canvas.measure_text(&text, size);
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
    );
    canvas.text(
        -hw + 8.0,
        hh - 8.0,
        &text,
        size,
        Color::from_rgba8(0, 255, 0, 255),
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

pub(crate) fn vertex_buffer(device: &wgpu::Device, vertex_capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("canvas_vertex_buffer"),
        size: (vertex_capacity * std::mem::size_of::<CanvasVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn required_vertex_buffer_capacity(current_capacity: usize, required_vertices: usize) -> usize {
    if required_vertices <= current_capacity {
        return current_capacity;
    }

    required_vertices
        .max(current_capacity.max(MAX_CANVAS_VERTICES))
        .checked_next_power_of_two()
        .unwrap_or(required_vertices)
}

pub(crate) fn render_pass<'a, F>(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &mut wgpu::Buffer,
    vertex_capacity: &mut usize,
    queue: &wgpu::Queue,
    canvases: &mut [Canvas],
    fonts: &[FontAtlas],
    texture_bind_group: F,
) where
    F: Fn(usize) -> Option<&'a wgpu::BindGroup>,
{
    for canvas in canvases.iter_mut() {
        canvas.finalize();
    }

    let verts: Vec<CanvasVertex> = canvases
        .iter()
        .flat_map(|c| c.verts.iter().copied())
        .collect();
    if verts.is_empty() {
        return;
    }
    if verts.len() > *vertex_capacity {
        *vertex_capacity = required_vertex_buffer_capacity(*vertex_capacity, verts.len());
        *vertex_buffer = self::vertex_buffer(device, *vertex_capacity);
    }
    queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&verts));

    let mut global_segments: Vec<(usize, usize, Option<[u32; 4]>, DrawTexture)> = Vec::new();
    let mut offset = 0usize;
    for canvas in canvases.iter() {
        if canvas.segments.is_empty() {
            if !canvas.verts.is_empty() {
                global_segments.push((offset, canvas.verts.len(), None, DrawTexture::Font(0)));
            }
        } else {
            for seg in &canvas.segments {
                global_segments.push((offset + seg.start, seg.count, seg.scissor, seg.texture));
            }
        }
        offset += canvas.verts.len();
    }

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
    pass.set_bind_group(0, &fonts[0].bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));

    let needs_per_segment = global_segments
        .iter()
        .any(|(_, _, s, texture)| s.is_some() || *texture != DrawTexture::Font(0));

    if needs_per_segment {
        let surface_w = canvases.first().map(|c| c.screen_size.0).unwrap_or(1);
        let surface_h = canvases.first().map(|c| c.screen_size.1).unwrap_or(1);
        let mut bound_texture = DrawTexture::Font(0);

        for (start, count, scissor, texture) in &global_segments {
            if *count == 0 {
                continue;
            }
            if *texture != bound_texture {
                match *texture {
                    DrawTexture::Font(font_id) => {
                        if let Some(atlas) = fonts.get(font_id) {
                            pass.set_bind_group(0, &atlas.bind_group, &[]);
                        }
                    }
                    DrawTexture::Texture(texture_id) => {
                        if let Some(bind_group) = texture_bind_group(texture_id) {
                            pass.set_bind_group(0, bind_group, &[]);
                        }
                    }
                }
                bound_texture = *texture;
            }
            if let Some([sx, sy, sw, sh]) = scissor {
                if *sw == 0 || *sh == 0 {
                    continue;
                }
                pass.set_scissor_rect(*sx, *sy, *sw, *sh);
            } else {
                pass.set_scissor_rect(0, 0, surface_w, surface_h);
            }
            pass.draw(*start as u32..(*start + *count) as u32, 0..1);
        }
    } else {
        pass.draw(0..verts.len() as u32, 0..1);
    }
}
