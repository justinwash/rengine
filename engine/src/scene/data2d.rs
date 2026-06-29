use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::{AssetError, AssetPack, Color};
use crate::canvas::Canvas;
use crate::renderer::{DrawParams, Frame};
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
    /// - `ui_w`, `ui_h`: size, multiplied by the instance scale
    /// - `ui_w_frac`, `ui_h_frac`: size as a fraction of the viewport (overrides
    ///   `ui_w`/`ui_h`), e.g. `ui_w_frac: 1.0` spans the full width
    /// - `ui_color`: `"r,g,b,a"` sRGB for `rect`/`text`
    /// - `ui_color_bottom`, `ui_color_top`: gradient ends / bevel shadow+highlight
    /// - `ui_radius`: corner radius for `rect`
    /// - `ui_line_w`: edge thickness for `bevel`
    /// - `ui_text`, `ui_text_size`: text contents and size
    pub fn draw_ui_primitive(&self, frame: &mut Frame, time: f32) {
        if self.property("ui").is_none() {
            return;
        }
        let (sw, sh) = frame.canvas(0).screen_size();
        let screen = (-(sw as f32) / 2.0, -(sh as f32) / 2.0, sw as f32, sh as f32);
        draw_ui_node(frame, screen, self.position, self.scale, time, |n| {
            self.property(n)
        });
    }
}

/// Resolve a UI node's rect from its `ui_*` properties against a `reference`
/// rect — the viewport for root nodes, or the parent's resolved rect for
/// children. `reference` is `(x, y, w, h)` with `(x, y)` the bottom-left corner
/// in canvas coords (centred, y-up); anchors/fractions/stretch/animation are all
/// relative to it, so nesting and screen-edge layout share one model.
fn resolve_ui_rect<'a>(
    reference: (f32, f32, f32, f32),
    position: Vec2,
    scale: Vec2,
    time: f32,
    get: impl Fn(&str) -> Option<&'a str>,
) -> (f32, f32, f32, f32) {
    let prop_f32 = |n: &str| get(n).and_then(|v| v.trim().parse::<f32>().ok());
    let prop_bool = |n: &str| matches!(get(n).map(str::trim), Some("true" | "1" | "yes"));

    let (rx, ry, rw, rh) = reference;
    let w_fixed = match prop_f32("ui_w_frac") {
        Some(f) => rw * f,
        None => prop_f32("ui_w").unwrap_or(0.0) * scale.x,
    };
    let h_fixed = match prop_f32("ui_h_frac") {
        Some(f) => rh * f,
        None => prop_f32("ui_h").unwrap_or(0.0) * scale.y,
    };
    // Named anchor (shorthand) or exact `ui_anchor_frac_x`/`_y` (0..1, Godot-style).
    let (mut ax, mut ay) = match get("ui_anchor").unwrap_or("center") {
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
    let (x, w) = if prop_bool("ui_stretch_x") {
        let ml = prop_f32("ui_margin_left").unwrap_or(0.0);
        let mr = prop_f32("ui_margin_right").unwrap_or(0.0);
        (rx + ml, (rw - ml - mr).max(0.0))
    } else {
        (ax + position.x, w_fixed)
    };
    let (y, h) = if prop_bool("ui_stretch_y") {
        let mb = prop_f32("ui_margin_bottom").unwrap_or(0.0);
        let mt = prop_f32("ui_margin_top").unwrap_or(0.0);
        (ry + mb, (rh - mb - mt).max(0.0))
    } else {
        (ay + position.y, h_fixed)
    };
    // Optional idle animation: sinusoidal bob (y) / sway (x) with a per-node phase.
    let phase = prop_f32("ui_phase").unwrap_or(0.0);
    let bob = prop_f32("ui_bob_amp").map_or(0.0, |amp| {
        (time * prop_f32("ui_bob_speed").unwrap_or(1.0) + phase).sin() * amp
    });
    let sway = prop_f32("ui_sway_amp").map_or(0.0, |amp| {
        (time * prop_f32("ui_sway_speed").unwrap_or(1.0) + phase).cos() * amp
    });
    (x + sway, y + bob, w, h)
}

/// Draw the `ui` primitive named by a node's props into the resolved `rect`.
fn draw_ui_kind<'a>(
    canvas: &mut Canvas,
    rect: (f32, f32, f32, f32),
    scale: Vec2,
    get: impl Fn(&str) -> Option<&'a str>,
) {
    let Some(kind) = get("ui") else {
        return;
    };
    let (x, y, w, h) = rect;
    let prop_f32 = |n: &str| get(n).and_then(|v| v.trim().parse::<f32>().ok());
    let prop_i64 = |n: &str| get(n).and_then(|v| v.trim().parse::<i64>().ok());
    match kind {
        "rect" => {
            let color = parse_srgb_color(get("ui_color"), Color::WHITE);
            let radius = prop_f32("ui_radius").unwrap_or(0.0);
            if radius > 0.5 {
                canvas.rounded_rect(x, y, w, h, radius, color);
            } else {
                canvas.rect(x, y, w, h, color);
            }
        }
        "gradient" => {
            let bottom = parse_srgb_color(get("ui_color_bottom"), Color::BLACK);
            let top = parse_srgb_color(get("ui_color_top"), Color::WHITE);
            canvas.rect_gradient(x, y, w, h, bottom, top);
        }
        "bevel" => {
            let highlight = parse_srgb_color(get("ui_color_top"), Color::WHITE);
            let shadow = parse_srgb_color(get("ui_color_bottom"), Color::BLACK);
            let line_w = prop_f32("ui_line_w").unwrap_or(1.5);
            canvas.bevel_rect(x, y, w, h, highlight, shadow, line_w);
        }
        "circle" => {
            let color = parse_srgb_color(get("ui_color"), Color::WHITE);
            let radius = prop_f32("ui_radius").unwrap_or(4.0) * scale.x;
            let segments = prop_i64("ui_segments").unwrap_or(20).clamp(3, 96) as u32;
            canvas.circle_filled(x, y, radius, segments, color);
        }
        "line" => {
            let color = parse_srgb_color(get("ui_color"), Color::WHITE);
            let line_w = prop_f32("ui_line_w").unwrap_or(1.0);
            canvas.line(x, y, x + w, y + h, line_w, color);
        }
        "text" => {
            let color = parse_srgb_color(get("ui_color"), Color::WHITE);
            let size = prop_f32("ui_text_size").unwrap_or(12.0);
            let text = get("ui_text").unwrap_or("");
            canvas.text(x, y, text, size, color);
        }
        _ => {}
    }
}

/// Resolve + draw a UI node onto its `ui_layer` canvas in `frame`; returns the
/// resolved rect for child layout.
pub(crate) fn draw_ui_node<'a>(
    frame: &mut Frame,
    reference: (f32, f32, f32, f32),
    position: Vec2,
    scale: Vec2,
    time: f32,
    get: impl Fn(&str) -> Option<&'a str>,
) -> (f32, f32, f32, f32) {
    let rect = resolve_ui_rect(reference, position, scale, time, &get);
    if get("ui").is_some() {
        let layer = get("ui_layer")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0)
            .max(0) as usize;
        draw_ui_kind(frame.canvas(layer), rect, scale, &get);
    }
    rect
}

/// Resolve + draw a UI node directly onto a caller-owned `canvas` (ignoring
/// `ui_layer`), so a whole scene can be drawn into an existing canvas at a
/// chosen z-position. Returns the resolved rect for child layout.
pub(crate) fn draw_ui_node_on<'a>(
    canvas: &mut Canvas,
    reference: (f32, f32, f32, f32),
    position: Vec2,
    scale: Vec2,
    time: f32,
    get: impl Fn(&str) -> Option<&'a str>,
) -> (f32, f32, f32, f32) {
    let rect = resolve_ui_rect(reference, position, scale, time, &get);
    draw_ui_kind(canvas, rect, scale, &get);
    rect
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
        let json_value: serde_json::Value =
            serde_json::from_str(&text).map_err(|source| AssetError::Json {
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

#[derive(Debug, Clone, Deserialize)]
struct EditorSceneDocumentDef {
    #[serde(default)]
    nodes: Vec<EditorSceneNodeDef>,
}

#[derive(Debug, Clone, Deserialize)]
struct EditorSceneNodeDef {
    id: u64,
    #[serde(default)]
    parent: Option<u64>,
    #[serde(default)]
    name: String,
    kind: EditorSceneNodeKind,
    #[serde(default)]
    position: [f32; 2],
    #[serde(default = "default_editor_size")]
    size: [f32; 2],
    #[serde(default = "default_editor_visible")]
    visible: bool,
    #[serde(default)]
    script_path: String,
    #[serde(default)]
    runtime_prefab: String,
    #[serde(default)]
    asset_alias: String,
    #[serde(default)]
    properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum EditorSceneNodeKind {
    Group,
    Empty,
    Camera2d,
    Sprite,
    Trigger,
    UiRoot,
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
        }
    }
}

fn scene_definition_from_json(
    path: &Path,
    json_value: serde_json::Value,
) -> Result<Scene2DDef, AssetError> {
    if json_value.get("nodes").is_some() {
        let document: EditorSceneDocumentDef =
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
    document: EditorSceneDocumentDef,
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
    nodes: &[EditorSceneNodeDef],
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
    nodes: &[EditorSceneNodeDef],
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

fn build_editor_child_ids(nodes: &[EditorSceneNodeDef]) -> HashMap<u64, Vec<u64>> {
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
    node: &EditorSceneNodeDef,
    nodes: &[EditorSceneNodeDef],
    node_indices: &HashMap<u64, usize>,
) -> bool {
    match node.kind {
        EditorSceneNodeKind::Group => true,
        EditorSceneNodeKind::Sprite => {
            nearest_group_ancestor(node.parent, nodes, node_indices).is_none()
        }
        _ => true,
    }
}

fn nearest_group_ancestor(
    mut node_id: Option<u64>,
    nodes: &[EditorSceneNodeDef],
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

fn editor_runtime_prefab_name(
    path: &Path,
    node: &EditorSceneNodeDef,
) -> Result<String, AssetError> {
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
    node: &EditorSceneNodeDef,
    prefab_name: &str,
    nodes: &[EditorSceneNodeDef],
    node_indices: &HashMap<u64, usize>,
    child_ids: &HashMap<u64, Vec<u64>>,
) -> Result<Prefab2DDef, AssetError> {
    if node.kind == EditorSceneNodeKind::Group {
        return Ok(Prefab2DDef {
            name: prefab_name.to_string(),
            sprites: group_prefab_sprites(path, node, nodes, node_indices, child_ids)?,
        });
    }

    if node.kind != EditorSceneNodeKind::Sprite {
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
    root: &EditorSceneNodeDef,
    nodes: &[EditorSceneNodeDef],
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
    root: &EditorSceneNodeDef,
    parent_id: u64,
    nodes: &[EditorSceneNodeDef],
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

        if child.kind == EditorSceneNodeKind::Sprite && child.visible {
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
    node: &EditorSceneNodeDef,
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

fn editor_instance_properties(node: &EditorSceneNodeDef) -> HashMap<String, String> {
    let mut properties = node.properties.clone();

    properties
        .entry("editor_id".to_string())
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
mod tests {
    use super::*;

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

    #[test]
    fn converts_editor_scene_document_into_runtime_scene_definition() {
        let mut spawn_properties = HashMap::new();
        spawn_properties.insert("team".to_string(), "player".to_string());

        let document = EditorSceneDocumentDef {
            nodes: vec![
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
        let document = EditorSceneDocumentDef {
            nodes: vec![
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
                EditorSceneNodeDef {
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
}
