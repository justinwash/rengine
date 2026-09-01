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
    /// Multiplier applied to every text size this canvas draws or measures —
    /// the accessibility knob a host exposes as "text size".
    ///
    /// Canvas state for exactly the reason `tracking` above is, only more so:
    /// a scale that reached drawing but not measuring would give every
    /// content-sized node a box the wrong size for the glyphs inside it, and
    /// every centred run an offset computed at a size it is not painted at.
    ///
    /// Applied at the **atlas boundary**, never at an entry point that
    /// delegates to another one, so a size is scaled exactly once however deep
    /// the call nests. `text_aligned_in` therefore passes its raw `size` to
    /// both `measure_text_in` and `text_with_font`, and each scales its own
    /// copy.
    ///
    /// Layout dimensions are *not* font sizes and deliberately do not ride
    /// this: a wrap width stays where the author put it, so larger text wraps
    /// sooner rather than the paragraph growing sideways off the panel.
    text_scale: f32,
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
            text_scale: 1.0,
        }
    }

    /// Set the multiplier applied to every text size drawn or measured on this
    /// canvas. Returns the previous value so a caller can restore it.
    ///
    /// `1.0` is "as authored" and is what every scene is written against, so a
    /// host that never calls this behaves exactly as it did before the knob
    /// existed.
    pub fn set_text_scale(&mut self, scale: f32) -> f32 {
        std::mem::replace(&mut self.text_scale, scale.max(0.01))
    }

    /// The multiplier currently applied to every text size.
    pub fn text_scale(&self) -> f32 {
        self.text_scale
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
        let size = size * self.text_scale;
        let (w, h) = match self.font_atlas_opt(font) {
            Some(atlas) => atlas.measure_text(text, size),
            None => crate::text::measure_builtin_text(text, size),
        };
        (w + Self::tracking_width_of(text, tracking), h)
    }

    /// The atlas for `font`, or `None` when no atlas is bound yet.
    ///
    /// Measuring is not drawing: content sizing runs over a `&Canvas` and only
    /// needs advances and a line box, both of which the builtin face knows
    /// without a GPU atlas. Drawing still asserts — a glyph really does need
    /// one — but a layout pass must not depend on having begun a frame.
    fn font_atlas_opt(&self, font: FontId) -> Option<&FontAtlas> {
        if self.atlas.is_null() {
            return None;
        }
        Some(self.font_atlas(font))
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

    /// Whether this canvas can emit glyphs at all.
    ///
    /// A layout-only pass (the content-sizing tests, and any headless caller
    /// that wants rects without pixels) runs on a canvas with no atlas bound.
    /// Measuring works there — the builtin metrics need no GPU — so text draw
    /// calls no-op instead of panicking, and layout still resolves fully.
    fn can_draw_text(&self) -> bool {
        !self.atlas.is_null()
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
        let (w, h) = self.font_atlas(font).measure_text(text, size * self.text_scale);
        (w + self.tracking_width(text), h)
    }

    /// [`line_height`](Self::line_height) in a specific font.
    pub fn line_height_in(&self, font: FontId, size: f32) -> f32 {
        let size = size * self.text_scale;
        match self.font_atlas_opt(font) {
            Some(atlas) => atlas.line_height(size),
            // Measuring without a bound atlas: the builtin face's own line box.
            None => crate::text::measure_builtin_text("", size).1,
        }
    }

    /// A canvas with no font table, for host-crate tests that need to measure
    /// or draw without a renderer.
    ///
    /// `new` stays crate-private because a real canvas is the renderer's to
    /// hand out; this is the deliberate test-only door, and it says so in the
    /// name. Text measured through it uses the engine's default metrics, which
    /// is all a layout assertion needs.
    pub fn for_test(screen_size: (u32, u32)) -> Self {
        Self::new(screen_size, std::ptr::null())
    }

    /// Every vertex drawn so far, for tests that need to assert on what
    /// actually came out rather than on the inputs that went in.
    ///
    /// A colour is the one thing a layout assertion cannot reach: an
    /// unresolved `{chalk}` still produces a correctly-positioned rect, just a
    /// white one, so "did the bindings resolve" is only answerable here.
    pub fn vertices(&self) -> &[CanvasVertex] {
        &self.verts
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

    /// Fill an arbitrary simple polygon.
    ///
    /// "Arbitrary" is the point: a triangle fan only fills convex shapes, and
    /// the interesting set dressing — an L-shaped grandstand, a kidney of
    /// gravel, a treeline — is concave. This ear-clips instead, which is
    /// correct for any simple (non-self-intersecting) polygon.
    ///
    /// Points are in canvas pixels, in either winding order.
    pub fn polygon(&mut self, points: &[(f32, f32)], color: Color) {
        if points.len() < 3 {
            return;
        }
        self.set_font(0);
        let c = color.to_array();
        let uv = WHITE_UV;
        let mut push = |a: (f32, f32), b: (f32, f32), d: (f32, f32)| {
            for (x, y) in [a, b, d] {
                let position = screen_to_ndc(x, y, self.screen_size);
                self.verts.push(CanvasVertex {
                    position,
                    color: c,
                    uv,
                });
            }
        };
        for [a, b, c] in triangulate(points) {
            push(a, b, c);
        }
    }
}

/// Twice the signed area of a polygon. Positive is counter-clockwise.
fn signed_area2(points: &[(f32, f32)]) -> f32 {
    let mut acc = 0.0;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        acc += a.0 * b.1 - b.0 * a.1;
    }
    acc
}

fn cross(o: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Whether `p` lies inside triangle `abc` (edges count as inside).
fn in_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let d1 = cross(a, b, p);
    let d2 = cross(b, c, p);
    let d3 = cross(c, a, p);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// Ear-clipping triangulation of a simple polygon.
///
/// Handles concave shapes, which is the whole reason this exists rather than a
/// fan. Self-intersecting input has no correct triangulation; rather than loop
/// forever this bails out by clipping the current vertex regardless, so a
/// malformed shape draws *something* instead of hanging the frame.
fn triangulate(points: &[(f32, f32)]) -> Vec<[(f32, f32); 3]> {
    let mut idx: Vec<usize> = (0..points.len()).collect();
    // Work counter-clockwise so "convex" has one meaning throughout.
    if signed_area2(points) < 0.0 {
        idx.reverse();
    }
    let mut out = Vec::with_capacity(points.len().saturating_sub(2));
    let mut guard = 0;
    while idx.len() > 3 {
        let n = idx.len();
        let mut clipped = false;
        for i in 0..n {
            let (ia, ib, ic) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
            let (a, b, c) = (points[ia], points[ib], points[ic]);
            if cross(a, b, c) <= 0.0 {
                continue; // reflex vertex: never an ear
            }
            let blocked = idx
                .iter()
                .filter(|&&j| j != ia && j != ib && j != ic)
                .any(|&j| in_triangle(points[j], a, b, c));
            if blocked {
                continue;
            }
            out.push([a, b, c]);
            idx.remove(i);
            clipped = true;
            break;
        }
        guard += 1;
        if !clipped || guard > points.len() * 2 {
            // Degenerate or self-intersecting: take the fan that remains
            // rather than spinning. Better a wrong picture than a hung frame.
            break;
        }
    }
    for i in 1..idx.len().saturating_sub(1) {
        out.push([points[idx[0]], points[idx[i]], points[idx[i + 1]]]);
    }
    out
}

impl Canvas {
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
        let size = size * self.text_scale;
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
        if !self.can_draw_text() {
            return;
        }
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
        let size = size * self.text_scale;
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

    /// An image rotated about its own centre.
    ///
    /// The canvas could only ever blit axis-aligned quads, which is fine for a
    /// HUD and wrong for anything that follows world geometry — a trackside
    /// building beside a curving road, a sign facing its corner. Without this
    /// the only way to draw such a thing was to hand-code its shape in Rust,
    /// which puts art outside the editor where nobody can author it.
    ///
    /// `radians` turns counter-clockwise, matching the sprite renderer's
    /// convention so a scene reads the same through either path.
    pub fn image_region_rotated(
        &mut self,
        texture: TextureId,
        cx: f32,
        cy: f32,
        w: f32,
        h: f32,
        uv_rect: [f32; 4],
        color: Color,
        radians: f32,
    ) {
        self.set_texture(DrawTexture::Texture(texture.0));

        let (sin, cos) = radians.sin_cos();
        let (hw, hh) = (w * 0.5, h * 0.5);
        // The four corners in the quad's own space, then rotated into place.
        // Screen space is converted last so the rotation happens in pixels and
        // stays square whatever the viewport's aspect.
        let corner = |dx: f32, dy: f32| {
            let (rx, ry) = (dx * cos - dy * sin, dx * sin + dy * cos);
            screen_to_ndc(cx + rx, cy + ry, self.screen_size)
        };

        let c = color.to_array();
        let [u0, v0, uw, vh] = uv_rect;
        let (u1, v_bottom) = (u0 + uw, v0 + vh);

        let bottom_left = CanvasVertex {
            position: corner(-hw, -hh),
            color: c,
            uv: [u0, v_bottom],
        };
        let bottom_right = CanvasVertex {
            position: corner(hw, -hh),
            color: c,
            uv: [u1, v_bottom],
        };
        let top_right = CanvasVertex {
            position: corner(hw, hh),
            color: c,
            uv: [u1, v0],
        };
        let top_left = CanvasVertex {
            position: corner(-hw, hh),
            color: c,
            uv: [u0, v0],
        };
        self.verts.extend_from_slice(&[
            bottom_left,
            bottom_right,
            top_right,
            bottom_left,
            top_right,
            top_left,
        ]);
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
        if !self.can_draw_text() {
            return;
        }
        let atlas = self.font_atlas(font);
        let offset = if align == TextAlign::Left {
            0.0
        } else {
            // Only the measurement scales here: `text_spans_with_font` below
            // scales its own copy of the raw `size`.
            let total_w: f32 = spans
                .iter()
                .map(|(s, _)| atlas.measure_text(s, size * self.text_scale).0)
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
        self.text_block_leaded_in(font, x, y, text, size, color, max_width, align, 1.0);
    }

    /// [`text_block_in`](Self::text_block_in) with a line-height multiplier —
    /// CSS `line-height:1.35`, which the mockups author on every prose block.
    ///
    /// `leading` scales the step between lines only; the first line still sits
    /// at `y`, so a block's ink starts where its rect does regardless.
    #[allow(clippy::too_many_arguments)]
    pub fn text_block_leaded_in(
        &mut self,
        font: FontId,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        max_width: f32,
        align: TextAlign,
        leading: f32,
    ) {
        if !self.can_draw_text() {
            return;
        }
        // ponytail: wraps on untracked widths, so a tracked block can overrun
        // `max_width` by its accumulated spacing. Tracking is for all-caps
        // chrome and wrapping is for prose; nothing authored is both. Give
        // `wrap_text` the tracking if that ever stops being true.
        // The wrap runs at the *painted* size against the author's own
        // `max_width`, so bigger text breaks sooner instead of overrunning the
        // panel. `text_block_lines_leaded_in` gets the raw size and scales its
        // own copy.
        let lines = wrap_text(text, size * self.text_scale, max_width, self.font_atlas(font));
        self.text_block_lines_leaded_in(font, x, y, &lines, size, color, align, leading);
    }

    /// How much room a wrapped block needs: the widest line it breaks into, and
    /// the height those lines occupy at `leading`.
    ///
    /// The measuring half of [`text_block_leaded_in`](Self::text_block_leaded_in)
    /// — same wrap, same metrics — so a content-sized block cannot size to
    /// something other than what it paints.
    pub fn measure_text_block_in(
        &self,
        font: FontId,
        text: &str,
        size: f32,
        max_width: f32,
        leading: f32,
    ) -> (f32, f32) {
        // Same measuring function the wrap and the widest-line scan both use,
        // so they cannot disagree about where the breaks fall.
        // Scaled in the closure only: `line_height_in` below scales its own.
        let measure = |run: &str| {
            let size = size * self.text_scale;
            match self.font_atlas_opt(font) {
                Some(atlas) => atlas.measure_text(run, size).0,
                None => crate::text::measure_builtin_text(run, size).0,
            }
        };
        let lines = wrap_text_measured(text, max_width, &measure);
        let widest = lines
            .iter()
            .map(|line| measure(line))
            .fold(0.0_f32, f32::max);
        (
            widest,
            Self::block_height(self.line_height_in(font, size), lines.len(), leading),
        )
    }

    /// The height `n` stacked line boxes occupy at `leading`.
    ///
    /// The last line contributes a full box, not a leaded one — extra leading
    /// is space *between* lines, so a block is `lh + (n-1) * lh * leading`.
    /// Getting this wrong pads every prose block with a phantom trailing gap.
    fn block_height(line_h: f32, lines: usize, leading: f32) -> f32 {
        match lines {
            0 => 0.0,
            n => line_h + (n - 1) as f32 * line_h * leading,
        }
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
        self.text_block_lines_leaded_in(font, x, y, lines, size, color, align, 1.0);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_block_lines_leaded_in(
        &mut self,
        font: FontId,
        x: f32,
        y: f32,
        lines: &[String],
        size: f32,
        color: Color,
        align: TextAlign,
        leading: f32,
    ) {
        if !self.can_draw_text() {
            return;
        }
        // `y` is the first line box's top; each subsequent line is one leaded
        // step lower, so the block occupies exactly `block_height`.
        // The step scales; `text_aligned_in` per line scales its own copy.
        let step = self.font_atlas(font).line_height(size * self.text_scale) * leading;
        for (i, line) in lines.iter().enumerate() {
            let ly = y - (i as f32) * step;
            self.text_aligned_in(font, x, ly, line, size, color, align);
        }
    }

    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        let (w, h) = self.atlas().measure_text(text, size * self.text_scale);
        (w + self.tracking_width(text), h)
    }

    pub fn line_height(&self, size: f32) -> f32 {
        self.atlas().line_height(size * self.text_scale)
    }
}

pub fn screen_to_ndc(x: f32, y: f32, screen_size: (u32, u32)) -> [f32; 2] {
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    [x / hw, y / hh]
}

pub fn wrap_text(text: &str, size: f32, max_width: f32, atlas: &FontAtlas) -> Vec<String> {
    wrap_text_measured(text, max_width, |run| atlas.measure_text(run, size).0)
}

/// [`wrap_text`](wrap_text) against any measuring function.
///
/// The wrap algorithm is the same whether the widths come from a live atlas or
/// the builtin metrics — content sizing runs before a frame begins and has no
/// atlas, and a block that wrapped differently when measured than when drawn
/// would size to the wrong height.
pub fn wrap_text_measured(
    text: &str,
    max_width: f32,
    measure: impl Fn(&str) -> f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let words: Vec<&str> = raw_line.split(' ').collect();
        let mut current = String::new();
        let mut current_w: f32 = 0.0;
        let space_w = measure(" ");
        for word in &words {
            let word_w = measure(word);
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
        let logical_size = (surface_w as f32, surface_h as f32);
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
                let [sx, sy, sw, sh] = scale_scissor(
                    [*sx, *sy, *sw, *sh],
                    logical_size,
                    viewport,
                );
                if sw == 0 || sh == 0 {
                    continue;
                }
                pass.set_scissor_rect(sx, sy, sw, sh);
            } else {
                let [x, y, w, h] = scale_scissor(
                    [0, 0, surface_w, surface_h],
                    logical_size,
                    viewport,
                );
                pass.set_scissor_rect(x, y, w, h);
            }
            pass.draw(*start as u32..(*start + *count) as u32, 0..1);
        }
    } else {
        pass.draw(0..verts.len() as u32, 0..1);
    }
}

fn scale_scissor(
    rect: [u32; 4],
    logical_size: (f32, f32),
    viewport: Option<(f32, f32, f32, f32)>,
) -> [u32; 4] {
    let Some((vx, vy, vw, vh)) = viewport else {
        return rect;
    };
    let sx = vw / logical_size.0.max(1.0);
    let sy = vh / logical_size.1.max(1.0);
    let x = (vx + rect[0] as f32 * sx).round().max(vx);
    let y = (vy + rect[1] as f32 * sy).round().max(vy);
    let right = (vx + (rect[0] + rect[2]) as f32 * sx).round().min(vx + vw);
    let bottom = (vy + (rect[1] + rect[3]) as f32 * sy).round().min(vy + vh);
    [
        x as u32,
        y as u32,
        (right - x).max(0.0) as u32,
        (bottom - y).max(0.0) as u32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_scissor_maps_logical_clip_into_a_physical_viewport() {
        assert_eq!(
            scale_scissor([40, 20, 200, 100], (640.0, 360.0), Some((50.0, 25.0, 1280.0, 720.0))),
            [130, 65, 400, 200]
        );
    }

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

    /// `text_scale` must land on every measuring path **exactly once**.
    ///
    /// Double-application is the whole risk of putting the scale at the atlas
    /// boundary: the entry points nest (`measure_text_block_in` calls
    /// `line_height_in`, `text_block_leaded_in` calls `wrap_text` *and*
    /// `text_block_lines_leaded_in`), so a scale added to both an outer
    /// function and the inner one it delegates to would quietly square. The
    /// test is the ratio: measuring at 2x must equal measuring at 1x doubled,
    /// not quadrupled.
    ///
    /// Runs on the builtin metrics — no atlas, no GPU — which is exactly the
    /// path content sizing takes.
    #[test]
    fn the_text_scale_applies_once_on_every_measuring_path() {
        let text = "MANAGE TYRES";
        let mut one = Canvas::for_test((800, 600));
        let mut two = Canvas::for_test((800, 600));
        assert_eq!(one.set_text_scale(1.0), 1.0, "default scale is 1.0");
        two.set_text_scale(2.0);

        let close = |a: f32, b: f32, what: &str| {
            assert!(
                (a - b).abs() < 0.01,
                "{what}: 2x gave {a}, expected {b} (double-scaled would be ~{})",
                b * 2.0
            );
        };

        // Every entry point a scene's content sizing can reach. Not
        // `measure_text_in`: it goes through `font_atlas`, which asserts, so it
        // is unreachable on an atlas-less canvas — it is only ever called
        // behind `can_draw_text`.
        close(
            two.measure_text_tracked(FontId::DEFAULT, text, 10.0, 0.0).0,
            one.measure_text_tracked(FontId::DEFAULT, text, 10.0, 0.0).0 * 2.0,
            "measure_text_tracked",
        );
        close(
            two.line_height_in(FontId::DEFAULT, 10.0),
            one.line_height_in(FontId::DEFAULT, 10.0) * 2.0,
            "line_height_in",
        );
        // The nested one: its height comes from `line_height_in`, its width
        // from a closure of its own.
        close(
            two.measure_text_block_in(FontId::DEFAULT, text, 10.0, 1_000.0, 1.0).1,
            one.measure_text_block_in(FontId::DEFAULT, text, 10.0, 1_000.0, 1.0).1 * 2.0,
            "measure_text_block_in height",
        );

        // A scale of 1.0 must be bit-identical to no scale at all, or every
        // recorded pixel baseline moves the day the knob lands.
        assert_eq!(
            one.measure_text_tracked(FontId::DEFAULT, text, 10.0, 0.0),
            Canvas::for_test((800, 600)).measure_text_tracked(FontId::DEFAULT, text, 10.0, 0.0),
        );

        // A wrap width is a layout dimension, not a font size: bigger text
        // must break into MORE lines inside the same box, never overrun it.
        // Wide enough that 1x fits several words per line — a box so narrow
        // that both scales collapse to one word per line proves nothing, since
        // `wrap_text` never breaks inside a word.
        let box_w = 220.0;
        let prose = "the quick brown fox jumps over the lazy dog and keeps running";
        let lines_at = |c: &Canvas| {
            c.measure_text_block_in(FontId::DEFAULT, prose, 10.0, box_w, 1.0).1
                / c.line_height_in(FontId::DEFAULT, 10.0)
        };
        assert!(
            lines_at(&two) > lines_at(&one),
            "2x text wrapped into {} lines against {} at 1x — the wrap width scaled with the text",
            lines_at(&two),
            lines_at(&one),
        );
    }
}

#[cfg(test)]
mod rotated_image_tests {
    use super::*;

    fn quad(canvas: &Canvas) -> Vec<[f32; 2]> {
        canvas.vertices().iter().map(|v| v.position).collect()
    }

    /// A zero rotation must land exactly where the axis-aligned blit does, or
    /// every existing authored image shifts the moment rotation is available.
    #[test]
    fn no_rotation_matches_the_plain_blit() {
        let uv = [0.0, 0.0, 1.0, 1.0];
        let mut plain = Canvas::for_test((200, 200));
        plain.image_region(TextureId(0), 40.0, 20.0, 60.0, 30.0, uv, Color::WHITE);

        let mut spun = Canvas::for_test((200, 200));
        spun.image_region_rotated(
            TextureId(0),
            40.0 + 30.0,
            20.0 + 15.0,
            60.0,
            30.0,
            uv,
            Color::WHITE,
            0.0,
        );

        // The four *corners*, not the vertex order: both paths emit two
        // triangles covering the same quad but wind them differently, and
        // asserting the emit order would pin an implementation detail rather
        // than the thing that shows on screen.
        let corners = |c: &Canvas| {
            let mut pts: Vec<[i32; 2]> = quad(c)
                .into_iter()
                .map(|p| [(p[0] * 10_000.0) as i32, (p[1] * 10_000.0) as i32])
                .collect();
            pts.sort();
            pts.dedup();
            pts
        };
        assert_eq!(
            corners(&plain),
            corners(&spun),
            "an unrotated blit must cover exactly the same quad"
        );
    }

    /// A quarter turn swaps the quad's width and height on screen. This is the
    /// property trackside art depends on: a building drawn along a road running
    /// north must be as tall as the road is long.
    #[test]
    fn a_quarter_turn_swaps_the_extents() {
        let uv = [0.0, 0.0, 1.0, 1.0];
        let mut canvas = Canvas::for_test((200, 200));
        canvas.image_region_rotated(
            TextureId(0),
            0.0,
            0.0,
            80.0,
            20.0,
            uv,
            Color::WHITE,
            std::f32::consts::FRAC_PI_2,
        );
        let pts = quad(&canvas);
        let xs: Vec<f32> = pts.iter().map(|p| p[0]).collect();
        let ys: Vec<f32> = pts.iter().map(|p| p[1]).collect();
        let span = |v: &[f32]| {
            v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min)
        };
        // NDC is scaled by the viewport, so compare the ratio rather than
        // absolute pixels: an 80x20 quad turned 90 degrees is 20 wide, 80 tall.
        assert!(
            span(&ys) > span(&xs) * 3.0,
            "a quarter turn did not stand the quad up: {} wide, {} tall",
            span(&xs),
            span(&ys)
        );
    }

    /// Rotation happens about the centre, so the quad stays put.
    #[test]
    fn rotation_pivots_on_the_centre() {
        let uv = [0.0, 0.0, 1.0, 1.0];
        let mut canvas = Canvas::for_test((200, 200));
        canvas.image_region_rotated(TextureId(0), 0.0, 0.0, 40.0, 40.0, uv, Color::WHITE, 0.7);
        let pts = quad(&canvas);
        let cx: f32 = pts.iter().map(|p| p[0]).sum::<f32>() / pts.len() as f32;
        let cy: f32 = pts.iter().map(|p| p[1]).sum::<f32>() / pts.len() as f32;
        assert!(
            cx.abs() < 1e-4 && cy.abs() < 1e-4,
            "the quad drifted off its pivot: ({cx}, {cy})"
        );
    }
}

#[cfg(test)]
mod polygon_tests {
    use super::*;

    /// A convex polygon triangulates into exactly n-2 triangles, which is the
    /// count any correct triangulation of a simple polygon produces.
    #[test]
    fn a_convex_polygon_yields_the_expected_triangle_count() {
        for n in 3..=8 {
            let pts: Vec<(f32, f32)> = (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU;
                    (a.cos() * 50.0, a.sin() * 50.0)
                })
                .collect();
            assert_eq!(triangulate(&pts).len(), n - 2, "{n}-gon");
        }
    }

    /// The case a triangle fan gets wrong.
    ///
    /// A **U**, deliberately — my first attempt used an L, and an L happens to
    /// be visible in its entirety from every one of its corners, so a fan from
    /// vertex 0 fills it correctly and the test could not fail. Two mutations
    /// passed before that showed up. A U's opening is not visible from its
    /// bottom-left corner, so a fan from there covers the gap and ear clipping
    /// must not.
    #[test]
    fn a_concave_polygon_stays_inside_itself() {
        // A U, counter-clockwise: uprights at x 0..15 and 45..60, joined
        // across the bottom, open between (15,20) and (45,60).
        let u = [
            (0.0, 0.0),
            (60.0, 0.0),
            (60.0, 60.0),
            (45.0, 60.0),
            (45.0, 20.0),
            (15.0, 20.0),
            (15.0, 60.0),
            (0.0, 60.0),
        ];
        let tris = triangulate(&u);
        assert_eq!(tris.len(), 6, "an 8-gon is six triangles");

        for [a, b, c] in tris {
            let cx = (a.0 + b.0 + c.0) / 3.0;
            let cy = (a.1 + b.1 + c.1) / 3.0;
            let in_opening = (15.0..45.0).contains(&cx) && cy > 20.0;
            assert!(
                !in_opening,
                "a triangle spilled into the U's opening at ({cx}, {cy})"
            );
        }
    }

    /// Winding must not matter: the same shape given clockwise fills the same.
    #[test]
    fn winding_order_does_not_change_the_result() {
        let ccw = [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)];
        let mut cw = ccw;
        cw.reverse();
        assert_eq!(triangulate(&ccw).len(), triangulate(&cw).len());
    }

    /// Degenerate input draws nothing rather than panicking, and a
    /// self-intersecting shape must not hang the frame.
    #[test]
    fn bad_input_is_survivable() {
        assert!(triangulate(&[]).is_empty());
        assert!(triangulate(&[(0.0, 0.0), (1.0, 1.0)]).is_empty());
        // A bowtie has no valid simple triangulation; it must still terminate.
        let bowtie = [(0.0, 0.0), (40.0, 40.0), (40.0, 0.0), (0.0, 40.0)];
        let _ = triangulate(&bowtie);
    }

    /// The canvas emits three vertices per triangle.
    #[test]
    fn the_canvas_emits_a_triangle_list() {
        let mut canvas = Canvas::for_test((200, 200));
        canvas.polygon(&[(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)], Color::WHITE);
        assert_eq!(canvas.vertices().len(), 6, "a quad is two triangles");
    }
}
