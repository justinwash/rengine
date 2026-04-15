use std::cell::RefCell;

#[derive(Debug, Clone)]
pub enum PostEffect {
    Vignette {
        intensity: f32,
        radius: f32,
        softness: f32,
    },
    Blur {
        radius: f32,
    },
    Bloom {
        threshold: f32,
        intensity: f32,
    },
    ColorGrade {
        brightness: f32,
        contrast: f32,
        saturation: f32,
    },
    Crt {
        scanline_intensity: f32,
        curvature: f32,
    },
    Pixelate {
        pixel_size: f32,
    },
    ChromaticAberration {
        offset: f32,
    },
    Invert,
    Custom {
        wgsl_source: String,
    },
}

impl PostEffect {
    fn shader_source(&self) -> String {
        match self {
            PostEffect::Vignette { .. } => FULLSCREEN_VERTEX.to_string() + VIGNETTE_FRAG,
            PostEffect::Blur { .. } => FULLSCREEN_VERTEX.to_string() + BLUR_FRAG,
            PostEffect::Bloom { .. } => FULLSCREEN_VERTEX.to_string() + BLOOM_FRAG,
            PostEffect::ColorGrade { .. } => FULLSCREEN_VERTEX.to_string() + COLOR_GRADE_FRAG,
            PostEffect::Crt { .. } => FULLSCREEN_VERTEX.to_string() + CRT_FRAG,
            PostEffect::Pixelate { .. } => FULLSCREEN_VERTEX.to_string() + PIXELATE_FRAG,
            PostEffect::ChromaticAberration { .. } => {
                FULLSCREEN_VERTEX.to_string() + CHROMATIC_ABERRATION_FRAG
            }
            PostEffect::Invert => FULLSCREEN_VERTEX.to_string() + INVERT_FRAG,
            PostEffect::Custom { wgsl_source } => wgsl_source.clone(),
        }
    }

    fn write_params(&self, buf: &mut [f32; 8]) {
        *buf = [0.0; 8];
        match self {
            PostEffect::Vignette {
                intensity,
                radius,
                softness,
            } => {
                buf[0] = *intensity;
                buf[1] = *radius;
                buf[2] = *softness;
            }
            PostEffect::Blur { radius } => {
                buf[0] = *radius;
            }
            PostEffect::Bloom {
                threshold,
                intensity,
            } => {
                buf[0] = *threshold;
                buf[1] = *intensity;
            }
            PostEffect::ColorGrade {
                brightness,
                contrast,
                saturation,
            } => {
                buf[0] = *brightness;
                buf[1] = *contrast;
                buf[2] = *saturation;
            }
            PostEffect::Crt {
                scanline_intensity,
                curvature,
            } => {
                buf[0] = *scanline_intensity;
                buf[1] = *curvature;
            }
            PostEffect::Pixelate { pixel_size } => {
                buf[0] = *pixel_size;
            }
            PostEffect::ChromaticAberration { offset } => {
                buf[0] = *offset;
            }
            PostEffect::Invert => {}
            PostEffect::Custom { .. } => {}
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PostFxUniforms {
    params: [f32; 8],
    resolution: [f32; 2],
    _pad: [f32; 2],
}

struct CompiledPass {
    pipeline: wgpu::RenderPipeline,
}

pub(crate) struct PostFxPipeline {
    passes: Vec<CompiledPass>,
    texture_a: wgpu::Texture,
    view_a: wgpu::TextureView,
    bind_group_a: wgpu::BindGroup,
    texture_b: wgpu::Texture,
    view_b: wgpu::TextureView,
    bind_group_b: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    pub(crate) texture_bgl: wgpu::BindGroupLayout,
    uniform_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    source_bind_group: Option<wgpu::BindGroup>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    format: wgpu::TextureFormat,
}

pub struct PostFxChain {
    pub(crate) effects: RefCell<Vec<PostEffect>>,
    pub(crate) dirty: RefCell<bool>,
}

impl PostFxChain {
    pub fn new() -> Self {
        Self {
            effects: RefCell::new(Vec::new()),
            dirty: RefCell::new(false),
        }
    }

    pub fn push(&self, effect: PostEffect) {
        self.effects.borrow_mut().push(effect);
        *self.dirty.borrow_mut() = true;
    }

    pub fn insert(&self, index: usize, effect: PostEffect) {
        let mut effects = self.effects.borrow_mut();
        let idx = index.min(effects.len());
        effects.insert(idx, effect);
        *self.dirty.borrow_mut() = true;
    }

    pub fn remove(&self, index: usize) {
        let mut effects = self.effects.borrow_mut();
        if index < effects.len() {
            effects.remove(index);
            *self.dirty.borrow_mut() = true;
        }
    }

    pub fn clear(&self) {
        self.effects.borrow_mut().clear();
        *self.dirty.borrow_mut() = true;
    }

    pub fn set(&self, index: usize, effect: PostEffect) {
        let mut effects = self.effects.borrow_mut();
        if index < effects.len() {
            effects[index] = effect;
            *self.dirty.borrow_mut() = true;
        }
    }

    pub fn len(&self) -> usize {
        self.effects.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.borrow().is_empty()
    }
}

impl Default for PostFxChain {
    fn default() -> Self {
        Self::new()
    }
}

impl PostFxPipeline {
    pub(crate) fn pass_count(&self) -> usize {
        self.passes.len()
    }

    pub(crate) fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("postfx_texture_bgl"),
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

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("postfx_uniform_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("postfx_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (texture_a, view_a, bind_group_a) =
            Self::create_target(device, width, height, format, &texture_bgl, &sampler, "A");
        let (texture_b, view_b, bind_group_b) =
            Self::create_target(device, width, height, format, &texture_bgl, &sampler, "B");

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("postfx_uniforms"),
            size: std::mem::size_of::<PostFxUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("postfx_uniform_bg"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            passes: Vec::new(),
            texture_a,
            view_a,
            bind_group_a,
            texture_b,
            view_b,
            bind_group_b,
            uniform_buffer,
            uniform_bind_group,
            texture_bgl,
            uniform_bgl,
            sampler,
            source_bind_group: None,
            width,
            height,
            format,
        }
    }

    pub(crate) fn set_source_view(
        &mut self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
    ) {
        self.source_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("postfx_source_bg"),
            layout: &self.texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
    }

    fn create_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        bgl: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("postfx_target_{label}")),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("postfx_bg_{label}")),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        (texture, view, bind_group)
    }

    pub(crate) fn rebuild(
        &mut self,
        device: &wgpu::Device,
        effects: &[PostEffect],
    ) {
        self.passes.clear();

        for (i, effect) in effects.iter().enumerate() {
            let source = effect.shader_source();
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("postfx_shader_{i}")),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("postfx_layout_{i}")),
                bind_group_layouts: &[&self.texture_bgl, &self.uniform_bgl],
                immediate_size: 0,
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("postfx_pipeline_{i}")),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.format,
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

            self.passes.push(CompiledPass { pipeline });
        }
    }

    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        let (ta, va, bga) = Self::create_target(
            device,
            width,
            height,
            self.format,
            &self.texture_bgl,
            &self.sampler,
            "A",
        );
        let (tb, vb, bgb) = Self::create_target(
            device,
            width,
            height,
            self.format,
            &self.texture_bgl,
            &self.sampler,
            "B",
        );
        self.texture_a = ta;
        self.view_a = va;
        self.bind_group_a = bga;
        self.texture_b = tb;
        self.view_b = vb;
        self.bind_group_b = bgb;
    }

    pub(crate) fn run(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        effects: &[PostEffect],
    ) -> Option<&wgpu::TextureView> {
        if self.passes.is_empty() || effects.is_empty() {
            return None;
        }

        let source_bg = match self.source_bind_group {
            Some(ref bg) => bg,
            None => return None,
        };

        let views = [&self.view_a, &self.view_b];
        let bind_groups = [&self.bind_group_a, &self.bind_group_b];

        let mut read_bg = source_bg;

        for (i, (pass, effect)) in self.passes.iter().zip(effects.iter()).enumerate() {
            let write_idx = i % 2;
            let write_view = views[write_idx];

            let mut params = [0.0f32; 8];
            effect.write_params(&mut params);
            let uniforms = PostFxUniforms {
                params,
                resolution: [self.width as f32, self.height as f32],
                _pad: [0.0; 2],
            };
            queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[uniforms]),
            );

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("postfx_pass_{i}")),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: write_view,
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
                rpass.set_pipeline(&pass.pipeline);
                rpass.set_bind_group(0, read_bg, &[]);
                rpass.set_bind_group(1, &self.uniform_bind_group, &[]);
                rpass.draw(0..3, 0..1);
            }

            read_bg = bind_groups[write_idx];
        }

        let last_write_idx = (self.passes.len() - 1) % 2;
        Some(views[last_write_idx])
    }

    pub(crate) fn last_output_bind_group(&self, pass_count: usize) -> &wgpu::BindGroup {
        let idx = (pass_count - 1) % 2;
        if idx == 0 {
            &self.bind_group_a
        } else {
            &self.bind_group_b
        }
    }
}

const FULLSCREEN_VERTEX: &str = r#"
struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) / 2.0, (1.0 - y) / 2.0);
    return out;
}

@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var s_source: sampler;

struct PostFxParams {
    params: array<f32, 8>,
    resolution: vec2<f32>,
    _pad: vec2<f32>,
};
@group(1) @binding(0) var<uniform> u: PostFxParams;

"#;

const VIGNETTE_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_source, s_source, in.uv);
    let intensity = u.params[0];
    let radius = u.params[1];
    let softness = u.params[2];
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(in.uv, center);
    let vignette = smoothstep(radius, radius - softness, dist);
    return vec4<f32>(color.rgb * mix(1.0, vignette, intensity), color.a);
}
"#;

const BLUR_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let radius = u.params[0];
    let texel = vec2<f32>(1.0 / u.resolution.x, 1.0 / u.resolution.y);
    var color = vec4<f32>(0.0);
    var total = 0.0;
    let samples = i32(clamp(radius, 1.0, 8.0));
    for (var x = -samples; x <= samples; x++) {
        for (var y = -samples; y <= samples; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel * (radius / f32(samples));
            color += textureSample(t_source, s_source, in.uv + offset);
            total += 1.0;
        }
    }
    return color / total;
}
"#;

const BLOOM_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let threshold = u.params[0];
    let intensity = u.params[1];
    let texel = vec2<f32>(1.0 / u.resolution.x, 1.0 / u.resolution.y);
    let color = textureSample(t_source, s_source, in.uv);
    let lum = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));

    var bloom = vec4<f32>(0.0);
    var total = 0.0;
    let rad = 4;
    for (var x = -rad; x <= rad; x++) {
        for (var y = -rad; y <= rad; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel * 2.0;
            let s = textureSample(t_source, s_source, in.uv + offset);
            let sl = dot(s.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            if sl > threshold {
                bloom += s;
                total += 1.0;
            }
        }
    }
    if total > 0.0 {
        bloom = bloom / total;
    }
    return vec4<f32>(color.rgb + bloom.rgb * intensity, color.a);
}
"#;

const COLOR_GRADE_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let brightness = u.params[0];
    let contrast = u.params[1];
    let saturation = u.params[2];
    var color = textureSample(t_source, s_source, in.uv);
    // brightness
    var rgb = color.rgb * brightness;
    // contrast
    rgb = (rgb - 0.5) * contrast + 0.5;
    // saturation
    let gray = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    rgb = mix(vec3<f32>(gray), rgb, saturation);
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);
}
"#;

const CRT_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let scanline_intensity = u.params[0];
    let curvature = u.params[1];
    // barrel distortion
    var uv = in.uv * 2.0 - 1.0;
    let d = length(uv);
    uv = uv * (1.0 + curvature * d * d);
    uv = (uv + 1.0) / 2.0;
    // clamp to edges
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    var color = textureSample(t_source, s_source, uv);
    // scanlines
    let scanline = sin(uv.y * u.resolution.y * 3.14159) * 0.5 + 0.5;
    color = vec4<f32>(color.rgb * mix(1.0, scanline, scanline_intensity), color.a);
    return color;
}
"#;

const PIXELATE_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let pixel_size = max(u.params[0], 1.0);
    let dx = pixel_size / u.resolution.x;
    let dy = pixel_size / u.resolution.y;
    let uv = vec2<f32>(
        floor(in.uv.x / dx) * dx + dx * 0.5,
        floor(in.uv.y / dy) * dy + dy * 0.5,
    );
    return textureSample(t_source, s_source, uv);
}
"#;

const CHROMATIC_ABERRATION_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let offset = u.params[0];
    let dir = in.uv - vec2<f32>(0.5, 0.5);
    let r = textureSample(t_source, s_source, in.uv + dir * offset).r;
    let g = textureSample(t_source, s_source, in.uv).g;
    let b = textureSample(t_source, s_source, in.uv - dir * offset).b;
    let a = textureSample(t_source, s_source, in.uv).a;
    return vec4<f32>(r, g, b, a);
}
"#;

const INVERT_FRAG: &str = r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_source, s_source, in.uv);
    return vec4<f32>(1.0 - color.rgb, color.a);
}
"#;
