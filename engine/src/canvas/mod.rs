use crate::assets::Color;
use crate::renderer::TextureId;
use crate::text::{FontAtlas, FontId, ATLAS_SIZE, FONT_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CanvasVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl CanvasVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub const MAX_CANVAS_VERTICES: usize = 8_000;

const WHITE_UV: [f32; 2] = [1.0 / ATLAS_SIZE as f32, 1.0 / ATLAS_SIZE as f32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DrawTexture {
    Font(usize),
    Texture(usize),
}

pub(crate) struct DrawSegment {
    pub start: usize,
    pub count: usize,
    pub scissor: Option<[u32; 4]>,
    pub texture: DrawTexture,
}

pub struct Canvas {
    pub(crate) verts: Vec<CanvasVertex>,
    pub(crate) segments: Vec<DrawSegment>,
    screen_size: (u32, u32),
    clip_stack: Vec<[u32; 4]>,
    segment_start: usize,
    current_texture: DrawTexture,
    atlas: *const FontAtlas,
    /// Every loaded atlas, so a caller holding only a [`FontId`] can measure
    /// and draw with it. `atlas` above is element 0 — the default font — kept
    /// as its own pointer so the no-font-table case (a `Canvas` built for a
    /// test with a null table) still behaves exactly as before.
    fonts: *const [FontAtlas],
    /// Extra pixels inserted after every glyph — CSS `letter-spacing`, which
    /// the pixel-art UI leans on heavily (70 uses across the mockups) to open
    /// up all-caps chrome.
    ///
    /// Canvas state rather than an argument on all nine text entry points:
    /// drawing, measuring and alignment must agree on it or centred text
    /// drifts by half the accumulated tracking, and threading one more `f32`
    /// through every signature to guarantee that is a much larger diff than
    /// setting it once around the call.
    tracking: f32,
}

impl Canvas {
    pub(crate) fn new(screen_size: (u32, u32), atlas: *const FontAtlas) -> Self {
        Self::with_fonts(screen_size, atlas, std::ptr::slice_from_raw_parts(atlas, 0))
    }

    pub(crate) fn with_fonts(
        screen_size: (u32, u32),
        atlas: *const FontAtlas,
        fonts: *const [FontAtlas],
    ) -> Self {
        Self {
            verts: Vec::new(),
            segments: Vec::new(),
            screen_size,
            clip_stack: Vec::new(),
            segment_start: 0,
            current_texture: DrawTexture::Font(0),
            atlas,
            fonts,
            tracking: 0.0,
        }
    }

    /// Set the extra spacing inserted after each glyph, in pixels at the
    /// drawn size. Applies to every subsequent text draw *and* measurement on
    /// this canvas until changed, so alignment and content sizing stay
    /// consistent with what is painted. Returns the previous value, so a
    /// caller can restore it without tracking that itself.
    pub fn set_tracking(&mut self, tracking: f32) -> f32 {
        std::mem::replace(&mut self.tracking, tracking)
    }

    /// The extra spacing currently applied after each glyph.
    pub fn tracking(&self) -> f32 {
        self.tracking
    }

    /// The width `tracking` adds to a run of `text`: one gap after every glyph
    /// except the last, so a single glyph and an empty string are unaffected
    /// and a trailing gap never pushes right-aligned text off its anchor.
    ///
    /// Counts the same glyphs the draw loop does — chars outside the atlas's
    /// ASCII range are skipped there, so counting them here would measure
    /// wider than it paints.
    fn tracking_width(&self, text: &str) -> f32 {
        Self::tracking_width_of(text, self.tracking)
    }

    fn tracking_width_of(text: &str, tracking: f32) -> f32 {
        if tracking == 0.0 {
            return 0.0;
        }
        let glyphs = text.chars().filter(|c| (*c as usize) < 128).count();
        tracking * (glyphs.saturating_sub(1)) as f32
    }

    /// [`measure_text_in`](Self::measure_text_in) at an explicit tracking
    /// rather than the canvas's current one, for the content-sizing pass —
    /// it holds a `&Canvas` and must measure a node's authored `ui_tracking`
    /// without mutating shared state mid-measure.
    pub fn measure_text_tracked(
        &self,
        font: FontId,
        text: &str,
        size: f32,
        tracking: f32,
    ) -> (f32, f32) {
        let (w, h) = self.font_atlas(font).measure_text(text, size);
        (w + Self::tracking_width_of(text, tracking), h)
    }

    fn atlas(&self) -> &FontAtlas {
        // SAFETY: `Canvas::new` stores a raw pointer to a `FontAtlas`.
        // The pointer is validated non-null below. The atlas lives inside
        // Engine for the entire program lifetime, so it always outlives
        // any Canvas instance.
        let ptr = self.atlas;
        assert!(
            !ptr.is_null(),
            "Canvas font atlas not initialized; call Frame::begin() before drawing text"
        );
        unsafe { &*ptr }
    }

    /// The atlas for `font`, or the default atlas if the table doesn't hold
    /// it. Falling back rather than panicking is deliberate: `ui_font` names a
    /// font the *scene* believes in, and a scene authored against a bundle the
    /// host didn't load should render in the default face, not crash.
    pub fn font_atlas(&self, font: FontId) -> &FontAtlas {
        // SAFETY: same lifetime argument as `atlas` — the table lives in the
        // renderer for the program's life and outlives every Canvas.
        let fonts = unsafe { self.fonts.as_ref() };
        match fonts {
            Some(fonts) => fonts.get(font.0).unwrap_or_else(|| self.atlas()),
            None => self.atlas(),
        }
    }

    /// [`measure_text`](Self::measure_text) in a specific font. Content sizing
    /// and text centring must go through this, not the default-font version,
    /// or a node drawn in one face is measured in another.
    pub fn measure_text_in(&self, font: FontId, text: &str, size: f32) -> (f32, f32) {
        let (w, h) = self.font_atlas(font).measure_text(text, size);
        (w + self.tracking_width(text), h)
    }

    /// [`line_height`](Self::line_height) in a specific font.
    pub fn line_height_in(&self, font: FontId, size: f32) -> f32 {
        self.font_atlas(font).line_height(size)
    }

    pub fn screen_size(&self) -> (u32, u32) {
        self.screen_size
    }

    pub fn push_clip(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.close_segment();

        let (sw, sh) = self.screen_size;
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;

        let px = ((x + hw).max(0.0)) as u32;
        let py = ((hh - y - h).max(0.0)) as u32;
        let pw = (w as u32).min(sw.saturating_sub(px));
        let ph = (h as u32).min(sh.saturating_sub(py));

        let mut rect = [px, py, pw, ph];

        if let Some(parent) = self.clip_stack.last() {
            let l = rect[0].max(parent[0]);
            let t = rect[1].max(parent[1]);
            let r = (rect[0] + rect[2]).min(parent[0] + parent[2]);
            let b = (rect[1] + rect[3]).min(parent[1] + parent[3]);
            if l >= r || t >= b {
                rect = [0, 0, 0, 0];
            } else {
                rect = [l, t, r - l, b - t];
            }
        }

        self.clip_stack.push(rect);
    }

    pub fn pop_clip(&mut self) {
        self.close_segment();
        self.clip_stack.pop();
    }

    fn close_segment(&mut self) {
        let count = self.verts.len() - self.segment_start;
        if count > 0 {
            self.segments.push(DrawSegment {
                start: self.segment_start,
                count,
                scissor: self.clip_stack.last().copied(),
                texture: self.current_texture,
            });
        }
        self.segment_start = self.verts.len();
    }

    fn set_font(&mut self, font_id: usize) {
        self.set_texture(DrawTexture::Font(font_id));
    }

    fn set_texture(&mut self, texture: DrawTexture) {
        if texture != self.current_texture {
            self.close_segment();
            self.current_texture = texture;
        }
    }

    pub(crate) fn finalize(&mut self) {
        self.close_segment();
    }

    pub fn shape(&mut self, triangles: &[CanvasVertex]) {
        self.set_font(0);
        self.verts.extend_from_slice(triangles);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.set_font(0);
        let [x0, y0] = screen_to_ndc(x, y, self.screen_size);
        let [x1, y1] = screen_to_ndc(x + w, y + h, self.screen_size);

        let c = color.to_array();
        let uv = WHITE_UV;
        let v0 = CanvasVertex {
            position: [x0, y0],
            color: c,
            uv,
        };
        let v1 = CanvasVertex {
            position: [x1, y0],
            color: c,
            uv,
        };
        let v2 = CanvasVertex {
            position: [x1, y1],
            color: c,
            uv,
        };
        let v3 = CanvasVertex {
            position: [x0, y1],
            color: c,
            uv,
        };
        self.verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
    }

    /// Fill a rect with a smooth vertical gradient (`bottom` at `y`, `top` at
    /// `y + h`), interpolated per-vertex by the GPU.
    pub fn rect_gradient(&mut self, x: f32, y: f32, w: f32, h: f32, bottom: Color, top: Color) {
        self.set_font(0);
        let [x0, y0] = screen_to_ndc(x, y, self.screen_size);
        let [x1, y1] = screen_to_ndc(x + w, y + h, self.screen_size);
        let cb = bottom.to_array();
        let ct = top.to_array();
        let uv = WHITE_UV;
        let v0 = CanvasVertex {
            position: [x0, y0],
            color: cb,
            uv,
        };
        let v1 = CanvasVertex {
            position: [x1, y0],
            color: cb,
            uv,
        };
        let v2 = CanvasVertex {
            position: [x1, y1],
            color: ct,
            uv,
        };
        let v3 = CanvasVertex {
            position: [x0, y1],
            color: ct,
            uv,
        };
        self.verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
    }

    /// Draw a raised-bevel outline around a rect: `highlight` on the top/left
    /// edges, `shadow` on the bottom/right (assuming a y-up coordinate space,
    /// i.e. `y + h` is the top edge).
    pub fn bevel_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        highlight: Color,
        shadow: Color,
        thickness: f32,
    ) {
        self.line(x, y + h, x + w, y + h, thickness, highlight);
        self.line(x, y, x, y + h, thickness, highlight);
        self.line(x, y, x + w, y, thickness, shadow);
        self.line(x + w, y, x + w, y + h, thickness, shadow);
    }

    /// Fill a rect with rounded corners of the given `radius`.
    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
        let r = radius.max(0.0).min(w * 0.5).min(h * 0.5);
        if r <= 0.5 {
            self.rect(x, y, w, h, color);
            return;
        }
        self.rect(x + r, y, w - 2.0 * r, h, color);
        self.rect(x, y + r, r, h - 2.0 * r, color);
        self.rect(x + w - r, y + r, r, h - 2.0 * r, color);
        self.circle_filled(x + r, y + r, r, 14, color);
        self.circle_filled(x + w - r, y + r, r, 14, color);
        self.circle_filled(x + r, y + h - r, r, 14, color);
        self.circle_filled(x + w - r, y + h - r, r, 14, color);
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: Color) {
        self.set_font(0);
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.0001 {
            return;
        }
        let nx = -dy / len * thickness * 0.5;
        let ny = dx / len * thickness * 0.5;

        let c = color.to_array();
        let uv = WHITE_UV;
        let a = screen_to_ndc(x0 + nx, y0 + ny, self.screen_size);
        let b = screen_to_ndc(x0 - nx, y0 - ny, self.screen_size);
        let cc = screen_to_ndc(x1 - nx, y1 - ny, self.screen_size);
        let d = screen_to_ndc(x1 + nx, y1 + ny, self.screen_size);

        let va = CanvasVertex {
            position: a,
            color: c,
            uv,
        };
        let vb = CanvasVertex {
            position: b,
            color: c,
            uv,
        };
        let vc = CanvasVertex {
            position: cc,
            color: c,
            uv,
        };
        let vd = CanvasVertex {
            position: d,
            color: c,
            uv,
        };
        self.verts.extend_from_slice(&[va, vc, vd, va, vb, vc]);
    }

    /// A connected run of line segments. Each segment is a quad, so on its own a
    /// bend would leave a wedge-shaped gap on the outside of the turn; a bevel
    /// join (one triangle spanning the two segments' outer corners) fills it.
    /// Straight-ish runs are unaffected — the gap only opens as the turn tightens.
    pub fn polyline(&mut self, points: &[(f32, f32)], thickness: f32, color: Color) {
        for pair in points.windows(2) {
            self.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, thickness, color);
        }
        let half = thickness * 0.5;
        let c = color.to_array();
        let uv = WHITE_UV;
        for w in points.windows(3) {
            let (p, q, r) = (w[0], w[1], w[2]);
            let d0 = (q.0 - p.0, q.1 - p.1);
            let d1 = (r.0 - q.0, r.1 - q.1);
            let l0 = (d0.0 * d0.0 + d0.1 * d0.1).sqrt();
            let l1 = (d1.0 * d1.0 + d1.1 * d1.1).sqrt();
            if l0 < 0.0001 || l1 < 0.0001 {
                continue;
            }
            // Turn direction decides which side the gap is on: the outside of the
            // bend. Cross product sign gives it.
            let cross = d0.0 * d1.1 - d0.1 * d1.0;
            if cross.abs() < 1e-6 {
                continue; // collinear: nothing to fill
            }
            let s = if cross > 0.0 { -1.0 } else { 1.0 };
            let n0 = (-d0.1 / l0 * half * s, d0.0 / l0 * half * s);
            let n1 = (-d1.1 / l1 * half * s, d1.0 / l1 * half * s);
            let center = screen_to_ndc(q.0, q.1, self.screen_size);
            let e0 = screen_to_ndc(q.0 + n0.0, q.1 + n0.1, self.screen_size);
            let e1 = screen_to_ndc(q.0 + n1.0, q.1 + n1.1, self.screen_size);
            self.verts.extend_from_slice(&[
                CanvasVertex {
                    position: center,
                    color: c,
                    uv,
                },
                CanvasVertex {
                    position: e0,
                    color: c,
                    uv,
                },
                CanvasVertex {
                    position: e1,
                    color: c,
                    uv,
                },
            ]);
        }
    }

    pub fn circle(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        thickness: f32,
        segments: u32,
        color: Color,
    ) {
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a0 = step * i as f32;
            let a1 = step * (i + 1) as f32;
            self.line(
                cx + a0.cos() * radius,
                cy + a0.sin() * radius,
                cx + a1.cos() * radius,
                cy + a1.sin() * radius,
                thickness,
                color,
            );
        }
    }

    pub fn circle_filled(&mut self, cx: f32, cy: f32, radius: f32, segments: u32, color: Color) {
        self.set_font(0);
        let c = color.to_array();
        let uv = WHITE_UV;
        let center = screen_to_ndc(cx, cy, self.screen_size);
        let vc = CanvasVertex {
            position: center,
            color: c,
            uv,
        };
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let a0 = step * i as f32;
            let a1 = step * (i + 1) as f32;
            let p0 = screen_to_ndc(
                cx + a0.cos() * radius,
                cy + a0.sin() * radius,
                self.screen_size,
            );
            let p1 = screen_to_ndc(
                cx + a1.cos() * radius,
                cy + a1.sin() * radius,
                self.screen_size,
            );
            let v0 = CanvasVertex {
                position: p0,
                color: c,
                uv,
            };
            let v1 = CanvasVertex {
                position: p1,
                color: c,
                uv,
            };
            self.verts.extend_from_slice(&[vc, v0, v1]);
        }
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
        let ptr = self.atlas;
        assert!(!ptr.is_null(), "Canvas font atlas not initialized");
        let atlas = unsafe { &*ptr };
        self.text_with_font(x, y, text, size, color, atlas);
    }

    /// Draw `text` with its **line box's top-left** at `(x, y)`.
    ///
    /// Top-left, not baseline: a node's rect is what the layout produced, and
    /// a caller that had to convert a rect into a baseline itself needed the
    /// font's ascent to do it — which is exactly the conversion every call
    /// site used to get wrong. `FontAtlas::baseline_below_top` owns it now,
    /// and the whole run lands inside a rect `line_height(size)` tall.
    pub fn text_with_font(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        atlas: &FontAtlas,
    ) {
        self.set_font(atlas.id().0);
        let scale = size / FONT_SIZE;
        let c = color.to_array();
        let baseline = atlas.baseline_below_top(y, size);
        let mut cursor_x = x;

        for ch in text.chars() {
            let idx = ch as usize;
            if idx >= 128 {
                continue;
            }
            let entry = match atlas.glyphs[idx] {
                Some(e) => e,
                None => continue,
            };

            if entry.width_px > 0.0 {
                let gx = cursor_x + entry.x_offset * scale;
                // `entry.y_offset` is `ymin` — the glyph's bottom relative to
                // the **baseline** — so it only means anything measured from
                // a baseline. Previously this subtracted it from
                // `line_height` (the tallest glyph's ink height at the time)
                // against a `y` that callers passed as a rect edge: three
                // different origins in one expression, which is why text drew
                // outside its own node.
                let gy = baseline + entry.y_offset * scale;
                let gw = entry.width_px * scale;
                let gh = entry.height_px * scale;

                let [x0, y0] = screen_to_ndc(gx, gy, self.screen_size);
                let [x1, y1] = screen_to_ndc(gx + gw, gy + gh, self.screen_size);

                let v0 = CanvasVertex {
                    position: [x0, y0],
                    color: c,
                    uv: [entry.u0, entry.v1],
                };
                let v1 = CanvasVertex {
                    position: [x1, y0],
                    color: c,
                    uv: [entry.u1, entry.v1],
                };
                let v2 = CanvasVertex {
                    position: [x1, y1],
                    color: c,
                    uv: [entry.u1, entry.v0],
                };
                let v3 = CanvasVertex {
                    position: [x0, y1],
                    color: c,
                    uv: [entry.u0, entry.v0],
                };
                self.verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
            }

            cursor_x += entry.advance * scale + self.tracking;
        }
    }

    pub fn text_aligned(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        self.text_aligned_in(FontId::DEFAULT, x, y, text, size, color, align);
    }

    /// [`text_aligned`](Self::text_aligned) in a specific font. Alignment is
    /// measured in the same font it draws in, which is the whole point.
    #[allow(clippy::too_many_arguments)]
    pub fn text_aligned_in(
        &mut self,
        font: FontId,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        let offset = if align == TextAlign::Left {
            0.0
        } else {
            // Through `measure_text_in`, not the atlas directly: alignment has
            // to account for `tracking` or a centred run drifts left by half
            // the spacing it actually draws with.
            let (w, _) = self.measure_text_in(font, text, size);
            match align {
                TextAlign::Center => -w / 2.0,
                TextAlign::Right => -w,
                TextAlign::Left => unreachable!(),
            }
        };
        // SAFETY-adjacent: re-resolve rather than holding `atlas` across the
        // `&mut self` call below.
        let atlas = self.font_atlas(font) as *const FontAtlas;
        self.text_with_font(x + offset, y, text, size, color, unsafe { &*atlas });
    }

    pub fn text_spans(&mut self, x: f32, y: f32, spans: &[(&str, Color)], size: f32) {
        let ptr = self.atlas;
        assert!(!ptr.is_null(), "Canvas font atlas not initialized");
        let atlas = unsafe { &*ptr };
        self.text_spans_with_font(x, y, spans, size, atlas);
    }

    pub fn text_spans_with_font(
        &mut self,
        x: f32,
        y: f32,
        spans: &[(&str, Color)],
        size: f32,
        atlas: &FontAtlas,
    ) {
        self.set_font(atlas.id().0);
        let scale = size / FONT_SIZE;
        let tracking = self.tracking;
        // `y` is the line box's top, as in `text_with_font`.
        let baseline = atlas.baseline_below_top(y, size);
        let mut cursor_x = x;

        for &(span_text, span_color) in spans {
            let c = span_color.to_array();
            for ch in span_text.chars() {
                let idx = ch as usize;
                if idx >= 128 {
                    continue;
                }
                let entry = match atlas.glyphs[idx] {
                    Some(e) => e,
                    None => continue,
                };

                if entry.width_px > 0.0 {
                    let gx = cursor_x + entry.x_offset * scale;
                    let gy = baseline + entry.y_offset * scale;
                    let gw = entry.width_px * scale;
                    let gh = entry.height_px * scale;

                    let [x0, y0] = screen_to_ndc(gx, gy, self.screen_size);
                    let [x1, y1] = screen_to_ndc(gx + gw, gy + gh, self.screen_size);

                    let v0 = CanvasVertex {
                        position: [x0, y0],
                        color: c,
                        uv: [entry.u0, entry.v1],
                    };
                    let v1 = CanvasVertex {
                        position: [x1, y0],
                        color: c,
                        uv: [entry.u1, entry.v1],
                    };
                    let v2 = CanvasVertex {
                        position: [x1, y1],
                        color: c,
                        uv: [entry.u1, entry.v0],
                    };
                    let v3 = CanvasVertex {
                        position: [x0, y1],
                        color: c,
                        uv: [entry.u0, entry.v0],
                    };
                    self.verts.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
                }

                cursor_x += entry.advance * scale + tracking;
            }
        }
    }

    pub fn image(&mut self, texture: TextureId, x: f32, y: f32, w: f32, h: f32) {
        self.image_colored(texture, x, y, w, h, Color::WHITE);
    }

    pub fn image_colored(
        &mut self,
        texture: TextureId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Color,
    ) {
        self.image_region(texture, x, y, w, h, [0.0, 0.0, 1.0, 1.0], color);
    }

    pub fn image_region(
        &mut self,
        texture: TextureId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv_rect: [f32; 4],
        color: Color,
    ) {
        self.set_texture(DrawTexture::Texture(texture.0));

        let [x0, y0] = screen_to_ndc(x, y, self.screen_size);
        let [x1, y1] = screen_to_ndc(x + w, y + h, self.screen_size);

        let c = color.to_array();
        let [u0, v0, uw, vh] = uv_rect;
        let u1 = u0 + uw;
        let v_bottom = v0 + vh;

        let bottom_left = CanvasVertex {
            position: [x0, y0],
            color: c,
            uv: [u0, v_bottom],
        };
        let bottom_right = CanvasVertex {
            position: [x1, y0],
            color: c,
            uv: [u1, v_bottom],
        };
        let top_right = CanvasVertex {
            position: [x1, y1],
            color: c,
            uv: [u1, v0],
        };
        let top_left = CanvasVertex {
            position: [x0, y1],
            color: c,
            uv: [u0, v0],
        };
        self.verts.extend_from_slice(&[
            bottom_left,
            top_right,
            bottom_right,
            bottom_left,
            top_left,
            top_right,
        ]);
    }

    pub fn text_spans_aligned(
        &mut self,
        x: f32,
        y: f32,
        spans: &[(&str, Color)],
        size: f32,
        align: TextAlign,
    ) {
        self.text_spans_aligned_in(FontId::DEFAULT, x, y, spans, size, align);
    }

    /// [`text_spans_aligned`](Self::text_spans_aligned) in a specific font.
    pub fn text_spans_aligned_in(
        &mut self,
        font: FontId,
        x: f32,
        y: f32,
        spans: &[(&str, Color)],
        size: f32,
        align: TextAlign,
    ) {
        let atlas = self.font_atlas(font);
        let offset = if align == TextAlign::Left {
            0.0
        } else {
            let total_w: f32 = spans
                .iter()
                .map(|(s, _)| atlas.measure_text(s, size).0)
                .sum();
            match align {
                TextAlign::Center => -total_w / 2.0,
                TextAlign::Right => -total_w,
                TextAlign::Left => unreachable!(),
            }
        };
        let atlas = self.font_atlas(font) as *const FontAtlas;
        self.text_spans_with_font(x + offset, y, spans, size, unsafe { &*atlas });
    }

    pub fn text_block(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        max_width: f32,
        align: TextAlign,
    ) {
        self.text_block_in(FontId::DEFAULT, x, y, text, size, color, max_width, align);
    }

    /// [`text_block`](Self::text_block) in a specific font — wrapping included,
    /// since where the lines break depends on which face measures them.
    #[allow(clippy::too_many_arguments)]
    pub fn text_block_in(
        &mut self,
        font: FontId,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        max_width: f32,
        align: TextAlign,
    ) {
        // ponytail: wraps on untracked widths, so a tracked block can overrun
        // `max_width` by its accumulated spacing. Tracking is for all-caps
        // chrome and wrapping is for prose; nothing authored is both. Give
        // `wrap_text` the tracking if that ever stops being true.
        let lines = wrap_text(text, size, max_width, self.font_atlas(font));
        self.text_block_lines_in(font, x, y, &lines, size, color, align);
    }

    pub(crate) fn text_block_lines(
        &mut self,
        x: f32,
        y: f32,
        lines: &[String],
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        self.text_block_lines_in(FontId::DEFAULT, x, y, lines, size, color, align);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_block_lines_in(
        &mut self,
        font: FontId,
        x: f32,
        y: f32,
        lines: &[String],
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        // `y` is the first line box's top; each subsequent line is one line
        // box lower, so the block occupies exactly `lines.len() * lh`.
        let lh = self.font_atlas(font).line_height(size);
        for (i, line) in lines.iter().enumerate() {
            let ly = y - (i as f32) * lh;
            self.text_aligned_in(font, x, ly, line, size, color, align);
        }
    }

    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        let (w, h) = self.atlas().measure_text(text, size);
        (w + self.tracking_width(text), h)
    }

    pub fn line_height(&self, size: f32) -> f32 {
        self.atlas().line_height(size)
    }
}

pub fn screen_to_ndc(x: f32, y: f32, screen_size: (u32, u32)) -> [f32; 2] {
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    [x / hw, y / hh]
}

pub fn wrap_text(text: &str, size: f32, max_width: f32, atlas: &FontAtlas) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let words: Vec<&str> = raw_line.split(' ').collect();
        let mut current = String::new();
        let mut current_w: f32 = 0.0;
        let space_w = atlas.measure_text(" ", size).0;
        for word in &words {
            let word_w = atlas.measure_text(word, size).0;
            if current.is_empty() {
                if word_w > max_width {
                    lines.push((*word).to_string());
                } else {
                    current = (*word).to_string();
                    current_w = word_w;
                }
            } else if current_w + space_w + word_w <= max_width {
                current.push(' ');
                current.push_str(word);
                current_w += space_w + word_w;
            } else {
                lines.push(current);
                if word_w > max_width {
                    lines.push((*word).to_string());
                    current = String::new();
                    current_w = 0.0;
                } else {
                    current = (*word).to_string();
                    current_w = word_w;
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(crate) fn draw_fps(canvas: &mut Canvas, fps: f32) {
    let screen_size = canvas.screen_size();
    let text = format!("{}", fps.round() as u32);
    let size = 16.0;
    let (text_w, _) = canvas.measure_text(&text, size);
    let bg_w = text_w + 8.0;
    let bg_h = size + 8.0;
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    canvas.rect(
        -hw + 4.0,
        hh - 4.0 - bg_h,
        bg_w,
        bg_h,
        Color::from_rgba8(0, 0, 0, 160),
    );
    canvas.text(
        -hw + 8.0,
        hh - 8.0,
        &text,
        size,
        Color::from_rgba8(0, 255, 0, 255),
    );
}

pub(crate) fn pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    font_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("canvas_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("canvas.wgsl").into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("canvas_pipeline_layout"),
        bind_group_layouts: &[font_bgl],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("canvas_pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[CanvasVertex::desc()],
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
    })
}

pub(crate) fn vertex_buffer(device: &wgpu::Device, vertex_capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("canvas_vertex_buffer"),
        size: (vertex_capacity * std::mem::size_of::<CanvasVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn required_vertex_buffer_capacity(current_capacity: usize, required_vertices: usize) -> usize {
    if required_vertices <= current_capacity {
        return current_capacity;
    }

    required_vertices
        .max(current_capacity.max(MAX_CANVAS_VERTICES))
        .checked_next_power_of_two()
        .unwrap_or(required_vertices)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_pass<'a, F>(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &mut wgpu::Buffer,
    vertex_capacity: &mut usize,
    queue: &wgpu::Queue,
    canvases: &mut [Canvas],
    fonts: &[FontAtlas],
    // When a fixed canvas is letterboxed inside a larger window, this is the
    // on-screen rect (physical px) the canvas draws into, so crisp UI text lines
    // up with the scaled sprite layer. `None` fills the whole `view`.
    viewport: Option<(f32, f32, f32, f32)>,
    texture_bind_group: F,
) where
    F: Fn(usize) -> Option<&'a wgpu::BindGroup>,
{
    for canvas in canvases.iter_mut() {
        canvas.finalize();
    }

    let verts: Vec<CanvasVertex> = canvases
        .iter()
        .flat_map(|c| c.verts.iter().copied())
        .collect();
    if verts.is_empty() {
        return;
    }
    if verts.len() > *vertex_capacity {
        *vertex_capacity = required_vertex_buffer_capacity(*vertex_capacity, verts.len());
        *vertex_buffer = self::vertex_buffer(device, *vertex_capacity);
    }
    queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&verts));

    let mut global_segments: Vec<(usize, usize, Option<[u32; 4]>, DrawTexture)> = Vec::new();
    let mut offset = 0usize;
    for canvas in canvases.iter() {
        if canvas.segments.is_empty() {
            if !canvas.verts.is_empty() {
                global_segments.push((offset, canvas.verts.len(), None, DrawTexture::Font(0)));
            }
        } else {
            for seg in &canvas.segments {
                global_segments.push((offset + seg.start, seg.count, seg.scissor, seg.texture));
            }
        }
        offset += canvas.verts.len();
    }

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("canvas_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    if let Some((vx, vy, vw, vh)) = viewport {
        if vw > 0.0 && vh > 0.0 {
            pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
        }
    }
    pass.set_bind_group(0, &fonts[0].bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));

    let needs_per_segment = global_segments
        .iter()
        .any(|(_, _, s, texture)| s.is_some() || *texture != DrawTexture::Font(0));

    if needs_per_segment {
        let surface_w = canvases.first().map(|c| c.screen_size.0).unwrap_or(1);
        let surface_h = canvases.first().map(|c| c.screen_size.1).unwrap_or(1);
        let mut bound_texture = DrawTexture::Font(0);

        for (start, count, scissor, texture) in &global_segments {
            if *count == 0 {
                continue;
            }
            if *texture != bound_texture {
                match *texture {
                    DrawTexture::Font(font_id) => {
                        if let Some(atlas) = fonts.get(font_id) {
                            pass.set_bind_group(0, &atlas.bind_group, &[]);
                        }
                    }
                    DrawTexture::Texture(texture_id) => {
                        if let Some(bind_group) = texture_bind_group(texture_id) {
                            pass.set_bind_group(0, bind_group, &[]);
                        }
                    }
                }
                bound_texture = *texture;
            }
            if let Some([sx, sy, sw, sh]) = scissor {
                if *sw == 0 || *sh == 0 {
                    continue;
                }
                pass.set_scissor_rect(*sx, *sy, *sw, *sh);
            } else {
                pass.set_scissor_rect(0, 0, surface_w, surface_h);
            }
            pass.draw(*start as u32..(*start + *count) as u32, 0..1);
        }
    } else {
        pass.draw(0..verts.len() as u32, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_widens_a_run_by_one_gap_per_glyph_gap() {
        // The arithmetic every text path shares: drawing advances the cursor
        // by `tracking` per glyph, so measuring and alignment must add
        // exactly one gap *between* glyphs — a trailing gap would push
        // right-aligned text off its anchor and centred text half a gap left.
        let w = |text: &str, tracking: f32| Canvas::tracking_width_of(text, tracking);

        // No tracking authored: every existing scene measures as before.
        assert_eq!(w("FORMULA R", 0.0), 0.0);
        // Nine glyphs, eight gaps.
        assert_eq!(w("FORMULA R", 6.0), 48.0);
        // A single glyph and an empty run have no gaps at all.
        assert_eq!(w("X", 6.0), 0.0);
        assert_eq!(w("", 6.0), 0.0);
        // Non-ASCII is skipped by the draw loop, so it must not be counted
        // here either — otherwise the measure is wider than the paint.
        assert_eq!(w("A\u{2022}B", 4.0), w("AB", 4.0));
    }
}
