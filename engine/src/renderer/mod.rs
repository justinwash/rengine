pub mod camera;
pub mod nineslice;
pub mod postfx;
pub mod sprite;
pub mod texture;

pub use camera::{Camera2D, CameraBounds};
pub use nineslice::NineSlice;
pub use postfx::{PostEffect, PostFxChain};
pub use sprite::{DrawParams, Sprite};
pub use texture::TextureId;

use crate::app::ScaleMode;
use crate::assets::Color;
use crate::canvas::{self, Canvas, DrawTexture};
use crate::text;
use crate::text::FontAtlas;

use postfx::PostFxPipeline;
use sprite::Vertex;

use std::cell::Cell;
use std::sync::Arc;
use winit::window::Window;

const MAX_SPRITES: usize = 10_000;
const MAX_VERTICES: usize = MAX_SPRITES * 4;
const MAX_INDICES: usize = MAX_SPRITES * 6;
const BYTES_PER_PIXEL: u32 = 4;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct RenderTarget {
    id: usize,
    texture: TextureId,
    width: u32,
    height: u32,
}

impl RenderTarget {
    pub fn texture_id(&self) -> TextureId {
        self.texture
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

struct TargetFrame {
    target_id: usize,
    texture: TextureId,
    frame: Frame,
}

pub struct Frame {
    pub(crate) sprites: Vec<DrawParams>,
    pub camera: Camera2D,
    pub clear_color: Color,

    pub(crate) canvases: Vec<Canvas>,
    render_targets: Vec<TargetFrame>,
    screen_size: (u32, u32),
    atlas: *const FontAtlas,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            sprites: Vec::with_capacity(256),
            camera: Camera2D::new(),
            clear_color: Color::BLACK,
            canvases: Vec::new(),
            render_targets: Vec::new(),
            screen_size: (1, 1),
            atlas: std::ptr::null(),
        }
    }

    pub fn screen_size(&self) -> (u32, u32) {
        self.screen_size
    }

    pub fn draw_sprite(&mut self, params: DrawParams) {
        self.sprites.push(params);
    }

    pub fn draw(&mut self, texture: TextureId, position: glam::Vec2, size: glam::Vec2) {
        self.sprites.push(DrawParams::new(texture, position, size));
    }

    pub fn draw_colored(
        &mut self,
        texture: TextureId,
        position: glam::Vec2,
        size: glam::Vec2,
        color: Color,
    ) {
        self.sprites
            .push(DrawParams::new(texture, position, size).with_color(color));
    }

    pub fn draw_nine_slice(
        &mut self,
        nine_slice: &NineSlice,
        position: glam::Vec2,
        size: glam::Vec2,
    ) {
        nine_slice.patches_into(position, size, &mut self.sprites);
    }

    pub fn begin(&mut self, screen_size: (u32, u32), atlas: &FontAtlas) {
        self.sprites.clear();
        self.canvases.clear();
        self.render_targets.clear();
        self.clear_color = Color::BLACK;
        self.screen_size = screen_size;
        self.atlas = atlas as *const FontAtlas;
    }

    pub fn canvas(&mut self, index: usize) -> &mut Canvas {
        assert!(
            !self.atlas.is_null(),
            "Frame font atlas not initialized; call begin() before canvas()"
        );
        let ss = self.screen_size;
        let a = self.atlas;
        if index >= self.canvases.len() {
            self.canvases.resize_with(index + 1, || Canvas::new(ss, a));
        }
        &mut self.canvases[index]
    }

    pub fn render_target(&mut self, target: &RenderTarget) -> &mut Frame {
        assert!(
            !self.atlas.is_null(),
            "Frame font atlas not initialized; call begin() before render_target()"
        );

        if let Some(index) = self
            .render_targets
            .iter()
            .position(|existing| existing.target_id == target.id)
        {
            return &mut self.render_targets[index].frame;
        }

        let atlas = unsafe { &*self.atlas };
        let mut frame = Frame::new();
        frame.begin(target.size(), atlas);
        self.render_targets.push(TargetFrame {
            target_id: target.id,
            texture: target.texture,
            frame,
        });
        &mut self.render_targets.last_mut().unwrap().frame
    }
}

fn push_active_render_target(active_targets: &mut Vec<usize>, texture_index: usize) {
    assert!(
        !active_targets.contains(&texture_index),
        "render target cycle detected while rendering texture {:?}",
        TextureId(texture_index)
    );
    active_targets.push(texture_index);
}

fn validate_render_target_frame(frame: &mut Frame, texture_index: usize) {
    let target_texture = TextureId(texture_index);
    assert!(
        frame
            .sprites
            .iter()
            .all(|sprite| sprite.texture != target_texture),
        "render target {:?} cannot sample itself in the sprite pass",
        target_texture
    );

    for canvas in &mut frame.canvases {
        canvas.finalize();
    }

    let uses_target_texture = frame.canvases.iter().any(|canvas| {
        canvas
            .segments
            .iter()
            .any(|segment| segment.texture == DrawTexture::Texture(texture_index))
    });
    assert!(
        !uses_target_texture,
        "render target {:?} cannot sample itself in the canvas pass",
        target_texture
    );
}

struct GpuTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

struct OffscreenTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    blit_pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
    scale_mode: Cell<ScaleMode>,
}

pub(crate) struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    projection_buffer: wgpu::Buffer,
    projection_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    textures: Vec<GpuTexture>,
    sampler: wgpu::Sampler,
    pub(crate) white_texture: TextureId,
    render_targets: Vec<TextureId>,

    canvas_pipeline: wgpu::RenderPipeline,
    canvas_vb: wgpu::Buffer,
    canvas_vb_capacity: usize,
    pub(crate) fonts: Vec<text::FontAtlas>,
    font_bgl: wgpu::BindGroupLayout,

    offscreen: Option<OffscreenTarget>,
    postfx: Option<PostFxPipeline>,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, present_mode: wgpu::PresentMode) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rengine_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                ..Default::default()
            })
            .await
            .expect("Failed to create GPU device");

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        let projection_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("projection_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&projection_bgl, &texture_bgl],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
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
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_data: Vec<u32> = (0..MAX_SPRITES as u32)
            .flat_map(|i| {
                let b = i * 4;
                [b, b + 1, b + 2, b + 2, b + 3, b]
            })
            .collect();

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_buffer"),
            size: (MAX_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&index_data));

        let projection_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("projection_buffer"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let projection_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("projection_bg"),
            layout: &projection_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: projection_buffer.as_entire_binding(),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let font_bgl = texture_bgl.clone();
        let canvas_pipeline = canvas::pipeline(&device, surface_format, &font_bgl);
        let canvas_vb_capacity = canvas::MAX_CANVAS_VERTICES;
        let canvas_vb = canvas::vertex_buffer(&device, canvas_vb_capacity);
        let font_atlas = text::font_atlas(&device, &queue, &font_bgl);

        let mut renderer = Self {
            surface,
            device,
            queue,
            surface_config,
            pipeline,
            vertex_buffer,
            index_buffer,
            projection_buffer,
            projection_bind_group,
            texture_bind_group_layout: texture_bgl,
            textures: Vec::new(),
            sampler,
            white_texture: TextureId(0),
            render_targets: Vec::new(),
            canvas_pipeline,
            canvas_vb,
            canvas_vb_capacity,
            fonts: vec![font_atlas],
            font_bgl,
            offscreen: None,
            postfx: None,
        };

        let white = renderer.create_texture(1, 1, &[255, 255, 255, 255]);
        renderer.white_texture = white;

        renderer
    }

    pub(crate) fn load_font(&mut self, font_bytes: &[u8]) -> text::FontId {
        let id = text::FontId(self.fonts.len());
        let atlas =
            text::build_atlas_from_bytes(&self.device, &self.queue, &self.font_bgl, font_bytes, id);
        self.fonts.push(atlas);
        id
    }

    fn build_texture(
        &self,
        width: u32,
        height: u32,
        pixels: Option<&[u8]>,
        render_target: bool,
        label: &'static str,
    ) -> GpuTexture {
        if let Some(pixels) = pixels {
            assert_eq!(
                pixels.len(),
                (width * height * 4) as usize,
                "pixel data length must match width × height × 4"
            );
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        if render_target {
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage,
            view_formats: &[],
        });

        if let Some(pixels) = pixels {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                size,
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        GpuTexture {
            _texture: texture,
            view,
            bind_group,
        }
    }

    fn padded_bytes_per_row(width: u32) -> u32 {
        let unpadded = width * BYTES_PER_PIXEL;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        unpadded.div_ceil(alignment) * alignment
    }

    pub fn create_texture(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureId {
        let id = TextureId(self.textures.len());
        self.textures.push(self.build_texture(
            width,
            height,
            Some(pixels),
            false,
            "sprite_texture",
        ));
        id
    }

    pub fn replace_texture(&mut self, id: TextureId, width: u32, height: u32, pixels: &[u8]) {
        self.textures[id.0] =
            self.build_texture(width, height, Some(pixels), false, "sprite_texture");
    }

    pub fn create_render_target(&mut self, width: u32, height: u32) -> RenderTarget {
        let texture = TextureId(self.textures.len());
        self.textures.push(self.build_texture(
            width.max(1),
            height.max(1),
            None,
            true,
            "render_target_texture",
        ));
        let id = self.render_targets.len();
        self.render_targets.push(texture);
        RenderTarget {
            id,
            texture,
            width: width.max(1),
            height: height.max(1),
        }
    }

    pub fn resize_render_target(&mut self, target: &mut RenderTarget, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        let known_texture = self
            .render_targets
            .get(target.id)
            .copied()
            .expect("invalid render target handle");
        assert_eq!(
            known_texture, target.texture,
            "render target handle is stale"
        );
        self.textures[target.texture.0] =
            self.build_texture(width, height, None, true, "render_target_texture");
        target.width = width;
        target.height = height;
    }

    fn upload_frame_batches(
        &mut self,
        frame: &Frame,
        proj_w: f32,
        proj_h: f32,
    ) -> Vec<(usize, u32)> {
        let projection = frame.camera.projection(proj_w, proj_h);
        self.queue.write_buffer(
            &self.projection_buffer,
            0,
            bytemuck::cast_slice(&projection.to_cols_array()),
        );

        let mut sorted: Vec<&DrawParams> = frame.sprites.iter().collect();
        sorted.sort_by(|a, b| {
            a.z_order
                .cmp(&b.z_order)
                .then(a.texture.0.cmp(&b.texture.0))
        });

        let mut vertices: Vec<Vertex> = Vec::with_capacity(sorted.len() * 4);
        for sp in &sorted {
            let (x, y, w, h) = (sp.position.x, sp.position.y, sp.size.x, sp.size.y);
            let color = sp.color.to_array();
            let [u0, v0, uw, vh] = sp.uv_rect;
            let (mut ul, mut ur) = (u0, u0 + uw);
            let (mut vt, mut vb) = (v0, v0 + vh);
            if sp.flip_x {
                std::mem::swap(&mut ul, &mut ur);
            }
            if sp.flip_y {
                std::mem::swap(&mut vt, &mut vb);
            }

            let ox = sp.origin.x;
            let oy = sp.origin.y;
            let corners = [
                [x - ox, y - oy + h],
                [x - ox + w, y - oy + h],
                [x - ox + w, y - oy],
                [x - ox, y - oy],
            ];

            let corners = if sp.rotation != 0.0 {
                let cos = sp.rotation.cos();
                let sin = sp.rotation.sin();
                let px = x;
                let py = y;
                corners.map(|[cx, cy]| {
                    let dx = cx - px;
                    let dy = cy - py;
                    [px + dx * cos - dy * sin, py + dx * sin + dy * cos]
                })
            } else {
                corners
            };

            let uvs = [[ul, vt], [ur, vt], [ur, vb], [ul, vb]];

            for index in 0..4 {
                vertices.push(Vertex {
                    position: corners[index],
                    tex_coords: uvs[index],
                    color,
                });
            }
        }

        if !vertices.is_empty() {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        let mut batches: Vec<(usize, u32)> = Vec::new();
        if !sorted.is_empty() {
            let mut cur_tex = sorted[0].texture.0;
            let mut count = 1u32;
            for sp in sorted.iter().skip(1) {
                if sp.texture.0 == cur_tex {
                    count += 1;
                } else {
                    batches.push((cur_tex, count));
                    cur_tex = sp.texture.0;
                    count = 1;
                }
            }
            batches.push((cur_tex, count));
        }

        batches
    }

    fn render_frame_to_texture(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &mut Frame,
        texture_index: usize,
        active_targets: &mut Vec<usize>,
    ) {
        push_active_render_target(active_targets, texture_index);
        self.render_nested_targets(encoder, frame, active_targets);
        validate_render_target_frame(frame, texture_index);
        let batches = self.upload_frame_batches(
            frame,
            frame.screen_size.0 as f32,
            frame.screen_size.1 as f32,
        );

        {
            let target_view = &self.textures[texture_index].view;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_target_sprite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(frame.clear_color.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !batches.is_empty() {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.projection_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                let mut sprite_offset: u32 = 0;
                for &(tex_idx, count) in &batches {
                    pass.set_bind_group(1, &self.textures[tex_idx].bind_group, &[]);
                    let idx_start = sprite_offset * 6;
                    let idx_count = count * 6;
                    pass.draw_indexed(idx_start..idx_start + idx_count, 0, 0..1);
                    sprite_offset += count;
                }
            }
        }

        let target_view = &self.textures[texture_index].view;
        let device = &self.device;
        let canvas_pipeline = &self.canvas_pipeline;
        let canvas_vb = &mut self.canvas_vb;
        let canvas_vb_capacity = &mut self.canvas_vb_capacity;
        let queue = &self.queue;
        let fonts = &self.fonts;
        let textures = &self.textures;
        canvas::render_pass(
            device,
            encoder,
            target_view,
            canvas_pipeline,
            canvas_vb,
            canvas_vb_capacity,
            queue,
            &mut frame.canvases,
            fonts,
            |texture_id| textures.get(texture_id).map(|texture| &texture.bind_group),
        );

        let popped = active_targets.pop();
        debug_assert_eq!(popped, Some(texture_index));
    }

    fn render_nested_targets(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &mut Frame,
        active_targets: &mut Vec<usize>,
    ) {
        for target_frame in &mut frame.render_targets {
            self.render_frame_to_texture(
                encoder,
                &mut target_frame.frame,
                target_frame.texture.0,
                active_targets,
            );
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    fn encode_frame_to_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &mut Frame,
        postfx_chain: &PostFxChain,
        final_view: &wgpu::TextureView,
        final_size: (u32, u32),
    ) {
        {
            let effects = postfx_chain.effects.borrow();
            let is_dirty = *postfx_chain.dirty.borrow();
            if effects.is_empty() {
                if is_dirty {
                    self.postfx = None;
                    *postfx_chain.dirty.borrow_mut() = false;
                }
            } else if self.offscreen.is_some() {
                let ofs = self.offscreen.as_ref().unwrap();
                let (w, h) = (ofs.width, ofs.height);
                if self.postfx.is_none() {
                    let mut pfx =
                        PostFxPipeline::new(&self.device, w, h, self.surface_config.format);
                    pfx.set_source_view(&self.device, &ofs.view);
                    self.postfx = Some(pfx);
                }
                let pfx = self.postfx.as_mut().unwrap();
                pfx.resize(&self.device, w, h);
                if is_dirty {
                    pfx.set_source_view(&self.device, &ofs.view);
                    pfx.rebuild(&self.device, &effects);
                    *postfx_chain.dirty.borrow_mut() = false;
                }
            }
        }

        let mut active_targets = Vec::new();
        self.render_nested_targets(encoder, frame, &mut active_targets);

        let (proj_w, proj_h) = match self.offscreen {
            Some(ref ofs) => (ofs.width as f32, ofs.height as f32),
            None => (
                self.surface_config.width as f32,
                self.surface_config.height as f32,
            ),
        };
        let batches = self.upload_frame_batches(frame, proj_w, proj_h);
        let sprite_target = match self.offscreen {
            Some(ref ofs) => &ofs.view,
            None => final_view,
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: sprite_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(frame.clear_color.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !batches.is_empty() {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.projection_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                let mut sprite_offset: u32 = 0;
                for &(tex_idx, count) in &batches {
                    pass.set_bind_group(1, &self.textures[tex_idx].bind_group, &[]);
                    let idx_start = sprite_offset * 6;
                    let idx_count = count * 6;
                    pass.draw_indexed(idx_start..idx_start + idx_count, 0, 0..1);
                    sprite_offset += count;
                }
            }
        }

        if let Some(ref ofs) = self.offscreen {
            let blit_source_bg = if let Some(ref pfx) = self.postfx {
                if pfx.pass_count() > 0 {
                    let effects = postfx_chain.effects.borrow();
                    pfx.run(encoder, &self.queue, &effects);
                    pfx.last_output_bind_group(pfx.pass_count())
                } else {
                    &ofs.bind_group
                }
            } else {
                &ofs.bind_group
            };

            let (vx, vy, vw, vh) = blit_viewport(
                ofs.scale_mode.get(),
                ofs.width,
                ofs.height,
                final_size.0,
                final_size.1,
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: final_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&ofs.blit_pipeline);
            pass.set_bind_group(0, blit_source_bg, &[]);
            pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
            pass.draw(0..3, 0..1);
        }

        let device = &self.device;
        let canvas_pipeline = &self.canvas_pipeline;
        let canvas_vb = &mut self.canvas_vb;
        let canvas_vb_capacity = &mut self.canvas_vb_capacity;
        let queue = &self.queue;
        let fonts = &self.fonts;
        let textures = &self.textures;
        canvas::render_pass(
            device,
            encoder,
            final_view,
            canvas_pipeline,
            canvas_vb,
            canvas_vb_capacity,
            queue,
            &mut frame.canvases,
            fonts,
            |texture_id| textures.get(texture_id).map(|texture| &texture.bind_group),
        );
    }

    pub fn render_frame(&mut self, frame: &mut Frame, postfx_chain: &PostFxChain) {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            Err(e) => {
                log::error!("Surface error: {e:?}");
                return;
            }
        };
        let swap_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });
        self.encode_frame_to_view(
            &mut encoder,
            frame,
            postfx_chain,
            &swap_view,
            (self.surface_config.width, self.surface_config.height),
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    pub fn capture_frame_rgba(
        &mut self,
        frame: &mut Frame,
        postfx_chain: &PostFxChain,
    ) -> Result<CapturedFrame, String> {
        let width = self.surface_config.width.max(1);
        let height = self.surface_config.height.max(1);
        let capture_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let capture_view = capture_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let padded_bytes_per_row = Self::padded_bytes_per_row(width);
        let output_size = padded_bytes_per_row as u64 * height as u64;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture_readback_buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("capture_frame_encoder"),
            });
        self.encode_frame_to_view(
            &mut encoder,
            frame,
            postfx_chain,
            &capture_view,
            (width, height),
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &capture_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        let map_result = receiver
            .recv()
            .map_err(|error| format!("capture readback channel failed: {error}"))?;
        map_result?;

        let mapped = slice.get_mapped_range();
        let mut rgba8 = vec![0; (width * height * BYTES_PER_PIXEL) as usize];
        let row_bytes = (width * BYTES_PER_PIXEL) as usize;
        let padded_row_bytes = padded_bytes_per_row as usize;
        for row in 0..height as usize {
            let source_start = row * padded_row_bytes;
            let source_end = source_start + row_bytes;
            let dest_start = row * row_bytes;
            let dest_end = dest_start + row_bytes;
            rgba8[dest_start..dest_end].copy_from_slice(&mapped[source_start..source_end]);
        }
        drop(mapped);
        readback_buffer.unmap();

        Ok(CapturedFrame {
            width,
            height,
            rgba8,
        })
    }

    #[allow(dead_code)]
    pub fn surface_width(&self) -> u32 {
        self.surface_config.width
    }

    #[allow(dead_code)]
    pub fn surface_height(&self) -> u32 {
        self.surface_config.height
    }

    pub fn init_offscreen(&mut self, width: u32, height: u32, scale_mode: ScaleMode) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let blit_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("blit_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
            });

        let blit_bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blit_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let blit_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit_bg"),
            layout: &blit_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        });

        let blit_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blit_pipeline_layout"),
                bind_group_layouts: &[&blit_bgl],
                immediate_size: 0,
            });

        let blit_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blit_pipeline"),
                layout: Some(&blit_layout),
                vertex: wgpu::VertexState {
                    module: &blit_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &blit_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });

        self.offscreen = Some(OffscreenTarget {
            _texture: texture,
            view,
            bind_group,
            blit_pipeline,
            width,
            height,
            scale_mode: Cell::new(scale_mode),
        });
    }

    pub fn set_scale_mode(&self, mode: ScaleMode) {
        if let Some(ref ofs) = self.offscreen {
            ofs.scale_mode.set(mode);
        }
    }
}

pub(crate) fn blit_viewport(
    scale_mode: ScaleMode,
    game_w: u32,
    game_h: u32,
    win_w: u32,
    win_h: u32,
) -> (f32, f32, f32, f32) {
    if game_w == 0 || game_h == 0 || win_w == 0 || win_h == 0 {
        return (0.0, 0.0, win_w as f32, win_h as f32);
    }
    let (gw, gh, ww, wh) = (game_w as f32, game_h as f32, win_w as f32, win_h as f32);
    match scale_mode {
        ScaleMode::Stretch => (0.0, 0.0, ww, wh),
        ScaleMode::Letterbox => {
            let scale = (ww / gw).min(wh / gh);
            let w = (gw * scale).round();
            let h = (gh * scale).round();
            (((ww - w) / 2.0).round(), ((wh - h) / 2.0).round(), w, h)
        }
        ScaleMode::PixelPerfect => {
            let scale = (win_w / game_w).min(win_h / game_h);
            if scale == 0 {
                let s = (ww / gw).min(wh / gh);
                let w = (gw * s).round();
                let h = (gh * s).round();
                (((ww - w) / 2.0).round(), ((wh - h) / 2.0).round(), w, h)
            } else {
                let w = game_w * scale;
                let h = game_h * scale;
                let x = (win_w - w) / 2;
                let y = (win_h - h) / 2;
                (x as f32, y as f32, w as f32, h as f32)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        push_active_render_target, validate_render_target_frame, DrawParams, Frame, RenderTarget,
        TextureId,
    };
    use crate::canvas::Canvas;

    #[test]
    fn render_target_exposes_texture_and_size() {
        let target = RenderTarget {
            id: 3,
            texture: TextureId(7),
            width: 320,
            height: 180,
        };

        assert_eq!(target.texture_id(), TextureId(7));
        assert_eq!(target.size(), (320, 180));
    }

    #[test]
    #[should_panic(expected = "render target cycle detected")]
    fn render_target_cycle_detection_panics() {
        let mut active_targets = vec![7];
        push_active_render_target(&mut active_targets, 7);
    }

    #[test]
    #[should_panic(expected = "cannot sample itself in the sprite pass")]
    fn render_target_validation_rejects_sprite_feedback() {
        let mut frame = Frame::new();
        frame.sprites.push(DrawParams::new(
            TextureId(7),
            glam::Vec2::ZERO,
            glam::Vec2::new(16.0, 16.0),
        ));
        validate_render_target_frame(&mut frame, 7);
    }

    #[test]
    #[should_panic(expected = "cannot sample itself in the canvas pass")]
    fn render_target_validation_rejects_canvas_feedback() {
        let mut frame = Frame::new();
        let mut canvas = Canvas::new((320, 180), std::ptr::null());
        canvas.image(TextureId(7), -16.0, -16.0, 32.0, 32.0);
        frame.canvases.push(canvas);
        validate_render_target_frame(&mut frame, 7);
    }
}
