use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::{AssetError, AssetPack, Color};
use crate::canvas::{Canvas, TextAlign};
use crate::renderer::{DrawParams, Frame};
use crate::text::FontId;
use crate::{TextureId, Vec2};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrefabSprite2DDef {
    pub asset: String,
    pub offset: [f32; 2],
    pub size: [f32; 2],
    #[serde(default = "default_color")]
    pub color: [f32; 4],
    #[serde(default)]
    pub uv_rect: Option<[f32; 4]>,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prefab2DDef {
    pub name: String,
    pub sprites: Vec<PrefabSprite2DDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneInstance2DDef {
    pub prefab: String,
    pub position: [f32; 2],
    #[serde(default = "default_scale")]
    pub scale: [f32; 2],
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Scene2DDef {
    #[serde(default)]
    pub prefabs: Vec<Prefab2DDef>,
    #[serde(default)]
    pub instances: Vec<SceneInstance2DDef>,
}

#[derive(Debug, Clone)]
pub struct PrefabSprite2D {
    pub texture: TextureId,
    pub offset: Vec2,
    pub size: Vec2,
    pub color: Color,
    pub uv_rect: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Debug, Clone)]
pub struct Prefab2D {
    pub name: String,
    pub sprites: Vec<PrefabSprite2D>,
}

#[derive(Debug, Clone)]
pub struct SceneInstance2D {
    pub prefab: String,
    pub position: Vec2,
    pub scale: Vec2,
    pub properties: HashMap<String, String>,
    sprites: Vec<PrefabSprite2D>,
}

impl SceneInstance2D {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(String::as_str)
    }

    pub fn property_bool(&self, name: &str) -> Option<bool> {
        self.property(name).and_then(parse_bool_property)
    }

    pub fn property_i64(&self, name: &str) -> Option<i64> {
        self.property(name)
            .and_then(|value| value.parse::<i64>().ok())
    }

    pub fn property_f32(&self, name: &str) -> Option<f32> {
        self.property(name)
            .and_then(|value| value.parse::<f32>().ok())
    }

    pub fn property_u64(&self, name: &str) -> Option<u64> {
        self.property(name)
            .and_then(|value| value.parse::<u64>().ok())
    }

    pub fn property_tags(&self, name: &str) -> Vec<&str> {
        self.property(name)
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.property_tags("tags")
            .into_iter()
            .any(|item| item == tag)
    }

    pub fn editor_node_id(&self) -> Option<u64> {
        self.property_u64("editor_node_id")
    }

    pub fn editor_parent_id(&self) -> Option<u64> {
        self.property_u64("editor_parent_id")
    }

    pub fn editor_visible(&self) -> Option<bool> {
        self.property_bool("editor_visible")
    }

    pub fn editor_name(&self) -> Option<&str> {
        self.property("editor_name")
    }

    pub fn script_path(&self) -> Option<&str> {
        self.property("script_path")
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// Authored script params: every `param_<name>` property with the prefix
    /// stripped. Empty when the node carries no params.
    pub fn script_params(&self) -> HashMap<String, String> {
        self.properties
            .iter()
            .filter_map(|(key, value)| {
                key.strip_prefix("param_")
                    .map(|name| (name.to_string(), value.clone()))
            })
            .collect()
    }

    /// Compiled sprite layers for this instance.
    ///
    /// Exposed to sibling scene modules (such as the runtime [`SceneWorld2D`])
    /// so live nodes can reuse the same render data the static scene draws.
    pub(crate) fn sprite_layers(&self) -> &[PrefabSprite2D] {
        &self.sprites
    }

    pub fn draw(&self, frame: &mut Frame) {
        self.draw_at(frame, 0.0);
    }

    /// Like [`draw`](Self::draw) but with an animation clock, so `ui_bob_*` /
    /// `ui_sway_*` node animations advance.
    pub fn draw_at(&self, frame: &mut Frame, time: f32) {
        for sprite in &self.sprites {
            frame.draw_sprite(
                DrawParams::new(
                    sprite.texture,
                    self.position + sprite.offset * self.scale,
                    sprite.size * self.scale,
                )
                .with_color(sprite.color)
                .with_uv_rect(sprite.uv_rect)
                .with_flip_x(sprite.flip_x)
                .with_flip_y(sprite.flip_y),
            );
        }
        self.draw_ui_primitive(frame, time);
    }

    /// Render an immediate-mode UI primitive (rect / gradient / bevel / text)
    /// described by this instance's `ui_*` properties, onto a canvas layer.
    ///
    /// This lets HUD/menu scenes be authored as plain scene data rather than
    /// hand-drawn in game code. Position is taken from the instance's transform
    /// (canvas units, y-up); colours are authored in sRGB display space.
    ///
    /// Recognised properties (all optional unless noted):
    /// - `ui`: `rect` | `gradient` | `bevel` | `text` | `circle` | `line`
    /// - `ui_layer`: canvas layer index (default 0; higher = on top)
    /// - `ui_anchor`: `center` (default) | `top`/`bottom`/`left`/`right` |
    ///   `top-left`/`top-right`/`bottom-left`/`bottom-right` — position is then an
    ///   offset from that screen anchor, so HUD nodes survive window resizes
    /// - `ui_origin_x`, `ui_origin_y`: which point of the node lands on the
    ///   anchor — `0` (default) its left/bottom edge, `0.5` its centre, `1` its
    ///   right/top edge. Needed to centre a node whose size is data-bound.
    /// - `ui_w`, `ui_h`: size, multiplied by the instance scale
    /// - `ui_w_frac`, `ui_h_frac`: size as a fraction of the viewport (overrides
    ///   `ui_w`/`ui_h`), e.g. `ui_w_frac: 1.0` spans the full width
    /// - `ui_color`: `"r,g,b,a"` sRGB for `rect`/`text`
    /// - `ui_color_bottom`, `ui_color_top`: gradient ends / bevel shadow+highlight
    /// - `ui_radius`: corner radius for `rect`
    /// - `ui_line_w`: edge thickness for `bevel`
    /// - `ui_border_w`, `ui_border_color`: an inset border on **any** kind,
    ///   with `ui_border_left`/`_right`/`_top`/`_bottom` overriding one side
    ///   (a divider rule is a one-sided border, not a separate line node).
    ///   Inset against an *authored* size, so a node stays the size it says it
    ///   is; **added** to a *measured* one (`ui_size: "content"`), which has no
    ///   stated size to inset against — CSS's `border-box` and `content-box`
    ///   respectively, each applied where it is the useful one
    /// - `ui_shadow_color`, `ui_shadow_x`, `ui_shadow_y`: a hard offset drop
    ///   shadow under **any** kind, `ui_shadow_y` positive = down (CSS sense)
    /// - `ui_text`, `ui_text_size`: text contents and size
    /// - `ui_tracking`: extra pixels after each glyph (CSS `letter-spacing`),
    ///   applied to drawing, measuring and alignment alike
    pub fn draw_ui_primitive(&self, frame: &mut Frame, time: f32) {
        if self.property("ui").is_none() {
            return;
        }
        let (sw, sh) = frame.canvas(0).screen_size();
        let screen = (-(sw as f32) / 2.0, -(sh as f32) / 2.0, sw as f32, sh as f32);
        draw_ui_node(
            frame,
            screen,
            self.position,
            self.scale,
            time,
            |n| self.property(n),
            self.sprites.first(),
        );
    }
}

/// Resolve a UI node's rect from its `ui_*` properties against a `reference`
/// rect — the viewport for root nodes, or the parent's resolved rect for
/// children. `reference` is `(x, y, w, h)` with `(x, y)` the bottom-left corner
/// in canvas coords (centred, y-up); anchors/fractions/stretch/animation are all
/// relative to it, so nesting and screen-edge layout share one model.
fn resolve_ui_rect(
    reference: (f32, f32, f32, f32),
    position: Vec2,
    scale: Vec2,
    time: f32,
    get: impl Fn(&str) -> Option<String>,
    pixel_grid: f32,
    content_size: Option<(f32, f32)>,
    flow_size: (Option<f32>, Option<f32>),
) -> (f32, f32, f32, f32) {
    let prop_f32 = |n: &str| ui_f32(&get, n);
    let prop_bool =
        |n: &str| matches!(get(n).as_deref().map(str::trim), Some("true" | "1" | "yes"));

    let (rx, ry, rw, rh) = reference;
    // `ui_size: "content"` (measured bottom-up by
    // `SceneWorld2D::measure_content_size` before this runs) overrides
    // literal/bound `ui_w`/`ui_h` — a node can be *either* sized by its
    // author or sized by its children, not both.
    //
    // Per axis, because "as wide as I say, as tall as my children need" is the
    // common panel: the mockups state a panel's width (640px) and let its rows
    // decide its height. `"content"` is both axes; `"content_h"` sizes only the
    // height, leaving `ui_w` to say the width (and `"content_w"` the reverse).
    // Without this a fixed-width panel had to have its height computed in the
    // host and passed back in as a binding — the hand-tuned constant this
    // overhaul exists to delete.
    let (content_w_axis, content_h_axis) = match get("ui_size").as_deref() {
        Some("content") => (true, true),
        Some("content_w") => (true, false),
        Some("content_h") => (false, true),
        _ => (false, false),
    };
    // `ui_grow` (E-A) wins on its own axis: the parent's flow already decided
    // this node's main-axis extent, and it is the whole meaning of the
    // property. `ui_size: "content"` still wins on the *other* axis, so a
    // growing row can size to its children vertically.
    let (grow_w, grow_h) = flow_size;
    let w_fixed = match grow_w.or_else(|| {
        content_size
            .filter(|_| content_w_axis)
            .map(|(w, _)| w * scale.x)
    }) {
        Some(w) => w,
        None => match prop_f32("ui_w_frac") {
            Some(f) => rw * f,
            None => prop_f32("ui_w").unwrap_or(0.0) * scale.x,
        },
    };
    let h_fixed = match grow_h.or_else(|| {
        content_size
            .filter(|_| content_h_axis)
            .map(|(_, h)| h * scale.y)
    }) {
        Some(h) => h,
        None => match prop_f32("ui_h_frac") {
            Some(f) => rh * f,
            None => prop_f32("ui_h").unwrap_or(0.0) * scale.y,
        },
    };
    // Named anchor (shorthand) or exact `ui_anchor_frac_x`/`_y` (0..1, Godot-style).
    let (mut ax, mut ay) = match get("ui_anchor").as_deref().unwrap_or("center") {
        "left" => (rx, ry + rh * 0.5),
        "right" => (rx + rw, ry + rh * 0.5),
        "top" => (rx + rw * 0.5, ry + rh),
        "bottom" => (rx + rw * 0.5, ry),
        "top-left" => (rx, ry + rh),
        "top-right" => (rx + rw, ry + rh),
        "bottom-left" => (rx, ry),
        "bottom-right" => (rx + rw, ry),
        _ => (rx + rw * 0.5, ry + rh * 0.5),
    };
    if let Some(fx) = prop_f32("ui_anchor_frac_x") {
        ax = rx + fx * rw;
    }
    if let Some(fy) = prop_f32("ui_anchor_frac_y") {
        ay = ry + fy * rh;
    }
    // Which point of the node lands on the anchor: 0 = its left/bottom edge
    // (the default, and what every existing scene assumes), 0.5 = its centre,
    // 1 = its right/top edge. Without this, centring a fixed-size box means
    // hand-offsetting `position` by half its size — which is impossible the
    // moment the size is data-bound, and is the single most-hit authoring
    // trap otherwise.
    //
    // On an axis the parent's flow decided (`ui_grow`, or a non-stretch
    // `ui_align`), the slot *is* the node: same size, already placed. So the
    // node's own centre is what belongs on the slot's centre, and the default
    // origin is 0.5 there rather than 0. Without this a grown or aligned
    // child puts its left edge on the slot's centre and draws half out of
    // its own slot — and every such node would need a paired
    // `ui_origin_x: 0.5` the author has to remember.
    let default_origin = |flow_decided: bool| if flow_decided { 0.5 } else { 0.0 };
    let origin_x = prop_f32("ui_origin_x").unwrap_or_else(|| default_origin(grow_w.is_some()));
    let origin_y = prop_f32("ui_origin_y").unwrap_or_else(|| default_origin(grow_h.is_some()));

    // ...and on a flow child's *cross* axis under the default
    // `ui_align: "stretch"`, the slot spans the whole cross extent and the flow
    // decides nothing. A child that does not fill it (no `ui_stretch_*`) used to
    // fall all the way back to the centre anchor at origin 0, which puts its
    // near edge on the slot's CENTRE and its far half outside the slot entirely.
    // For a child that is itself what sized the row, that is half its own height
    // of overlap onto its neighbour — invisible on a one-line node, and a
    // wrapped 3-line log entry drawn straight through the entry above it.
    //
    // CSS is the model and gets this right: under `align-items: stretch` an item
    // with a definite cross size behaves as `flex-start`. So does this now — the
    // same cross-start edge `apply_cross_align` uses for `CrossAlign::Start`
    // (left for a column, and **top** for a row, which is y-up).
    //
    // Derived from `flow_size` rather than a new field: a flow child's main axis
    // is always decided, so exactly one of these shapes can occur —
    // `(Some, None)` is a row child with y undecided, `(None, Some)` a column
    // child with x undecided, and `(None, None)` is not a flow child at all.
    //
    // An author who stated an anchor or an origin on the axis keeps it: this is
    // the default for scenes that said nothing, not an override of ones that did.
    let stated = |anchor_frac: &str, origin: &str| {
        get("ui_anchor").is_some() || prop_f32(anchor_frac).is_some() || prop_f32(origin).is_some()
    };
    let cross_x_start =
        grow_w.is_none() && grow_h.is_some() && !stated("ui_anchor_frac_x", "ui_origin_x");
    let cross_y_start =
        grow_h.is_none() && grow_w.is_some() && !stated("ui_anchor_frac_y", "ui_origin_y");

    let (x, w) = if prop_bool("ui_stretch_x") {
        let ml = prop_f32("ui_margin_left").unwrap_or(0.0);
        let mr = prop_f32("ui_margin_right").unwrap_or(0.0);
        (rx + ml, (rw - ml - mr).max(0.0))
    } else if cross_x_start {
        (rx + position.x, w_fixed)
    } else {
        (ax + position.x - origin_x * w_fixed, w_fixed)
    };
    let (y, h) = if prop_bool("ui_stretch_y") {
        let mb = prop_f32("ui_margin_bottom").unwrap_or(0.0);
        let mt = prop_f32("ui_margin_top").unwrap_or(0.0);
        (ry + mb, (rh - mb - mt).max(0.0))
    } else if cross_y_start {
        // y-up: the row's cross-start is its TOP, so the child's top edge lands
        // on the slot's top and it grows downward from there.
        (ry + rh - h_fixed + position.y, h_fixed)
    } else {
        (ay + position.y - origin_y * h_fixed, h_fixed)
    };
    // Optional idle animation: sinusoidal bob (y) / sway (x) with a per-node phase.
    let phase = prop_f32("ui_phase").unwrap_or(0.0);
    let bob = prop_f32("ui_bob_amp").map_or(0.0, |amp| {
        (time * prop_f32("ui_bob_speed").unwrap_or(1.0) + phase).sin() * amp
    });
    let sway = prop_f32("ui_sway_amp").map_or(0.0, |amp| {
        (time * prop_f32("ui_sway_speed").unwrap_or(1.0) + phase).cos() * amp
    });
    let (x, y, w, h) = (x + sway, y + bob, w, h);
    // E9: quantise to the host's pixel grid so fractional/stretch layout never
    // drifts a node off the art's one-pixel-size rule. Opt out per node with
    // `ui_snap: false` (e.g. a smoothly animated/bobbing element); the host
    // opts the whole scene in via `SceneWorld2D::set_pixel_grid`.
    if pixel_grid > 0.0 && !matches!(get("ui_snap").as_deref(), Some("false" | "0" | "no")) {
        let snap = |v: f32| (v / pixel_grid).round() * pixel_grid;
        let snap_size = |v: f32| ((v / pixel_grid).round() * pixel_grid).max(pixel_grid);
        (snap(x), snap(y), snap_size(w), snap_size(h))
    } else {
        (x, y, w, h)
    }
}

/// Which font a node's text draws and *measures* in (E-C).
///
/// `ui_font` is a numeric [`FontId`] — the id the host got back from
/// `Engine::load_font`, which the host publishes as a binding
/// (`ui_font: "{font_hud}"`) exactly the way `palette::theme()` publishes
/// colours. That keeps alias→id resolution in the host, where the asset
/// bundle already lives, instead of giving the engine a second name registry
/// that would need loading, validating and editing.
///
/// Absent or unparseable → font 0, so every existing scene is unchanged.
pub(crate) fn node_font(get: &impl Fn(&str) -> Option<String>) -> FontId {
    get("ui_font")
        .and_then(|v| v.trim().parse::<usize>().ok())
        .map_or(FontId::DEFAULT, FontId)
}

/// A wrapped block's line-height multiplier — CSS `line-height:1.35`, which
/// the mockups author on every prose block (card text, archetype blurbs, perk
/// descriptions).
///
/// A multiplier rather than an absolute: the mockups write it as one, and it
/// stays correct when the same node is authored at a different `ui_text_size`.
/// Absent → `1.0`, the font's own line box, so existing blocks are unchanged.
pub(crate) fn node_leading(get: &impl Fn(&str) -> Option<String>) -> f32 {
    get("ui_line_height")
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|v| *v > 0.0)
        .unwrap_or(1.0)
}

/// A node's authored border, if it has one: `(color, per-side widths)` in
/// `(left, right, bottom, top)` order.
///
/// `ui_border_w` + `ui_border_color` is the whole feature for the common case
/// (`border:2px solid #3f4854`, which the mockups use 97 times); the four
/// per-side widths exist because a divider rule is a one-sided border
/// (`border-top`, 34 more uses) and authoring that as a separate line node
/// would put the panel's edge in two places. Widths are in the node's own
/// space and scale with it, like every other `ui_*` metric.
///
/// Returns `None` when nothing is authored or the border would be invisible,
/// so a node without one costs two failed property lookups and no drawing.
fn node_border(get: &dyn Fn(&str) -> Option<String>, scale: Vec2) -> Option<(Color, [f32; 4])> {
    let f32_of = |n: &str| get(n).and_then(|v| v.trim().parse::<f32>().ok());
    let all = f32_of("ui_border_w");
    let side = |n: &str| (f32_of(n).or(all).unwrap_or(0.0)).max(0.0);
    // x-scale for the vertical edges, y-scale for the horizontal ones: a
    // border tracks the edge it sits on, so a node stretched in one axis
    // doesn't get a lopsided outline.
    let widths = [
        side("ui_border_left") * scale.x,
        side("ui_border_right") * scale.x,
        side("ui_border_bottom") * scale.y,
        side("ui_border_top") * scale.y,
    ];
    let color = parse_srgb_color(get("ui_border_color").as_deref(), Color::WHITE);
    (widths.iter().any(|w| *w > 0.0) && color.a > 0.0).then_some((color, widths))
}

/// How much a node's own border adds to a **measured** size, as
/// `(horizontal, vertical)`.
///
/// [`border_rects`] draws the border *inside* the rect, which is right for an
/// authored size: a panel that says it is 640px wide is 640px wide, border
/// included. A measured size has no such statement to honour — the content
/// decides it — so the border has to be added on top, exactly as CSS's default
/// `box-sizing: content-box` does. Without this a content-sized box paints its
/// border over its own padding: the mockup's 2px-bordered info boxes came out
/// 4px short and their 10px of padding rendered as 8.
///
/// `scale` is the node's own, so the extent tracks the edge it sits on the same
/// way [`node_border`]'s widths do.
pub(crate) fn border_extent(get: &dyn Fn(&str) -> Option<String>, scale: Vec2) -> (f32, f32) {
    match node_border(get, scale) {
        Some((_, [left, right, bottom, top])) => (left + right, bottom + top),
        None => (0.0, 0.0),
    }
}

/// The edge rects of a border drawn *inside* `rect`, CSS `box-sizing:
/// border-box` style — the edges eat into the node's own rect rather than
/// growing it, which is what the mockups assume and what keeps a bordered
/// panel the size it says it is.
///
/// Verticals span the full height and the horizontals fill only between them,
/// so the corners are covered exactly once — overlapping them would show
/// through as a darker square on a translucent border colour. Zero-width
/// sides are omitted, so a one-sided rule is one rect.
///
/// Split out from the drawing so the geometry is testable: a `Canvas` needs a
/// live renderer, which a unit test has no way to build.
fn border_rects(rect: (f32, f32, f32, f32), widths: [f32; 4]) -> Vec<(f32, f32, f32, f32)> {
    let (x, y, w, h) = rect;
    let [left, right, bottom, top] = widths;
    // Clamped so a border thicker than the node it edges stays inside it
    // rather than the two sides overdrawing past each other.
    let left = left.min(w);
    let right = right.min(w - left);
    let bottom = bottom.min(h);
    let top = top.min(h - bottom);
    let inner_x = x + left;
    let inner_w = (w - left - right).max(0.0);
    [
        (x, y, left, h),
        (x + w - right, y, right, h),
        (inner_x, y, inner_w, bottom),
        (inner_x, y + h - top, inner_w, top),
    ]
    .into_iter()
    .filter(|(_, _, rw, rh)| *rw > 0.0 && *rh > 0.0)
    .collect()
}

fn draw_border(canvas: &mut Canvas, rect: (f32, f32, f32, f32), color: Color, widths: [f32; 4]) {
    for (x, y, w, h) in border_rects(rect, widths) {
        canvas.rect(x, y, w, h, color);
    }
}

/// The stripes of a 45° hatch fill: CSS
/// `repeating-linear-gradient(45deg, <hatch> 0 <w>, transparent <w> <w+gap>)`,
/// as `(x0, y0, x1, y1)` line centres in canvas space.
///
/// The mockups use it for every "nothing here yet" plate — a card's art
/// placeholder, an empty garage slot — where a flat fill reads as a bug and a
/// texture would be an asset. It is a decoration on any kind, like the border
/// and the shadow above it, because the alternative is a stack of hand-placed
/// 1px Panels per stripe.
///
/// Stripes run bottom-left to top-right. `pitch` is the CSS period — measured
/// **perpendicular** to the stripes, as a gradient's colour stops are — so a
/// 5px band with a 5px gap repeats every 10px across its own axis, not every
/// 10px along x. The two differ by √2 at 45°, which is the difference between
/// matching the mockup and being 30% too dense.
///
/// [`Canvas::line`] already thickens along the normal, so the stripe *width*
/// needs no such conversion; only the spacing between them does.
///
/// They start before the rect's left edge and end past its right so the corners
/// are covered; the caller clips.
///
/// Split out from the drawing so the geometry is testable without a live
/// renderer, exactly as [`border_rects`] is.
fn hatch_lines(
    rect: (f32, f32, f32, f32),
    pitch: f32,
    diag: f32,
) -> impl Iterator<Item = (f32, f32, f32, f32)> {
    let (x, y, w, h) = rect;
    // A 45° line through the rect is anchored by its x-intercept at the rect's
    // bottom edge, and a perpendicular period of `pitch` is a horizontal one of
    // `pitch * sqrt(2)`.
    let step = pitch * std::f32::consts::SQRT_2;
    // The first covering stripe starts a full height to the left (its top-right
    // corner is the rect's bottom-left); the last starts at the right edge.
    let count = ((w + h) / step).ceil().max(0.0) as usize;
    (0..count).map(move |i| {
        let x0 = x - h + i as f32 * step;
        (x0, y, x0 + diag, y + diag)
    })
}

/// A node's authored hatch, if it has one: `(color, pitch, stripe width)`.
///
/// `ui_hatch_color` alone is enough — `ui_hatch_w` (stripe) and `ui_hatch_gap`
/// (the transparent run after it) both default to the mockups' 5px.
fn node_hatch(get: &dyn Fn(&str) -> Option<String>, scale: Vec2) -> Option<(Color, f32, f32)> {
    let authored = get("ui_hatch_color")?;
    let color = parse_srgb_color(Some(authored.as_str()), Color::WHITE);
    let f32_of = |n: &str, d: f32| {
        get(n)
            .and_then(|v| v.trim().parse::<f32>().ok())
            .unwrap_or(d)
            .max(0.0)
    };
    // One scale for both: the stripes are diagonal, so scaling them per axis
    // would shear the 45° they are defined by.
    let s = scale.x.min(scale.y);
    let (stripe, gap) = (f32_of("ui_hatch_w", 5.0) * s, f32_of("ui_hatch_gap", 5.0) * s);
    let pitch = stripe + gap;
    (color.a > 0.0 && stripe > 0.0 && pitch > 0.0).then_some((color, pitch, stripe))
}

fn draw_hatch(canvas: &mut Canvas, rect: (f32, f32, f32, f32), color: Color, pitch: f32, w: f32) {
    let (rx, ry, rw, rh) = rect;
    if rw <= 0.0 || rh <= 0.0 {
        return;
    }
    canvas.push_clip(rx, ry, rw, rh);
    // A 45° line of thickness `w` needs to be drawn `w` thick along its own
    // normal, which `Canvas::line` already does — the diagonal only sets how
    // far each stripe runs, and one that spans the rect's diagonal extent
    // reaches both far corners.
    let diag = rw + rh;
    for (x0, y0, x1, y1) in hatch_lines(rect, pitch, diag) {
        canvas.line(x0, y0, x1, y1, w, color);
    }
    canvas.pop_clip();
}

/// An unauthored Button bar/marker is invisible rather than white, so a bare
/// Button is just its label until those are given colours.
const BUTTON_TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);

/// Draw the `ui` primitive named by a node's props into the resolved `rect`.
/// `sprite` is the node's own first sprite layer, used only by `"image"`.
/// `has_children` is used only by `"button"` (see its arm below).
///
/// Two decorations apply to *every* kind rather than being kinds of their own,
/// because the alternative is authoring a stack of Panels to fake one visual:
///
/// - **`ui_shadow_color` (+ `ui_shadow_x`/`ui_shadow_y`)** — a hard offset
///   drop shadow painted under the node, matching the mockups' pixel-art
///   `box-shadow`/`text-shadow` (no blur; there is no blurred shadow anywhere
///   in the set). On a text node it offsets the glyphs, on anything else the
///   rect.
/// - **`ui_border_w`/`ui_border_color`** (see [`node_border`]) — painted last,
///   over the node's own fill and under its children.
///
/// Both resolve `_hover`/`_press`/`_focus` variants like any other property,
/// since `get` has already applied the interaction state.
fn draw_ui_kind(
    canvas: &mut Canvas,
    rect: (f32, f32, f32, f32),
    scale: Vec2,
    get: impl Fn(&str) -> Option<String>,
    sprite: Option<&PrefabSprite2D>,
    has_children: bool,
) {
    // `&dyn` at the boundary, not `impl`: the shadow pass calls back in with a
    // wrapper closure, and a generic parameter would try to monomorphize that
    // nesting forever.
    draw_ui_kind_dyn(canvas, rect, scale, &get, sprite, has_children);
}

fn draw_ui_kind_dyn(
    canvas: &mut Canvas,
    rect: (f32, f32, f32, f32),
    scale: Vec2,
    get: &dyn Fn(&str) -> Option<String>,
    sprite: Option<&PrefabSprite2D>,
    has_children: bool,
) {
    let Some(kind) = get("ui") else {
        return;
    };
    let (x, y, w, h) = rect;
    let prop_f32 = |n: &str| ui_f32(&get, n);
    let prop_i64 = |n: &str| get(n).and_then(|v| v.trim().parse::<i64>().ok());
    let font = node_font(&get);
    // A shadow is the same primitive drawn once more, offset and recoloured,
    // so it recurses with the shadow properties stripped rather than every
    // arm below growing a shadow branch of its own.
    if let Some(shadow) = get("ui_shadow_color") {
        let color = parse_srgb_color(Some(shadow.as_str()), Color::BLACK);
        let dx = prop_f32("ui_shadow_x").unwrap_or(0.0) * scale.x;
        // y-down to match the mockups' CSS, where a positive offset drops the
        // shadow *below* the node; canvas space is y-up.
        let dy = -prop_f32("ui_shadow_y").unwrap_or(0.0) * scale.y;
        if color.a > 0.0 && (dx != 0.0 || dy != 0.0) {
            let flat = |n: &str| match n {
                // A shadow is a flat silhouette: its border and its hatch are
                // detail that would show through as a second, offset pattern.
                "ui_shadow_color" | "ui_border_color" | "ui_hatch_color" => None,
                // The whole silhouette takes the shadow colour, whichever
                // property the kind paints itself with.
                "ui_color" | "ui_bar_color" | "ui_marker_color" | "ui_color_top"
                | "ui_color_bottom" => Some(shadow.clone()),
                _ => get(n),
            };
            draw_ui_kind_dyn(
                canvas,
                (x + dx, y + dy, w, h),
                scale,
                &flat,
                sprite,
                has_children,
            );
        }
    }
    // `ui_tracking` is canvas state for the duration of this node's paint, so
    // every text arm below — and the alignment each one measures with — picks
    // it up without a per-arm branch. Restored after, since the canvas is
    // shared with every other node in the frame.
    let prev_tracking = canvas.set_tracking(prop_f32("ui_tracking").unwrap_or(0.0) * scale.x);
    let border = node_border(&get, scale);
    let hatch = node_hatch(&get, scale);
    // The box the text arms below lay their line into: the node's rect inset by
    // its own padding, CSS `padding` on a box that holds text rather than
    // children. `ui_pad_*` already insets a *flow container's* children; a leaf
    // that paints its own text (a card header, `padding:9px 10px`) got no
    // inset at all, so its label sat flush against its own border. Same
    // property, same meaning, on the one kind of node that was missing it —
    // fills and borders still use the full rect, as in `box-sizing:border-box`.
    let (tx, ty, tw, th) = {
        let pad = |n: &str, s: f32| prop_f32(n).unwrap_or(0.0) * s;
        let (l, r) = (pad("ui_pad_left", scale.x), pad("ui_pad_right", scale.x));
        let (b, t) = (pad("ui_pad_bottom", scale.y), pad("ui_pad_top", scale.y));
        (x + l, y + b, (w - l - r).max(0.0), (h - t - b).max(0.0))
    };
    // Painted after the kind's own fill, below — a closure so the early-return
    // arms can't forget it. The hatch goes over the fill and under the border,
    // which is the order CSS paints a background layer in.
    let draw = |canvas: &mut Canvas| {
        if let Some((color, pitch, stripe)) = hatch {
            draw_hatch(canvas, rect, color, pitch, stripe);
        }
        if let Some((color, widths)) = border {
            draw_border(canvas, rect, color, widths);
        }
    };
    match kind.as_str() {
        "rect" => {
            let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
            let radius = prop_f32("ui_radius").unwrap_or(0.0);
            if radius > 0.5 {
                canvas.rounded_rect(x, y, w, h, radius, color);
            } else {
                canvas.rect(x, y, w, h, color);
            }
        }
        "gradient" => {
            let bottom = parse_srgb_color(get("ui_color_bottom").as_deref(), Color::BLACK);
            let top = parse_srgb_color(get("ui_color_top").as_deref(), Color::WHITE);
            canvas.rect_gradient(x, y, w, h, bottom, top);
        }
        "bevel" => {
            let highlight = parse_srgb_color(get("ui_color_top").as_deref(), Color::WHITE);
            let shadow = parse_srgb_color(get("ui_color_bottom").as_deref(), Color::BLACK);
            let line_w = prop_f32("ui_line_w").unwrap_or(1.5);
            canvas.bevel_rect(x, y, w, h, highlight, shadow, line_w);
        }
        "circle" => {
            let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
            let radius = prop_f32("ui_radius").unwrap_or(4.0) * scale.x;
            let segments = prop_i64("ui_segments").unwrap_or(20).clamp(3, 96) as u32;
            canvas.circle_filled(x, y, radius, segments, color);
        }
        "line" => {
            let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
            let line_w = prop_f32("ui_line_w").unwrap_or(1.0);
            canvas.line(x, y, x + w, y + h, line_w, color);
        }
        "polygon" => {
            // `ui_points`: "x,y x,y ..." in the node's own 0..1 box, so a shape
            // authored once scales to whatever size the node resolves to and a
            // person can type it without knowing the layout.
            //
            // The point of this kind is that set dressing can be *drawn* rather
            // than coded: a grandstand, a gravel trap, a treeline are all
            // concave outlines, and before this the only way to get one was to
            // hand-write its geometry in the host's Rust.
            let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
            let pts: Vec<(f32, f32)> = get("ui_points")
                .unwrap_or_default()
                .split_whitespace()
                .filter_map(|pair| {
                    let (px, py) = pair.split_once(',')?;
                    Some((
                        x + w * px.trim().parse::<f32>().ok()?,
                        y + h * py.trim().parse::<f32>().ok()?,
                    ))
                })
                .collect();
            canvas.polygon(&pts, color);
        }
        "image" => {
            // The node's own sprites[0] — already a resolved TextureId, so an
            // authored image needs no new asset plumbing (unlike every other
            // kind, which paints with no texture at all).
            if let Some(sprite) = sprite {
                let color = parse_srgb_color(get("ui_color").as_deref(), sprite.color);
                // `ui_rotation`, in degrees, turning counter-clockwise about
                // the node's own centre. Authored art that has to follow world
                // geometry — a building beside a curving road, a sign facing
                // its corner — could not be expressed at all before this, so
                // the only way to draw it was to hand-code the shape in Rust,
                // which puts the art outside the editor.
                //
                // Degrees rather than radians because a scene is authored by a
                // person: "30" is a thing somebody can type and reason about.
                let spin = ui_f32(&get, "ui_rotation").unwrap_or(0.0);
                if spin.abs() < f32::EPSILON {
                    canvas.image_region(sprite.texture, x, y, w, h, sprite.uv_rect, color);
                } else {
                    canvas.image_region_rotated(
                        sprite.texture,
                        x + w * 0.5,
                        y + h * 0.5,
                        w,
                        h,
                        sprite.uv_rect,
                        color,
                        spin.to_radians(),
                    );
                }
            }
        }
        // A Button with authored children draws no marker or label of its
        // own: those children are its content and draw themselves in the
        // normal top-down pass, with Button contributing the hit rect and the
        // interaction state they inherit.
        //
        // Its **bar still draws** when one is authored, because a fill and
        // content are not alternatives — the mockups' menu rows are a div
        // with a background that also holds a caret and a label, and a
        // selected row needs both. Deferring the bar too meant an authored
        // `ui_bar_color_focus` silently painted nothing, and the only way to
        // get a fill under content was a sibling Panel behind it: the
        // stacked-node shape we are removing. Absent `ui_bar_color` still
        // paints nothing, so a Button that really is only its children is
        // unchanged.
        "button" if has_children => {
            let bar = parse_srgb_color(get("ui_bar_color").as_deref(), BUTTON_TRANSPARENT);
            if bar.a > 0.0 {
                let radius = prop_f32("ui_radius").unwrap_or(0.0);
                if radius > 0.5 {
                    canvas.rounded_rect(x, y, w, h, radius, bar);
                } else {
                    canvas.rect(x, y, w, h, bar);
                }
            }
        }
        "button" => {
            // Unauthored bar/marker default to invisible, so a bare
            // Button is just its label until those are given colours.
            const TRANSPARENT: Color = BUTTON_TRANSPARENT;
            // One node draws what used to be three hand-wired siblings kept
            // in sync by matching `ui_color_hover` values: a highlight bar,
            // an optional leading marker, and the label. `get` has already
            // applied the interaction state (see `resolve_ui_property`), so
            // `ui_bar_color`/`ui_color`/`ui_marker_color` each pick up their
            // own `_hover`/`_focus`/`_press` variant here.
            // Opt-in sprite background: an authored `asset_alias` (resolved
            // to `sprite` by the compiler, same as `Image`) paints instead
            // of the flat `ui_bar_color` fill, for a button that is a
            // pixel-art panel rather than a plain rect.
            if let Some(sprite) = sprite {
                let tint = parse_srgb_color(get("ui_bar_color").as_deref(), sprite.color);
                canvas.image_region(sprite.texture, x, y, w, h, sprite.uv_rect, tint);
            } else {
                let bar = parse_srgb_color(get("ui_bar_color").as_deref(), TRANSPARENT);
                if bar.a > 0.0 {
                    let radius = prop_f32("ui_radius").unwrap_or(0.0);
                    if radius > 0.5 {
                        canvas.rounded_rect(x, y, w, h, radius, bar);
                    } else {
                        canvas.rect(x, y, w, h, bar);
                    }
                }
            }

            let size = prop_f32("ui_text_size").unwrap_or(12.0);
            // `line_height` needs the font atlas, so it is computed lazily —
            // a button that draws only its bar must not touch text metrics.
            // The line box's TOP, centred in the button's padded height. `ty`
            // is that box's BOTTOM (the canvas is y-up), so the top of a
            // centred line box is half the slack down from its top — not half
            // the slack up from its bottom, which is a whole line box lower.
            let line_top =
                |canvas: &Canvas| ty + th - (th - canvas.line_height_in(font, size)) * 0.5;

            let marker = get("ui_marker").unwrap_or_default();
            if !marker.is_empty() {
                let marker_color = parse_srgb_color(get("ui_marker_color").as_deref(), TRANSPARENT);
                if marker_color.a > 0.0 {
                    // Sits `ui_marker_inset` in from the button's leading
                    // edge, so the marker tracks the bar rather than
                    // needing its own hand-placed node.
                    let inset = prop_f32("ui_marker_inset").unwrap_or(10.0);
                    let top = line_top(canvas);
                    canvas.text_aligned_in(
                        font,
                        tx + inset,
                        top,
                        &marker,
                        size,
                        marker_color,
                        TextAlign::Left,
                    );
                }
            }

            let text = ui_text_or_placeholder(&get);
            if !text.is_empty() {
                let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
                let align = parse_text_align(get("ui_text_align").as_deref());
                let anchor_x = match align {
                    TextAlign::Left => tx,
                    TextAlign::Center => tx + tw * 0.5,
                    TextAlign::Right => tx + tw,
                };
                let top = line_top(canvas);
                canvas.text_aligned_in(font, anchor_x, top, &text, size, color, align);
            }
        }
        "text" => {
            let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
            let size = prop_f32("ui_text_size").unwrap_or(12.0);
            let text = ui_text_or_placeholder(&get);
            let align = parse_text_align(get("ui_text_align").as_deref());
            let anchor_x = match align {
                TextAlign::Left => tx,
                TextAlign::Center => tx + tw * 0.5,
                TextAlign::Right => tx + tw,
            };
            // Canvas text takes the line box's TOP. `ty` is the padded box's
            // bottom (y-up), so a line centred in `th` starts half the slack
            // below its top — which for the common `th == line_height` case
            // is exactly that top, and the whole run lands inside it.
            let line_h = canvas.line_height_in(font, size);
            let line_top = ty + th - (th - line_h) * 0.5;
            canvas.text_aligned_in(font, anchor_x, line_top, &text, size, color, align);
        }
        "text_block" => {
            let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
            let size = prop_f32("ui_text_size").unwrap_or(12.0);
            let text = ui_text_or_placeholder(&get);
            let align = parse_text_align(get("ui_text_align").as_deref());
            let wrap_w = prop_f32("ui_wrap_w").unwrap_or(tw);
            let anchor_x = match align {
                TextAlign::Left => tx,
                TextAlign::Center => tx + tw * 0.5,
                TextAlign::Right => tx + tw,
            };
            canvas.text_block_leaded_in(
                font,
                anchor_x,
                ty + th,
                &text,
                size,
                color,
                wrap_w,
                align,
                node_leading(&get),
            );
        }
        "text_spans" => {
            // Numbered rows (ui_span_0_text/ui_span_0_color, ui_span_1_...)
            // rather than one delimited string — each span's text is often
            // itself free-form ("+2.3s pace"), so a delimiter would just be
            // one more thing an authored value could collide with.
            let size = prop_f32("ui_text_size").unwrap_or(12.0);
            let align = parse_text_align(get("ui_text_align").as_deref());
            let mut span_texts = Vec::new();
            let mut span_colors = Vec::new();
            for i in 0.. {
                let Some(text) = get(&format!("ui_span_{i}_text")) else {
                    break;
                };
                let color =
                    parse_srgb_color(get(&format!("ui_span_{i}_color")).as_deref(), Color::WHITE);
                span_texts.push(text);
                span_colors.push(color);
            }
            let spans: Vec<(&str, Color)> = span_texts
                .iter()
                .map(String::as_str)
                .zip(span_colors.iter().copied())
                .collect();
            let line_h = canvas.line_height_in(font, size);
            let line_top = ty + th - (th - line_h) * 0.5;
            canvas.text_spans_aligned_in(font, tx, line_top, &spans, size, align);
        }
        "polyline" => {
            // "x0,y0;x1,y1;..." offsets from the rect's origin — the same
            // convention every other kind uses for its own geometry.
            let color = parse_srgb_color(get("ui_color").as_deref(), Color::WHITE);
            let line_w = prop_f32("ui_line_w").unwrap_or(1.0);
            let raw = get("ui_points").unwrap_or_default();
            let points: Vec<(f32, f32)> = raw
                .split(';')
                .filter_map(|pair| {
                    let (px, py) = pair.split_once(',')?;
                    Some((
                        x + px.trim().parse::<f32>().ok()?,
                        y + py.trim().parse::<f32>().ok()?,
                    ))
                })
                .collect();
            if points.len() >= 2 {
                canvas.polyline(&points, line_w, color);
            }
        }
        _ => {}
    }
    draw(canvas);
    canvas.set_tracking(prev_tracking);
}

/// Whether a node draws, honouring `ui_visible_placeholder` when `ui_visible`
/// holds an unresolved `{key}`.
///
/// An unresolved flag stays *visible*, which is the right default for ordinary
/// nodes — a preview showing too much beats a preview showing nothing. It is
/// exactly wrong for a modal: Formula R's reward scrim is a full-screen rect at
/// alpha 215 gated on `{card_visible}`, so an editor with nothing bound drew it
/// over the entire race screen and hid every panel underneath.
///
/// `ui_visible_placeholder: "false"` lets the scene say what a *preview* should
/// show. Like `ui_text_placeholder`, a running game binds the flag and never
/// reaches this, so it cannot desync from real behaviour.
fn ui_visible(get: &impl Fn(&str) -> Option<String>) -> bool {
    let authored = get("ui_visible");
    let value = match authored.as_deref().map(str::trim) {
        Some(value) if contains_unresolved_binding(value) => get("ui_visible_placeholder"),
        _ => authored,
    };
    !matches!(
        value.as_deref().map(str::trim),
        Some("false" | "0" | "no")
    )
}

/// The text a node should draw: its `ui_text`, or its `ui_text_placeholder`
/// when `ui_text` still holds an unresolved `{key}`.
///
/// A binding-less host — the editor previewing a scene — has no
/// `{circuit_name}` to supply, and drawing the literal `{circuit_name}` is
/// both ugly and the wrong *width*, so a placeholder authored alongside it
/// previews the real thing at a realistic length.
///
/// A running game never sees this: it binds the key, nothing is left in
/// braces, and the placeholder is ignored. That is what makes it safe to
/// author — it cannot mask a missing binding at runtime, only stand in for one
/// that was never going to exist.
///
/// Partially-resolved text ("LAP {lap_current}") counts as unresolved: a
/// half-substituted string is not something any host wants on screen.
fn ui_text_or_placeholder(get: &impl Fn(&str) -> Option<String>) -> String {
    let text = get("ui_text").unwrap_or_default();
    if !contains_unresolved_binding(&text) {
        return text;
    }
    match get("ui_text_placeholder") {
        Some(placeholder) if !placeholder.is_empty() => placeholder,
        // No placeholder authored: draw nothing rather than a raw `{key}`.
        // Empty is honest — it shows the author that this node is dynamic and
        // has nothing to preview — while the literal reads as a real label.
        _ => String::new(),
    }
}

/// A numeric `ui_*` property, falling back to `<name>_placeholder` when its
/// `{key}` was never bound — the same contract as `ui_text_placeholder`.
///
/// Not every bound number is a font metric a host can measure for itself.
/// Formula R's title menu authors `ui_h: "{row_h}"`, where `row_h` is
/// arithmetic over other authored values (`title_bindings`). Unresolved, it
/// parses as nothing, every row collapses to zero height, and all five menu
/// labels draw on top of each other.
///
/// A running game binds the key and never reaches the placeholder, so this
/// cannot drift from real behaviour.
fn ui_f32(get: &impl Fn(&str) -> Option<String>, name: &str) -> Option<f32> {
    let raw = get(name);
    match raw.as_deref().map(str::trim) {
        Some(value) if contains_unresolved_binding(value) => get(&format!("{name}_placeholder"))
            .and_then(|v| v.trim().parse::<f32>().ok()),
        _ => raw.and_then(|v| v.trim().parse::<f32>().ok()),
    }
}

/// Whether a resolved string still carries a `{key}` the host never bound.
fn contains_unresolved_binding(text: &str) -> bool {
    let Some(open) = text.find('{') else {
        return false;
    };
    text[open + 1..]
        .find('}')
        .is_some_and(|close| close > 0 && !text[open + 1..open + 1 + close].contains(' '))
}

/// Parse `ui_text_align`: `left` (default) | `center` | `right`.
fn parse_text_align(value: Option<&str>) -> TextAlign {
    match value.map(str::trim) {
        Some("center") => TextAlign::Center,
        Some("right") => TextAlign::Right,
        _ => TextAlign::Left,
    }
}

/// A named scope of values a `ui_*` property template can reference via
/// `{key}` placeholders (E2 data binding). Just a string map — repeaters
/// (E3) push per-item fields onto a scope before drawing each instance.
pub type Bindings = HashMap<String, String>;

/// Named collections a `ui: "repeat"` node's `ui_repeat_source` can reference
/// (E3): one [`Bindings`] scope per item. `SceneWorld2D::sync_repeaters`
/// reconciles a repeat node's instance children to the named collection's
/// length and stores each instance's scope, so `ui_text: "P{pos}  {name}"`
/// resolves against whichever item that instance represents.
pub type RepeaterSources = HashMap<String, Vec<Bindings>>;

/// Substitute every `{key}` in `template` with `bindings[key]`, leaving
/// unknown placeholders untouched (so a scope missing one field doesn't wreck
/// the whole string) and returning the input unchanged (no allocation) when
/// there is nothing to substitute — the common case for the vast majority of
/// `ui_*` properties, which are plain literals.
pub(crate) fn substitute_bindings<'a>(template: &'a str, bindings: &Bindings) -> Cow<'a, str> {
    if !template.as_bytes().contains(&b'{') {
        return Cow::Borrowed(template);
    }
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        match after_open.find('}') {
            Some(close) => {
                let key = &after_open[..close];
                match bindings.get(key) {
                    Some(value) => out.push_str(value),
                    None => {
                        out.push('{');
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after_open[close + 1..];
            }
            None => {
                // Unterminated `{` — keep it literal rather than dropping the
                // rest of the string.
                out.push('{');
                rest = after_open;
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// A node's live interaction state for the frame being drawn (E6). Drives
/// `ui_color_hover`/`ui_color_press`/`ui_color_focus` overrides — the host
/// (`SceneLayer2D`) tracks which node is hovered/pressed/focused and passes
/// the right state in per node at draw time.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct UiInteractionState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
}

/// Resolve + draw a UI node onto its `ui_layer` canvas in `frame`; returns the
/// resolved rect for child layout and whether `ui_visible` allowed it to draw.
pub(crate) fn draw_ui_node<'a>(
    frame: &mut Frame,
    reference: (f32, f32, f32, f32),
    position: Vec2,
    scale: Vec2,
    time: f32,
    get: impl Fn(&str) -> Option<&'a str>,
    sprite: Option<&PrefabSprite2D>,
) -> ((f32, f32, f32, f32), bool) {
    draw_ui_node_with_bindings(
        frame,
        reference,
        position,
        scale,
        time,
        |n| get(n).map(str::to_owned),
        &Bindings::new(),
        sprite,
        0.0,
        UiInteractionState::default(),
        None,
        (None, None),
        false,
    )
}

/// Like [`draw_ui_node`] but every `ui_*` value is substituted through
/// `bindings` first (E2), so `ui_text: "P{pos}"` reads live data instead of a
/// literal string. Returns the resolved rect (still needed as the reference
/// frame for children even when this node's own primitive is hidden) and
/// whether `ui_visible` allowed it to draw — the caller uses that to decide
/// whether the node should be hit-testable (E1's "clickable where drawn"
/// contract shouldn't apply to a node that drew nothing). `sprite` backs
/// `ui: "image"` (E5) — the node's own first sprite layer, already a resolved
/// `TextureId`, so an authored image needs no new asset plumbing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_ui_node_with_bindings(
    frame: &mut Frame,
    reference: (f32, f32, f32, f32),
    position: Vec2,
    scale: Vec2,
    time: f32,
    get: impl Fn(&str) -> Option<String>,
    bindings: &Bindings,
    sprite: Option<&PrefabSprite2D>,
    pixel_grid: f32,
    state: UiInteractionState,
    content_size: Option<(f32, f32)>,
    flow_size: (Option<f32>, Option<f32>),
    has_children: bool,
) -> ((f32, f32, f32, f32), bool) {
    let get = |n: &str| get(n).map(|v| substitute_bindings(&v, bindings).into_owned());
    let get = |n: &str| resolve_interaction_property(&get, n, state);
    let rect = resolve_ui_rect(
        reference,
        position,
        scale,
        time,
        &get,
        pixel_grid,
        content_size,
        flow_size,
    );
    let visible = ui_visible(&get);
    if visible && get("ui").is_some() {
        let layer = get("ui_layer")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0)
            .max(0) as usize;
        draw_ui_kind(frame.canvas(layer), rect, scale, &get, sprite, has_children);
    }
    (rect, visible)
}

/// Resolve + draw a UI node directly onto a caller-owned `canvas` (ignoring
/// `ui_layer`), so a whole scene can be drawn into an existing canvas at a
/// chosen z-position. Returns the resolved rect for child layout and whether
/// `ui_visible` allowed it to draw. Every `ui_*` value may contain `{key}`
/// placeholders substituted through `bindings` first (E2); pass an empty
/// [`Bindings`] for a scope-free draw. `sprite` backs `ui: "image"` (E5).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_ui_node_on_with_bindings<'a>(
    canvas: &mut Canvas,
    reference: (f32, f32, f32, f32),
    position: Vec2,
    scale: Vec2,
    time: f32,
    // Borrowed, not owned. Every node reads about thirty properties a frame —
    // 21,660 of them on Formula R's race screen — and the caller used to hand
    // each one over as a freshly allocated `String`. The node outlives this
    // call, so there was never anything to own.
    get: impl Fn(&str) -> Option<&'a str>,
    bindings: &Bindings,
    sprite: Option<&PrefabSprite2D>,
    pixel_grid: f32,
    state: UiInteractionState,
    content_size: Option<(f32, f32)>,
    flow_size: (Option<f32>, Option<f32>),
    has_children: bool,
) -> ((f32, f32, f32, f32), bool) {
    let get = |n: &str| get(n).map(|v| substitute_bindings(v, bindings).into_owned());
    let get = |n: &str| resolve_interaction_property(&get, n, state);
    let rect = resolve_ui_rect(
        reference,
        position,
        scale,
        time,
        &get,
        pixel_grid,
        content_size,
        flow_size,
    );
    let visible = ui_visible(&get);
    if visible {
        draw_ui_kind(canvas, rect, scale, &get, sprite, has_children);
    }
    (rect, visible)
}

/// E6 per-state property overrides: `ui_color_press` wins while `pressed`,
/// else `ui_color_hover` while `hovered`, else `ui_color_focus` while
/// `focused`, else the base `ui_color`. Pressed beats hovered beats focused
/// so a click mid-hover reads as "pressed," not a blend of both. Only
/// `ui_color` is state-aware today — the plan names exactly this property;
/// extend the `overridable` list below if a future kind needs its own.
fn resolve_interaction_property(
    get: &impl Fn(&str) -> Option<String>,
    name: &str,
    state: UiInteractionState,
) -> Option<String> {
    // Any property may carry state variants, not a fixed list of three.
    //
    // This was an allowlist of `ui_color`/`ui_bar_color`/`ui_marker_color`,
    // which silently ignored every property added since — `ui_border_color`,
    // `ui_shadow_color`, `ui_tracking` — so an authored
    // `ui_border_color_focus` did nothing and looked like a scene bug. An
    // allowlist here has to be updated by whoever adds a property, and
    // forgetting is invisible; a suffix lookup cannot be forgotten, and a
    // variant nobody authored is just a failed lookup.
    if state.pressed {
        if let Some(v) = get(&format!("{name}_press")) {
            return Some(v);
        }
    }
    if state.hovered {
        if let Some(v) = get(&format!("{name}_hover")) {
            return Some(v);
        }
    }
    if state.focused {
        if let Some(v) = get(&format!("{name}_focus")) {
            return Some(v);
        }
    }
    get(name)
}

/// Parse a `"r,g,b[,a]"` sRGB triplet/quad (0–255 per channel) into a [`Color`],
/// falling back to `default` if absent or malformed.
fn parse_srgb_color(value: Option<&str>, default: Color) -> Color {
    let Some(value) = value else {
        return default;
    };
    let parts: Vec<f32> = value
        .split(',')
        .filter_map(|p| p.trim().parse::<f32>().ok())
        .collect();
    let chan = |i: usize| parts.get(i).copied().unwrap_or(0.0).clamp(0.0, 255.0) as u8;
    match parts.len() {
        3 => Color::from_srgb8(chan(0), chan(1), chan(2), 255),
        4 => Color::from_srgb8(chan(0), chan(1), chan(2), chan(3)),
        _ => default,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneScriptBinding2D {
    pub instance_index: usize,
    pub prefab: String,
    pub script_path: String,
    pub editor_node_id: Option<u64>,
    pub editor_parent_id: Option<u64>,
    pub editor_name: Option<String>,
    /// Authored script parameters, collected from the node's `param_<name>`
    /// properties (prefix stripped). Lets one registered script be configured
    /// per-instance instead of needing a distinct `script_path` per behavior.
    pub params: HashMap<String, String>,
}

impl SceneScriptBinding2D {
    /// Raw string value of an authored `param_<name>`, if present.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    pub fn param_f32(&self, name: &str) -> Option<f32> {
        self.param(name).and_then(|v| v.trim().parse::<f32>().ok())
    }

    pub fn param_bool(&self, name: &str) -> Option<bool> {
        self.param(name).and_then(parse_bool_property)
    }

    /// Parse a `"r,g,b[,a]"` sRGB param into a [`Color`], or `default` if
    /// absent/malformed.
    pub fn param_color(&self, name: &str, default: Color) -> Color {
        parse_srgb_color(self.param(name), default)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scene2D {
    instances: Vec<SceneInstance2D>,
}

impl Scene2D {
    pub fn load_from_path(path: &Path, assets: &AssetPack) -> Result<Self, AssetError> {
        let text = std::fs::read_to_string(path).map_err(|source| AssetError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_json_str(path, &text, assets)
    }

    /// Parse and compile a scene from an in-memory JSON string rather than a
    /// file on disk. `path` is used only to label errors (`AssetError::Json`
    /// etc.) — it need not exist. Lets a caller (e.g. the editor's live
    /// preview) run a document through the exact same pipeline as
    /// [`load_from_path`](Self::load_from_path) without a round-trip to disk.
    pub fn from_json_str(path: &Path, text: &str, assets: &AssetPack) -> Result<Self, AssetError> {
        let json_value: serde_json::Value =
            serde_json::from_str(text).map_err(|source| AssetError::Json {
                path: path.to_path_buf(),
                source,
            })?;
        let definition = scene_definition_from_json(path, json_value)?;
        Self::from_definition(path, definition, assets)
    }

    pub fn from_definition(
        path: &Path,
        definition: Scene2DDef,
        assets: &AssetPack,
    ) -> Result<Self, AssetError> {
        let prefabs = compile_prefabs(path, &definition.prefabs, assets)?;
        let mut instances = Vec::with_capacity(definition.instances.len());

        for instance in definition.instances {
            let Some(prefab) = prefabs.get(&instance.prefab) else {
                return Err(AssetError::scene_message(
                    path,
                    format!("instance references missing prefab '{}'", instance.prefab),
                ));
            };

            instances.push(SceneInstance2D {
                prefab: instance.prefab,
                position: Vec2::from_array(instance.position),
                scale: Vec2::from_array(instance.scale),
                properties: instance.properties,
                sprites: prefab.sprites.clone(),
            });
        }

        Ok(Self { instances })
    }

    pub fn instances(&self) -> &[SceneInstance2D] {
        &self.instances
    }

    pub fn by_prefab<'a>(&'a self, prefab: &'a str) -> impl Iterator<Item = &'a SceneInstance2D> {
        self.instances
            .iter()
            .filter(move |instance| instance.prefab == prefab)
    }

    pub fn instance_by_editor_name(&self, editor_name: &str) -> Option<&SceneInstance2D> {
        self.instances
            .iter()
            .find(|instance| instance.editor_name() == Some(editor_name))
    }

    pub fn instance_by_editor_node_id(&self, editor_node_id: u64) -> Option<&SceneInstance2D> {
        self.instances
            .iter()
            .find(|instance| instance.editor_node_id() == Some(editor_node_id))
    }

    pub fn by_tag<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a SceneInstance2D> {
        self.instances
            .iter()
            .filter(move |instance| instance.has_tag(tag))
    }

    pub fn script_bindings(&self) -> Vec<SceneScriptBinding2D> {
        self.instances
            .iter()
            .enumerate()
            .filter_map(|(instance_index, instance)| {
                let script_path = instance.script_path()?.to_string();
                Some(SceneScriptBinding2D {
                    instance_index,
                    prefab: instance.prefab.clone(),
                    script_path,
                    editor_node_id: instance.editor_node_id(),
                    editor_parent_id: instance.editor_parent_id(),
                    editor_name: instance.editor_name().map(str::to_string),
                    params: instance.script_params(),
                })
            })
            .collect()
    }

    pub fn draw(&self, frame: &mut Frame) {
        for instance in &self.instances {
            instance.draw(frame);
        }
    }

    /// Draw with an animation clock so `ui_bob_*` / `ui_sway_*` node animations
    /// advance; pass `engine.time().total_time()`.
    pub fn draw_animated(&self, frame: &mut Frame, time: f32) {
        for instance in &self.instances {
            instance.draw_at(frame, time);
        }
    }
}

fn compile_prefabs(
    path: &Path,
    defs: &[Prefab2DDef],
    assets: &AssetPack,
) -> Result<HashMap<String, Prefab2D>, AssetError> {
    let mut prefabs = HashMap::new();

    for prefab in defs {
        let mut sprites = Vec::with_capacity(prefab.sprites.len());
        for sprite in &prefab.sprites {
            let Some(texture) = assets.texture_id(&sprite.asset) else {
                return Err(AssetError::scene_message(
                    path,
                    format!(
                        "prefab '{}' references missing asset alias '{}'",
                        prefab.name, sprite.asset
                    ),
                ));
            };

            sprites.push(PrefabSprite2D {
                texture,
                offset: Vec2::from_array(sprite.offset),
                size: Vec2::from_array(sprite.size),
                color: Color::new(
                    sprite.color[0],
                    sprite.color[1],
                    sprite.color[2],
                    sprite.color[3],
                ),
                uv_rect: sprite.uv_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]),
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
            });
        }

        prefabs.insert(
            prefab.name.clone(),
            Prefab2D {
                name: prefab.name.clone(),
                sprites,
            },
        );
    }

    Ok(prefabs)
}

fn default_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_scale() -> [f32; 2] {
    [1.0, 1.0]
}

pub(crate) fn parse_bool_property(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorSceneDocument {
    #[serde(default)]
    pub nodes: Vec<EditorSceneNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorSceneNode {
    pub id: u64,
    #[serde(default)]
    pub parent: Option<u64>,
    #[serde(default)]
    pub name: String,
    pub kind: EditorSceneNodeKind,
    #[serde(default)]
    pub position: [f32; 2],
    #[serde(default = "default_editor_size")]
    pub size: [f32; 2],
    #[serde(default = "default_editor_visible")]
    pub visible: bool,
    #[serde(default)]
    pub script_path: String,
    #[serde(default)]
    pub runtime_prefab: String,
    #[serde(default)]
    pub asset_alias: String,
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

/// What a scene node *is*, as authored.
///
/// The UI kinds below name a node's role directly, so `kind` is the single
/// source of truth and the `ui` draw property is derived from it (see
/// [`EditorSceneNodeKind::ui_kind`]). The older arrangement — every UI node
/// authored as `UiRoot` with its real type hidden in a `ui` string — left
/// `kind` carrying no information (567 of Formula R's 631 nodes were
/// `UiRoot`) and made a sprite need *both* `Sprite` and `ui: "image"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorSceneNodeKind {
    Group,
    Empty,
    Camera2d,
    Sprite,
    Trigger,
    /// Deprecated: a UI node whose real type lives in its `ui` property.
    /// Kept so existing documents keep loading; the typed kinds below
    /// replace it.
    UiRoot,

    /// Non-interactive text.
    Label,
    /// A filled rect used as a surface.
    Panel,
    /// A textured node. Resolves `asset_alias` *and* draws it — one type
    /// where `Sprite` + `ui: "image"` used to be needed together.
    Image,
    /// Interactive: text plus hover/focus colours plus an action.
    Button,
    /// Lays out children; draws nothing itself.
    Layout,
    /// Repeats its one authored child per item (`ui_repeat_*`).
    List,
    /// A non-rect primitive; which one is in `ui_shape`
    /// (line/gradient/circle/bevel/polyline/text_block/text_spans).
    Shape,
    /// Invisible, carries `script_path` and `param_*` — an action with no
    /// visual of its own.
    Action,
}

impl EditorSceneNodeKind {
    fn property_value(self) -> &'static str {
        match self {
            Self::Group => "Group",
            Self::Empty => "Empty",
            Self::Camera2d => "Camera2D",
            Self::Sprite => "Sprite",
            Self::Trigger => "Trigger",
            Self::UiRoot => "UI Root",
            Self::Label => "Label",
            Self::Panel => "Panel",
            Self::Image => "Image",
            Self::Button => "Button",
            Self::Layout => "Layout",
            Self::List => "List",
            Self::Shape => "Shape",
            Self::Action => "Action",
        }
    }

    /// The `ui` draw kind this node type implies, if any.
    ///
    /// `None` means "this kind doesn't draw itself" (a layout container, an
    /// action, or a non-UI kind). `Shape` returns `None` too: which
    /// primitive it is comes from its own `ui_shape` property, since one
    /// `Shape` kind covers several draw primitives that differ only in
    /// which canvas call they make.
    pub fn ui_kind(self) -> Option<&'static str> {
        match self {
            Self::Label => Some("text"),
            Self::Panel => Some("rect"),
            Self::Image => Some("image"),
            Self::Button => Some("button"),
            Self::List => Some("repeat"),
            _ => None,
        }
    }

    /// Whether this kind resolves `asset_alias` into a prefab sprite.
    pub fn carries_sprite(self) -> bool {
        matches!(self, Self::Sprite | Self::Image | Self::Button)
    }

    /// Whether an alias-less sprite is tolerated (compiles to no sprite,
    /// drawing the kind's non-textured fallback) rather than a hard error.
    /// `Sprite` alone stays strict — it has nothing to be without a texture.
    fn tolerates_missing_alias(self) -> bool {
        matches!(self, Self::Image | Self::Button)
    }
}

fn scene_definition_from_json(
    path: &Path,
    json_value: serde_json::Value,
) -> Result<Scene2DDef, AssetError> {
    if json_value.get("nodes").is_some() {
        let document: EditorSceneDocument =
            serde_json::from_value(json_value).map_err(|source| AssetError::Json {
                path: path.to_path_buf(),
                source,
            })?;
        scene_definition_from_editor_document(path, document)
    } else {
        serde_json::from_value(json_value).map_err(|source| AssetError::Json {
            path: path.to_path_buf(),
            source,
        })
    }
}

fn scene_definition_from_editor_document(
    path: &Path,
    document: EditorSceneDocument,
) -> Result<Scene2DDef, AssetError> {
    let node_indices = build_editor_node_indices(path, &document.nodes)?;
    validate_editor_node_parents(path, &document.nodes, &node_indices)?;
    let child_ids = build_editor_child_ids(&document.nodes);
    let mut prefabs = Vec::new();
    let mut prefab_indices = HashMap::new();
    let mut instances = Vec::with_capacity(document.nodes.len());

    for node in &document.nodes {
        if !should_emit_editor_instance(node, &document.nodes, &node_indices) {
            continue;
        }

        let prefab_name = editor_runtime_prefab_name(path, &node)?;
        let mut prefab = prefab_from_editor_node(
            path,
            node,
            &prefab_name,
            &document.nodes,
            &node_indices,
            &child_ids,
        )?;
        canonicalize_prefab(&mut prefab);

        if let Some(index) = prefab_indices.get(prefab_name.as_str()) {
            if prefabs[*index] != prefab {
                return Err(AssetError::scene_message(
                    path,
                    format!(
                        "editor nodes map to runtime prefab '{}' with conflicting visual definitions",
                        prefab_name
                    ),
                ));
            }
        } else {
            prefab_indices.insert(prefab_name.clone(), prefabs.len());
            prefabs.push(prefab);
        }

        instances.push(SceneInstance2DDef {
            prefab: prefab_name,
            position: node.position,
            scale: default_scale(),
            properties: editor_instance_properties(node),
        });
    }

    Ok(Scene2DDef { prefabs, instances })
}

fn build_editor_node_indices(
    path: &Path,
    nodes: &[EditorSceneNode],
) -> Result<HashMap<u64, usize>, AssetError> {
    let mut indices = HashMap::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        if let Some(previous_index) = indices.insert(node.id, index) {
            return Err(AssetError::scene_message(
                path,
                format!(
                    "editor scene contains duplicate node id {} at indices {} and {}",
                    node.id, previous_index, index
                ),
            ));
        }
    }
    Ok(indices)
}

fn validate_editor_node_parents(
    path: &Path,
    nodes: &[EditorSceneNode],
    node_indices: &HashMap<u64, usize>,
) -> Result<(), AssetError> {
    for node in nodes {
        let mut ancestors = HashSet::new();
        let mut current_parent = node.parent;

        while let Some(parent_id) = current_parent {
            if parent_id == node.id {
                let message = if node.parent == Some(node.id) {
                    format!(
                        "editor node '{}' ({}) cannot parent itself",
                        node.name, node.id
                    )
                } else {
                    format!(
                        "editor node '{}' ({}) participates in a parent cycle",
                        node.name, node.id
                    )
                };

                return Err(AssetError::scene_message(path, message));
            }

            if !ancestors.insert(parent_id) {
                return Err(AssetError::scene_message(
                    path,
                    format!(
                        "editor node '{}' ({}) participates in a parent cycle",
                        node.name, node.id
                    ),
                ));
            }

            let Some(parent_index) = node_indices.get(&parent_id) else {
                return Err(AssetError::scene_message(
                    path,
                    format!(
                        "editor node '{}' ({}) references missing parent {}",
                        node.name, node.id, parent_id
                    ),
                ));
            };

            current_parent = nodes[*parent_index].parent;
        }
    }

    Ok(())
}

fn build_editor_child_ids(nodes: &[EditorSceneNode]) -> HashMap<u64, Vec<u64>> {
    let mut child_ids = HashMap::new();
    for node in nodes {
        if let Some(parent) = node.parent {
            child_ids
                .entry(parent)
                .or_insert_with(Vec::new)
                .push(node.id);
        }
    }
    child_ids
}

fn should_emit_editor_instance(
    node: &EditorSceneNode,
    nodes: &[EditorSceneNode],
    node_indices: &HashMap<u64, usize>,
) -> bool {
    match node.kind {
        EditorSceneNodeKind::Group => true,
        kind if kind.carries_sprite() => {
            nearest_group_ancestor(node.parent, nodes, node_indices).is_none()
        }
        _ => true,
    }
}

fn nearest_group_ancestor(
    mut node_id: Option<u64>,
    nodes: &[EditorSceneNode],
    node_indices: &HashMap<u64, usize>,
) -> Option<u64> {
    while let Some(parent_id) = node_id {
        let Some(index) = node_indices.get(&parent_id) else {
            return None;
        };
        let parent = &nodes[*index];
        if parent.kind == EditorSceneNodeKind::Group {
            return Some(parent_id);
        }
        node_id = parent.parent;
    }

    None
}

fn editor_runtime_prefab_name(path: &Path, node: &EditorSceneNode) -> Result<String, AssetError> {
    let prefab_name = if node.runtime_prefab.trim().is_empty() {
        node.name.trim()
    } else {
        node.runtime_prefab.trim()
    };

    if prefab_name.is_empty() {
        return Err(AssetError::scene_message(
            path,
            format!(
                "editor node {} must have either a node name or a runtime prefab override",
                node.id
            ),
        ));
    }

    Ok(prefab_name.to_string())
}

fn prefab_from_editor_node(
    path: &Path,
    node: &EditorSceneNode,
    prefab_name: &str,
    nodes: &[EditorSceneNode],
    node_indices: &HashMap<u64, usize>,
    child_ids: &HashMap<u64, Vec<u64>>,
) -> Result<Prefab2DDef, AssetError> {
    if node.kind == EditorSceneNodeKind::Group {
        return Ok(Prefab2DDef {
            name: prefab_name.to_string(),
            sprites: group_prefab_sprites(path, node, nodes, node_indices, child_ids)?,
        });
    }

    // A node whose sprite is optional (Image, Button) with no alias yet is
    // mid-authoring, not an error: failing here would abort the *whole*
    // scene the moment someone adds one in the editor, before they pick a
    // texture. It compiles to no sprite and draws its non-textured fallback
    // until one is set. `Sprite` stays strict — an alias-less pure sprite
    // node has nothing else to be.
    if !node.kind.carries_sprite()
        || (node.kind.tolerates_missing_alias() && node.asset_alias.trim().is_empty())
    {
        return Ok(Prefab2DDef {
            name: prefab_name.to_string(),
            sprites: Vec::new(),
        });
    }

    let sprite = prefab_sprite_from_editor_node(path, node, node.position)?;

    Ok(Prefab2DDef {
        name: prefab_name.to_string(),
        sprites: vec![sprite],
    })
}

fn group_prefab_sprites(
    path: &Path,
    root: &EditorSceneNode,
    nodes: &[EditorSceneNode],
    node_indices: &HashMap<u64, usize>,
    child_ids: &HashMap<u64, Vec<u64>>,
) -> Result<Vec<PrefabSprite2DDef>, AssetError> {
    let mut sprites = Vec::new();
    collect_group_prefab_sprites(
        path,
        root,
        root.id,
        nodes,
        node_indices,
        child_ids,
        &mut sprites,
    )?;
    Ok(sprites)
}

fn canonicalize_prefab(prefab: &mut Prefab2DDef) {
    prefab.sprites.sort_unstable_by(compare_prefab_sprites);
}

fn compare_prefab_sprites(left: &PrefabSprite2DDef, right: &PrefabSprite2DDef) -> Ordering {
    left.asset
        .cmp(&right.asset)
        .then_with(|| compare_f32_arrays(&left.offset, &right.offset))
        .then_with(|| compare_f32_arrays(&left.size, &right.size))
        .then_with(|| compare_f32_arrays(&left.color, &right.color))
        .then_with(|| compare_optional_f32_arrays(&left.uv_rect, &right.uv_rect))
        .then_with(|| left.flip_x.cmp(&right.flip_x))
        .then_with(|| left.flip_y.cmp(&right.flip_y))
}

fn compare_f32_arrays<const N: usize>(left: &[f32; N], right: &[f32; N]) -> Ordering {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left.total_cmp(right))
        .find(|ordering| *ordering != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

fn compare_optional_f32_arrays<const N: usize>(
    left: &Option<[f32; N]>,
    right: &Option<[f32; N]>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_f32_arrays(left, right),
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn collect_group_prefab_sprites(
    path: &Path,
    root: &EditorSceneNode,
    parent_id: u64,
    nodes: &[EditorSceneNode],
    node_indices: &HashMap<u64, usize>,
    child_ids: &HashMap<u64, Vec<u64>>,
    sprites: &mut Vec<PrefabSprite2DDef>,
) -> Result<(), AssetError> {
    let Some(children) = child_ids.get(&parent_id) else {
        return Ok(());
    };

    let mut stack = children.iter().rev().copied().collect::<Vec<_>>();
    let mut visited = HashSet::new();

    while let Some(child_id) = stack.pop() {
        if !visited.insert(child_id) {
            continue;
        }

        let Some(index) = node_indices.get(&child_id) else {
            continue;
        };
        let child = &nodes[*index];

        if child.kind == EditorSceneNodeKind::Group {
            continue;
        }

        // Same tolerance as the standalone case above: an alias-less Image
        // or Button contributes no sprite rather than failing the scene.
        let alias_pending =
            child.kind.tolerates_missing_alias() && child.asset_alias.trim().is_empty();
        if child.kind.carries_sprite() && child.visible && !alias_pending {
            sprites.push(prefab_sprite_from_editor_node(path, child, root.position)?);
        }

        if let Some(grandchildren) = child_ids.get(&child.id) {
            stack.extend(grandchildren.iter().rev().copied());
        }
    }

    Ok(())
}

fn prefab_sprite_from_editor_node(
    path: &Path,
    node: &EditorSceneNode,
    root_position: [f32; 2],
) -> Result<PrefabSprite2DDef, AssetError> {
    let asset_alias = node.asset_alias.trim();
    if asset_alias.is_empty() {
        return Err(AssetError::scene_message(
            path,
            format!(
                "editor sprite node '{}' ({}) is missing an asset alias",
                node.name, node.id
            ),
        ));
    }

    if node.size[0] <= 0.0 || node.size[1] <= 0.0 {
        return Err(AssetError::scene_message(
            path,
            format!(
                "editor sprite node '{}' ({}) must have a positive size",
                node.name, node.id
            ),
        ));
    }

    Ok(PrefabSprite2DDef {
        asset: asset_alias.to_string(),
        offset: [
            node.position[0] - root_position[0],
            node.position[1] - root_position[1],
        ],
        size: node.size,
        color: default_color(),
        uv_rect: None,
        flip_x: false,
        flip_y: false,
    })
}

fn editor_instance_properties(node: &EditorSceneNode) -> HashMap<String, String> {
    let mut properties = node.properties.clone();

    // The typed kinds imply their own draw kind, so `ui` no longer has to be
    // authored alongside `kind`. `or_insert_with` keeps an authored `ui`
    // winning, which is what lets the legacy `UiRoot` + `ui: "..."` documents
    // and the typed ones coexist while the corpus migrates.
    if let Some(ui_kind) = node.kind.ui_kind() {
        properties
            .entry("ui".to_string())
            .or_insert_with(|| ui_kind.to_string());
    } else if node.kind == EditorSceneNodeKind::Shape {
        // One `Shape` kind covers several primitives that differ only in
        // which canvas call they make; `ui_shape` picks which.
        if let Some(shape) = node.properties.get("ui_shape").cloned() {
            properties.entry("ui".to_string()).or_insert(shape);
        }
    }

    properties
        .entry("editor_node_id".to_string())
        .or_insert_with(|| node.id.to_string());
    properties
        .entry("editor_name".to_string())
        .or_insert_with(|| node.name.clone());
    properties
        .entry("editor_kind".to_string())
        .or_insert_with(|| node.kind.property_value().to_string());
    properties
        .entry("editor_visible".to_string())
        .or_insert_with(|| node.visible.to_string());
    properties
        .entry("editor_size_x".to_string())
        .or_insert_with(|| node.size[0].to_string());
    properties
        .entry("editor_size_y".to_string())
        .or_insert_with(|| node.size[1].to_string());

    if let Some(parent) = node.parent {
        properties
            .entry("editor_parent_id".to_string())
            .or_insert_with(|| parent.to_string());
    }

    if !node.script_path.trim().is_empty() {
        properties
            .entry("script_path".to_string())
            .or_insert_with(|| node.script_path.trim().to_string());
    }

    if !node.asset_alias.trim().is_empty() {
        properties
            .entry("asset_alias".to_string())
            .or_insert_with(|| node.asset_alias.trim().to_string());
    }

    properties
}

fn default_editor_size() -> [f32; 2] {
    [88.0, 56.0]
}

fn default_editor_visible() -> bool {
    true
}

#[cfg(test)]
mod placeholder_tests {
    use super::*;
    use std::collections::HashMap;

    fn getter(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    /// The editor's case: nothing bound `{circuit_name}`, so the authored
    /// placeholder stands in at a realistic width.
    #[test]
    fn an_unresolved_binding_draws_its_placeholder() {
        let get = getter(&[
            ("ui_text", "{circuit_name}"),
            ("ui_text_placeholder", "AUTODROMO NAZIONALE"),
        ]);
        assert_eq!(ui_text_or_placeholder(&get), "AUTODROMO NAZIONALE");
    }

    /// The running game's case, and the one that makes the property safe to
    /// author: once the host binds the key there is nothing in braces, so the
    /// real value wins and the placeholder can never mask live data.
    #[test]
    fn resolved_text_ignores_the_placeholder() {
        let get = getter(&[
            ("ui_text", "MONZA"),
            ("ui_text_placeholder", "AUTODROMO NAZIONALE"),
        ]);
        assert_eq!(ui_text_or_placeholder(&get), "MONZA");
    }

    /// Half-substituted text is not something any host wants on screen.
    #[test]
    fn partially_resolved_text_still_counts_as_unresolved() {
        let get = getter(&[
            ("ui_text", "LAP {lap_current}"),
            ("ui_text_placeholder", "LAP 12"),
        ]);
        assert_eq!(ui_text_or_placeholder(&get), "LAP 12");
    }

    /// No placeholder authored: draw nothing rather than a raw `{key}`, which
    /// reads as a real label and is the wrong width besides.
    #[test]
    fn an_unresolved_binding_with_no_placeholder_draws_nothing() {
        let get = getter(&[("ui_text", "{lap_total}")]);
        assert_eq!(ui_text_or_placeholder(&get), "");
    }

    /// Braces are not always a binding. Prose that happens to contain one must
    /// survive, or authored copy silently becomes an empty string.
    #[test]
    fn text_with_braces_that_are_not_a_binding_is_left_alone() {
        for literal in ["{}", "{ not a key }", "100% {", "a } b"] {
            let get = getter(&[("ui_text", literal)]);
            assert_eq!(
                ui_text_or_placeholder(&get),
                literal,
                "{literal:?} is not a binding and must draw as authored"
            );
        }
    }

    /// The reward-scrim bug: a full-screen modal gated on a live flag drew
    /// over the whole editor preview, because an unresolved `ui_visible`
    /// correctly defaults to visible.
    #[test]
    fn an_unresolved_visibility_flag_honours_its_placeholder() {
        let get = getter(&[
            ("ui_visible", "{card_visible}"),
            ("ui_visible_placeholder", "false"),
        ]);
        assert!(!ui_visible(&get), "the scrim must stay hidden in a preview");
    }

    /// Unchanged for every node that authors no placeholder: an unresolved
    /// flag still means visible, because showing too much beats showing
    /// nothing. 46 authored uses across the game's scenes rely on this.
    #[test]
    fn an_unresolved_flag_without_a_placeholder_stays_visible() {
        let get = getter(&[("ui_visible", "{founding}")]);
        assert!(ui_visible(&get));
    }

    /// A running game binds the flag, so the placeholder is never consulted —
    /// the same property that makes it safe to author.
    #[test]
    fn a_resolved_visibility_flag_ignores_the_placeholder() {
        let hidden = getter(&[("ui_visible", "false"), ("ui_visible_placeholder", "true")]);
        assert!(!ui_visible(&hidden));
        let shown = getter(&[("ui_visible", "true"), ("ui_visible_placeholder", "false")]);
        assert!(ui_visible(&shown));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_bindings_replaces_known_keys_and_leaves_the_rest() {
        let mut bindings = Bindings::new();
        bindings.insert("pos".to_string(), "3".to_string());
        bindings.insert("name".to_string(), "Reyes".to_string());

        assert_eq!(
            substitute_bindings("P{pos}  {name}", &bindings),
            "P3  Reyes"
        );
        // Unknown key: left as a literal placeholder rather than dropped —
        // silently vanishing text is worse than an obviously-wrong string.
        assert_eq!(substitute_bindings("{missing}", &bindings), "{missing}");
        // No `{` at all: returned unchanged with no allocation (Cow::Borrowed).
        assert!(matches!(
            substitute_bindings("plain text", &bindings),
            Cow::Borrowed("plain text")
        ));
        // Unterminated `{`: kept literal instead of eating the rest of the
        // string looking for a `}` that never comes.
        assert_eq!(substitute_bindings("a{b", &bindings), "a{b");
    }

    #[test]
    fn ui_origin_centres_a_node_on_its_anchor() {
        // `ui_anchor` puts a node's bottom-left corner on the anchor point, so
        // centring a fixed-size box otherwise means hand-offsetting `position`
        // by half its size — which cannot be done at all once the size is
        // data-bound (a selection bar sized to its label, say).
        let viewport = (-320.0, -240.0, 640.0, 480.0);
        let resolve = |props: &[(&str, &str)]| {
            let map: std::collections::HashMap<String, String> = props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            resolve_ui_rect(
                viewport,
                Vec2::ZERO,
                Vec2::ONE,
                0.0,
                |n| map.get(n).cloned(),
                0.0,
                None,
                (None, None),
            )
        };

        // Default (origin 0): bottom-left corner lands on the centre anchor.
        let corner = resolve(&[("ui_anchor", "center"), ("ui_w", "200"), ("ui_h", "40")]);
        assert_eq!((corner.0, corner.1), (0.0, 0.0));

        // origin 0.5 on both axes: the node's own centre lands there instead.
        let centred = resolve(&[
            ("ui_anchor", "center"),
            ("ui_w", "200"),
            ("ui_h", "40"),
            ("ui_origin_x", "0.5"),
            ("ui_origin_y", "0.5"),
        ]);
        assert_eq!((centred.0, centred.1), (-100.0, -20.0));
        assert_eq!((centred.2, centred.3), (200.0, 40.0), "size is unchanged");

        // origin 1: the far edge lands on the anchor.
        let far = resolve(&[
            ("ui_anchor", "center"),
            ("ui_w", "200"),
            ("ui_h", "40"),
            ("ui_origin_x", "1"),
            ("ui_origin_y", "1"),
        ]);
        assert_eq!((far.0, far.1), (-200.0, -40.0));

        // Stretch owns the axis outright, so an origin on that axis is inert
        // rather than shifting a node that has no fixed size to offset by.
        let stretched = resolve(&[
            ("ui_stretch_x", "true"),
            ("ui_origin_x", "0.5"),
            ("ui_h", "40"),
        ]);
        assert_eq!((stretched.0, stretched.2), (-320.0, 640.0));
    }

    #[test]
    fn ui_polyline_draws_a_segment_per_point_pair() {
        // E5: ui: "polyline" reads "x0,y0;x1,y1;..." offsets from the rect's
        // origin — no font needed, so this can run against a null-atlas
        // Canvas like the E1 tests do.
        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        let mut props = HashMap::new();
        props.insert("ui".to_string(), "polyline".to_string());
        props.insert("ui_points".to_string(), "0,0;10,0;10,10".to_string());
        props.insert("ui_color".to_string(), "255,0,0".to_string());

        assert_eq!(canvas.verts.len(), 0);
        draw_ui_kind(
            &mut canvas,
            (0.0, 0.0, 50.0, 50.0),
            Vec2::ONE,
            |n| props.get(n).cloned(),
            None,
            false,
        );
        // Two segments (3 points), 6 verts per segment quad at minimum.
        assert!(
            canvas.verts.len() >= 12,
            "expected at least 2 line segments worth of geometry, got {} verts",
            canvas.verts.len()
        );
    }

    #[test]
    fn ui_polyline_with_fewer_than_two_points_draws_nothing() {
        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        let mut props = HashMap::new();
        props.insert("ui".to_string(), "polyline".to_string());
        props.insert("ui_points".to_string(), "5,5".to_string());

        draw_ui_kind(
            &mut canvas,
            (0.0, 0.0, 50.0, 50.0),
            Vec2::ONE,
            |n| props.get(n).cloned(),
            None,
            false,
        );
        assert_eq!(canvas.verts.len(), 0);
    }

    #[test]
    fn ui_image_draws_only_when_a_sprite_is_present() {
        // E5: ui: "image" paints the node's own sprites[0] — already a
        // resolved TextureId — into the resolved rect. No font needed
        // (image_region doesn't touch text), so a null-atlas Canvas works.
        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        let mut props = HashMap::new();
        props.insert("ui".to_string(), "image".to_string());

        // No sprite: nothing to draw, no panic.
        draw_ui_kind(
            &mut canvas,
            (0.0, 0.0, 50.0, 50.0),
            Vec2::ONE,
            |n| props.get(n).cloned(),
            None,
            false,
        );
        assert_eq!(canvas.verts.len(), 0);

        let sprite = PrefabSprite2D {
            texture: TextureId(0),
            offset: Vec2::ZERO,
            size: Vec2::ONE,
            color: Color::WHITE,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
        };
        draw_ui_kind(
            &mut canvas,
            (0.0, 0.0, 50.0, 50.0),
            Vec2::ONE,
            |n| props.get(n).cloned(),
            Some(&sprite),
            false,
        );
        // image_region emits one quad (6 verts).
        assert_eq!(canvas.verts.len(), 6);
    }

    #[test]
    fn scene_instance_typed_property_helpers_parse_metadata() {
        let mut properties = HashMap::new();
        properties.insert("editor_visible".to_string(), "true".to_string());
        properties.insert("editor_node_id".to_string(), "42".to_string());
        properties.insert("editor_parent_id".to_string(), "7".to_string());
        properties.insert("editor_name".to_string(), "pit_wall".to_string());
        properties.insert("priority".to_string(), "-3".to_string());
        properties.insert("opacity".to_string(), "0.75".to_string());
        properties.insert("tags".to_string(), "hud, overlay, telemetry".to_string());
        properties.insert("script_path".to_string(), "scripts/pit_wall.rs".to_string());

        let instance = SceneInstance2D {
            prefab: "pit_panel".to_string(),
            position: Vec2::ZERO,
            scale: Vec2::new(1.0, 1.0),
            properties,
            sprites: Vec::new(),
        };

        assert_eq!(instance.editor_visible(), Some(true));
        assert_eq!(instance.editor_node_id(), Some(42));
        assert_eq!(instance.editor_parent_id(), Some(7));
        assert_eq!(instance.editor_name(), Some("pit_wall"));
        assert_eq!(instance.property_i64("priority"), Some(-3));
        assert_eq!(instance.property_f32("opacity"), Some(0.75));
        assert_eq!(
            instance.property_tags("tags"),
            vec!["hud", "overlay", "telemetry"]
        );
        assert!(instance.has_tag("overlay"));
        assert_eq!(instance.script_path(), Some("scripts/pit_wall.rs"));
    }

    #[test]
    fn scene_script_bindings_collect_only_instances_with_scripts() {
        let mut with_script = HashMap::new();
        with_script.insert("script_path".to_string(), "scripts/title.rs".to_string());
        with_script.insert("editor_node_id".to_string(), "1".to_string());
        with_script.insert("editor_name".to_string(), "title_root".to_string());
        with_script.insert("param_command".to_string(), "PushPace".to_string());
        with_script.insert("param_speed".to_string(), "120".to_string());
        with_script.insert("param_loop".to_string(), "true".to_string());
        with_script.insert("param_tint".to_string(), "200,40,40,255".to_string());

        let mut without_script = HashMap::new();
        without_script.insert("editor_node_id".to_string(), "2".to_string());

        let scene = Scene2D {
            instances: vec![
                SceneInstance2D {
                    prefab: "title_ui".to_string(),
                    position: Vec2::ZERO,
                    scale: Vec2::new(1.0, 1.0),
                    properties: with_script,
                    sprites: Vec::new(),
                },
                SceneInstance2D {
                    prefab: "decor".to_string(),
                    position: Vec2::ZERO,
                    scale: Vec2::new(1.0, 1.0),
                    properties: without_script,
                    sprites: Vec::new(),
                },
            ],
        };

        let bindings = scene.script_bindings();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].instance_index, 0);
        assert_eq!(bindings[0].prefab, "title_ui");
        assert_eq!(bindings[0].script_path, "scripts/title.rs");
        assert_eq!(bindings[0].editor_node_id, Some(1));
        assert_eq!(bindings[0].editor_name.as_deref(), Some("title_root"));

        // `param_<name>` properties are collected (prefix stripped) and read
        // back through the typed accessors; non-param props are excluded.
        let binding = &bindings[0];
        assert_eq!(binding.params.len(), 4);
        assert!(!binding.params.contains_key("command_path"));
        assert_eq!(binding.param("command"), Some("PushPace"));
        assert_eq!(binding.param_f32("speed"), Some(120.0));
        assert_eq!(binding.param_bool("loop"), Some(true));
        assert_eq!(
            binding.param_color("tint", Color::BLACK),
            Color::from_srgb8(200, 40, 40, 255)
        );
    }

    #[test]
    fn scene_can_lookup_instances_by_editor_metadata_and_tags() {
        let mut title_properties = HashMap::new();
        title_properties.insert("editor_node_id".to_string(), "1".to_string());
        title_properties.insert("editor_name".to_string(), "title_root".to_string());
        title_properties.insert("tags".to_string(), "menu, root".to_string());

        let mut hud_properties = HashMap::new();
        hud_properties.insert("editor_node_id".to_string(), "2".to_string());
        hud_properties.insert("editor_name".to_string(), "hud_panel".to_string());
        hud_properties.insert("tags".to_string(), "hud, overlay".to_string());

        let scene = Scene2D {
            instances: vec![
                SceneInstance2D {
                    prefab: "title_ui".to_string(),
                    position: Vec2::ZERO,
                    scale: Vec2::new(1.0, 1.0),
                    properties: title_properties,
                    sprites: Vec::new(),
                },
                SceneInstance2D {
                    prefab: "hud".to_string(),
                    position: Vec2::ZERO,
                    scale: Vec2::new(1.0, 1.0),
                    properties: hud_properties,
                    sprites: Vec::new(),
                },
            ],
        };

        assert_eq!(
            scene
                .instance_by_editor_name("title_root")
                .map(|instance| instance.prefab.as_str()),
            Some("title_ui")
        );
        assert_eq!(
            scene
                .instance_by_editor_node_id(2)
                .map(|instance| instance.prefab.as_str()),
            Some("hud")
        );

        let hud_tags: Vec<_> = scene
            .by_tag("hud")
            .map(|instance| instance.prefab.as_str())
            .collect();
        assert_eq!(hud_tags, vec!["hud"]);
    }

    /// A node of `kind` with `properties`, for the kind-derivation tests.
    fn typed_node(id: u64, kind: EditorSceneNodeKind, pairs: &[(&str, &str)]) -> EditorSceneNode {
        EditorSceneNode {
            id,
            parent: None,
            name: format!("node{id}"),
            kind,
            position: [0.0, 0.0],
            size: [10.0, 10.0],
            visible: true,
            script_path: String::new(),
            runtime_prefab: String::new(),
            asset_alias: String::new(),
            properties: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn compiled_properties(node: EditorSceneNode) -> HashMap<String, String> {
        let document = EditorSceneDocument { nodes: vec![node] };
        let def = scene_definition_from_editor_document(Path::new("<test>"), document).unwrap();
        def.instances.into_iter().next().unwrap().properties
    }

    #[test]
    fn typed_kinds_derive_their_own_ui_draw_kind() {
        // The point of the typed kinds: `kind` alone says what a node is, so
        // `ui` no longer has to be authored next to it.
        for (kind, expected) in [
            (EditorSceneNodeKind::Label, "text"),
            (EditorSceneNodeKind::Panel, "rect"),
            (EditorSceneNodeKind::Image, "image"),
            (EditorSceneNodeKind::Button, "button"),
            (EditorSceneNodeKind::List, "repeat"),
        ] {
            let props = compiled_properties(typed_node(1, kind, &[]));
            assert_eq!(
                props.get("ui").map(String::as_str),
                Some(expected),
                "{kind:?} should derive ui: {expected}"
            );
        }

        // Kinds that draw nothing of their own derive no `ui` at all.
        for kind in [
            EditorSceneNodeKind::Layout,
            EditorSceneNodeKind::Action,
            EditorSceneNodeKind::Empty,
        ] {
            let props = compiled_properties(typed_node(1, kind, &[]));
            assert!(props.get("ui").is_none(), "{kind:?} should derive no ui");
        }

        // Shape defers to ui_shape, since one kind covers several primitives.
        let props = compiled_properties(typed_node(
            1,
            EditorSceneNodeKind::Shape,
            &[("ui_shape", "circle")],
        ));
        assert_eq!(props.get("ui").map(String::as_str), Some("circle"));
    }

    #[test]
    fn an_authored_ui_property_still_wins_over_the_derived_one() {
        // What lets the legacy `UiRoot` + `ui: "..."` documents and the typed
        // ones coexist while the corpus migrates: derivation only fills in.
        let props = compiled_properties(typed_node(
            1,
            EditorSceneNodeKind::Label,
            &[("ui", "text_block")],
        ));
        assert_eq!(props.get("ui").map(String::as_str), Some("text_block"));

        // And a legacy UiRoot node derives nothing, so its authored ui stands.
        let props = compiled_properties(typed_node(
            1,
            EditorSceneNodeKind::UiRoot,
            &[("ui", "rect")],
        ));
        assert_eq!(props.get("ui").map(String::as_str), Some("rect"));
    }

    #[test]
    fn image_resolves_an_asset_alias_the_way_sprite_does() {
        // The doubling this removes: a textured node used to need *both*
        // `kind: Sprite` (to resolve the alias into a prefab sprite) and
        // `ui: "image"` (to draw it). `Image` does both.
        let mut node = typed_node(1, EditorSceneNodeKind::Image, &[]);
        node.asset_alias = "car_side".to_string();
        node.size = [100.0, 34.0];

        let document = EditorSceneDocument { nodes: vec![node] };
        let def = scene_definition_from_editor_document(Path::new("<test>"), document).unwrap();
        let prefab = def.prefabs.first().expect("one prefab");
        assert_eq!(prefab.sprites.len(), 1, "Image compiles a prefab sprite");
        assert_eq!(prefab.sprites[0].asset, "car_side");
    }

    #[test]
    fn a_buttons_bar_paints_only_when_a_bar_colour_is_authored() {
        // The composite's non-text half, which is what's new: the highlight
        // bar a hand-wired sibling `ui: "rect"` node used to provide.
        // (Text needs a real font atlas, which `Canvas::new(_, null)` has
        // not got — same constraint the ui_image test notes.)
        let draw = |pairs: &[(&str, &str)]| {
            let props: HashMap<String, String> = pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let mut canvas = Canvas::new((200, 100), std::ptr::null());
            draw_ui_kind(
                &mut canvas,
                (0.0, 0.0, 210.0, 38.0),
                Vec2::ONE,
                |n| props.get(n).cloned(),
                None,
                false,
            );
            canvas.verts.len()
        };

        // A bar colour paints one quad.
        assert_eq!(
            draw(&[("ui", "button"), ("ui_bar_color", "230,178,60,34")]),
            6
        );
        // No bar colour authored paints nothing — a bare Button is its label
        // only, not an opaque black box.
        assert_eq!(draw(&[("ui", "button")]), 0);
        // An explicitly transparent bar is also nothing, which is how the
        // idle state of a selection bar is authored.
        assert_eq!(
            draw(&[("ui", "button"), ("ui_bar_color", "230,178,60,0")]),
            0
        );
    }

    #[test]
    fn a_buttons_bar_colour_follows_the_interaction_state() {
        // The whole reason the four sibling nodes existed: each carried its
        // own `ui_color_hover`. Now one node's bar/marker/label each resolve
        // their own state variant.
        let props: HashMap<String, String> = [
            ("ui", "button"),
            ("ui_bar_color", "230,178,60,0"),
            ("ui_bar_color_hover", "230,178,60,34"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let hovered = UiInteractionState {
            hovered: true,
            ..Default::default()
        };
        let resolved =
            resolve_interaction_property(&|n: &str| props.get(n).cloned(), "ui_bar_color", hovered);
        assert_eq!(resolved.as_deref(), Some("230,178,60,34"));

        let idle = UiInteractionState::default();
        let resolved =
            resolve_interaction_property(&|n: &str| props.get(n).cloned(), "ui_bar_color", idle);
        assert_eq!(resolved.as_deref(), Some("230,178,60,0"));
    }

    #[test]
    fn any_property_can_carry_interaction_variants() {
        // This was an allowlist of three colour properties, so every property
        // added later — `ui_border_color`, `ui_shadow_color`, `ui_tracking` —
        // silently ignored its own `_focus`/`_hover`/`_press` variant. The
        // title screen's selected row authored `ui_border_color_focus` and
        // got nothing, which reads as a scene bug rather than an engine one.
        let props: HashMap<String, String> = [
            ("ui_border_color", "63,72,84,255"),
            ("ui_border_color_focus", "224,161,60,255"),
            ("ui_tracking", "2"),
            ("ui_tracking_hover", "4"),
            ("ui_shadow_color", "0,0,0,128"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let get = |n: &str| props.get(n).cloned();
        let resolve = |name: &str, state| resolve_interaction_property(&get, name, state);

        let focused = UiInteractionState {
            focused: true,
            ..Default::default()
        };
        let hovered = UiInteractionState {
            hovered: true,
            ..Default::default()
        };
        let idle = UiInteractionState::default();

        assert_eq!(
            resolve("ui_border_color", focused).as_deref(),
            Some("224,161,60,255"),
            "a border takes its focus variant"
        );
        assert_eq!(
            resolve("ui_border_color", idle).as_deref(),
            Some("63,72,84,255")
        );
        // Not just colours: anything a node reads through `get`.
        assert_eq!(resolve("ui_tracking", hovered).as_deref(), Some("4"));
        assert_eq!(resolve("ui_tracking", idle).as_deref(), Some("2"));
        // A property with no variant authored is unaffected in every state.
        assert_eq!(
            resolve("ui_shadow_color", focused).as_deref(),
            Some("0,0,0,128")
        );
        // And an absent property stays absent rather than resolving to a
        // variant of itself.
        assert_eq!(resolve("ui_radius", focused), None);
    }

    #[test]
    fn a_button_with_a_sprite_paints_it_instead_of_the_flat_bar() {
        // The opt-in: an authored asset_alias makes Button a textured panel
        // rather than a flat-colour one, for a button that's pixel art.
        let with_sprite = {
            let mut canvas = Canvas::new((200, 100), std::ptr::null());
            let mut props = HashMap::new();
            props.insert("ui".to_string(), "button".to_string());
            let sprite = PrefabSprite2D {
                texture: TextureId(0),
                offset: Vec2::ZERO,
                size: Vec2::ONE,
                color: Color::WHITE,
                uv_rect: [0.0, 0.0, 1.0, 1.0],
                flip_x: false,
                flip_y: false,
            };
            draw_ui_kind(
                &mut canvas,
                (0.0, 0.0, 210.0, 38.0),
                Vec2::ONE,
                |n| props.get(n).cloned(),
                Some(&sprite),
                false,
            );
            canvas.verts.len()
        };
        // image_region emits one quad, same as the plain "image" kind does.
        assert_eq!(with_sprite, 6);
    }

    #[test]
    fn button_kind_resolves_an_asset_alias_but_tolerates_its_absence() {
        // The opt-in must not be a trap: a Button authored with no texture
        // (the common case, a flat-colour bar) compiles fine — same
        // tolerance as `Image`, extended to `Button`.
        let props = compiled_properties(typed_node(1, EditorSceneNodeKind::Button, &[]));
        assert_eq!(props.get("ui").map(String::as_str), Some("button"));

        let mut node = typed_node(1, EditorSceneNodeKind::Button, &[]);
        node.asset_alias = "menu_panel".to_string();
        node.size = [210.0, 38.0];
        let document = EditorSceneDocument { nodes: vec![node] };
        let def = scene_definition_from_editor_document(Path::new("<test>"), document).unwrap();
        let prefab = def.prefabs.first().expect("one prefab");
        assert_eq!(
            prefab.sprites.len(),
            1,
            "an authored alias compiles a sprite"
        );
        assert_eq!(prefab.sprites[0].asset, "menu_panel");
    }

    #[test]
    fn a_button_node_derives_ui_so_it_is_hit_testable_by_its_own_name() {
        // Formula R hit-tests a button through `resolved_rect`, which is only
        // populated for nodes with a `ui` property. Button deriving its own
        // `ui` is what lets the game click the node it authored rather than
        // an inner bar node named by convention.
        let props = compiled_properties(typed_node(1, EditorSceneNodeKind::Button, &[]));
        assert_eq!(props.get("ui").map(String::as_str), Some("button"));
    }

    #[test]
    fn an_image_without_an_alias_yet_compiles_instead_of_failing_the_scene() {
        // Adding an Image in the editor must not blank the screen until a
        // texture is picked — compile_prefabs failing is scene-wide, so a
        // half-authored node would take everything else down with it.
        let props = compiled_properties(typed_node(1, EditorSceneNodeKind::Image, &[]));
        assert_eq!(props.get("ui").map(String::as_str), Some("image"));

        // A pure Sprite node stays strict: it has nothing else to be.
        let document = EditorSceneDocument {
            nodes: vec![typed_node(1, EditorSceneNodeKind::Sprite, &[])],
        };
        assert!(
            scene_definition_from_editor_document(Path::new("<test>"), document).is_err(),
            "an alias-less Sprite is still an error"
        );
    }

    #[test]
    fn converts_editor_scene_document_into_runtime_scene_definition() {
        let mut spawn_properties = HashMap::new();
        spawn_properties.insert("team".to_string(), "player".to_string());

        let document = EditorSceneDocument {
            nodes: vec![
                EditorSceneNode {
                    id: 1,
                    parent: None,
                    name: "player_spawn".to_string(),
                    kind: EditorSceneNodeKind::Empty,
                    position: [96.0, 288.0],
                    size: [88.0, 56.0],
                    visible: true,
                    script_path: "scripts/player_spawn.rs".to_string(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: spawn_properties,
                },
                EditorSceneNode {
                    id: 2,
                    parent: None,
                    name: "tree_cluster".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [128.0, 512.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 3,
                    parent: Some(2),
                    name: "tree".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [128.0, 512.0],
                    size: [32.0, 32.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "tree".to_string(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 4,
                    parent: Some(2),
                    name: "tree_highlight".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [160.0, 496.0],
                    size: [16.0, 16.0],
                    visible: false,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "gem".to_string(),
                    properties: HashMap::new(),
                },
            ],
        };

        let definition =
            scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
                .expect("editor scene should convert to a runtime scene definition");

        assert_eq!(definition.prefabs.len(), 2);
        assert_eq!(definition.instances.len(), 2);

        assert_eq!(
            definition.prefabs[1],
            Prefab2DDef {
                name: "tree_cluster".to_string(),
                sprites: vec![PrefabSprite2DDef {
                    asset: "tree".to_string(),
                    offset: [0.0, 0.0],
                    size: [32.0, 32.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    uv_rect: None,
                    flip_x: false,
                    flip_y: false,
                }],
            }
        );

        let spawn = &definition.instances[0];
        assert_eq!(spawn.prefab, "player_spawn");
        assert_eq!(spawn.properties.get("team"), Some(&"player".to_string()));
        assert_eq!(
            spawn.properties.get("script_path"),
            Some(&"scripts/player_spawn.rs".to_string())
        );
        assert_eq!(
            spawn.properties.get("editor_kind"),
            Some(&"Empty".to_string())
        );
        assert_eq!(
            spawn.properties.get("editor_size_x"),
            Some(&"88".to_string())
        );

        let tree_cluster = &definition.instances[1];
        assert_eq!(tree_cluster.prefab, "tree_cluster");
        assert_eq!(tree_cluster.position, [128.0, 512.0]);
    }

    #[test]
    fn rejects_conflicting_prefab_visuals_from_editor_scene_document() {
        let document = EditorSceneDocument {
            nodes: vec![
                EditorSceneNode {
                    id: 1,
                    parent: None,
                    name: "tree".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [0.0, 0.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 2,
                    parent: Some(1),
                    name: "tree".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [0.0, 0.0],
                    size: [32.0, 32.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "tree".to_string(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 3,
                    parent: None,
                    name: "tree".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [64.0, 64.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 4,
                    parent: Some(3),
                    name: "tree_glow".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [64.0, 64.0],
                    size: [16.0, 16.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "gem".to_string(),
                    properties: HashMap::new(),
                },
            ],
        };

        let error = scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
            .expect_err("prefab name reuse with different visuals should fail");

        assert!(
            error.to_string().contains("conflicting visual definitions"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn allows_equivalent_prefab_visuals_with_different_child_order() {
        let document = EditorSceneDocument {
            nodes: vec![
                EditorSceneNode {
                    id: 1,
                    parent: None,
                    name: "tree_cluster".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [0.0, 0.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 2,
                    parent: Some(1),
                    name: "tree".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [0.0, 0.0],
                    size: [32.0, 32.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "tree".to_string(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 3,
                    parent: Some(1),
                    name: "gem".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [16.0, -8.0],
                    size: [16.0, 16.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "gem".to_string(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 4,
                    parent: None,
                    name: "tree_cluster".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [64.0, 64.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 5,
                    parent: Some(4),
                    name: "gem".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [80.0, 56.0],
                    size: [16.0, 16.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "gem".to_string(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 6,
                    parent: Some(4),
                    name: "tree".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [64.0, 64.0],
                    size: [32.0, 32.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "tree".to_string(),
                    properties: HashMap::new(),
                },
            ],
        };

        let definition =
            scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
                .expect("equivalent prefab visuals should coalesce even if child order differs");

        assert_eq!(definition.prefabs.len(), 1);
        assert_eq!(definition.instances.len(), 2);
        assert_eq!(definition.prefabs[0].name, "tree_cluster");
        assert_eq!(definition.prefabs[0].sprites.len(), 2);
        assert_eq!(definition.prefabs[0].sprites[0].asset, "gem");
        assert_eq!(definition.prefabs[0].sprites[1].asset, "tree");
    }

    #[test]
    fn rejects_duplicate_editor_node_ids() {
        let document = EditorSceneDocument {
            nodes: vec![
                EditorSceneNode {
                    id: 1,
                    parent: None,
                    name: "first".to_string(),
                    kind: EditorSceneNodeKind::Empty,
                    position: [0.0, 0.0],
                    size: [88.0, 56.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 1,
                    parent: None,
                    name: "second".to_string(),
                    kind: EditorSceneNodeKind::Empty,
                    position: [64.0, 64.0],
                    size: [88.0, 56.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
            ],
        };

        let error = scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
            .expect_err("duplicate node ids should fail fast");

        assert!(
            error.to_string().contains("duplicate node id 1"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_dangling_editor_parent_ids() {
        let document = EditorSceneDocument {
            nodes: vec![EditorSceneNode {
                id: 1,
                parent: Some(99),
                name: "orphan".to_string(),
                kind: EditorSceneNodeKind::Sprite,
                position: [0.0, 0.0],
                size: [32.0, 32.0],
                visible: true,
                script_path: String::new(),
                runtime_prefab: String::new(),
                asset_alias: "tree".to_string(),
                properties: HashMap::new(),
            }],
        };

        let error = scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
            .expect_err("missing parent references should fail fast");

        assert!(
            error.to_string().contains("missing parent 99"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_self_parenting_editor_nodes() {
        let document = EditorSceneDocument {
            nodes: vec![EditorSceneNode {
                id: 1,
                parent: Some(1),
                name: "loop".to_string(),
                kind: EditorSceneNodeKind::Empty,
                position: [0.0, 0.0],
                size: [88.0, 56.0],
                visible: true,
                script_path: String::new(),
                runtime_prefab: String::new(),
                asset_alias: String::new(),
                properties: HashMap::new(),
            }],
        };

        let error = scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
            .expect_err("self-parenting should fail fast");

        assert!(
            error.to_string().contains("cannot parent itself"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_editor_parent_cycles() {
        let document = EditorSceneDocument {
            nodes: vec![
                EditorSceneNode {
                    id: 1,
                    parent: Some(2),
                    name: "first".to_string(),
                    kind: EditorSceneNodeKind::Empty,
                    position: [0.0, 0.0],
                    size: [88.0, 56.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 2,
                    parent: Some(1),
                    name: "second".to_string(),
                    kind: EditorSceneNodeKind::Empty,
                    position: [64.0, 64.0],
                    size: [88.0, 56.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
            ],
        };

        let error = scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
            .expect_err("parent cycles should fail fast");

        assert!(
            error.to_string().contains("parent cycle"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn collects_group_prefab_sprites_through_empty_descendants() {
        let document = EditorSceneDocument {
            nodes: vec![
                EditorSceneNode {
                    id: 1,
                    parent: None,
                    name: "crate_stack".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [128.0, 256.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 2,
                    parent: Some(1),
                    name: "anchor".to_string(),
                    kind: EditorSceneNodeKind::Empty,
                    position: [140.0, 268.0],
                    size: [88.0, 56.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 3,
                    parent: Some(2),
                    name: "crate".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [156.0, 280.0],
                    size: [32.0, 32.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "crate".to_string(),
                    properties: HashMap::new(),
                },
            ],
        };

        let definition =
            scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
                .expect("descendant sprites under non-group nodes should be collected");

        let group_prefab = definition
            .prefabs
            .iter()
            .find(|prefab| prefab.name == "crate_stack")
            .expect("group prefab should be present");

        assert_eq!(group_prefab.sprites.len(), 1);
        assert_eq!(group_prefab.sprites[0].asset, "crate");
        assert_eq!(group_prefab.sprites[0].offset, [28.0, 24.0]);
    }

    #[test]
    fn nested_groups_export_as_separate_instances() {
        let document = EditorSceneDocument {
            nodes: vec![
                EditorSceneNode {
                    id: 1,
                    parent: None,
                    name: "wagon".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [100.0, 100.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 2,
                    parent: Some(1),
                    name: "wagon_body".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [100.0, 100.0],
                    size: [48.0, 32.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "tree".to_string(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 3,
                    parent: Some(1),
                    name: "wagon_lantern".to_string(),
                    kind: EditorSceneNodeKind::Group,
                    position: [124.0, 84.0],
                    size: [120.0, 72.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: String::new(),
                    properties: HashMap::new(),
                },
                EditorSceneNode {
                    id: 4,
                    parent: Some(3),
                    name: "lantern_glow".to_string(),
                    kind: EditorSceneNodeKind::Sprite,
                    position: [124.0, 84.0],
                    size: [16.0, 16.0],
                    visible: true,
                    script_path: String::new(),
                    runtime_prefab: String::new(),
                    asset_alias: "gem".to_string(),
                    properties: HashMap::new(),
                },
            ],
        };

        let definition =
            scene_definition_from_editor_document(Path::new("editor.scene.json"), document)
                .expect("nested groups should export as separate prefab instances");

        assert_eq!(definition.prefabs.len(), 2);
        assert_eq!(definition.instances.len(), 2);

        assert_eq!(definition.prefabs[0].sprites.len(), 1);
        assert_eq!(definition.prefabs[1].sprites.len(), 1);
        assert_eq!(definition.prefabs[0].name, "wagon");
        assert_eq!(definition.prefabs[1].name, "wagon_lantern");
    }

    #[test]
    fn ui_font_selects_a_font_and_defaults_to_zero() {
        // E-C: `ui_font` is a numeric FontId the host publishes as a binding.
        // Absent or junk must fall back to font 0, or every scene authored
        // before E-C would start measuring against a face it never asked for.
        let font = |props: &[(&str, &str)]| {
            let map: HashMap<String, String> = props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            node_font(&|n: &str| map.get(n).cloned())
        };
        assert_eq!(font(&[]), FontId::DEFAULT);
        assert_eq!(font(&[("ui_font", "0")]), FontId::DEFAULT);
        assert_eq!(font(&[("ui_font", "2")]), FontId(2));
        // Whitespace is tolerated the way every other numeric ui_* prop is.
        assert_eq!(font(&[("ui_font", " 1 ")]), FontId(1));
        // A binding that failed to resolve leaves the literal `{font_hud}`;
        // rendering in the default face beats panicking mid-frame.
        assert_eq!(font(&[("ui_font", "{font_hud}")]), FontId::DEFAULT);
    }

    #[test]
    fn ui_border_is_opt_in_and_per_side() {
        // A border on any kind, so the mockups' 97 `border:2px solid` panels
        // stay one node each instead of becoming a fill plus four edge rects.
        let border = |props: &[(&str, &str)], scale: Vec2| {
            let map: HashMap<String, String> = props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            node_border(&|n: &str| map.get(n).cloned(), scale)
        };
        let one = Vec2::new(1.0, 1.0);

        // Nothing authored: every existing scene must be untouched.
        assert!(border(&[], one).is_none());
        // A width with no colour still draws (white default), but a colour
        // with no width must not — otherwise `ui_border_color` alone, the
        // easy authoring slip, would paint the node's whole rect.
        assert!(border(&[("ui_border_color", "63,72,84,255")], one).is_none());

        let (color, widths) = border(
            &[("ui_border_w", "2"), ("ui_border_color", "63,72,84,255")],
            one,
        )
        .expect("a width and a colour is a border");
        assert_eq!(widths, [2.0, 2.0, 2.0, 2.0]);
        assert_eq!(color.to_srgb8(), (63, 72, 84, 255));

        // One side overrides `ui_border_w`; the rest keep it. This is what
        // makes a divider rule a border rather than a second node.
        let (_, widths) = border(
            &[
                ("ui_border_w", "2"),
                ("ui_border_top", "0"),
                ("ui_border_color", "63,72,84,255"),
            ],
            one,
        )
        .expect("partial border");
        assert_eq!(widths, [2.0, 2.0, 2.0, 0.0]);

        // A lone side with no `ui_border_w` is the `border-top:3px` case.
        let (_, widths) = border(
            &[("ui_border_top", "3"), ("ui_border_color", "63,72,84,255")],
            one,
        )
        .expect("one-sided border");
        assert_eq!(widths, [0.0, 0.0, 0.0, 3.0]);

        // Vertical edges scale in x, horizontal in y — a stretched node gets
        // an even outline, not a lopsided one.
        let (_, widths) = border(
            &[("ui_border_w", "2"), ("ui_border_color", "63,72,84,255")],
            Vec2::new(3.0, 5.0),
        )
        .expect("scaled border");
        assert_eq!(widths, [6.0, 6.0, 10.0, 10.0]);

        // A fully transparent border is nothing to draw.
        assert!(border(
            &[("ui_border_w", "2"), ("ui_border_color", "63,72,84,0")],
            one
        )
        .is_none());
    }

    #[test]
    fn hatch_lines_cover_the_rect_at_45_degrees() {
        // `repeating-linear-gradient(45deg, X 0 5px, Y 5px 10px)` — a 10px
        // pitch of 45° stripes, which is the mockups' placeholder plate.
        let rect = (0.0, 0.0, 100.0, 50.0);
        let lines: Vec<_> = hatch_lines(rect, 10.0, 150.0).collect();
        // Every stripe is exactly 45°: equal run and rise.
        for (x0, y0, x1, y1) in &lines {
            assert!(
                ((x1 - x0) - (y1 - y0)).abs() < 1e-3,
                "45 degrees: {:?}",
                (x0, y0, x1, y1)
            );
        }
        // Starting a full height to the left so the bottom-left corner is
        // covered.
        assert!((lines[0].0 - -50.0).abs() < 1e-3, "first x0: {}", lines[0].0);
        // The CSS period is perpendicular to the stripes, so the *horizontal*
        // spacing is pitch * sqrt(2). Measuring the true distance between two
        // parallel 45° lines from their x-intercepts gives the pitch back.
        let dx = lines[1].0 - lines[0].0;
        assert!(
            (dx / std::f32::consts::SQRT_2 - 10.0).abs() < 1e-3,
            "10px perpendicular period, {dx} horizontal"
        );
        // The last stripe starts at or past the right edge, so the top-right
        // corner is covered too.
        assert_eq!(lines.len(), 11);
        assert!(
            lines.last().unwrap().0 >= 100.0 - dx,
            "reaches the right edge: {}",
            lines.last().unwrap().0
        );
        // A zero-area rect asks for nothing rather than looping forever.
        assert_eq!(hatch_lines((0.0, 0.0, 0.0, 0.0), 10.0, 0.0).count(), 0);
    }

    #[test]
    fn a_hatch_needs_only_its_colour() {
        // `ui_hatch_color` alone is a full hatch: the stripe and gap default to
        // the mockups' 5px, so the common case is one property.
        let props: HashMap<&str, &str> = HashMap::from([("ui_hatch_color", "34,39,47,255")]);
        let get = |n: &str| props.get(n).map(|v| v.to_string());
        let (color, pitch, stripe) = node_hatch(&get, Vec2::new(1.0, 1.0)).expect("hatched");
        assert_eq!(color.to_srgb8(), (34, 39, 47, 255));
        assert!((pitch - 10.0).abs() < 1e-3, "5px stripe + 5px gap");
        assert!((stripe - 5.0).abs() < 1e-3);
        // No colour, no hatch — and no cost beyond one failed lookup.
        let none = |_: &str| None;
        assert!(node_hatch(&none, Vec2::new(1.0, 1.0)).is_none());
        // A fully transparent hatch is nothing to draw, like an invisible border.
        let clear: HashMap<&str, &str> = HashMap::from([("ui_hatch_color", "34,39,47,0")]);
        let get_clear = |n: &str| clear.get(n).map(|v| v.to_string());
        assert!(node_hatch(&get_clear, Vec2::new(1.0, 1.0)).is_none());
    }

    #[test]
    fn border_rects_inset_and_cover_each_corner_once() {
        let rect = (0.0, 0.0, 100.0, 50.0);
        let rects = border_rects(rect, [2.0, 2.0, 2.0, 2.0]);
        // Verticals span the full height; horizontals fill only between them,
        // so no pixel is painted twice.
        assert_eq!(
            rects,
            vec![
                (0.0, 0.0, 2.0, 50.0),
                (98.0, 0.0, 2.0, 50.0),
                (2.0, 0.0, 96.0, 2.0),
                (2.0, 48.0, 96.0, 2.0),
            ]
        );
        // Every edge stays inside the node's own rect (`box-sizing:
        // border-box`) — a bordered panel is the size it says it is.
        let (x, y, w, h) = rect;
        for (rx, ry, rw, rh) in &rects {
            assert!(*rx >= x && *ry >= y && rx + rw <= x + w && ry + rh <= y + h);
        }
        // Total area == perimeter band, which is only true with no overlap.
        let painted: f32 = rects.iter().map(|(_, _, rw, rh)| rw * rh).sum();
        assert_eq!(painted, 100.0 * 50.0 - 96.0 * 46.0);

        // A one-sided rule is one rect, not four.
        assert_eq!(
            border_rects(rect, [0.0, 0.0, 0.0, 3.0]),
            vec![(0.0, 47.0, 100.0, 3.0)]
        );
        // A border thicker than the node stays inside it rather than the
        // opposing sides overdrawing past each other.
        for (rx, ry, rw, rh) in border_rects((0.0, 0.0, 4.0, 4.0), [10.0, 10.0, 10.0, 10.0]) {
            assert!(rx >= 0.0 && ry >= 0.0 && rx + rw <= 4.0 && ry + rh <= 4.0);
        }
    }
}

#[cfg(test)]
mod polygon_kind_tests {
    use super::*;
    use std::collections::HashMap;

    fn getter(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    /// Authored points are fractions of the node's own box, so a shape drawn
    /// once fits whatever size it resolves to.
    ///
    /// Compared against the same triangle drawn in absolute pixels: counting
    /// vertices would pass even if the fractions were used as pixels.
    #[test]
    fn points_are_read_as_fractions_of_the_node() {
        let get = getter(&[("ui", "polygon"), ("ui_points", "0,0 1,0 0.5,1")]);
        let mut authored = Canvas::for_test((400, 400));
        draw_ui_kind(
            &mut authored,
            (20.0, 10.0, 100.0, 50.0),
            Vec2::ONE,
            &get,
            None,
            false,
        );

        // The same triangle, in the pixels the fractions must resolve to.
        let mut expected = Canvas::for_test((400, 400));
        expected.polygon(
            &[(20.0, 10.0), (120.0, 10.0), (70.0, 60.0)],
            Color::from_srgb8(255, 255, 255, 255),
        );

        let positions = |c: &Canvas| {
            let mut p: Vec<[i32; 2]> = c
                .vertices()
                .iter()
                .map(|v| [(v.position[0] * 1e4) as i32, (v.position[1] * 1e4) as i32])
                .collect();
            p.sort_unstable();
            p
        };
        assert_eq!(positions(&authored), positions(&expected));
    }

    /// A malformed list draws nothing rather than panicking — a half-typed
    /// shape is a normal state while authoring one.
    #[test]
    fn a_malformed_point_list_is_survivable() {
        for points in ["", "garbage", "0,0", "0,0 1,0", "0,0 1 0.5,1", "a,b c,d e,f"] {
            let get = getter(&[("ui", "polygon"), ("ui_points", points)]);
            let mut canvas = Canvas::for_test((400, 400));
            draw_ui_kind(&mut canvas, (0.0, 0.0, 10.0, 10.0), Vec2::ONE, &get, None, false);
            assert!(
                canvas.vertices().len() % 3 == 0,
                "{points:?} emitted a partial triangle"
            );
        }
    }
}
