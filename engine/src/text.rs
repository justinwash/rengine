use std::cell::RefCell;
use std::collections::HashMap;
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
    /// ASCII, indexed by code point: the hot path. For a smooth face these
    /// are metrics at [`FONT_SIZE`] only — what measuring needs — and the
    /// pixels come from [`sized`](Self::sized) at the size drawn.
    pub(crate) glyphs: [Option<GlyphEntry>; 128],
    /// Every [`EXTENDED`] character the face has.
    pub(crate) extended: HashMap<char, GlyphEntry>,
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
    /// A smooth face's glyphs at the sizes they are drawn at, filled as text
    /// asks for them. `None` for a pixel face, whose atlas is complete at load.
    sized: Option<RefCell<SizedGlyphs>>,
    pub(crate) id: FontId,
}

/// A smooth face's glyphs, rasterised at the physical pixel size each run is
/// drawn at, so a glyph lands 1:1 on the screen instead of being resampled.
///
/// Resampling is what made every smooth face soft: one 48px raster shrunk to
/// 10-20px through mips and trilinear filtering, at fractional positions,
/// blurs every stroke twice. A UI draws text at a handful of sizes, so a cache
/// keyed by size stays small.
struct SizedGlyphs {
    font: fontdue::Font,
    texture: wgpu::Texture,
    /// Keyed by character and size in 1/16 px.
    glyphs: HashMap<(char, u32), GlyphEntry>,
    shelf: Shelf,
    /// Rasterised since the last [`FontAtlas::flush`] and not yet on the GPU:
    /// each cell's `[x, y, w, h]` in texels and its RGBA texels.
    pending: Vec<([u32; 4], Vec<u8>)>,
    /// A glyph did not fit. The cache starts over once the frame that
    /// overflowed has been drawn — its quads still point at the old cells.
    overflowed: bool,
}

/// Side of a smooth face's glyph cache, in texels. Text at four sizes and
/// twice the pixel density uses a fraction of it.
const SIZED_ATLAS_SIZE: u32 = 2048;
/// Sizes are cached in steps of 1/16 px.
const SIZE_STEPS_PER_PX: f32 = 16.0;

/// Packs cells left to right in rows, starting past the white block.
struct Shelf {
    x: u32,
    y: u32,
    row_height: u32,
}

impl Shelf {
    fn new() -> Self {
        Self {
            x: WHITE_BLOCK,
            y: 0,
            row_height: 0,
        }
    }

    /// The top-left corner of a free `w` x `h` cell, or `None` when the atlas
    /// is full.
    fn place(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.x + w > SIZED_ATLAS_SIZE {
            self.x = 0;
            self.y += self.row_height.max(WHITE_BLOCK);
            self.row_height = 0;
        }
        if w > SIZED_ATLAS_SIZE || self.y + h > SIZED_ATLAS_SIZE {
            return None;
        }
        let at = (self.x, self.y);
        self.x += w;
        self.row_height = self.row_height.max(h);
        Some(at)
    }
}

impl FontAtlas {
    pub fn id(&self) -> FontId {
        self.id
    }

    pub fn white_uv(&self) -> [f32; 2] {
        self.white_uv
    }

    /// The scale a run at `size` draws this face's glyphs at.
    ///
    /// A pixel face never goes below 1:1. Under its own grid, nearest
    /// sampling drops whole rows and columns of its pixels: Silkscreen at 7px
    /// loses one in eight, and "PUSH PACE" read "FUSH FHLE". Measuring goes
    /// through here too, so a run laid out at 7px is laid out at the 8px it
    /// draws at.
    pub(crate) fn scale(&self, size: f32) -> f32 {
        let scale = size / self.raster_size;
        match self.pixel {
            true => scale.max(1.0),
            false => scale,
        }
    }

    /// Whether this is a pixel face (`FontRaster::Pixel`).
    pub fn is_pixel(&self) -> bool {
        self.pixel
    }

    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        let scale = self.scale(size);
        let mut width: f32 = 0.0;
        for ch in text.chars() {
            self.each_glyph(ch, |_, e| width += e.advance * scale);
        }
        (width, self.line_height * scale)
    }

    /// A smooth face's glyph for `ch` rasterised at `px` physical pixels, with
    /// every field in those pixels. `None` for a pixel face, or when the cache
    /// is full this frame.
    ///
    /// Only the pixels come from here. Advances still come from the
    /// [`FONT_SIZE`] metrics, scaled, so a run draws exactly as wide as it
    /// measured: rasterising at the drawn size moves no layout.
    pub(crate) fn sized_glyph(&self, ch: char, px: f32) -> Option<GlyphEntry> {
        let mut cache = self.sized.as_ref()?.borrow_mut();
        let steps = (px * SIZE_STEPS_PER_PX).round().max(1.0) as u32;
        if let Some(entry) = cache.glyphs.get(&(ch, steps)) {
            return Some(*entry);
        }
        if cache.overflowed {
            return None;
        }
        let (metrics, bitmap) = cache.font.rasterize(ch, steps as f32 / SIZE_STEPS_PER_PX);
        let (w, h) = (metrics.width as u32, metrics.height as u32);
        let mut entry = GlyphEntry {
            u0: 0.0,
            v0: 0.0,
            u1: 0.0,
            v1: 0.0,
            width_px: w as f32,
            height_px: h as f32,
            x_offset: metrics.xmin as f32,
            y_offset: metrics.ymin as f32,
            advance: metrics.advance_width,
        };
        if w > 0 && h > 0 {
            // A clear texel on every side, so a quad that lands off the pixel
            // grid (a letterboxed canvas) filters in nothing from a neighbour.
            let (cw, ch_) = (w + 2, h + 2);
            let Some((x, y)) = cache.shelf.place(cw, ch_) else {
                log::warn!("glyph cache full; it starts over next frame");
                cache.overflowed = true;
                return None;
            };
            let mut texels = vec![0u8; (cw * ch_ * 4) as usize];
            for gy in 0..h {
                for gx in 0..w {
                    let a = bitmap[(gy * w + gx) as usize];
                    let o = (((gy + 1) * cw + gx + 1) * 4) as usize;
                    texels[o..o + 4].copy_from_slice(&[255, 255, 255, a]);
                }
            }
            cache.pending.push(([x, y, cw, ch_], texels));
            let s = SIZED_ATLAS_SIZE as f32;
            entry.u0 = (x + 1) as f32 / s;
            entry.v0 = (y + 1) as f32 / s;
            entry.u1 = (x + 1 + w) as f32 / s;
            entry.v1 = (y + 1 + h) as f32 / s;
        }
        cache.glyphs.insert((ch, steps), entry);
        Some(entry)
    }

    /// Upload the glyphs rasterised since the last flush. The canvas pass
    /// calls this before it draws, once every quad of the frame is built.
    pub(crate) fn flush(&self, queue: &wgpu::Queue) {
        let Some(sized) = &self.sized else {
            return;
        };
        let mut cache = sized.borrow_mut();
        let SizedGlyphs {
            texture,
            pending,
            glyphs,
            shelf,
            overflowed,
            ..
        } = &mut *cache;
        for ([x, y, w, h], texels) in pending.drain(..) {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &texels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }
        if *overflowed {
            glyphs.clear();
            *shelf = Shelf::new();
            *overflowed = false;
        }
    }

    /// The glyph this face draws `ch` with, if it has one.
    fn glyph(&self, ch: char) -> Option<GlyphEntry> {
        match ch as usize {
            idx @ 0..=127 => self.glyphs[idx],
            _ => self.extended.get(&ch).copied(),
        }
    }

    /// Hand `f` the glyphs that draw `ch`, each with the character it is: its
    /// own, or, when the face lacks it, its [`ascii_stand_in`]'s. Returns
    /// whether anything was drawn.
    ///
    /// The one lookup every draw and measure goes through, so a character
    /// cannot measure one way and paint another. And a character is never
    /// dropped without the caller hearing about it: it used to be, and every
    /// `·` and `—` in a game's text vanished without a trace.
    pub(crate) fn each_glyph(&self, ch: char, mut f: impl FnMut(char, GlyphEntry)) -> bool {
        if let Some(entry) = self.glyph(ch) {
            f(ch, entry);
            return true;
        }
        let Some(stand_in) = ascii_stand_in(ch) else {
            return false;
        };
        let mut drew = false;
        for c in stand_in.chars() {
            if let Some(entry) = self.glyph(c) {
                f(c, entry);
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

/// How a face is rasterised.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontRaster {
    /// An outline face, rasterised at the physical pixel size each run is
    /// drawn at and placed on whole pixels, the way a desktop draws text.
    ///
    /// It used to be rasterised once at [`FONT_SIZE`] and shrunk to the
    /// 10-20px UI text is set at through a mipmapped, trilinear-filtered
    /// atlas, at fractional positions. Every stroke was resampled twice, so
    /// every smooth face read soft at every size.
    Smooth,
    /// A pixel face, drawn on its own grid: rasterised at the size where one
    /// font pixel is one texel (`native_px`), and sampled nearest-neighbour so
    /// every font pixel stays a solid block at any size from `native_px` up.
    /// Silkscreen's grid is 125 units on a 1000 em, so its `native_px` is 8.
    Pixel { native_px: f32 },
}

/// The opaque block in an atlas's corner that solid fills sample.
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
    let native_px = match raster {
        FontRaster::Smooth => {
            return build_sized_atlas(device, queue, bind_group_layout, font, id)
        }
        FontRaster::Pixel { native_px } => native_px,
    };
    // A glyph set that does not fit the default atlas tries a larger one
    // rather than dropping characters.
    for size in [ATLAS_SIZE, ATLAS_SIZE * 2] {
        if let Some(atlas) =
            try_build_pixel_atlas(device, queue, bind_group_layout, &font, id, native_px, size)
        {
            return atlas;
        }
    }
    panic!("font glyphs do not fit a {}px atlas", ATLAS_SIZE * 2);
}

/// `(ascent, line_height)` at `px`: the font's own vertical metrics, not the
/// tallest glyph's ink box. A line box is `ascent - descent + line_gap` — the
/// same number a browser uses for `line-height: normal`, which is what the
/// mockups are laid out against. Measuring ink instead made every rect
/// shorter than a real line and left the baseline undefined.
fn line_metrics(font: &fontdue::Font, px: f32) -> (f32, f32) {
    match font.horizontal_line_metrics(px) {
        Some(m) => (m.ascent, m.new_line_size),
        // No hhea/OS2 table: fall back to the em box, which is at least
        // self-consistent (baseline at 80% is the usual default).
        None => (px * 0.8, px),
    }
}

/// The characters an atlas covers: ASCII, then the [`EXTENDED`] characters the
/// face really has. Past ASCII a missing character is left out: `rasterize`
/// on one draws the face's .notdef box, which is worse than the ASCII stand-in
/// [`FontAtlas::each_glyph`] falls back to.
fn atlas_glyphs(
    font: &fontdue::Font,
    mut entry: impl FnMut(char) -> Option<GlyphEntry>,
) -> ([Option<GlyphEntry>; 128], HashMap<char, GlyphEntry>) {
    let mut glyphs: [Option<GlyphEntry>; 128] = [None; 128];
    for c in 32u8..127 {
        glyphs[c as usize] = entry(c as char);
    }
    let mut extended = HashMap::new();
    for &ch in EXTENDED {
        if font.has_glyph(ch) {
            if let Some(e) = entry(ch) {
                extended.insert(ch, e);
            }
        }
    }
    (glyphs, extended)
}

fn atlas_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    filter: wgpu::FilterMode,
) -> wgpu::BindGroup {
    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("font_sampler"),
        mag_filter: filter,
        min_filter: filter,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("font_bind_group"),
        layout,
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
    })
}

fn atlas_texture(device: &wgpu::Device, label: &'static str, size: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

/// A smooth face: metrics at [`FONT_SIZE`] for measuring, and an empty glyph
/// cache that drawing fills at the sizes it draws
/// ([`FontAtlas::sized_glyph`]).
fn build_sized_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    font: fontdue::Font,
    id: FontId,
) -> FontAtlas {
    let (ascent, line_height) = line_metrics(&font, FONT_SIZE);
    let (glyphs, extended) = atlas_glyphs(&font, |ch| {
        let m = font.metrics(ch, FONT_SIZE);
        let ink = m.width > 0 && m.height > 0;
        (ink || m.advance_width > 0.0).then_some(GlyphEntry {
            u0: 0.0,
            v0: 0.0,
            u1: 0.0,
            v1: 0.0,
            width_px: if ink { m.width as f32 } else { 0.0 },
            height_px: if ink { m.height as f32 } else { 0.0 },
            x_offset: if ink { m.xmin as f32 } else { 0.0 },
            y_offset: if ink { m.ymin as f32 } else { 0.0 },
            advance: m.advance_width,
        })
    });

    let texture = atlas_texture(device, "glyph_cache", SIZED_ATLAS_SIZE);
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &vec![255u8; (WHITE_BLOCK * WHITE_BLOCK * 4) as usize],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(WHITE_BLOCK * 4),
            rows_per_image: Some(WHITE_BLOCK),
        },
        wgpu::Extent3d {
            width: WHITE_BLOCK,
            height: WHITE_BLOCK,
            depth_or_array_layers: 1,
        },
    );
    // Linear, though a glyph drawn on the grid samples texel centres and gets
    // exactly its own coverage: a canvas that is stretched rather than
    // letterboxed is still smoothed rather than dropped.
    let bind_group =
        atlas_bind_group(device, bind_group_layout, &texture, wgpu::FilterMode::Linear);
    let centre = WHITE_BLOCK as f32 / 2.0 / SIZED_ATLAS_SIZE as f32;

    FontAtlas {
        bind_group,
        glyphs,
        extended,
        white_uv: [centre, centre],
        line_height,
        ascent,
        raster_size: FONT_SIZE,
        pixel: false,
        sized: Some(RefCell::new(SizedGlyphs {
            font,
            texture,
            glyphs: HashMap::new(),
            shelf: Shelf::new(),
            pending: Vec::new(),
            overflowed: false,
        })),
        id,
    }
}

/// A pixel face's atlas, complete at load: every glyph at `native_px`, one
/// font pixel to a texel. `None` when the glyphs do not fit `atlas_size`.
fn try_build_pixel_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    font: &fontdue::Font,
    id: FontId,
    native_px: f32,
    atlas_size: u32,
) -> Option<FontAtlas> {
    const PAD: u32 = 1;
    let mut pixels = vec![0u8; (atlas_size * atlas_size * 4) as usize];
    for y in 0..WHITE_BLOCK {
        for x in 0..WHITE_BLOCK {
            let offset = ((y * atlas_size + x) * 4) as usize;
            pixels[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    let white_uv = [1.0 / atlas_size as f32, 1.0 / atlas_size as f32];

    let mut cursor_x: u32 = WHITE_BLOCK + PAD;
    let mut cursor_y: u32 = 0;
    let mut row_height: u32 = 0;
    let (ascent, line_height) = line_metrics(font, native_px);

    let mut full = false;
    let (glyphs, extended) = atlas_glyphs(font, |ch| {
        let (metrics, bitmap) = font.rasterize(ch, native_px);
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
        if cursor_x + gw + PAD > atlas_size {
            cursor_x = 0;
            cursor_y += row_height + PAD;
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
                // Coverage on the face's own grid is all-or-nothing; the
                // threshold cleans up any edge the rasteriser half-lit.
                let a = if bitmap[src] >= 128 { 255 } else { 0 };
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
        cursor_x += gw + PAD;
        row_height = row_height.max(gh);
        Some(entry)
    });
    if full {
        return None;
    }

    let texture = atlas_texture(device, "font_atlas", atlas_size);
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
            bytes_per_row: Some(atlas_size * 4),
            rows_per_image: Some(atlas_size),
        },
        wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
    );
    let bind_group =
        atlas_bind_group(device, bind_group_layout, &texture, wgpu::FilterMode::Nearest);

    Some(FontAtlas {
        bind_group,
        glyphs,
        extended,
        white_uv,
        line_height,
        ascent,
        raster_size: native_px,
        pixel: true,
        sized: None,
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
