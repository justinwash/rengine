#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub(crate) usize);

impl FontId {
    pub const DEFAULT: FontId = FontId(0);
}

#[derive(Clone, Copy)]
pub(crate) struct GlyphEntry {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub width_px: f32,
    pub height_px: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub advance: f32,
}

pub(crate) const ATLAS_SIZE: u32 = 512;
pub(crate) const FONT_SIZE: f32 = 48.0;

pub struct FontAtlas {
    pub bind_group: wgpu::BindGroup,
    pub(crate) glyphs: [Option<GlyphEntry>; 128],
    white_uv: [f32; 2],
    pub(crate) line_height: f32,
    pub(crate) id: FontId,
}

impl FontAtlas {
    pub fn id(&self) -> FontId {
        self.id
    }

    pub fn white_uv(&self) -> [f32; 2] {
        self.white_uv
    }

    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        let scale = size / FONT_SIZE;
        let mut width: f32 = 0.0;
        for ch in text.chars() {
            let idx = ch as usize;
            if idx < 128 {
                if let Some(e) = self.glyphs[idx] {
                    width += e.advance * scale;
                }
            }
        }
        (width, self.line_height * scale)
    }

    pub fn line_height(&self, size: f32) -> f32 {
        self.line_height * (size / FONT_SIZE)
    }
}

pub fn font_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("font_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
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
    })
}

pub fn font_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> FontAtlas {
    let font_bytes = include_bytes!("../assets/font.ttf");
    build_atlas_from_bytes(device, queue, bind_group_layout, font_bytes, FontId::DEFAULT)
}

pub(crate) fn build_atlas_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    font_bytes: &[u8],
    id: FontId,
) -> FontAtlas {
    let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
        .expect("failed to parse font");

    let mut pixels = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];

    for y in 0..2u32 {
        for x in 0..2u32 {
            let offset = ((y * ATLAS_SIZE + x) * 4) as usize;
            pixels[offset] = 255;
            pixels[offset + 1] = 255;
            pixels[offset + 2] = 255;
            pixels[offset + 3] = 255;
        }
    }
    let white_uv = [1.0 / ATLAS_SIZE as f32, 1.0 / ATLAS_SIZE as f32];

    let mut glyphs: [Option<GlyphEntry>; 128] = [None; 128];

    let mut cursor_x: u32 = 4;
    let mut cursor_y: u32 = 0;
    let mut row_height: u32 = 0;
    let mut line_height: f32 = 0.0;

    for c in 32u8..127 {
        let ch = c as char;
        let (metrics, bitmap) = font.rasterize(ch, FONT_SIZE);
        if metrics.width == 0 || metrics.height == 0 {
            let advance = metrics.advance_width;
            if advance > 0.0 {
                glyphs[c as usize] = Some(GlyphEntry {
                    u0: white_uv[0],
                    v0: white_uv[1],
                    u1: white_uv[0],
                    v1: white_uv[1],
                    width_px: 0.0,
                    height_px: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                    advance,
                });
            }
            continue;
        }

        let gw = metrics.width as u32;
        let gh = metrics.height as u32;

        if cursor_x + gw + 1 > ATLAS_SIZE {
            cursor_x = 0;
            cursor_y += row_height + 1;
            row_height = 0;
        }

        if cursor_y + gh > ATLAS_SIZE {
            break;
        }

        for gy in 0..gh {
            for gx in 0..gw {
                let src = (gy * gw + gx) as usize;
                let dst = (((cursor_y + gy) * ATLAS_SIZE + cursor_x + gx) * 4) as usize;
                pixels[dst] = 255;
                pixels[dst + 1] = 255;
                pixels[dst + 2] = 255;
                pixels[dst + 3] = bitmap[src];
            }
        }

        let u0 = cursor_x as f32 / ATLAS_SIZE as f32;
        let v0 = cursor_y as f32 / ATLAS_SIZE as f32;
        let u1 = (cursor_x + gw) as f32 / ATLAS_SIZE as f32;
        let v1 = (cursor_y + gh) as f32 / ATLAS_SIZE as f32;

        let h = metrics.height as f32;
        if h > line_height {
            line_height = h;
        }

        glyphs[c as usize] = Some(GlyphEntry {
            u0,
            v0,
            u1,
            v1,
            width_px: gw as f32,
            height_px: gh as f32,
            x_offset: metrics.xmin as f32,
            y_offset: metrics.ymin as f32,
            advance: metrics.advance_width,
        });

        cursor_x += gw + 1;
        if gh > row_height {
            row_height = gh;
        }
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("font_atlas"),
        size: wgpu::Extent3d {
            width: ATLAS_SIZE,
            height: ATLAS_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(ATLAS_SIZE * 4),
            rows_per_image: Some(ATLAS_SIZE),
        },
        wgpu::Extent3d {
            width: ATLAS_SIZE,
            height: ATLAS_SIZE,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("font_sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("font_bind_group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    FontAtlas {
        bind_group,
        glyphs,
        white_uv,
        line_height,
        id,
    }
}
