pub mod camera;
pub mod mesh;

pub use camera::Camera3D;
pub use mesh::{cube_mesh, floor_quad, wall_quad, MeshId, Vertex3D};

use crate::app::ScaleMode;
use crate::assets::Color;
use crate::canvas::{self, Canvas};
use crate::text;
use glam::{Mat4, Quat, Vec3};
use mesh::Vertex3D as V3;

use std::cell::Cell;
use std::sync::Arc;
use winit::window::Window;

const MAX_VERTICES: usize = 200_000;
const MAX_INDICES: usize = 400_000;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
    ambient: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct DrawCmd3D {
    pub mesh: MeshId,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl DrawCmd3D {
    pub fn new(mesh: MeshId, position: Vec3) -> Self {
        Self {
            mesh,
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = Vec3::splat(scale);
        self
    }

    pub(crate) fn transform_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

#[derive(Debug, Clone)]
pub struct Viewmodel3D {
    pub camera: Camera3D,
    pub(crate) draws: Vec<DrawCmd3D>,
}

impl Viewmodel3D {
    pub fn new() -> Self {
        let mut camera = Camera3D::new();
        camera.z_near = 0.01;
        camera.z_far = 16.0;
        camera.fov_y = 50.0f32.to_radians();
        Self {
            camera,
            draws: Vec::with_capacity(32),
        }
    }

    pub fn draw_mesh(&mut self, mesh: MeshId, position: Vec3) {
        self.draws.push(DrawCmd3D::new(mesh, position));
    }

    pub fn draw_mesh_transformed(&mut self, cmd: DrawCmd3D) {
        self.draws.push(cmd);
    }
}

pub struct Frame3D {
    pub camera: Camera3D,
    pub viewmodel: Viewmodel3D,
    pub clear_color: Color,
    pub light_dir: Vec3,
    pub light_color: Color,
    pub light_intensity: f32,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub(crate) draws: Vec<DrawCmd3D>,

    pub(crate) raw_verts: Vec<V3>,
    pub(crate) raw_idxs: Vec<u32>,

    pub(crate) canvases: Vec<Canvas>,
    screen_size: (u32, u32),
}

impl Frame3D {
    pub fn new(screen_size: (u32, u32)) -> Self {
        Self {
            camera: Camera3D::new(),
            viewmodel: Viewmodel3D::new(),
            clear_color: Color::from_rgba8(40, 40, 50, 255),
            light_dir: Vec3::new(0.4, 0.8, 0.3).normalize(),
            light_color: Color::WHITE,
            light_intensity: 0.8,
            ambient_color: Color::WHITE,
            ambient_intensity: 0.3,
            draws: Vec::with_capacity(256),
            raw_verts: Vec::new(),
            raw_idxs: Vec::new(),
            canvases: Vec::new(),
            screen_size,
        }
    }

    pub fn screen_size(&self) -> (u32, u32) {
        self.screen_size
    }

    pub fn draw_mesh(&mut self, mesh: MeshId, position: Vec3) {
        self.draws.push(DrawCmd3D::new(mesh, position));
    }

    pub fn draw_mesh_transformed(&mut self, cmd: DrawCmd3D) {
        self.draws.push(cmd);
    }

    pub fn draw_viewmodel_mesh(&mut self, mesh: MeshId, position: Vec3) {
        self.viewmodel.draw_mesh(mesh, position);
    }

    pub fn draw_raw(&mut self, vertices: &[V3], indices: &[u32]) {
        let base = self.raw_verts.len() as u32;
        self.raw_verts.extend_from_slice(vertices);
        self.raw_idxs.extend(indices.iter().map(|i| i + base));
    }

    pub fn canvas(&mut self, index: usize) -> &mut Canvas {
        let ss = self.screen_size;
        if index >= self.canvases.len() {
            self.canvases.resize_with(index + 1, || Canvas::new(ss));
        }
        &mut self.canvases[index]
    }
}

struct GpuMesh {
    vertices: Vec<V3>,
    indices: Vec<u32>,
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

pub(crate) struct Renderer3D {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    meshes: Vec<GpuMesh>,

    canvas_pipeline: wgpu::RenderPipeline,
    canvas_vb: wgpu::Buffer,
    pub(crate) fonts: Vec<text::FontAtlas>,
    font_bgl: wgpu::BindGroupLayout,

    offscreen: Option<OffscreenTarget>,
}

impl Renderer3D {
    pub async fn new(window: Arc<Window>, present_mode: wgpu::PresentMode) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rengine3d_device"),
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

        let depth_view =
            Self::create_depth_texture(&device, surface_config.width, surface_config.height);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh3d_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mesh3d.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer_3d"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bgl_3d"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bg_3d"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh3d_pipeline_layout"),
            bind_group_layouts: &[&uniform_bgl],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[V3::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer_3d"),
            size: (MAX_VERTICES * std::mem::size_of::<V3>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_buffer_3d"),
            size: (MAX_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let font_bgl = text::font_bind_group_layout(&device);
        let canvas_pipeline = canvas::pipeline(&device, surface_format, &font_bgl);
        let canvas_vb = canvas::vertex_buffer(&device);
        let font_atlas = text::font_atlas(&device, &queue, &font_bgl);

        Self {
            surface,
            device,
            queue,
            surface_config,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            depth_view,
            meshes: Vec::new(),
            canvas_pipeline,
            canvas_vb,
            fonts: vec![font_atlas],
            font_bgl,
            offscreen: None,
        }
    }

    pub(crate) fn load_font(&mut self, font_bytes: &[u8]) -> text::FontId {
        let id = text::FontId(self.fonts.len());
        let atlas =
            text::build_atlas_from_bytes(&self.device, &self.queue, &self.font_bgl, font_bytes, id);
        self.fonts.push(atlas);
        id
    }

    fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        tex.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn create_mesh(&mut self, vertices: Vec<V3>, indices: Vec<u32>) -> MeshId {
        let id = MeshId(self.meshes.len());
        self.meshes.push(GpuMesh { vertices, indices });
        id
    }

    pub fn replace_mesh(&mut self, id: MeshId, vertices: Vec<V3>, indices: Vec<u32>) {
        self.meshes[id.0] = GpuMesh { vertices, indices };
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            if self.offscreen.is_none() {
                self.depth_view = Self::create_depth_texture(&self.device, width, height);
            }
        }
    }

    pub fn render_frame(&mut self, frame: &mut Frame3D) {
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

        let (mesh_target, aspect) = match self.offscreen {
            Some(ref ofs) => (&ofs.view, ofs.width as f32 / ofs.height as f32),
            None => (
                &swap_view,
                self.surface_config.width as f32 / self.surface_config.height as f32,
            ),
        };
        let vp = frame.camera.view_projection(aspect);

        let uniforms = Uniforms {
            view_proj: vp.to_cols_array_2d(),
            light_dir: [frame.light_dir.x, frame.light_dir.y, frame.light_dir.z, 0.0],
            light_color: [
                frame.light_color.r,
                frame.light_color.g,
                frame.light_color.b,
                frame.light_intensity,
            ],
            ambient: [
                frame.ambient_color.r,
                frame.ambient_color.g,
                frame.ambient_color.b,
                frame.ambient_intensity,
            ],
        };

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let (all_verts, all_idxs) =
            self.build_draw_geometry(&frame.draws, &frame.raw_verts, &frame.raw_idxs);
        let num_indices = all_idxs.len() as u32;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame3d_encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mesh3d_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: mesh_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(frame.clear_color.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if num_indices > 0 {
                self.queue
                    .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_verts));
                self.queue
                    .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_idxs));
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..num_indices, 0, 0..1);
            }
        }

        if !frame.viewmodel.draws.is_empty() {
            let vm_vp = frame.viewmodel.camera.view_projection(aspect);
            let vm_uniforms = Uniforms {
                view_proj: vm_vp.to_cols_array_2d(),
                light_dir: [frame.light_dir.x, frame.light_dir.y, frame.light_dir.z, 0.0],
                light_color: [
                    frame.light_color.r,
                    frame.light_color.g,
                    frame.light_color.b,
                    frame.light_intensity,
                ],
                ambient: [
                    frame.ambient_color.r,
                    frame.ambient_color.g,
                    frame.ambient_color.b,
                    frame.ambient_intensity,
                ],
            };

            let (vm_verts, vm_idxs) =
                self.build_viewmodel_geometry(&frame.viewmodel.camera, &frame.viewmodel.draws);
            if !vm_idxs.is_empty() {
                self.queue.write_buffer(
                    &self.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[vm_uniforms]),
                );
                self.queue
                    .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vm_verts));
                self.queue
                    .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&vm_idxs));

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("mesh3d_viewmodel_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: mesh_target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..vm_idxs.len() as u32, 0, 0..1);
            }

            self.queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }

        if let Some(ref ofs) = self.offscreen {
            let (vx, vy, vw, vh) = crate::renderer::blit_viewport(
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
            pass.set_bind_group(0, &ofs.bind_group, &[]);
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
            &self.fonts,
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
        self.depth_view = Self::create_depth_texture(&self.device, width, height);

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_target_3d"),
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
                label: Some("blit_shader_3d"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../renderer/blit.wgsl").into()),
            });

        let blit_bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blit_bgl_3d"),
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
            label: Some("blit_sampler_3d"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit_bg_3d"),
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
                label: Some("blit_pipeline_layout_3d"),
                bind_group_layouts: &[&blit_bgl],
                immediate_size: 0,
            });

        let blit_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blit_pipeline_3d"),
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

    fn build_draw_geometry(
        &self,
        draws: &[DrawCmd3D],
        raw_verts: &[V3],
        raw_idxs: &[u32],
    ) -> (Vec<V3>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut idxs = Vec::new();

        for cmd in draws {
            let mesh = &self.meshes[cmd.mesh.0];
            let base = verts.len() as u32;
            let mat = cmd.transform_matrix();
            let normal_mat = glam::Mat3::from_quat(cmd.rotation);
            for v in &mesh.vertices {
                let mut moved = *v;
                let pos = Vec3::new(v.position[0], v.position[1], v.position[2]);
                let transformed = mat.transform_point3(pos);
                moved.position = [transformed.x, transformed.y, transformed.z];
                let n = Vec3::new(v.normal[0], v.normal[1], v.normal[2]);
                let transformed_n = (normal_mat * n).normalize_or_zero();
                moved.normal = [transformed_n.x, transformed_n.y, transformed_n.z];
                verts.push(moved);
            }
            idxs.extend(mesh.indices.iter().map(|i| i + base));
        }

        if !raw_verts.is_empty() {
            let base = verts.len() as u32;
            verts.extend_from_slice(raw_verts);
            idxs.extend(raw_idxs.iter().map(|i| i + base));
        }

        (verts, idxs)
    }

    fn build_viewmodel_geometry(
        &self,
        camera: &Camera3D,
        draws: &[DrawCmd3D],
    ) -> (Vec<V3>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        let camera_from_view = camera.view_matrix().inverse();

        for cmd in draws {
            let mesh = &self.meshes[cmd.mesh.0];
            let base = verts.len() as u32;
            let local_mat = cmd.transform_matrix();
            let normal_mat = glam::Mat3::from_quat(cmd.rotation);
            for v in &mesh.vertices {
                let mut moved = *v;
                let local_pos = Vec3::new(v.position[0], v.position[1], v.position[2]);
                let local_transformed = local_mat.transform_point3(local_pos);
                let world_position = camera_from_view.transform_point3(local_transformed);
                let local_normal = Vec3::new(v.normal[0], v.normal[1], v.normal[2]);
                let rotated_normal = normal_mat * local_normal;
                let world_normal = camera_from_view
                    .transform_vector3(rotated_normal)
                    .normalize_or_zero();
                moved.position = [world_position.x, world_position.y, world_position.z];
                moved.normal = [world_normal.x, world_normal.y, world_normal.z];
                verts.push(moved);
            }
            idxs.extend(mesh.indices.iter().map(|i| i + base));
        }

        (verts, idxs)
    }
}
