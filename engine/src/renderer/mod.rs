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
use crate::canvas::{self, Canvas};
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

pub struct Frame {
    pub(crate) sprites: Vec<DrawParams>,
    pub camera: Camera2D,
    pub clear_color: Color,

    pub(crate) canvases: Vec<Canvas>,
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
}

struct GpuTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
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

    canvas_pipeline: wgpu::RenderPipeline,
    canvas_vb: wgpu::Buffer,
    pub(crate) font_atlas: text::FontAtlas,

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

        let font_bgl = text::font_bind_group_layout(&device);
        let canvas_pipeline = canvas::pipeline(&device, surface_format, &font_bgl);
        let canvas_vb = canvas::vertex_buffer(&device);
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
            canvas_pipeline,
            canvas_vb,
            font_atlas,
            offscreen: None,
            postfx: None,
        };

        let white = renderer.create_texture(1, 1, &[255, 255, 255, 255]);
        renderer.white_texture = white;

        renderer
    }

    pub fn create_texture(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureId {
        assert_eq!(
            pixels.len(),
            (width * height * 4) as usize,
            "pixel data length must match width × height × 4"
        );

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

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

        let id = TextureId(self.textures.len());
        self.textures.push(GpuTexture {
            _texture: texture,
            _view: view,
            bind_group,
        });
        id
    }

    pub fn replace_texture(&mut self, id: TextureId, width: u32, height: u32, pixels: &[u8]) {
        assert_eq!(
            pixels.len(),
            (width * height * 4) as usize,
            "pixel data length must match width × height × 4"
        );

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

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

        self.textures[id.0] = GpuTexture {
            _texture: texture,
            _view: view,
            bind_group,
        };
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
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

        let (sprite_target, proj_w, proj_h) = match self.offscreen {
            Some(ref ofs) => (&ofs.view, ofs.width as f32, ofs.height as f32),
            None => (
                &swap_view,
                self.surface_config.width as f32,
                self.surface_config.height as f32,
            ),
        };

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

            for i in 0..4 {
                vertices.push(Vertex {
                    position: corners[i],
                    tex_coords: uvs[i],
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

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

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
                    pfx.run(&mut encoder, &self.queue, &effects);
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
                self.surface_config.width,
                self.surface_config.height,
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &swap_view,
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

        canvas::render_pass(
            &mut encoder,
            &swap_view,
            &self.canvas_pipeline,
            &self.canvas_vb,
            &self.queue,
            &mut frame.canvases,
            &self.font_atlas,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
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
