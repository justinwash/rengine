use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub(crate) usize);

impl FontId {
    pub const DEFAULT: FontId = FontId(0);

    /// The raw index, for a host that has to hand this id to something that
    /// only speaks numbers — publishing it to a scene as an `ui_font` binding,
    /// say. Construction stays crate-private: an id is only meaningful if the
    /// renderer actually loaded that font.
    pub fn index(self) -> usize {
        self.0
    }
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

struct BuiltinFontMetrics {
    advances: [f32; 128],
    line_height: f32,
}

static BUILTIN_FONT_METRICS: OnceLock<BuiltinFontMetrics> = OnceLock::new();

fn builtin_font_metrics() -> &'static BuiltinFontMetrics {
    BUILTIN_FONT_METRICS.get_or_init(|| {
        let font = fontdue::Font::from_bytes(
            &include_bytes!("../assets/font.ttf")[..],
            fontdue::FontSettings::default(),
        )
        .expect("failed to parse font");
        let mut advances = [0.0; 128];
        for c in 32u8..127 {
            advances[c as usize] = font.metrics(c as char, FONT_SIZE).advance_width;
        }
        // Same definition as `build_atlas_from_bytes`: the font's own line
        // box, not the tallest glyph's ink.
        let line_height = font
            .horizontal_line_metrics(FONT_SIZE)
            .map_or(FONT_SIZE, |m| m.new_line_size);

        BuiltinFontMetrics {
            advances,
            line_height,
        }
    })
}

pub(crate) fn measure_builtin_text(text: &str, size: f32) -> (f32, f32) {
    let metrics = builtin_font_metrics();
    let scale = size / FONT_SIZE;
    let mut width = 0.0;

    for ch in text.chars() {
        let idx = ch as usize;
        if idx < metrics.advances.len() {
            width += metrics.advances[idx] * scale;
        }
    }

    (width, metrics.line_height * scale)
}

pub struct FontAtlas {
    pub bind_group: wgpu::BindGroup,
    pub(crate) glyphs: [Option<GlyphEntry>; 128],
    white_uv: [f32; 2],
    /// The font's own line box at [`FONT_SIZE`]: `ascent - descent +
    /// line_gap`, the same number CSS calls `normal` line-height.
    ///
    /// This used to be the tallest glyph's *ink* height, which is a different
    /// quantity entirely and is why text drew outside its own node's rect:
    /// a rect sized to it was shorter than a line, and the draw then measured
    /// the baseline down from a top that didn't exist.
    pub(crate) line_height: f32,
    /// Distance from the baseline up to the line box's top, at [`FONT_SIZE`].
    /// Positive. This is what turns a rect into a baseline.
    pub(crate) ascent: f32,
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

    /// Where the baseline sits inside a line box whose **top** is at `top`
    /// (y-up canvas coords, so the baseline is below it).
    ///
    /// The one place the rect→baseline conversion lives. Every text path goes
    /// through it, so a node's ink lands inside the rect the layout gave it
    /// by construction rather than by each call site guessing.
    pub fn baseline_below_top(&self, top: f32, size: f32) -> f32 {
        top - self.ascent * (size / FONT_SIZE)
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
    build_atlas_from_bytes(
        device,
        queue,
        bind_group_layout,
        font_bytes,
        FontId::DEFAULT,
    )
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

    // The font's own vertical metrics, not the tallest glyph's ink box. A
    // line box is `ascent - descent + line_gap` — the same number a browser
    // uses for `line-height: normal`, which is what the mockups are laid out
    // against. Measuring ink instead made every rect shorter than a real
    // line and left the baseline undefined.
    let (ascent, line_height) = match font.horizontal_line_metrics(FONT_SIZE) {
        Some(m) => (m.ascent, m.new_line_size),
        // No hhea/OS2 table: fall back to the em box, which is at least
        // self-consistent (baseline at 80% is the usual default).
        None => (FONT_SIZE * 0.8, FONT_SIZE),
    };

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
        ascent,
        id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rect→baseline contract, on the real vendored faces.
    ///
    /// This is the bug that cost the most during the UI overhaul: `y` meant
    /// "rect bottom" to the layout, "baseline" to the draw, and `line_height`
    /// meant "tallest glyph's ink" to one and "line box" to the other. Text
    /// drew tens of pixels outside its own node and every screen had been
    /// hand-nudged against the wrong output.
    ///
    /// What must hold: a line box `line_height(size)` tall, with the baseline
    /// `ascent` below its top, puts every glyph's ink inside that box.
    fn assert_ink_fits_line_box(bytes: &[u8], name: &str) {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .expect("vendored font should parse");
        for size in [10.0f32, 12.0, 18.0, 44.0] {
            let m = font
                .horizontal_line_metrics(size)
                .expect("a UI font declares vertical metrics");
            // Same numbers the atlas stores and `baseline_below_top` uses.
            let (line_height, ascent) = (m.new_line_size, m.ascent);
            let top = 0.0f32;
            let baseline = top - ascent;
            let bottom = top - line_height;
            for ch in " !ABCFMWgjpqy0123456789".chars() {
                let g = font.metrics(ch, size);
                if g.height == 0 {
                    continue;
                }
                // Canvas places a glyph's bottom at `baseline + ymin`.
                let ink_bottom = baseline + g.ymin as f32;
                let ink_top = ink_bottom + g.height as f32;
                assert!(
                    ink_top <= top + 0.5,
                    "{name} @{size}: '{ch}' ink rises above its line box \
                     (ink_top={ink_top}, box_top={top})"
                );
                assert!(
                    ink_bottom >= bottom - 0.5,
                    "{name} @{size}: '{ch}' ink drops below its line box \
                     (ink_bottom={ink_bottom}, box_bottom={bottom})"
                );
            }
        }
    }

    #[test]
    fn builtin_font_ink_stays_inside_its_line_box() {
        assert_ink_fits_line_box(&include_bytes!("../assets/font.ttf")[..], "builtin");
    }

    #[test]
    fn line_height_is_the_font_line_box_not_the_tallest_glyph() {
        // The distinction the old code collapsed. `line_height` must be the
        // font's declared line box (ascent - descent + gap), which is
        // strictly taller than any single glyph's ink — a face whose glyphs
        // sit high in the em box (Silkscreen declares ascent 49.44 with a cap
        // height of 28) is exactly where measuring ink instead goes wrong.
        let font = fontdue::Font::from_bytes(
            &include_bytes!("../assets/font.ttf")[..],
            fontdue::FontSettings::default(),
        )
        .unwrap();
        let m = font.horizontal_line_metrics(FONT_SIZE).unwrap();
        let tallest_ink = (32u8..127)
            .map(|c| font.metrics(c as char, FONT_SIZE).height as f32)
            .fold(0.0, f32::max);
        assert!(
            m.new_line_size >= tallest_ink,
            "a line box must hold the tallest glyph: line={} ink={tallest_ink}",
            m.new_line_size
        );
        assert!((m.new_line_size - (m.ascent - m.descent + m.line_gap)).abs() < 1e-3);
    }
}
