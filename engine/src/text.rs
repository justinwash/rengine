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

/// Square, in texels. 1024 holds ASCII plus [`EXTENDED`] at [`FONT_SIZE`]
/// with room to spare; 512 held ASCII alone.
pub(crate) const ATLAS_SIZE: u32 = 1024;
pub(crate) const FONT_SIZE: f32 = 48.0;

/// The characters past ASCII an atlas rasterizes, when its face has them:
/// Latin-1, and the punctuation, arrows and shapes game text reaches for.
pub(crate) const EXTENDED: &[char] = &[
    // Latin-1 Supplement, U+00A0..=U+00FF.
    '\u{a0}', '¡', '¢', '£', '¤', '¥', '¦', '§', '¨', '©', 'ª', '«', '¬', '\u{ad}', '®', '¯',
    '°', '±', '²', '³', '´', 'µ', '¶', '·', '¸', '¹', 'º', '»', '¼', '½', '¾', '¿',
    'À', 'Á', 'Â', 'Ã', 'Ä', 'Å', 'Æ', 'Ç', 'È', 'É', 'Ê', 'Ë', 'Ì', 'Í', 'Î', 'Ï',
    'Ð', 'Ñ', 'Ò', 'Ó', 'Ô', 'Õ', 'Ö', '×', 'Ø', 'Ù', 'Ú', 'Û', 'Ü', 'Ý', 'Þ', 'ß',
    'à', 'á', 'â', 'ã', 'ä', 'å', 'æ', 'ç', 'è', 'é', 'ê', 'ë', 'ì', 'í', 'î', 'ï',
    'ð', 'ñ', 'ò', 'ó', 'ô', 'õ', 'ö', '÷', 'ø', 'ù', 'ú', 'û', 'ü', 'ý', 'þ', 'ÿ',
    // Punctuation.
    '–', '—', '‘', '’', '‚', '“', '”', '„', '•', '…', '′', '″', '‹', '›', '€', '™',
    // Arrows, maths, shapes.
    '←', '↑', '→', '↓', '−', '≈', '≠', '≤', '≥', '▲', '▼', '▶', '◀', '●', '○', '■', '□',
    '★', '☆', '✓', '✗',
];

/// What to draw for a character the face lacks, so that nothing is skipped
/// silently: a dash for a dash, three dots for an ellipsis, the bare letter
/// for an accented one.
pub(crate) fn ascii_stand_in(ch: char) -> Option<&'static str> {
    Some(match ch {
        '\u{a0}' | '\u{2007}' | '\u{2009}' | '\u{202f}' => " ",
        '–' | '—' | '−' | '‒' | '―' | '·' | '•' | '\u{ad}' => "-",
        '…' => "...",
        '‘' | '’' | '‚' | '′' | '´' | '`' => "'",
        '“' | '”' | '„' | '″' | '«' | '»' => "\"",
        '‹' | '◀' => "<",
        '›' | '▶' => ">",
        '×' => "x",
        '÷' => "/",
        '←' => "<-",
        '→' => "->",
        '↑' | '▲' => "^",
        '↓' | '▼' => "v",
        '≤' => "<=",
        '≥' => ">=",
        '≈' => "~",
        '≠' => "!=",
        '±' => "+/-",
        '●' | '■' | '★' => "*",
        '○' | '□' | '☆' => "o",
        '✓' => "+",
        '✗' => "x",
        '€' => "EUR",
        '£' => "GBP",
        '™' => "TM",
        '©' => "(c)",
        '®' => "(R)",
        '°' => "o",
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A",
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'Æ' => "AE",
        'æ' => "ae",
        'Ç' => "C",
        'ç' => "c",
        'È' | 'É' | 'Ê' | 'Ë' => "E",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'Ì' | 'Í' | 'Î' | 'Ï' => "I",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'Ñ' => "N",
        'ñ' => "n",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "O",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
        'Ù' | 'Ú' | 'Û' | 'Ü' => "U",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'Ý' => "Y",
        'ý' | 'ÿ' => "y",
        'ß' => "ss",
        'Ð' => "D",
        'ð' => "d",
        'Þ' => "Th",
        'þ' => "th",
        '¡' => "!",
        '¿' => "?",
        '¹' => "1",
        '²' => "2",
        '³' => "3",
        '¼' => "1/4",
        '½' => "1/2",
        '¾' => "3/4",
        '¢' => "c",
        '¤' => "$",
        '¥' => "Y",
        'µ' => "u",
        'ª' => "a",
        'º' => "o",
        '¬' | '¯' => "-",
        '¨' => "\"",
        '¸' => ",",
        '§' => "S",
        '¶' => "P",
        '¦' => "|",
        _ => return None,
    })
}

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
    /// ASCII, indexed by code point: the hot path.
    pub(crate) glyphs: [Option<GlyphEntry>; 128],
    /// Every [`EXTENDED`] character the face has.
    pub(crate) extended: std::collections::HashMap<char, GlyphEntry>,
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
    /// The size the glyphs were rasterised at: [`FONT_SIZE`] for a smooth
    /// face, the native grid for a pixel one. Every metric above is at this
    /// size, and `size / raster_size` is the scale any other size draws at.
    pub(crate) raster_size: f32,
    /// A pixel face (`FontRaster::Pixel`): nearest-sampled, drawn on whole
    /// pixels.
    pub(crate) pixel: bool,
    pub(crate) id: FontId,
}

impl FontAtlas {
    pub fn id(&self) -> FontId {
        self.id
    }

    pub fn white_uv(&self) -> [f32; 2] {
        self.white_uv
    }

    /// The scale a run at `size` draws this face's glyphs at.
    pub(crate) fn scale(&self, size: f32) -> f32 {
        size / self.raster_size
    }

    /// Whether this is a pixel face (`FontRaster::Pixel`).
    pub fn is_pixel(&self) -> bool {
        self.pixel
    }

    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        let scale = self.scale(size);
        let mut width: f32 = 0.0;
        for ch in text.chars() {
            self.each_glyph(ch, |e| width += e.advance * scale);
        }
        (width, self.line_height * scale)
    }

    /// The glyph this face draws `ch` with, if it has one.
    fn glyph(&self, ch: char) -> Option<GlyphEntry> {
        match ch as usize {
            idx @ 0..=127 => self.glyphs[idx],
            _ => self.extended.get(&ch).copied(),
        }
    }

    /// Hand `f` the glyphs that draw `ch`: its own, or, when the face lacks
    /// it, its [`ascii_stand_in`]'s. Returns whether anything was drawn.
    ///
    /// The one lookup every draw and measure goes through, so a character
    /// cannot measure one way and paint another. And a character is never
    /// dropped without the caller hearing about it: it used to be, and every
    /// `·` and `—` in a game's text vanished without a trace.
    pub(crate) fn each_glyph(&self, ch: char, mut f: impl FnMut(GlyphEntry)) -> bool {
        if let Some(entry) = self.glyph(ch) {
            f(entry);
            return true;
        }
        let Some(stand_in) = ascii_stand_in(ch) else {
            return false;
        };
        let mut drew = false;
        for c in stand_in.chars() {
            if let Some(entry) = self.glyph(c) {
                f(entry);
                drew = true;
            }
        }
        drew
    }

    pub fn line_height(&self, size: f32) -> f32 {
        self.line_height * self.scale(size)
    }

    /// Where the baseline sits inside a line box whose **top** is at `top`
    /// (y-up canvas coords, so the baseline is below it).
    ///
    /// The one place the rect→baseline conversion lives. Every text path goes
    /// through it, so a node's ink lands inside the rect the layout gave it
    /// by construction rather than by each call site guessing.
    pub fn baseline_below_top(&self, top: f32, size: f32) -> f32 {
        top - self.ascent * self.scale(size)
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
        FontRaster::Smooth,
    )
}

/// How a face is rasterised into its atlas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontRaster {
    /// An outline face, rasterised once at [`FONT_SIZE`] and drawn at any size
    /// through a mipmapped, trilinear-filtered atlas. Without the mips a 48px
    /// atlas drawn at 8-14px sampled a few texels out of every 3x3 to 6x6
    /// block, so thin strokes dropped out and 2 read as 8 (a 2026-10-06
    /// playtest finding).
    Smooth,
    /// A pixel face, drawn on its own grid: rasterised at the size where one
    /// font pixel is one texel (`native_px`), and sampled nearest-neighbour so
    /// every font pixel stays a solid block at any size from `native_px` up.
    /// Silkscreen's grid is 125 units on a 1000 em, so its `native_px` is 8.
    Pixel { native_px: f32 },
}

/// The space between glyphs in a smooth atlas, and the grid each glyph's
/// corner is aligned to: enough that the mip levels text is drawn from (down
/// to 1/8 scale) never average a neighbour's edge into a glyph.
const SMOOTH_GLYPH_PAD: u32 = 8;
/// A smooth atlas's mip chain stops at 1/16 scale.
const SMOOTH_MIP_LEVELS: u32 = 5;
/// The opaque block in the atlas's corner that solid fills sample. Big enough
/// to stay white at every mip level.
const WHITE_BLOCK: u32 = 8;

pub(crate) fn build_atlas_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    font_bytes: &[u8],
    id: FontId,
    raster: FontRaster,
) -> FontAtlas {
    let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
        .expect("failed to parse font");
    // A glyph set that does not fit the default atlas tries a larger one
    // rather than dropping characters.
    for size in [ATLAS_SIZE, ATLAS_SIZE * 2] {
        if let Some(atlas) =
            try_build_atlas(device, queue, bind_group_layout, &font, id, raster, size)
        {
            return atlas;
        }
    }
    panic!("font glyphs do not fit a {}px atlas", ATLAS_SIZE * 2);
}

#[allow(clippy::too_many_arguments)]
fn try_build_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    font: &fontdue::Font,
    id: FontId,
    raster: FontRaster,
    atlas_size: u32,
) -> Option<FontAtlas> {
    let (raster_size, pixel) = match raster {
        FontRaster::Smooth => (FONT_SIZE, false),
        FontRaster::Pixel { native_px } => (native_px, true),
    };
    let (pad, align) = match pixel {
        true => (1, 1),
        false => (SMOOTH_GLYPH_PAD, SMOOTH_GLYPH_PAD),
    };
    let mut pixels = vec![0u8; (atlas_size * atlas_size * 4) as usize];

    for y in 0..WHITE_BLOCK {
        for x in 0..WHITE_BLOCK {
            let offset = ((y * atlas_size + x) * 4) as usize;
            pixels[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    let white_uv = [1.0 / atlas_size as f32, 1.0 / atlas_size as f32];

    let mut glyphs: [Option<GlyphEntry>; 128] = [None; 128];
    let mut extended = std::collections::HashMap::new();

    let mut cursor_x: u32 = WHITE_BLOCK + pad;
    let mut cursor_y: u32 = 0;
    let mut row_height: u32 = 0;
    let round_up = |v: u32| v.div_ceil(align) * align;

    // The font's own vertical metrics, not the tallest glyph's ink box. A
    // line box is `ascent - descent + line_gap` — the same number a browser
    // uses for `line-height: normal`, which is what the mockups are laid out
    // against. Measuring ink instead made every rect shorter than a real
    // line and left the baseline undefined.
    let (ascent, line_height) = match font.horizontal_line_metrics(raster_size) {
        Some(m) => (m.ascent, m.new_line_size),
        // No hhea/OS2 table: fall back to the em box, which is at least
        // self-consistent (baseline at 80% is the usual default).
        None => (raster_size * 0.8, raster_size),
    };

    let mut full = false;
    // Rasterize one character into the atlas. `None` when the atlas is full.
    let mut pack = |ch: char| -> Option<GlyphEntry> {
        let (metrics, bitmap) = font.rasterize(ch, raster_size);
        if metrics.width == 0 || metrics.height == 0 {
            // A space: an advance and no ink.
            return (metrics.advance_width > 0.0).then_some(GlyphEntry {
                u0: white_uv[0],
                v0: white_uv[1],
                u1: white_uv[0],
                v1: white_uv[1],
                width_px: 0.0,
                height_px: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
                advance: metrics.advance_width,
            });
        }

        let gw = metrics.width as u32;
        let gh = metrics.height as u32;

        if cursor_x + gw + pad > atlas_size {
            cursor_x = 0;
            cursor_y = round_up(cursor_y + row_height + pad);
            row_height = 0;
        }

        if cursor_y + gh > atlas_size {
            full = true;
            return None;
        }

        for gy in 0..gh {
            for gx in 0..gw {
                let src = (gy * gw + gx) as usize;
                let dst = (((cursor_y + gy) * atlas_size + cursor_x + gx) * 4) as usize;
                // A pixel face's coverage on its own grid is all-or-nothing;
                // the threshold cleans up any edge the rasteriser half-lit.
                let a = match pixel {
                    true => if bitmap[src] >= 128 { 255 } else { 0 },
                    false => bitmap[src],
                };
                pixels[dst..dst + 4].copy_from_slice(&[255, 255, 255, a]);
            }
        }

        let entry = GlyphEntry {
            u0: cursor_x as f32 / atlas_size as f32,
            v0: cursor_y as f32 / atlas_size as f32,
            u1: (cursor_x + gw) as f32 / atlas_size as f32,
            v1: (cursor_y + gh) as f32 / atlas_size as f32,
            width_px: gw as f32,
            height_px: gh as f32,
            x_offset: metrics.xmin as f32,
            y_offset: metrics.ymin as f32,
            advance: metrics.advance_width,
        };

        cursor_x = round_up(cursor_x + gw + pad);
        row_height = row_height.max(gh);
        Some(entry)
    };

    for c in 32u8..127 {
        glyphs[c as usize] = pack(c as char);
    }
    // Past ASCII, only what the face really has: `rasterize` on a character
    // the face lacks draws its .notdef box, which is worse than the ASCII
    // stand-in `FontAtlas::each_glyph` falls back to.
    for &ch in EXTENDED {
        if font.has_glyph(ch) {
            if let Some(entry) = pack(ch) {
                extended.insert(ch, entry);
            }
        }
    }
    if full {
        return None;
    }

    let mip_level_count = match pixel {
        true => 1,
        false => SMOOTH_MIP_LEVELS,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("font_atlas"),
        size: wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        mip_level_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Level 0, then each level a 2x2 box average of the one above. Colour is
    // white throughout, so only coverage is averaged.
    let mut level = pixels;
    let mut level_size = atlas_size;
    for mip in 0..mip_level_count {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &level,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(level_size * 4),
                rows_per_image: Some(level_size),
            },
            wgpu::Extent3d {
                width: level_size,
                height: level_size,
                depth_or_array_layers: 1,
            },
        );
        if mip + 1 < mip_level_count {
            let next = level_size / 2;
            let mut down = vec![0u8; (next * next * 4) as usize];
            for y in 0..next {
                for x in 0..next {
                    let a = |dx: u32, dy: u32| {
                        level[(((y * 2 + dy) * level_size + x * 2 + dx) * 4 + 3) as usize] as u32
                    };
                    let avg = ((a(0, 0) + a(1, 0) + a(0, 1) + a(1, 1) + 2) / 4) as u8;
                    let o = ((y * next + x) * 4) as usize;
                    down[o..o + 4].copy_from_slice(&[255, 255, 255, avg]);
                }
            }
            level = down;
            level_size = next;
        }
    }

    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("font_sampler"),
        mag_filter: match pixel {
            true => wgpu::FilterMode::Nearest,
            false => wgpu::FilterMode::Linear,
        },
        min_filter: match pixel {
            true => wgpu::FilterMode::Nearest,
            false => wgpu::FilterMode::Linear,
        },
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
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

    Some(FontAtlas {
        bind_group,
        glyphs,
        extended,
        white_uv,
        line_height,
        ascent,
        raster_size,
        pixel,
        id,
    })
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

    /// Nothing is dropped by a face that has ASCII: every character an atlas
    /// tries past ASCII has a stand-in made of printable ASCII, for the faces
    /// that lack it.
    #[test]
    fn every_extended_character_has_an_ascii_stand_in() {
        for &ch in EXTENDED {
            let stand_in = ascii_stand_in(ch)
                .unwrap_or_else(|| panic!("no stand-in for {ch:?} (U+{:04X})", ch as u32));
            assert!(
                !stand_in.is_empty() && stand_in.chars().all(|c| (' '..='~').contains(&c)),
                "{ch:?}'s stand-in {stand_in:?} is not printable ASCII"
            );
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
