pub mod camera;
pub mod mesh;

pub use camera::Camera3D;
pub use mesh::{cube_mesh, floor_quad, wall_quad, MeshId, Vertex3D};

use crate::assets::Color;
use crate::hud::{self, HudVertex};
use glam::Vec3;
use mesh::Vertex3D as V3;

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
}


pub struct Frame3D {
    pub camera: Camera3D,
    pub clear_color: Color,
    pub light_dir: Vec3,
    pub light_color: Color,
    pub light_intensity: f32,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub(crate) draws: Vec<DrawCmd3D>,

    pub(crate) raw_verts: Vec<V3>,
    pub(crate) raw_idxs: Vec<u32>,

    pub(crate) hud_verts: Vec<HudVertex>,
}

impl Frame3D {
    pub fn new() -> Self {
        Self {
            camera: Camera3D::new(),
            clear_color: Color::from_rgba8(40, 40, 50, 255),
            light_dir: Vec3::new(0.4, 0.8, 0.3).normalize(),
            light_color: Color::WHITE,
            light_intensity: 0.8,
            ambient_color: Color::WHITE,
            ambient_intensity: 0.3,
            draws: Vec::with_capacity(256),
            raw_verts: Vec::new(),
            raw_idxs: Vec::new(),
            hud_verts: Vec::new(),
        }
    }


    pub fn draw_mesh(&mut self, mesh: MeshId, position: Vec3) {
        self.draws.push(DrawCmd3D { mesh, position });
    }


    pub fn draw_raw(&mut self, vertices: &[V3], indices: &[u32]) {
        let base = self.raw_verts.len() as u32;
        self.raw_verts.extend_from_slice(vertices);
        self.raw_idxs.extend(indices.iter().map(|i| i + base));
    }


    pub fn hud_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Color,
        screen_size: (u32, u32),
    ) {
        hud::push_rect(&mut self.hud_verts, x, y, w, h, color, screen_size);
    }


    pub fn hud_crosshair(
        &mut self,
        size: f32,
        thickness: f32,
        color: Color,
        screen_size: (u32, u32),
    ) {
        hud::push_crosshair(&mut self.hud_verts, size, thickness, color, screen_size);
    }


    pub fn hud_number(
        &mut self,
        x: f32,
        y: f32,
        value: u32,
        scale: f32,
        color: Color,
        screen_size: (u32, u32),
    ) {
        hud::push_number(&mut self.hud_verts, x, y, value, scale, color, screen_size);
    }


    pub fn hud_fps(&mut self, fps: f32, screen_size: (u32, u32)) {
        hud::push_fps(&mut self.hud_verts, fps, screen_size);
    }
}


struct GpuMesh {
    vertices: Vec<V3>,
    indices: Vec<u32>,
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

    hud_pipeline: wgpu::RenderPipeline,
    hud_vertex_buffer: wgpu::Buffer,
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("rengine3d_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    ..Default::default()
                },
            )
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


        let hud_pipeline = hud::create_hud_pipeline(&device, surface_format);
        let hud_vertex_buffer = hud::create_hud_vertex_buffer(&device);

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
            hud_pipeline,
            hud_vertex_buffer,
        }
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


    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.depth_view = Self::create_depth_texture(&self.device, width, height);
        }
    }


    pub fn render_frame(&mut self, frame: &Frame3D) {
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
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let aspect = self.surface_config.width as f32 / self.surface_config.height as f32;
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


        let mut all_verts: Vec<V3> = Vec::new();
        let mut all_idxs: Vec<u32> = Vec::new();


        for cmd in &frame.draws {
            let mesh = &self.meshes[cmd.mesh.0];
            let base = all_verts.len() as u32;
            for v in &mesh.vertices {
                let mut moved = *v;
                moved.position[0] += cmd.position.x;
                moved.position[1] += cmd.position.y;
                moved.position[2] += cmd.position.z;
                all_verts.push(moved);
            }
            all_idxs.extend(mesh.indices.iter().map(|i| i + base));
        }


        if !frame.raw_verts.is_empty() {
            let base = all_verts.len() as u32;
            all_verts.extend_from_slice(&frame.raw_verts);
            all_idxs.extend(frame.raw_idxs.iter().map(|i| i + base));
        }

        if !all_verts.is_empty() {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_verts));
            self.queue
                .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_idxs));
        }

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
                    view: &view,
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
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..num_indices, 0, 0..1);
            }
        }


        hud::render_hud_pass(
            &mut encoder,
            &view,
            &self.hud_pipeline,
            &self.hud_vertex_buffer,
            &self.queue,
            &frame.hud_verts,
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
}
