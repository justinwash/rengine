use image::GenericImageView;
use wgpu::util::DeviceExt;
use wgpu::{
    Device, Extent3d, Queue, SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

#[derive(Clone)]
pub struct Sprite {
    pub image_path: String,
    pub position: (f32, f32),
    pub size: (f32, f32),
}

impl Sprite {
    pub fn new(image_path: &str, position: (f32, f32), size: (f32, f32)) -> Self {
        Self {
            image_path: image_path.to_string(),
            position,
            size,
        }
    }
}

pub struct SpriteRenderer {
    pub sprites: Vec<Sprite>,
    // Internal: map image_path to wgpu texture/view/sampler
    pub textures:
        std::collections::HashMap<String, (wgpu::Texture, wgpu::TextureView, wgpu::Sampler)>,
}

impl SpriteRenderer {
    pub fn new() -> Self {
        Self {
            sprites: Vec::new(),
            textures: std::collections::HashMap::new(),
        }
    }
    pub fn add_sprite(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }
    pub fn render(&mut self, wgpu_ctx: &mut crate::window::WgpuContext) {
        use bytemuck::{Pod, Zeroable};
        use image::GenericImageView;
        use wgpu::*;
        let device = &wgpu_ctx.device;
        let queue = &wgpu_ctx.queue;
        let surface = &wgpu_ctx.surface;
        let surface_texture = match surface.get_current_texture() {
            Ok(tex) => tex,
            Err(e) => {
                eprintln!("Failed to acquire next swap chain texture: {e}");
                return;
            }
        };
        let view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("SpriteRenderer Encoder"),
        });
        // Clear the screen
        {
            let _rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        #[repr(C)]
        #[derive(Clone, Copy, Pod, Zeroable)]
        struct SpriteUniforms {
            sprite_pos: [f32; 2],
            sprite_size: [f32; 2],
            screen_size: [f32; 2],
        }
        let size = surface_texture.texture.size();
        let screen_size = [size.width as f32, size.height as f32];
        let vertices: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Sprite Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Sprite Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });
        let vs_src = include_str!("sprite.vert.wgsl");
        let fs_src = include_str!("sprite.frag.wgsl");
        let vs_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Sprite VS"),
            source: ShaderSource::Wgsl(vs_src.into()),
        });
        let fs_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Sprite FS"),
            source: ShaderSource::Wgsl(fs_src.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Sprite Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new((3 * std::mem::size_of::<[f32; 2]>()) as _),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Sprite Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vs_module,
                entry_point: Some("main"),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &fs_module,
                entry_point: Some("main"),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        for sprite in &self.sprites {
            // Load texture if not already loaded
            if !self.textures.contains_key(&sprite.image_path) {
                let img = match image::open(&sprite.image_path) {
                    Ok(img) => img.to_rgba8(),
                    Err(e) => {
                        eprintln!("Failed to load sprite image {}: {e}", sprite.image_path);
                        continue;
                    }
                };
                let (width, height) = img.dimensions();
                let texture_size = Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                };
                let texture = device.create_texture(&TextureDescriptor {
                    label: Some("sprite_texture"),
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &img,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * width),
                        rows_per_image: Some(height),
                    },
                    texture_size,
                );
                let view = texture.create_view(&TextureViewDescriptor::default());
                let sampler = device.create_sampler(&SamplerDescriptor {
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                });
                self.textures
                    .insert(sprite.image_path.clone(), (texture, view, sampler));
            }
            let (_, sprite_view, sampler) = self.textures.get(&sprite.image_path).unwrap();
            let uniforms = SpriteUniforms {
                sprite_pos: [sprite.position.0, sprite.position.1],
                sprite_size: [sprite.size.0, sprite.size.1],
                screen_size,
            };
            let uniform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
                label: Some("Sprite Uniform Buffer"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Sprite Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(sprite_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(sampler),
                    },
                ],
            });
            // Render to the surface view, not the sprite texture view!
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Sprite Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view, // <-- use the surface view here
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rpass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint16);
            rpass.draw_indexed(0..6, 0, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}
