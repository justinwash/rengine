//! Mutable runtime scene graph for 2D games.
//!
//! [`Scene2D`] is intentionally immutable render data: it is what the asset
//! pipeline loads and what the static [`Scene2D::draw`] path renders. That is
//! the right contract for passive content, but it cannot express *gameplay* —
//! scripts have no way to move, hide, retag, spawn, or despawn nodes at runtime.
//!
//! [`SceneWorld2D`] closes that gap. It is built from a loaded [`Scene2D`] and
//! owns a live, mutable node graph addressed through stable generational
//! [`NodeHandle2D`]s. Handles survive despawns safely: once a node is removed,
//! any handle pointing at its slot is permanently invalidated, so stale handles
//! resolve to `None` instead of silently aliasing a different node.
//!
//! This is the foundation the script host builds on: a script can hold a
//! handle across frames and mutate the node it refers to without re-parsing
//! string property maps every tick.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use crate::canvas::Canvas;
use crate::layout::{Justify, Track};
use crate::renderer::{DrawParams, Frame};
use crate::{Rect, Vec2};

use super::anim2d::{sample_track, AnimatedProperty, SceneAnimClip};
use crate::assets::Color;
use super::data2d::{parse_bool_property, Bindings, PrefabSprite2D, RepeaterSources, Scene2D};

/// The node property that names a nested scene to expand from a [`SceneLibrary`].
pub const NESTED_SCENE_PROPERTY: &str = "nested_scene";

/// Maximum nested-scene expansion depth, a backstop against runaway recursion
/// even when the per-path cycle guard would otherwise catch a loop.
const MAX_NESTED_SCENE_DEPTH: usize = 32;

/// A name-addressed collection of loaded scenes used to expand nested-scene
/// references (a node carrying a [`NESTED_SCENE_PROPERTY`] property whose value
/// is a scene alias). Build it once from your loaded assets, then pass it to
/// [`SceneWorld2D::instantiate_scene_tree`].
#[derive(Default)]
pub struct SceneLibrary {
    scenes: HashMap<String, Scene2D>,
}

impl SceneLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, alias: impl Into<String>, scene: Scene2D) {
        self.scenes.insert(alias.into(), scene);
    }

    pub fn get(&self, alias: &str) -> Option<&Scene2D> {
        self.scenes.get(alias)
    }

    pub fn contains(&self, alias: &str) -> bool {
        self.scenes.contains_key(alias)
    }

    pub fn len(&self) -> usize {
        self.scenes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scenes.is_empty()
    }
}

/// A stable reference to a node inside a [`SceneWorld2D`].
///
/// Handles are generational: each slot tracks a generation counter that is
/// bumped on despawn, so a handle from a previous occupant never resolves to a
/// newer node that reused the same slot. Handles are cheap `Copy` values safe
/// to store in script state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle2D {
    index: u32,
    generation: u32,
}

/// Local transform of a node: position, rotation (radians), and scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl Transform2D {
    pub fn from_position(position: Vec2) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    /// Compose `self` (parent) with a child's local transform, producing the
    /// child's transform in the parent's space.
    fn compose(&self, child: &Transform2D) -> Transform2D {
        Transform2D {
            position: self.position + rotate_vec(child.position * self.scale, self.rotation),
            rotation: self.rotation + child.rotation,
            scale: self.scale * child.scale,
        }
    }
}

/// A single live node in the runtime scene graph.
///
/// A node carries authoring identity (name, source prefab, script path, editor
/// ids), mutable runtime state (transform, visibility, tags, properties),
/// optional sprite layers used for rendering, and hierarchy links.
#[derive(Debug, Clone)]
pub struct SceneNode2D {
    name: Option<String>,
    prefab: String,
    script_path: Option<String>,
    editor_node_id: Option<u64>,
    transform: Transform2D,
    visible: bool,
    tags: Vec<String>,
    properties: HashMap<String, String>,
    sprites: Vec<PrefabSprite2D>,
    parent: Option<NodeHandle2D>,
    children: Vec<NodeHandle2D>,
    /// The `ui_*`-resolved rect from the most recent draw, if this node has a
    /// `ui` property. Populated by `draw_node`/`draw_node_on_canvas` so
    /// `node_bounds`/`resolved_rect` can return the same rect the node was
    /// drawn at instead of recomputing layout with different math. `Cell`
    /// because draw takes `&self` — this is a cache, not observable state.
    ui_rect: Cell<Option<Rect>>,
    /// The `(w, h)` a `ui_size: "content"` node measured itself to fit its
    /// children, from the measure pre-pass `SceneWorld2D::measure_content_size`
    /// runs just before each draw. `None` for every node that sizes itself
    /// from a literal/bound `ui_w`/`ui_h` instead (the vast majority — this
    /// is opt-in). `Cell` for the same reason as `ui_rect`: a draw-time
    /// cache, not observable state, so draw can stay `&self`.
    content_size: Cell<Option<(f32, f32)>>,
    /// Size assigned to this node by its parent's flow layout, per axis:
    /// `(w, h)`, each `None` unless the flow decided it.
    ///
    /// Needed because a node resolves its own rect from the slot it is handed,
    /// and a slot is only a *reference*: without this, `ui_grow: "1"` would
    /// widen the slot while the node itself still resolved to its (absent)
    /// `ui_w` of zero, and every growing panel would draw nothing. Making
    /// `ui_grow` imply the fill is the whole point — needing to pair it with
    /// `ui_stretch_x` would be one more two-property gotcha to remember.
    ///
    /// The same argument covers the *cross* axis under `ui_align` (E-B): a
    /// non-stretch alignment sizes the slot to the child's own extent and
    /// places it, so the child must take that slot rather than re-anchoring
    /// inside it — otherwise `ui_align: center` needs a paired
    /// `ui_origin_x: 0.5` on every child, which is exactly the two-property
    /// gotcha this field exists to avoid.
    ///
    /// `Cell` for the same reason as `content_size`: a draw-time cache.
    flow_size: Cell<(Option<f32>, Option<f32>)>,
    /// Per-instance binding overlay (E3 repeaters): set by
    /// `SceneWorld2D::sync_repeaters` on each cloned instance root, merged
    /// over the ambient `Bindings` when drawing that instance, so
    /// `ui_text: "P{pos}"` resolves against *this* item. `None` for every
    /// ordinary, non-repeated node.
    instance_bindings: Option<Bindings>,
}

impl SceneNode2D {
    /// Create a bare logical node with no sprites, useful for spawn points,
    /// markers, and script-driven entities.
    pub fn new(prefab: impl Into<String>) -> Self {
        Self {
            name: None,
            prefab: prefab.into(),
            script_path: None,
            editor_node_id: None,
            transform: Transform2D::default(),
            visible: true,
            tags: Vec::new(),
            properties: HashMap::new(),
            sprites: Vec::new(),
            parent: None,
            children: Vec::new(),
            ui_rect: Cell::new(None),
            content_size: Cell::new(None),
            flow_size: Cell::new((None, None)),
            instance_bindings: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_transform(mut self, transform: Transform2D) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_position(mut self, position: Vec2) -> Self {
        self.transform.position = position;
        self
    }

    pub fn with_sprites(mut self, sprites: Vec<PrefabSprite2D>) -> Self {
        self.sprites = sprites;
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn prefab(&self) -> &str {
        &self.prefab
    }

    pub fn script_path(&self) -> Option<&str> {
        self.script_path.as_deref()
    }

    pub fn editor_node_id(&self) -> Option<u64> {
        self.editor_node_id
    }

    pub fn transform(&self) -> Transform2D {
        self.transform
    }

    pub fn transform_mut(&mut self) -> &mut Transform2D {
        &mut self.transform
    }

    pub fn position(&self) -> Vec2 {
        self.transform.position
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.transform.position = position;
    }

    pub fn translate(&mut self, delta: Vec2) {
        self.transform.position += delta;
    }

    pub fn rotation(&self) -> f32 {
        self.transform.rotation
    }

    pub fn set_rotation(&mut self, radians: f32) {
        self.transform.rotation = radians;
    }

    pub fn scale(&self) -> Vec2 {
        self.transform.scale
    }

    pub fn set_scale(&mut self, scale: Vec2) {
        self.transform.scale = scale;
    }

    /// The node's authored pickable size, if any: from explicit `w`/`h`
    /// properties, falling back to the editor's `editor_size_x`/`editor_size_y`.
    /// This is what gives a node hit-testable [`bounds`](SceneWorld2D::node_bounds)
    /// when it has no sprites — so a size set in the editor "just works".
    pub fn size(&self) -> Option<Vec2> {
        let explicit = self.property_f32("w").zip(self.property_f32("h"));
        let authored = explicit.or_else(|| {
            self.property_f32("editor_size_x")
                .zip(self.property_f32("editor_size_y"))
        });
        authored.map(|(w, h)| Vec2::new(w, h))
    }

    /// Set the node's pickable size (stored as `w`/`h` properties), so code can
    /// give a node hit bounds without stringly-typed property writes.
    pub fn set_size(&mut self, size: Vec2) {
        self.set_property("w", format!("{}", size.x));
        self.set_property("h", format!("{}", size.y));
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|item| item == tag)
    }

    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.iter().any(|item| item == &tag) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, tag: &str) -> bool {
        let before = self.tags.len();
        self.tags.retain(|item| item != tag);
        self.tags.len() != before
    }

    pub fn sprites(&self) -> &[PrefabSprite2D] {
        &self.sprites
    }

    pub fn set_sprites(&mut self, sprites: Vec<PrefabSprite2D>) {
        self.sprites = sprites;
    }

    pub fn parent(&self) -> Option<NodeHandle2D> {
        self.parent
    }

    pub fn children(&self) -> &[NodeHandle2D] {
        &self.children
    }

    /// This instance's per-item binding overlay, if `sync_repeaters` (E3) set
    /// one — `None` for every node that isn't a repeater instance.
    pub fn instance_bindings(&self) -> Option<&Bindings> {
        self.instance_bindings.as_ref()
    }

    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(String::as_str)
    }

    pub fn property_bool(&self, name: &str) -> Option<bool> {
        self.property(name).and_then(parse_bool_property)
    }

    pub fn property_i64(&self, name: &str) -> Option<i64> {
        self.property(name).and_then(|value| value.parse().ok())
    }

    pub fn property_u64(&self, name: &str) -> Option<u64> {
        self.property(name).and_then(|value| value.parse().ok())
    }

    pub fn property_f32(&self, name: &str) -> Option<f32> {
        self.property(name).and_then(|value| value.parse().ok())
    }

    pub fn set_property(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.properties.insert(name.into(), value.into());
    }

    pub fn remove_property(&mut self, name: &str) -> Option<String> {
        self.properties.remove(name)
    }

    pub fn properties(&self) -> &HashMap<String, String> {
        &self.properties
    }
}

struct Slot {
    generation: u32,
    node: Option<SceneNode2D>,
}

/// A detached copy of a live subtree (E3 repeaters): a repeat node's
/// original authored child, captured once so it survives being despawned
/// from the live graph after the first sync. `materialize_template` clones
/// it back into new live nodes, once per repeated item.
#[derive(Clone)]
struct NodeTemplate {
    node: SceneNode2D,
    children: Vec<NodeTemplate>,
}

/// A mutable, hierarchical runtime scene graph addressed by [`NodeHandle2D`].
///
/// Build one from a loaded scene with [`SceneWorld2D::from_scene`], then look
/// nodes up by handle, name, editor id, tag, or prefab and mutate them in
/// place. Spawning and despawning preserve handle safety through generational
/// slots, and [`SceneWorld2D::draw`] renders the live state (respecting
/// visibility and composed parent transforms).
#[derive(Default)]
pub struct SceneWorld2D {
    slots: Vec<Slot>,
    free: Vec<u32>,
    roots: Vec<NodeHandle2D>,
    by_name: HashMap<String, NodeHandle2D>,
    by_editor_id: HashMap<u64, NodeHandle2D>,
    /// One captured template per `ui: "repeat"` node (E3), keyed by that
    /// node's handle — set on first `sync_repeaters` call, reused every sync
    /// after (the template itself never changes once a screen is authored;
    /// only the instance count/scopes do).
    repeat_templates: HashMap<NodeHandle2D, NodeTemplate>,
    /// Grid cell size (canvas units) that resolved `ui_*` rects snap to (E9),
    /// or `0.0` for no snapping (the default). Fractional/stretch layout
    /// produces fractional rects by construction; a host with a fixed-pixel-
    /// size art style (see `SceneWorld2D::set_pixel_grid`) needs those
    /// quantised or a repeater dividing rows into a panel will silently drift
    /// off the art's pixel grid. Off by default so existing screens/tests are
    /// unaffected until a host opts in.
    pixel_grid: Cell<f32>,
    /// Node currently under the pointer (E6), or `None`. Drives
    /// `ui_color_hover` at draw and the mouse-focus handoff in
    /// `focus_move`/`set_focus_to_hovered`.
    hovered: Cell<Option<NodeHandle2D>>,
    /// Node currently pointer-down on (E6), or `None`. Drives
    /// `ui_color_press`.
    pressed: Cell<Option<NodeHandle2D>>,
    /// Node currently keyboard/gamepad-focused (E6), or `None`. Drives
    /// `ui_color_focus` and is what `Enter`/activate-the-focused-node acts
    /// on. `None` by default — a screen with no `ui_focusable` nodes never
    /// pays for this.
    focused: Cell<Option<NodeHandle2D>>,
    /// The canvas's [`text_scale`](crate::Canvas::text_scale), latched at the
    /// top of each draw so the layout can resolve `em` lengths.
    ///
    /// Latched rather than set by the host: the canvas is the one place the
    /// scale is authoritative, and a second setter would be free to disagree
    /// with it. The layout passes are `&self` and do not carry a canvas — the
    /// flow measures a child's authored `ui_w` with no atlas anywhere in
    /// reach — so this is how that number gets to them.
    ///
    /// `Option` because the struct is `#[derive(Default)]` and a bare `f32`
    /// would default to *zero* — every `em` length would collapse before the
    /// first draw latched anything. `None` reads as `1.0`.
    text_scale: Cell<Option<f32>>,
    /// Scene-authored keyframe clips, copied from the source document when the
    /// world is instantiated (`instantiate_scene_tree`). This is the authored
    /// animation data — the clips themselves live in the scene JSON.
    animations: Vec<SceneAnimClip>,
    /// Clips currently playing, in play order. A later play wins a shared
    /// target; `apply_animations` samples them at the clock the host feeds.
    active_anims: Vec<ActiveAnim>,
}

/// One playing clip: which clip, when it started on the host clock, and the
/// per-target authored state captured at play so a finished clip can restore.
struct ActiveAnim {
    clip: usize,
    started_at: f32,
    targets: Vec<AnimTargetState>,
    /// track.target name → index into `targets`.
    target_index: HashMap<String, usize>,
}

/// One frame's sampled values for one target node, accumulated from every
/// track that names it so axis pairs (offset_x + offset_y) compose before
/// anything is written.
#[derive(Default, Clone)]
struct AnimSampleAccum {
    dx: Option<f32>,
    dy: Option<f32>,
    rot: Option<f32>,
    scale: Option<f32>,
    alpha: Option<f32>,
    visible: Option<f32>,
}

/// The state of one animated node when its clip started.
#[derive(Clone)]
struct AnimTargetState {
    handle: NodeHandle2D,
    rest: Transform2D,
    rest_visible: bool,
    rest_ui_visible_set: bool,
    /// The node's authored `ui_rotation` (degrees), when it is a UI node. The
    /// canvas draw path reads `ui_rotation`, not the node's transform rotation,
    /// so a rotation track has to write both — this is the rest value it
    /// returns to on clip end. `None` when the node has no `ui` property (a
    /// pure sprite, whose rotation lives on the transform) or no authored
    /// `ui_rotation`.
    rest_ui_rotation: Option<f32>,
    /// Which fill carries the node's alpha, and its base value at clip start.
    /// `None` when the node has no animatable fill (no `ui_color`, no sprite).
    fill: Option<AnimTargetFill>,
}

#[derive(Clone, Copy)]
enum AnimTargetFill {
    /// The alpha rides in the node's `ui_color` property ("r,g,b,a"). `rgb`
    /// is captured at play so the per-frame rewrite never depends on the
    /// property staying parseable.
    UiColor { rgb: [f32; 3], base: f32 },
    /// The alpha rides in the leading sprite's tint.
    Sprite { rgb: [f32; 3], base: f32 },
}

/// Capture the authored state of a node when its clip starts: the transform,
/// visibility, and the fill whose alpha the clip will drive. `None` when the
/// node no longer exists.
fn capture_anim_target(world: &SceneWorld2D, handle: NodeHandle2D) -> Option<AnimTargetState> {
    let node = world.get(handle)?;
    let is_ui = node.property("ui").is_some();
    let fill = if !node.sprites().is_empty() {
        let c = node.sprites()[0].color;
        Some(AnimTargetFill::Sprite {
            rgb: [c.r, c.g, c.b],
            base: c.a,
        })
    } else if is_ui {
        parse_ui_color(node.property("ui_color")).map(|(rgb, base)| AnimTargetFill::UiColor { rgb, base })
    } else {
        None
    };
    Some(AnimTargetState {
        handle,
        rest: node.transform(),
        rest_visible: node.is_visible(),
        rest_ui_visible_set: is_ui,
        rest_ui_rotation: if is_ui {
            node.property("ui_rotation")
                .and_then(|v| v.trim().parse::<f32>().ok())
        } else {
            None
        },
        fill,
    })
}

/// Parse a `ui_color` "r,g,b,a" string; `None` when a component is not a
/// number (a placeholder like `"{chalk},255"` is left to the binder).
fn parse_ui_color(value: Option<&str>) -> Option<([f32; 3], f32)> {
    let parts: Vec<f32> = value?
        .split(',')
        .take(4)
        .map(|s| s.trim().parse().ok())
        .collect::<Option<Vec<f32>>>()?;
    if parts.len() != 4 {
        return None;
    }
    Some(([parts[0], parts[1], parts[2]], parts[3]))
}

/// Write one frame's sampled values onto a target node.
fn write_anim_target(
    world: &mut SceneWorld2D,
    target: &AnimTargetState,
    s: &AnimSampleAccum,
) {
    if s.dx.is_some() || s.dy.is_some() {
        if let Some(n) = world.get_mut(target.handle) {
            n.set_position(Vec2::new(
                target.rest.position.x + s.dx.unwrap_or(0.0),
                target.rest.position.y + s.dy.unwrap_or(0.0),
            ));
        }
    }
    if let Some(deg) = s.rot {
        if let Some(n) = world.get_mut(target.handle) {
            n.set_rotation(target.rest.rotation + deg.to_radians());
        }
        // The canvas draw path renders a UI node's rotation from its
        // `ui_rotation` property, not the node transform — so a rotation
        // track has to drive both, or a UI-authored scene's rotation animates
        // the transform and nothing moves on screen.
        if target.rest_ui_visible_set {
            let base = target.rest_ui_rotation.unwrap_or(0.0);
            if let Some(n) = world.get_mut(target.handle) {
                n.set_property("ui_rotation", (base + deg).to_string());
            }
        }
    }
    if let Some(scale) = s.scale {
        if let Some(n) = world.get_mut(target.handle) {
            n.set_scale(Vec2::new(
                target.rest.scale.x * scale,
                target.rest.scale.y * scale,
            ));
        }
    }
    if let Some(v) = s.visible {
        let visible = v != 0.0;
        if target.rest_ui_visible_set {
            if let Some(n) = world.get_mut(target.handle) {
                n.set_property("ui_visible", if visible { "true" } else { "false" });
            }
        } else if let Some(n) = world.get_mut(target.handle) {
            n.set_visible(visible);
        }
    }
    if let Some(alpha) = s.alpha {
        if let Some(fill) = target.fill {
            set_anim_fill(world, target.handle, fill, alpha);
        }
    }
}

/// Apply `multiplier` to the target's captured fill alpha.
fn set_anim_fill(
    world: &mut SceneWorld2D,
    handle: NodeHandle2D,
    fill: AnimTargetFill,
    multiplier: f32,
) {
    match fill {
        AnimTargetFill::UiColor { rgb, base } => {
            let alpha = (base * multiplier).clamp(0.0, 255.0);
            let color = format!("{},{},{},{}", rgb[0], rgb[1], rgb[2], alpha);
            if let Some(n) = world.get_mut(handle) {
                n.set_property("ui_color", color);
            }
        }
        AnimTargetFill::Sprite { rgb, base } => {
            let alpha = (base * multiplier).clamp(0.0, 1.0);
            if let Some(n) = world.get_mut(handle) {
                let mut sprites = n.sprites().to_vec();
                if let Some(first) = sprites.first_mut() {
                    first.color = Color {
                        r: rgb[0],
                        g: rgb[1],
                        b: rgb[2],
                        a: alpha,
                    };
                }
                n.set_sprites(sprites);
            }
        }
    }
}

/// Put every target back to the transform, visibility and fill it had when
/// its clip started.
fn restore_anim_targets(world: &mut SceneWorld2D, targets: &[AnimTargetState]) {
    for target in targets {
        if let Some(n) = world.get_mut(target.handle) {
            n.set_position(target.rest.position);
            n.set_rotation(target.rest.rotation);
            n.set_scale(target.rest.scale);
            n.set_visible(target.rest_visible);
            if target.rest_ui_visible_set {
                n.set_property("ui_visible", if target.rest_visible { "true" } else { "false" });
                // Restore the authored rotation; a node that had none returns
                // to upright so a later run of the same clip starts clean.
                let rest = target.rest_ui_rotation.unwrap_or(0.0);
                n.set_property("ui_rotation", rest.to_string());
            }
        }
        if let Some(fill) = target.fill {
            set_anim_fill(world, target.handle, fill, 1.0);
        }
    }
}

impl SceneWorld2D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the grid cell size (canvas units) every resolved `ui_*` rect snaps
    /// to (E9) — pass the host's own pixel-art unit (e.g. `sprites::pixel()`
    /// in Formula R). `0.0` disables snapping (the default). Applies to every
    /// node from the next draw onward unless a node opts out with
    /// `ui_snap: false`.
    pub fn set_pixel_grid(&mut self, cell: f32) {
        self.pixel_grid.set(cell.max(0.0));
    }

    /// The scene-authored clips this world can play (`animations` from the
    /// source document, copied in [`instantiate_scene_tree`](Self::instantiate_scene_tree)).
    pub fn animations(&self) -> &[SceneAnimClip] {
        &self.animations
    }

    /// Start the authored clip `id` at `started_at` on the host's clock. The
    /// clock is the caller's: `apply_animations` is fed the same time the host
    /// runs its gameplay on, so a vignette advances with the race, not on a
    /// private timer. Returns whether the clip exists. Replaying a playing
    /// clip restarts it; a later play of a *different* clip wins any property
    /// both animate while they both run.
    pub fn play_animation(&mut self, id: &str, started_at: f32) -> bool {
        let Some(clip_index) = self.animations.iter().position(|c| c.id == id) else {
            return false;
        };
        let clip = self.animations[clip_index].clone();
        let mut seen = std::collections::HashSet::new();
        let mut targets = Vec::new();
        let mut target_index = HashMap::new();
        for track in &clip.tracks {
            if !seen.insert(track.target.clone()) || target_index.contains_key(&track.target) {
                continue;
            }
            let Some(handle) = self.find_by_name(&track.target) else {
                continue;
            };
            let Some(state) = capture_anim_target(self, handle) else {
                continue;
            };
            target_index.insert(track.target.clone(), targets.len());
            targets.push(state);
        }
        self.active_anims.retain(|a| a.clip != clip_index);
        self.active_anims.push(ActiveAnim {
            clip: clip_index,
            started_at,
            targets,
            target_index,
        });
        true
    }

    /// Stop an active clip now, restoring its targets to the state they had
    /// when it started.
    pub fn stop_animation(&mut self, id: &str) {
        let Some(index) = self
            .active_anims
            .iter()
            .position(|a| &self.animations[a.clip].id == id)
        else {
            return;
        };
        let active = self.active_anims.remove(index);
        restore_anim_targets(self, &active.targets);
    }

    /// Whether the authored clip `id` is currently playing.
    pub fn is_animation_playing(&self, id: &str) -> bool {
        self.active_anims
            .iter()
            .any(|a| &self.animations[a.clip].id == id)
    }

    /// Sample every active clip at `time` (seconds on the host's clock) and
    /// write each track's value onto its target node. Call this each frame
    /// before drawing the world. A finished non-looping clip restores its
    /// targets to the state they had at play and leaves the active set;
    /// looping clips wrap on `duration`.
    pub fn apply_animations(&mut self, time: f32) {
        if self.active_anims.is_empty() {
            return;
        }
        let mut finished: Vec<usize> = Vec::new();
        for i in 0..self.active_anims.len() {
            let (clip_index, started_at) = {
                let a = &self.active_anims[i];
                (a.clip, a.started_at)
            };
            let clip = self.animations[clip_index].clone();
            let mut clip_t = time - started_at;
            if clip.duration > 0.0 && clip_t >= clip.duration {
                if clip.looping {
                    clip_t %= clip.duration;
                } else {
                    finished.push(i);
                    continue;
                }
            }
            let clip_t = clip_t.max(0.0);
            let targets = self.active_anims[i].targets.clone();
            let mut accs: Vec<AnimSampleAccum> = vec![AnimSampleAccum::default(); targets.len()];
            for track in &clip.tracks {
                let Some(value) = sample_track(track, clip_t) else {
                    continue;
                };
                let Some(&ti) = self.active_anims[i].target_index.get(&track.target) else {
                    continue;
                };
                let acc = &mut accs[ti];
                match track.property {
                    AnimatedProperty::OffsetX => acc.dx = Some(value),
                    AnimatedProperty::OffsetY => acc.dy = Some(value),
                    AnimatedProperty::RotationDeg => acc.rot = Some(value),
                    AnimatedProperty::Scale => acc.scale = Some(value),
                    AnimatedProperty::Alpha => acc.alpha = Some(value),
                    AnimatedProperty::Visible => acc.visible = Some(value),
                }
            }
            for (ti, target) in targets.iter().enumerate() {
                write_anim_target(self, target, &accs[ti]);
            }
        }
        // Restore after the writes: a finished clip's restoration must be the
        // last thing to touch its nodes for this frame, so a live clip that
        // also targets them still wins. Reverse order keeps earlier indices
        // valid as later ones are removed.
        for &i in finished.iter().rev() {
            let active = self.active_anims.remove(i);
            restore_anim_targets(self, &active.targets);
        }
    }

    /// The text scale the layout resolves `em` lengths against — the canvas's
    /// own, latched by the draw entry points. `1.0` until the first draw.
    fn text_scale(&self) -> f32 {
        self.text_scale.get().unwrap_or(1.0)
    }

    /// Every `ui_focusable: true` node, in `ui_focus_order` order (ties break
    /// on `visible_draw_order`, i.e. document order — stable and needs no new
    /// bookkeeping). This is the list `focus_move`/keyboard traversal steps
    /// through and the source of truth for "what can be Tabbed/arrowed to."
    /// Invisible nodes (a hidden overlay's buttons) are excluded, same as
    /// `visible_draw_order`.
    pub fn focusable_order(&self) -> Vec<NodeHandle2D> {
        let mut order: Vec<(i64, usize, NodeHandle2D)> = self
            .visible_draw_order()
            .into_iter()
            .enumerate()
            .filter_map(|(doc_index, handle)| {
                let node = self.get(handle)?;
                matches!(node.property("ui_focusable"), Some("true" | "1" | "yes")).then(|| {
                    (
                        node.property_i64("ui_focus_order").unwrap_or(0),
                        doc_index,
                        handle,
                    )
                })
            })
            .collect();
        order.sort_by_key(|&(order, doc_index, _)| (order, doc_index));
        order.into_iter().map(|(_, _, handle)| handle).collect()
    }

    /// The currently keyboard/gamepad-focused node (E6), if any.
    pub fn focused(&self) -> Option<NodeHandle2D> {
        self.focused.get()
    }

    /// Set focus directly to `handle` (or clear it with `None`) — e.g. mouse
    /// hover taking over focus, or a screen focusing its first field on
    /// enter. Does not check `ui_focusable`; callers that want the
    /// arrow-key-traversal contract should go through `focus_move` instead.
    pub fn set_focus(&self, handle: Option<NodeHandle2D>) {
        self.focused.set(handle);
    }

    /// Move focus by `delta` through [`focusable_order`] (`+1` next, `-1`
    /// previous), wrapping at both ends. No-op if nothing is focusable. If
    /// nothing is currently focused, `delta > 0` lands on the first
    /// focusable node and `delta < 0` on the last — either way "one step
    /// from off the list" rather than requiring a caller to seed focus
    /// first. Returns the newly focused handle.
    pub fn focus_move(&self, delta: i32) -> Option<NodeHandle2D> {
        let order = self.focusable_order();
        if order.is_empty() {
            self.focused.set(None);
            return None;
        }
        let len = order.len() as i32;
        let current = self
            .focused
            .get()
            .and_then(|h| order.iter().position(|&o| o == h));
        let next_index = match current {
            Some(i) => ((i as i32 + delta).rem_euclid(len)) as usize,
            None if delta >= 0 => 0,
            None => (len - 1) as usize,
        };
        let next = order[next_index];
        self.focused.set(Some(next));
        Some(next)
    }

    /// Node currently under the pointer (E6), or `None`. Set by the host each
    /// frame from `hit_test`; read back by `draw` to resolve
    /// `ui_color_hover`.
    pub fn hovered(&self) -> Option<NodeHandle2D> {
        self.hovered.get()
    }

    /// See [`hovered`](Self::hovered).
    pub fn set_hovered(&self, handle: Option<NodeHandle2D>) {
        self.hovered.set(handle);
    }

    /// Node currently pointer-down on (E6), or `None`. Drives
    /// `ui_color_press`.
    pub fn pressed(&self) -> Option<NodeHandle2D> {
        self.pressed.get()
    }

    /// See [`pressed`](Self::pressed).
    pub fn set_pressed(&self, handle: Option<NodeHandle2D>) {
        self.pressed.set(handle);
    }

    /// Build a live world from a loaded [`Scene2D`].
    ///
    /// Convenience wrapper over [`SceneWorld2D::instantiate_scene`] into a fresh
    /// world with no parent and no offset. Carries the scene's authored
    /// animation clips with it, so a world built this way can play them.
    pub fn from_scene(scene: &Scene2D) -> Self {
        let mut world = SceneWorld2D::new();
        world.animations = scene.animations.clone();
        world.instantiate_scene(scene, None, Transform2D::default());
        world
    }

    /// Instantiate a loaded [`Scene2D`] into this world as a subtree and return
    /// the handles of its top-level (instance) roots.
    ///
    /// This is the runtime reuse primitive behind nested scenes and prefab
    /// instances. The scene's own hierarchy is reconstructed from each
    /// instance's editor node/parent ids (an instance whose `editor_parent_id`
    /// resolves to another instance becomes its child; everything else is an
    /// instance root). Instance roots are then attached under `parent` (or added
    /// as world roots when `None`), and `offset` is composed onto each so the
    /// whole instance can be placed. Transform, visibility, tags, and sprite
    /// layers are seeded from the instances so the subtree renders identically
    /// to the static scene until gameplay mutates it.
    ///
    /// Name and editor-id lookups register first-wins, so instantiating the same
    /// scene twice never clobbers the first copy's entries; later copies stay
    /// reachable through the returned handles and the node hierarchy.
    pub fn instantiate_scene(
        &mut self,
        scene: &Scene2D,
        parent: Option<NodeHandle2D>,
        offset: Transform2D,
    ) -> Vec<NodeHandle2D> {
        let parent = parent.filter(|handle| self.contains(*handle));
        let instances = scene.instances();

        // First pass: spawn every instance as a detached node and remember the
        // handle for each editor node id so the second pass can wire parents.
        let mut handles = Vec::with_capacity(instances.len());
        let mut handle_by_editor_id: HashMap<u64, NodeHandle2D> = HashMap::new();

        for instance in instances {
            let mut node = SceneNode2D::new(instance.prefab.clone());
            node.name = instance.editor_name().map(str::to_string);
            node.script_path = instance.script_path().map(str::to_string);
            node.editor_node_id = instance.editor_node_id();
            node.transform = Transform2D {
                position: instance.position,
                rotation: instance.property_f32("rotation").unwrap_or(0.0),
                scale: instance.scale,
            };
            node.visible = instance.editor_visible().unwrap_or(true);
            node.tags = instance
                .property_tags("tags")
                .into_iter()
                .map(str::to_string)
                .collect();
            node.properties = instance.properties.clone();
            node.sprites = instance.sprite_layers().to_vec();

            let handle = self.insert_detached(node);
            handles.push(handle);
            if let Some(editor_id) = instance.editor_node_id() {
                handle_by_editor_id.insert(editor_id, handle);
            }
        }

        // Second pass: attach each node to its in-scene parent or collect it as
        // an instance root, and populate the name/editor-id lookups (first wins).
        let mut roots = Vec::new();
        for (instance, handle) in instances.iter().zip(handles.iter().copied()) {
            let in_scene_parent = instance
                .editor_parent_id()
                .and_then(|parent_id| handle_by_editor_id.get(&parent_id).copied());

            match in_scene_parent {
                Some(parent_handle) if parent_handle != handle => {
                    self.set_parent_link(handle, Some(parent_handle));
                }
                _ => roots.push(handle),
            }

            let name = self
                .get(handle)
                .and_then(|node| node.name().map(str::to_string));
            let editor_id = self.get(handle).and_then(|node| node.editor_node_id());
            if let Some(name) = name {
                self.by_name.entry(name).or_insert(handle);
            }
            if let Some(editor_id) = editor_id {
                self.by_editor_id.entry(editor_id).or_insert(handle);
            }
        }

        // Attach the instance roots under the requested parent (or as world
        // roots) and compose the placement offset onto each.
        for &root in &roots {
            if let Some(node) = self.get_mut(root) {
                let local = node.transform;
                node.transform = offset.compose(&local);
            }
            match parent {
                Some(parent_handle) => self.set_parent_link(root, Some(parent_handle)),
                None => self.roots.push(root),
            }
        }

        roots
    }

    /// Instantiate a scene and recursively expand any nested-scene references it
    /// (or its expanded children) contain.
    ///
    /// After the scene is composed via [`SceneWorld2D::instantiate_scene`], every
    /// instantiated node carrying a [`NESTED_SCENE_PROPERTY`] property has the
    /// named scene looked up in `library` and instantiated as a child subtree of
    /// that node. Expansion is recursive, so nested scenes may themselves nest.
    ///
    /// A per-path alias set rejects reference cycles (a scene that nests itself,
    /// directly or transitively) and a hard depth cap backstops any remaining
    /// runaway; unknown aliases are skipped. Returns the top-level instance roots.
    pub fn instantiate_scene_tree(
        &mut self,
        scene: &Scene2D,
        library: &SceneLibrary,
        parent: Option<NodeHandle2D>,
        offset: Transform2D,
    ) -> Vec<NodeHandle2D> {
        // The scene's authored clips come with it; a later scene's clip of the
        // same id wins (play_animation finds the first match).
        for clip in &scene.animations {
            let Some(entry) = self.animations.iter_mut().find(|c| c.id == clip.id) else {
                self.animations.push(clip.clone());
                continue;
            };
            *entry = clip.clone();
        }
        let mut active = HashSet::new();
        self.instantiate_scene_tree_inner(scene, library, parent, offset, 0, &mut active)
    }

    fn instantiate_scene_tree_inner(
        &mut self,
        scene: &Scene2D,
        library: &SceneLibrary,
        parent: Option<NodeHandle2D>,
        offset: Transform2D,
        depth: usize,
        active: &mut HashSet<String>,
    ) -> Vec<NodeHandle2D> {
        let roots = self.instantiate_scene(scene, parent, offset);
        if depth >= MAX_NESTED_SCENE_DEPTH {
            return roots;
        }

        // Collect nested-scene references across the freshly instantiated subtree
        // before expanding any, so a node's expansion does not feed back into this
        // pass.
        let mut to_expand: Vec<(NodeHandle2D, String)> = Vec::new();
        let mut stack = roots.clone();
        while let Some(handle) = stack.pop() {
            if let Some(node) = self.get(handle) {
                stack.extend(node.children().iter().copied());
                if let Some(alias) = node.property(NESTED_SCENE_PROPERTY) {
                    to_expand.push((handle, alias.to_string()));
                }
            }
        }

        for (handle, alias) in to_expand {
            if active.contains(&alias) {
                continue; // cycle: this alias is already being expanded on this path
            }
            let Some(nested) = library.get(&alias) else {
                continue; // unknown alias: leave the host node as a plain marker
            };
            // `library` and `self` are distinct, so the immutable scene borrow
            // happily coexists with the mutable world borrow during recursion.
            active.insert(alias.clone());
            self.instantiate_scene_tree_inner(
                nested,
                library,
                Some(handle),
                Transform2D::default(),
                depth + 1,
                active,
            );
            active.remove(&alias);
        }

        roots
    }

    /// Spawn a new node as a root and return its handle.
    pub fn spawn(&mut self, node: SceneNode2D) -> NodeHandle2D {
        let name = node.name().map(str::to_string);
        let editor_id = node.editor_node_id();
        let handle = self.insert_detached(node);
        self.roots.push(handle);
        if let Some(name) = name {
            self.by_name.entry(name).or_insert(handle);
        }
        if let Some(editor_id) = editor_id {
            self.by_editor_id.entry(editor_id).or_insert(handle);
        }
        handle
    }

    /// Spawn a new node as a child of `parent` and return its handle. If
    /// `parent` is stale the node is spawned as a root instead.
    pub fn spawn_child(&mut self, parent: NodeHandle2D, node: SceneNode2D) -> NodeHandle2D {
        let handle = self.spawn(node);
        self.reparent(handle, Some(parent));
        handle
    }

    /// Reconcile every live `ui: "repeat"` node's instance children against
    /// `repeaters` (E3): a repeat node's original authored child (captured
    /// once, on its first sync) is the *template*; after syncing, the node's
    /// live `children()` are `repeaters[ui_repeat_source].len()` clones of
    /// that template, each carrying its own [`Bindings`] scope
    /// (`SceneNode2D::instance_bindings`) from `repeaters[source][i]`.
    ///
    /// Call once per frame, before `draw_at`/`draw_to_canvas` — an explicit
    /// step rather than something draw itself does, so `draw_at`/
    /// `draw_to_canvas` can stay `&self` (spawning/despawning instance nodes
    /// needs `&mut self`, and threading that through the whole draw
    /// recursion would force every caller — including scenes with zero
    /// repeaters — onto a `&mut self` draw path).
    ///
    /// A repeat node with no matching entry in `repeaters` (unknown source
    /// name, or none supplied) is left with zero instances — nothing to
    /// repeat, not an error.
    pub fn sync_repeaters(&mut self, repeaters: &RepeaterSources) {
        let repeat_nodes: Vec<NodeHandle2D> = self
            .all_handles()
            .into_iter()
            .filter(|&handle| self.get(handle).and_then(|n| n.property("ui")) == Some("repeat"))
            .collect();

        for handle in repeat_nodes {
            self.sync_one_repeater(handle, repeaters);
        }
    }

    fn sync_one_repeater(&mut self, handle: NodeHandle2D, repeaters: &RepeaterSources) {
        let Some(node) = self.get(handle) else {
            return;
        };
        // `ui_repeat_source` is optional once a node authors its own items:
        // a purely authored list (a fixed menu, a legend) needs no supplier.
        let source_name = node.property("ui_repeat_source").map(str::to_string);
        let authored = node
            .property("ui_repeat_items")
            .and_then(parse_repeat_items);

        // A live source wins over authored items: the authored list is the
        // fallback for "nobody supplied this", not a default to merge with.
        // Note the supplier only has to *name* the source to take over — an
        // empty Vec is a real answer ("no rows right now") and correctly
        // yields zero instances rather than falling back to authored rows.
        let supplied = source_name.as_deref().and_then(|name| repeaters.get(name));
        let items: Option<&Vec<Bindings>> = supplied.or(authored.as_ref());
        let item_count = items.map_or(0, Vec::len);

        // First sync: the node's one authored child is the template — capture
        // it as data, then clear the live children so what remains under this
        // node is purely instances (flow layout in `resolve_child_references`
        // lays out whatever `children()` currently is, and a leftover
        // never-despawned template node would just be an extra untemplated row).
        if self.repeat_templates.get(&handle).is_none() {
            let Some(template_child) = self.get(handle).and_then(|n| n.children.first().copied())
            else {
                return; // nothing authored under this repeat node yet
            };
            let template = self.capture_template(template_child);
            self.despawn(template_child);
            self.repeat_templates.insert(handle, template);
        }

        let current: Vec<NodeHandle2D> =
            self.get(handle).map_or(Vec::new(), |n| n.children.clone());
        match current.len().cmp(&item_count) {
            std::cmp::Ordering::Greater => {
                for extra in &current[item_count..] {
                    self.despawn(*extra);
                }
            }
            std::cmp::Ordering::Less => {
                let Some(template) = self.repeat_templates.get(&handle).cloned() else {
                    return;
                };
                for _ in current.len()..item_count {
                    self.materialize_template(&template, handle);
                }
            }
            std::cmp::Ordering::Equal => {}
        }

        // Refresh every instance's scope every sync (not just newly-spawned
        // ones) — the same index can represent a different item across
        // frames as the source collection changes, e.g. a sorted standings
        // list re-ranking lap to lap.
        let instances: Vec<NodeHandle2D> =
            self.get(handle).map_or(Vec::new(), |n| n.children.clone());
        if let Some(items) = items {
            // Cloned up front: `items` borrows either `repeaters` or the
            // node's own authored property, and the loop needs `&mut self`.
            let scopes: Vec<Bindings> = items.clone();
            for (instance, scope) in instances.iter().zip(scopes.into_iter()) {
                if let Some(node) = self.get_mut(*instance) {
                    node.instance_bindings = Some(scope);
                }
            }
        }
    }

    /// Authored repeat rows, if this node has any that parse.
    ///
    /// See [`parse_repeat_items`] for the format.
    pub fn authored_repeat_items(&self, handle: NodeHandle2D) -> Option<Vec<Bindings>> {
        self.get(handle)?
            .property("ui_repeat_items")
            .and_then(parse_repeat_items)
    }

    /// Deep-copy a live subtree into a detached [`NodeTemplate`] — the node's
    /// own fields (via `SceneNode2D`'s `Clone`) plus each child captured the
    /// same way, recursively. Hierarchy links (`parent`/`children` handles)
    /// on the captured `SceneNode2D` are meaningless once detached from the
    /// live graph; `materialize_template` rebuilds them via `spawn_child`.
    fn capture_template(&self, handle: NodeHandle2D) -> NodeTemplate {
        let mut node = self
            .get(handle)
            .cloned()
            .expect("capture_template called with a live handle");
        let children = node
            .children
            .iter()
            .map(|&child| self.capture_template(child))
            .collect();
        // The captured node is a *blueprint*, not a live node: its `children`
        // are handles into a world where the template has already been
        // despawned. `materialize_template` re-links real children as it
        // recurses, so carrying the old list forward left every instance with
        // one dangling handle per template child, in front of its real ones.
        // Invisible until a flow laid the instance out — then those phantoms
        // took a zero-width track each and pushed every real child right by a
        // gap apiece.
        node.children.clear();
        NodeTemplate { node, children }
    }

    /// Spawn one live clone of `template` as a new child of `parent` (and its
    /// children as clones under that, recursively). Returns the new
    /// instance's root handle.
    fn materialize_template(
        &mut self,
        template: &NodeTemplate,
        parent: NodeHandle2D,
    ) -> NodeHandle2D {
        let handle = self.spawn_child(parent, template.node.clone());
        for child_template in &template.children {
            self.materialize_template(child_template, handle);
        }
        handle
    }

    /// Every live node handle, roots and descendants, in no particular order.
    fn all_handles(&self) -> Vec<NodeHandle2D> {
        let mut out = Vec::new();
        for root in self.roots.clone() {
            self.collect_all(root, &mut out);
        }
        out
    }

    fn collect_all(&self, handle: NodeHandle2D, out: &mut Vec<NodeHandle2D>) {
        let Some(node) = self.get(handle) else {
            return;
        };
        out.push(handle);
        for child in node.children.clone() {
            self.collect_all(child, out);
        }
    }

    /// Remove a node and its entire subtree. All handles into the removed
    /// subtree become permanently invalid.
    pub fn despawn(&mut self, handle: NodeHandle2D) -> bool {
        if !self.contains(handle) {
            return false;
        }

        // Detach from parent / root set first so we do not leave dangling links.
        self.unlink_from_parent(handle);
        self.roots.retain(|root| *root != handle);

        self.despawn_subtree(handle);
        true
    }

    fn despawn_subtree(&mut self, handle: NodeHandle2D) {
        let children = match self.get(handle) {
            Some(node) => node.children.clone(),
            None => return,
        };
        for child in children {
            self.despawn_subtree(child);
        }

        if let Some(slot) = self.slots.get_mut(handle.index as usize) {
            if slot.generation == handle.generation {
                if let Some(node) = slot.node.take() {
                    if let Some(name) = node.name {
                        if self.by_name.get(&name) == Some(&handle) {
                            self.by_name.remove(&name);
                        }
                    }
                    if let Some(editor_id) = node.editor_node_id {
                        if self.by_editor_id.get(&editor_id) == Some(&handle) {
                            self.by_editor_id.remove(&editor_id);
                        }
                    }
                }
                slot.generation = slot.generation.wrapping_add(1);
                self.free.push(handle.index);
            }
        }
    }

    pub fn contains(&self, handle: NodeHandle2D) -> bool {
        self.slots
            .get(handle.index as usize)
            .is_some_and(|slot| slot.generation == handle.generation && slot.node.is_some())
    }

    pub fn get(&self, handle: NodeHandle2D) -> Option<&SceneNode2D> {
        let slot = self.slots.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.node.as_ref()
    }

    pub fn get_mut(&mut self, handle: NodeHandle2D) -> Option<&mut SceneNode2D> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.node.as_mut()
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|slot| slot.node.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn roots(&self) -> &[NodeHandle2D] {
        &self.roots
    }

    pub fn parent(&self, handle: NodeHandle2D) -> Option<NodeHandle2D> {
        self.get(handle).and_then(|node| node.parent)
    }

    pub fn children(&self, handle: NodeHandle2D) -> Vec<NodeHandle2D> {
        self.get(handle)
            .map(|node| node.children.clone())
            .unwrap_or_default()
    }

    /// Iterate over every live node handle in the world (unordered).
    pub fn handles(&self) -> impl Iterator<Item = NodeHandle2D> + '_ {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.node.as_ref().map(|_| NodeHandle2D {
                index: index as u32,
                generation: slot.generation,
            })
        })
    }

    pub fn find_by_name(&self, name: &str) -> Option<NodeHandle2D> {
        self.by_name.get(name).copied()
    }

    /// Override a `ui: "button"` or `ui: "text"` node's label past whatever
    /// was authored in the editor — the Button's own built-in label for a
    /// simple button, or a `Label` child's `ui_text` for a composite one.
    /// A button's text is a starting value, not a constant — a "Continue"
    /// that reads "Pick Two Drivers" until a precondition is met, or "Load
    /// Game" greyed out when there's no save, is game state driving text on
    /// one authored node, not several mutually-exclusive label siblings
    /// toggled by a `ui_visible` binding.
    pub fn set_button_text(&mut self, handle: NodeHandle2D, text: impl Into<String>) {
        if let Some(node) = self.get_mut(handle) {
            node.set_property("ui_text", text.into());
        }
    }

    /// [`set_button_text`](Self::set_button_text)'s colour counterpart — the
    /// idle-state `ui_color`; hover/focus/press still resolve their own
    /// `ui_color_*` variant on top. The disabled-button case: same node,
    /// same position, a duller colour and no interaction, rather than a
    /// second label node shown in its place.
    pub fn set_button_color(&mut self, handle: NodeHandle2D, color: impl Into<String>) {
        if let Some(node) = self.get_mut(handle) {
            node.set_property("ui_color", color.into());
        }
    }

    pub fn find_by_editor_id(&self, editor_node_id: u64) -> Option<NodeHandle2D> {
        self.by_editor_id.get(&editor_node_id).copied()
    }

    pub fn by_tag(&self, tag: &str) -> Vec<NodeHandle2D> {
        self.handles()
            .filter(|handle| self.get(*handle).is_some_and(|node| node.has_tag(tag)))
            .collect()
    }

    pub fn by_prefab(&self, prefab: &str) -> Vec<NodeHandle2D> {
        self.handles()
            .filter(|handle| {
                self.get(*handle)
                    .is_some_and(|node| node.prefab() == prefab)
            })
            .collect()
    }

    /// Move a node under a new parent (or to the root set when `new_parent` is
    /// `None`). No-ops if either handle is stale, if the move is a self-parent,
    /// or if it would create a cycle (parenting a node under its own descendant).
    pub fn reparent(&mut self, handle: NodeHandle2D, new_parent: Option<NodeHandle2D>) -> bool {
        if !self.contains(handle) {
            return false;
        }
        if let Some(parent) = new_parent {
            if parent == handle || !self.contains(parent) || self.is_descendant(parent, handle) {
                return false;
            }
        }

        self.unlink_from_parent(handle);
        self.roots.retain(|root| *root != handle);

        match new_parent {
            Some(parent) => self.set_parent_link(handle, Some(parent)),
            None => {
                self.set_parent_link(handle, None);
                self.roots.push(handle);
            }
        }
        true
    }

    /// The fully composed world transform of a node, folding in every ancestor.
    pub fn world_transform(&self, handle: NodeHandle2D) -> Option<Transform2D> {
        let node = self.get(handle)?;
        match node.parent {
            Some(parent) => {
                let parent_transform = self.world_transform(parent)?;
                Some(parent_transform.compose(&node.transform))
            }
            None => Some(node.transform),
        }
    }

    /// Axis-aligned world-space bounds used for pointer hit-testing.
    ///
    /// A node with a `ui` property returns the rect it was last drawn at (see
    /// [`resolved_rect`](Self::resolved_rect)) — authored UI is then clickable
    /// exactly where it is drawn, using the same anchor/stretch/fraction math
    /// as rendering instead of a second hand-computed hitbox. Otherwise prefers
    /// an explicit interactive size from the node's `w`/`h` properties (how
    /// editor-authored markers carry their box), falling back to the union of
    /// the node's sprite layers. Returns `None` for nodes with no pickable area
    /// and no cached ui rect (e.g. a `ui` node before its first draw).
    /// Rotation is not yet folded into the non-ui hit rect — those bounds are
    /// the node's axis-aligned extent at its composed world position/scale.
    pub fn node_bounds(&self, handle: NodeHandle2D) -> Option<Rect> {
        let node = self.get(handle)?;
        if node.property("ui").is_some() {
            return node.ui_rect.get();
        }
        let world = self.world_transform(handle)?;

        // An authored size (explicit `w`/`h` or the editor's size) is the
        // node's pickable box, so editor-set sizes work without extra wiring.
        if let Some(size) = node.size() {
            return Some(Rect::from_pos_size(world.position, size * world.scale));
        }

        let sprites = node.sprites();
        if sprites.is_empty() {
            return None;
        }
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for sprite in sprites {
            let a = sprite.offset * world.scale;
            let b = (sprite.offset + sprite.size) * world.scale;
            min = min.min(a).min(b);
            max = max.max(a).max(b);
        }
        Some(Rect::from_pos_size(world.position + min, max - min))
    }

    /// The `ui_*`-resolved rect a `ui` node was drawn at on the most recent
    /// `draw_at`/`draw_to_canvas` pass, in canvas coordinates. `None` before
    /// the first draw or for a node with no `ui` property. This is the escape
    /// hatch a half-migrated screen paints legacy content inside — and what
    /// the `ui: "custom"` carve-out runs on.
    pub fn resolved_rect(&self, handle: NodeHandle2D) -> Option<Rect> {
        self.get(handle)?.ui_rect.get()
    }

    /// Visible nodes in draw order (parents before children, siblings in order),
    /// skipping invisible subtrees — the same set [`SceneWorld2D::draw`] emits.
    pub fn visible_draw_order(&self) -> Vec<NodeHandle2D> {
        let mut out = Vec::new();
        for root in self.roots.clone() {
            self.collect_visible(root, &mut out);
        }
        out
    }

    fn collect_visible(&self, handle: NodeHandle2D, out: &mut Vec<NodeHandle2D>) {
        let Some(node) = self.get(handle) else {
            return;
        };
        if !node.is_visible() {
            return;
        }
        out.push(handle);
        for child in node.children().to_vec() {
            self.collect_visible(child, out);
        }
    }

    /// Topmost visible node whose bounds contain `point` (in world space).
    ///
    /// "Topmost" means last-drawn: children sit above parents and later
    /// siblings above earlier ones, matching what the player sees.
    pub fn hit_test(&self, point: Vec2) -> Option<NodeHandle2D> {
        self.hit_test_all(point).into_iter().next()
    }

    /// Every visible node whose bounds contain `point`, topmost first.
    pub fn hit_test_all(&self, point: Vec2) -> Vec<NodeHandle2D> {
        let mut order = self.visible_draw_order();
        order.reverse();
        order
            .into_iter()
            .filter(|handle| {
                self.node_bounds(*handle)
                    .is_some_and(|rect| rect.contains_point(point))
            })
            .collect()
    }

    /// Draw every visible node, parents before children, composing parent
    /// transforms. An invisible node hides its whole subtree.
    pub fn draw(&self, frame: &mut Frame) {
        self.draw_at(frame, 0.0);
    }

    /// Draw with an animation clock so `ui_bob_*` / `ui_sway_*` node animations
    /// advance; pass `engine.time().total_time()`.
    pub fn draw_at(&self, frame: &mut Frame, time: f32) {
        self.draw_at_with_bindings(frame, time, &Bindings::new());
    }

    /// Like [`draw_at`](Self::draw_at), but every `ui_*` property may contain
    /// `{key}` placeholders resolved against `bindings` before it's parsed
    /// (E2 data binding) — e.g. `ui_text: "P{pos}  {name}"`. Empty bindings
    /// behave exactly like `draw_at`.
    pub fn draw_at_with_bindings(&self, frame: &mut Frame, time: f32, bindings: &Bindings) {
        self.text_scale.set(Some(frame.canvas(0).text_scale()));
        let (sw, sh) = frame.canvas(0).screen_size();
        let screen = (-(sw as f32) / 2.0, -(sh as f32) / 2.0, sw as f32, sh as f32);
        let roots = self.roots.clone();
        for &root in &roots {
            if self.subtree_wants_content_size(root) {
                self.measure_content_size(root, frame.canvas(0), bindings);
            }
        }
        for root in roots {
            self.draw_node(
                root,
                &Transform2D::default(),
                screen,
                time,
                frame,
                bindings,
                Default::default(),
            );
        }
    }

    /// One reference rect per child of `parent`, honouring `ui_layout` (E4)
    /// when set on `parent`: `"column"`/`"row"` flows the children in order
    /// via [`Stack`], each taking `ui_gap` from `parent`'s properties and its
    /// own `ui_h`/`ui_w` (column/row respectively) as its main-axis extent.
    /// The returned rect spans the full cross-axis, but a child still resolves
    /// its own rect against it exactly like any other reference — an
    /// `ui_stretch_x`/`ui_stretch_y` child fills the slot, one with no
    /// stretch anchors within it as usual. Absent `ui_layout` returns
    /// `parent_rect` unchanged for every child — today's behaviour, zero cost.
    ///
    /// Two passes, not one: every child's main-axis extent is measured before
    /// any child is placed, because proportional (`ui_grow`) and justified
    /// (`ui_justify`) distribution both need the *sum* of the siblings before
    /// item 0 can be positioned. A packed-from-the-start `Stack` cannot express
    /// either. The measure pass is a property read per child — no recursion —
    /// so the second pass costs one extra walk of an already-cloned slice.
    ///
    /// A child's main-axis extent resolves in priority order (E-A + E-G):
    ///
    /// 1. `ui_grow: "<weight>"` → a weighted track sharing the leftover space.
    ///    `"1"` is the common case; the value is a weight, so `"2"` takes twice
    ///    the share of a `"1"` sibling.
    /// 2. `ui_size: "content"` → its measured content size, already computed by
    ///    the bottom-up pre-pass that runs *before* layout, so the value is
    ///    simply read. This is what lets a sidebar be exactly as wide as its
    ///    widest row instead of carrying a hand-measured constant.
    /// 3. literal `ui_w`/`ui_h` → a fixed track. Today's behaviour, unchanged.
    ///
    /// Cross-axis (E-B): `ui_align` on the *parent* — `"stretch"` (default,
    /// today's behaviour: the slot spans the full cross-axis), or
    /// `"start"`/`"center"`/`"end"`, which size the slot to the child's own
    /// cross extent and place it within the container.
    ///
    /// With no `ui_grow` on any child, leftover main-axis space is distributed
    /// by the parent's `ui_justify` (`"start"` by default — packed from the
    /// start, exactly as before).
    ///
    /// A child may also author `ui_lead`: extra space *before* it on the main
    /// axis, on top of the uniform `ui_gap` — the mockups' `margin-top:34px`
    /// between blocks. Leading only, and its own property rather than
    /// `ui_margin_*`, which already means something else on a stretched node.
    ///
    /// Main-axis size is still read directly here rather than via
    /// `resolve_ui_rect`, which also resolves *position*; `ui_w_frac` /
    /// `ui_stretch_x` on the main axis remain unsupported in a flow container —
    /// `ui_grow` is how a flow child asks for a share of the space.
    ///
    /// Absent `ui_layout` returns `parent_rect` unchanged for every child, and
    /// absent `ui_grow`/`ui_justify`/`ui_align` reproduces the previous
    /// packed-from-the-start flow exactly.
    /// `ui_layout: "grid"` — children in row-major order across a fixed
    /// number of columns, wrapping onto as many rows as they need.
    ///
    /// Two ways to say how wide a column is, matching the two things authors
    /// actually want:
    ///
    /// - `ui_cols: <n>` — exactly `n` equal columns sharing the width, which
    ///   is CSS `repeat(n, 1fr)`. A layout that must stay `n` across at any
    ///   size (the mockups' `1fr 1fr` panels).
    /// - `ui_col_w: <px>` — as many `px`-wide columns as fit, which is CSS
    ///   `repeat(auto-fill, px)`. A gallery that reflows: a deck view gets
    ///   more cards per row on a wider window without re-authoring.
    ///
    /// `ui_gap` spaces both axes (`ui_gap_y` overrides the vertical one), and
    /// `ui_row_h` sets the row height — absent, rows are as tall as the
    /// tallest cell in them, measured the same way a flow measures a child.
    ///
    /// A grid is not a flow, so `ui_grow`/`ui_justify`/`ui_lead` do not apply
    /// to its children: a cell's size is the track it lands in. `ui_absolute`
    /// children are still taken out, exactly as in a flow.
    fn resolve_grid_references(
        &self,
        parent: &SceneNode2D,
        parent_rect: (f32, f32, f32, f32),
        children: &[NodeHandle2D],
        bindings: &Bindings,
    ) -> Vec<(f32, f32, f32, f32)> {
        let get = |name: &str| {
            parent
                .property(name)
                .map(|v| super::data2d::substitute_bindings(v, bindings).into_owned())
        };
        let scale = self.text_scale();
        let prop_f32 = |name: &str| get(name).and_then(|v| super::data2d::parse_length(&v, scale));
        let pad_left = prop_f32("ui_pad_left").unwrap_or(0.0);
        let pad_right = prop_f32("ui_pad_right").unwrap_or(0.0);
        let pad_top = prop_f32("ui_pad_top").unwrap_or(0.0);
        // No `ui_pad_bottom`: rows are laid out downward from the top and the
        // grid runs as many as its children need, so nothing is measured
        // against the bottom edge. A grid inside a fixed-height parent can
        // overflow it, same as a column does.
        let gap_x = prop_f32("ui_gap").unwrap_or(0.0);
        let gap_y = prop_f32("ui_gap_y").unwrap_or(gap_x);

        let (px, py, pw, ph) = parent_rect;
        let inner_w = (pw - pad_left - pad_right).max(0.0);
        let left = px + pad_left;
        let top = py + ph - pad_top;

        let is_absolute = |child: NodeHandle2D| {
            self.get(child).is_some_and(|c| {
                matches!(
                    c.property("ui_absolute").map(str::trim),
                    Some("true" | "1" | "yes")
                )
            })
        };
        let flowed: Vec<NodeHandle2D> = children
            .iter()
            .copied()
            .filter(|&c| !is_absolute(c))
            .collect();

        // Columns: an explicit count, else as many fixed-width ones as fit.
        let cols = match prop_f32("ui_cols") {
            Some(n) if n >= 1.0 => n as usize,
            _ => match prop_f32("ui_col_w") {
                Some(w) if w > 0.0 => (((inner_w + gap_x) / (w + gap_x)).floor() as usize).max(1),
                _ => flowed.len().max(1),
            },
        };
        let col_w = ((inner_w - gap_x * (cols as f32 - 1.0)) / cols as f32).max(0.0);

        // Row height: authored, else the tallest cell in the whole grid, so
        // rows line up rather than each being its own height. Uniform rows
        // are what every grid in the mockups is, and they keep a repeater's
        // instances interchangeable.
        let cell_h = |child: NodeHandle2D| -> f32 {
            let Some(c) = self.get(child) else {
                return 0.0;
            };
            if let Some((_, h)) = c.content_size.get() {
                return h;
            }
            let merged;
            let child_bindings = match &c.instance_bindings {
                Some(scope) => {
                    merged = merge_bindings(bindings, scope);
                    &merged
                }
                None => bindings,
            };
            c.property("ui_h")
                .map(|v| super::data2d::substitute_bindings(v, child_bindings).into_owned())
                .and_then(|v| super::data2d::parse_length(&v, scale))
                .unwrap_or(0.0)
                .max(0.0)
                * c.transform.scale.y
        };
        let row_h = prop_f32("ui_row_h")
            .unwrap_or_else(|| flowed.iter().map(|&c| cell_h(c)).fold(0.0, f32::max));

        let mut next = 0usize;
        children
            .iter()
            .map(|&child| {
                if is_absolute(child) {
                    if let Some(c) = self.get(child) {
                        c.flow_size.set((None, None));
                    }
                    return parent_rect;
                }
                let i = next;
                next += 1;
                let (row, col) = (i / cols, i % cols);
                let x = left + col as f32 * (col_w + gap_x);
                // y-up: row 0 is at the top, each row one step lower.
                let y = top - (row + 1) as f32 * row_h - row as f32 * gap_y;
                // The cell IS the child on both axes — same reasoning as a
                // flow-decided axis, so a grid item needs no `ui_stretch_*`
                // or `ui_origin_*` to fill the cell it was placed in.
                if let Some(c) = self.get(child) {
                    c.flow_size.set((Some(col_w), Some(row_h)));
                }
                (x, y, col_w, row_h)
            })
            .collect()
    }

    fn resolve_child_references(
        &self,
        parent: &SceneNode2D,
        parent_rect: (f32, f32, f32, f32),
        children: &[NodeHandle2D],
        bindings: &Bindings,
    ) -> Vec<(f32, f32, f32, f32)> {
        let get = |node: &SceneNode2D, name: &str| {
            node.property(name)
                .map(|v| super::data2d::substitute_bindings(v, bindings).into_owned())
        };
        let Some(layout) = get(parent, "ui_layout") else {
            return vec![parent_rect; children.len()];
        };
        let vertical = match layout.as_str() {
            "column" => true,
            "row" => false,
            "grid" => return self.resolve_grid_references(parent, parent_rect, children, bindings),
            _ => return vec![parent_rect; children.len()],
        };
        let scale = self.text_scale();
        let prop_f32 = |name: &str| get(parent, name).and_then(|v| super::data2d::parse_length(&v, scale));
        let pad_left = prop_f32("ui_pad_left").unwrap_or(0.0);
        let pad_right = prop_f32("ui_pad_right").unwrap_or(0.0);
        let pad_top = prop_f32("ui_pad_top").unwrap_or(0.0);
        let pad_bottom = prop_f32("ui_pad_bottom").unwrap_or(0.0);
        let gap = prop_f32("ui_gap").unwrap_or(0.0);
        let justify = parse_justify(get(parent, "ui_justify").as_deref());
        let align = parse_align(get(parent, "ui_align").as_deref());

        let (px, py, pw, ph) = parent_rect;
        let padded = Rect::new(
            px + pad_left,
            py + pad_bottom,
            (pw - pad_left - pad_right).max(0.0),
            (ph - pad_top - pad_bottom).max(0.0),
        );

        // `ui_absolute: true` is CSS `position:absolute` — the child is taken
        // out of the flow entirely and resolves against the parent's *full*
        // rect (padding included, as in CSS), so `ui_anchor` + `position`
        // place it from whichever edge it names. The mockups lean on this for
        // every overlay: the title's corner flags and its bottom apron, the
        // race screen's modals, every badge pinned to a panel corner.
        //
        // Without it those children would take a slot in the column and push
        // their siblings down, so the only alternative is hoisting them out
        // of the parent they visually belong to — which is exactly the kind
        // of structural divergence from the markup this is here to avoid.
        //
        // `ui_visible: false` is taken out for the same reason, and it is CSS
        // `display:none` rather than `visibility:hidden`: the mockups switch
        // between screen variants with `<sc-if>`, which removes the element.
        // A hidden node that kept its track left a variant-sized hole — the
        // sponsor screen shows one of two titles, one of two subtitles and one
        // of two figures, and reserving the unshown one of each pushed
        // everything below it down by a whole line.
        let out_of_flow: Vec<bool> = children
            .iter()
            .map(|&child| {
                self.get(child).is_some_and(|c| {
                    // Substituted: `ui_visible` is nearly always a binding
                    // (`{founding}`), and the raw `"{founding}"` matches
                    // neither "false" nor "true".
                    let hidden = c
                        .property("ui_visible")
                        .map(|v| super::data2d::substitute_bindings(v, bindings))
                        .is_some_and(|v| matches!(v.trim(), "false" | "0" | "no"));
                    hidden
                        || matches!(
                            c.property("ui_absolute").map(str::trim),
                            Some("true" | "1" | "yes")
                        )
                })
            })
            .collect();
        // The flow lays out only the in-flow children; the rest are spliced
        // back into their original positions at the end so the returned
        // vector still lines up index-for-index with `children`.
        let in_flow: Vec<NodeHandle2D> = children
            .iter()
            .zip(&out_of_flow)
            .filter(|(_, out)| !**out)
            .map(|(c, _)| *c)
            .collect();
        if in_flow.len() != children.len() {
            let flowed = self.resolve_child_references(parent, parent_rect, &in_flow, bindings);
            let mut flowed = flowed.into_iter();
            return out_of_flow
                .iter()
                .map(|abs| {
                    if *abs {
                        // Full rect, not `padded`: CSS positions an absolute
                        // child against its containing block's padding box.
                        parent_rect
                    } else {
                        flowed.next().unwrap_or(parent_rect)
                    }
                })
                .collect();
        }
        let children = &in_flow[..];

        // --- Pass 1: measure ------------------------------------------------
        // Per child: its main-axis track, and its own cross-axis extent (only
        // read when `ui_align` is not the default `stretch`).
        let mut tracks: Vec<Track> = Vec::with_capacity(children.len());
        let mut cross: Vec<f32> = Vec::with_capacity(children.len());
        let mut lead: Vec<f32> = Vec::with_capacity(children.len());
        let mut auto_lead: Vec<bool> = Vec::with_capacity(children.len());
        for &child in children {
            let Some(child_node) = self.get(child) else {
                tracks.push(Track::Fixed(0.0));
                cross.push(0.0);
                lead.push(0.0);
                auto_lead.push(false);
                continue;
            };
            // Substituted, not raw: the parent's own layout properties go
            // through `get` above, and a child's size has to as well or
            // `ui_h: "{row_h}"` silently measures as zero and every slot
            // collapses. A repeater instance's own scope (E3) overlays the
            // ambient one here exactly as it does when the child draws —
            // per-item sizing is the point of a repeater.
            let merged;
            let child_bindings = match &child_node.instance_bindings {
                Some(scope) => {
                    merged = merge_bindings(bindings, scope);
                    &merged
                }
                None => bindings,
            };
            let child_get = |name: &str| {
                child_node
                    .property(name)
                    .map(|v| super::data2d::substitute_bindings(v, child_bindings).into_owned())
            };
            let (scale_main, scale_cross) = if vertical {
                (child_node.transform.scale.y, child_node.transform.scale.x)
            } else {
                (child_node.transform.scale.x, child_node.transform.scale.y)
            };
            let (main_prop, cross_prop) = if vertical {
                ("ui_h", "ui_w")
            } else {
                ("ui_w", "ui_h")
            };
            let literal = |name: &str| child_get(name).and_then(|v| super::data2d::parse_length(&v, scale));
            // Per axis: a child that sizes to content only vertically still
            // takes its *width* from its authored `ui_w`, so a 640px-wide
            // panel in a column keeps its stated width and only its height
            // follows its rows.
            let (wants_w, wants_h) = match child_node.property("ui_size") {
                Some("content") => (true, true),
                Some("content_w") => (true, false),
                Some("content_h") => (false, true),
                _ => (false, false),
            };
            let content = child_node.content_size.get();
            let axis = |want: bool, pick: fn((f32, f32)) -> f32| content.filter(|_| want).map(pick);
            let (content_main, content_cross) = if vertical {
                (axis(wants_h, |(_, h)| h), axis(wants_w, |(w, _)| w))
            } else {
                (axis(wants_w, |(w, _)| w), axis(wants_h, |(_, h)| h))
            };

            // Space *before* this child on the main axis, on top of the
            // parent's uniform `ui_gap` — the mockups' `margin-top:34px`
            // between blocks of a column, which a single gap can't express
            // and which would otherwise be an empty spacer node per block.
            //
            // Its own property rather than reusing `ui_margin_top`/`_left`:
            // those already mean "inset this stretched node's rect" in
            // `resolve_ui_rect`, and a flow child that also stretches would
            // apply both and land at twice the offset. One name, one meaning,
            // and `ui_lead` reads as the flow concept it is on either axis.
            //
            // Leading only: a trailing space is the next child's leading one,
            // and supporting both invites CSS's margin-collapse rules.
            // Negative is allowed: the mockups' `margin-left:-2px` overlaps
            // one segment onto the previous, which is a real pixel-art idiom
            // (the title screen's cars are three rects joined that way).
            //
            // `"auto"` is CSS's `margin-top:auto` — the child takes all the
            // slack before it, which is how the mockups pin a card's footer to
            // the bottom of a fixed-height card whose middle rows size
            // themselves. Resolved to pixels after the measure pass, since the
            // slack is not known until every sibling has been measured.
            let authored_lead = child_get("ui_lead");
            auto_lead.push(authored_lead.as_deref().map(str::trim) == Some("auto"));
            lead.push(
                authored_lead
                    .and_then(|v| super::data2d::parse_length(&v, scale))
                    .unwrap_or(0.0)
                    * scale_main,
            );

            let grow = child_get("ui_grow").and_then(|v| v.trim().parse::<f32>().ok());
            tracks.push(match grow {
                Some(weight) if weight > 0.0 => Track::Weight(weight),
                _ => Track::Fixed(
                    content_main
                        .or_else(|| literal(main_prop))
                        .unwrap_or(0.0)
                        .max(0.0)
                        * scale_main,
                ),
            });
            cross.push(
                content_cross
                    .or_else(|| literal(cross_prop))
                    .unwrap_or(0.0)
                    .max(0.0)
                    * scale_cross,
            );
        }

        // --- Pass 2: place --------------------------------------------------
        // Growing children mean the leftover space is already spoken for, so
        // `justify` only applies when nothing grows — matching flexbox, where
        // `justify-content` has no visible effect once a child has `flex:1`.
        let grows = tracks.iter().any(|t| !matches!(t, Track::Fixed(_)));
        // `ui_lead: "auto"` resolved: the container's slack, split evenly
        // between the auto leads that asked for it, exactly as flexbox splits
        // it between `margin:auto`s. A growing sibling has already eaten the
        // slack, so — again as in flexbox — there is nothing left to take and
        // an auto lead falls back to zero.
        let autos = auto_lead.iter().filter(|a| **a).count();
        if autos > 0 && !grows {
            let content: f32 = tracks
                .iter()
                .map(|t| match t {
                    Track::Fixed(px) => px.max(0.0),
                    _ => 0.0,
                })
                .sum::<f32>()
                + lead.iter().sum::<f32>()
                + gap * (children.len().saturating_sub(1)) as f32;
            let axis = if vertical {
                padded.height
            } else {
                padded.width
            };
            let share = ((axis - content) / autos as f32).max(0.0);
            for (l, auto) in lead.iter_mut().zip(&auto_lead) {
                if *auto {
                    *l = share;
                }
            }
        }
        // A leading margin is space the child does not occupy, so it rides
        // inside that child's own track and is trimmed off the front of the
        // slot afterwards — the same fold-then-trim the justified path below
        // already uses for `gap`. Folding it in (rather than shifting the
        // child after placement) is what makes a growing sibling see the
        // margin as space already spoken for, so the row still sums to the
        // container instead of overflowing by the margins.
        let padded_tracks: Vec<Track> = tracks
            .iter()
            .zip(&lead)
            .map(|(t, m)| match t {
                // A weighted track has no pixels to fold into, so a growing
                // child's own margin is taken out of its slot at trim time
                // below; that is the one case where the margin comes out of
                // the child's share rather than the container's slack, which
                // is also what flexbox does with `flex:1` plus a margin.
                Track::Fixed(px) => Track::Fixed(px + m),
                other => *other,
            })
            .collect();
        let slots = if grows {
            if vertical {
                padded.distribute_v(&padded_tracks, gap)
            } else {
                padded.distribute_h(&padded_tracks, gap)
            }
        } else {
            // `justify_*` spaces items by slack alone, so the authored `gap`
            // is folded into the sizes and removed afterwards. Simpler than a
            // second justify variant that also knows about gaps.
            let sizes: Vec<f32> = padded_tracks
                .iter()
                .enumerate()
                .map(|(i, t)| match t {
                    Track::Fixed(px) => px + if i + 1 < tracks.len() { gap } else { 0.0 },
                    _ => 0.0,
                })
                .collect();
            let placed = if vertical {
                padded.justify_v(&sizes, justify)
            } else {
                padded.justify_h(&sizes, justify)
            };
            placed
                .into_iter()
                .enumerate()
                .map(|(i, r)| {
                    let trim = if i + 1 < tracks.len() { gap } else { 0.0 };
                    if vertical {
                        // y-up: a row's own extent is at its top, so trimming
                        // the folded-in gap moves the origin up, not down.
                        Rect::new(r.x, r.y + trim, r.width, (r.height - trim).max(0.0))
                    } else {
                        Rect::new(r.x, r.y, (r.width - trim).max(0.0), r.height)
                    }
                })
                .collect()
        };

        // Trim each folded-in leading margin back off the front of its slot,
        // so the child draws after the space rather than inside it. One pass
        // over both branches, since both placed with the margin folded in.
        let slots: Vec<Rect> = slots
            .into_iter()
            .zip(&lead)
            .map(|(r, &m)| {
                if m == 0.0 {
                    r
                } else if vertical {
                    // y-up again: the leading edge of a column child is its
                    // top, so the margin comes off the top of the slot. A
                    // negative lead adds height instead, pulling the child up
                    // to overlap its predecessor.
                    Rect::new(r.x, r.y, r.width, (r.height - m).max(0.0))
                } else {
                    Rect::new(r.x + m, r.y, (r.width - m).max(0.0), r.height)
                }
            })
            .collect();

        let aligned: Vec<Rect> = slots
            .into_iter()
            .zip(&cross)
            .map(|(slot, &cross_extent)| apply_cross_align(slot, vertical, align, cross_extent))
            .collect();

        // E-D: `ui_scroll_y` shifts this container's children along the column
        // before `ui_clip` (E5) trims them — 22 timing-tower rows in a 250px
        // panel, or a race-control log longer than its box. The value is how
        // far *down the list* the view has scrolled, so it moves children up
        // the screen, which is +y here.
        //
        // Applied to the flow's own slots rather than to each child's rect, so
        // it lands on both draw paths and on the hit rects at once: a click
        // after a scroll hits the row the player can actually see. Rust owns
        // the number — no input handling, no scrollbar, no momentum; the
        // mockups show none of those.
        let aligned: Vec<Rect> = match prop_f32("ui_scroll_y") {
            Some(dy) if dy != 0.0 => aligned
                .into_iter()
                .map(|r| Rect::new(r.x, r.y + dy, r.width, r.height))
                .collect(),
            _ => aligned,
        };

        // Tell each child the size the flow decided for it, so resolving its
        // own rect takes the slot instead of collapsing to its (absent)
        // ui_w/ui_h or re-anchoring inside it.
        //
        // The **main axis is always decided**: the flow placed this child
        // there, and the slot is either the share it grew to or exactly its
        // own measured extent. Either way the child belongs *at* its slot,
        // which is the one thing a flow is for.
        //
        // The **cross axis is decided only under a non-stretch `ui_align`**,
        // where the slot was likewise sized to the child and placed. The
        // default `stretch` decides nothing: the slot spans the whole cross
        // axis and filling it stays the child's own `ui_stretch_*` opt-in,
        // exactly as every scene authored before `ui_align` assumes.
        //
        // Cleared otherwise, so a node that leaves a flow does not keep a
        // stale size from a previous frame.
        let cross_decided = align != CrossAlign::Stretch;
        for (&child, slot) in children.iter().zip(&aligned) {
            if let Some(child_node) = self.get(child) {
                let main = if vertical { slot.height } else { slot.width };
                let cross = cross_decided
                    .then(|| if vertical { slot.width } else { slot.height })
                    .filter(|e| *e > 0.0);
                child_node.flow_size.set(if vertical {
                    (cross, Some(main))
                } else {
                    (Some(main), cross)
                });
            }
        }

        aligned
            .into_iter()
            .map(|slot| (slot.x, slot.y, slot.width, slot.height))
            .collect()
    }

    fn draw_node(
        &self,
        handle: NodeHandle2D,
        parent_world: &Transform2D,
        parent_ui_rect: (f32, f32, f32, f32),
        time: f32,
        frame: &mut Frame,
        bindings: &Bindings,
        inherited: crate::scene::data2d::UiInteractionState,
    ) {
        let Some(node) = self.get(handle) else {
            return;
        };
        if !node.visible {
            return;
        }

        let world = parent_world.compose(&node.transform);
        for sprite in &node.sprites {
            let offset = rotate_vec(sprite.offset * world.scale, world.rotation);
            frame.draw_sprite(
                DrawParams::new(
                    sprite.texture,
                    world.position + offset,
                    sprite.size * world.scale,
                )
                .with_color(sprite.color)
                .with_uv_rect(sprite.uv_rect)
                .with_flip_x(sprite.flip_x)
                .with_flip_y(sprite.flip_y)
                .with_rotation(world.rotation),
            );
        }

        // A repeater instance's own scope (E3) overlays the ambient bindings
        // for this node and everything under it, so ui_text: "P{pos}" resolves
        // against *this* item without every descendant needing to know it's
        // inside a repeat.
        let merged_bindings;
        let effective_bindings = match &node.instance_bindings {
            Some(scope) => {
                merged_bindings = merge_bindings(bindings, scope);
                &merged_bindings
            }
            None => bindings,
        };

        // UI primitive (if any) laid out relative to the parent's rect; the
        // resolved rect becomes the reference frame for this node's children.
        let state = self.interaction_state(handle, inherited);
        let (ui_rect, ui_visible) = crate::scene::data2d::draw_ui_node_with_bindings(
            frame,
            parent_ui_rect,
            node.transform.position,
            node.transform.scale,
            time,
            |n| node.property(n).map(str::to_owned),
            effective_bindings,
            node.sprites.first(),
            self.pixel_grid.get(),
            state,
            node.content_size.get(),
            node.flow_size.get(),
            !node.children.is_empty(),
        );
        // A hidden ui node (ui_visible: false) drew nothing, so it shouldn't
        // be hit-testable at a rect that has no on-screen primitive behind it.
        if ui_visible && node.property("ui").is_some() {
            let (x, y, w, h) = ui_rect;
            node.ui_rect
                .set(Some(Rect::from_pos_size(Vec2::new(x, y), Vec2::new(w, h))));
        } else {
            node.ui_rect.set(None);
        }

        // `ui_visible: false` is `display:none`, and that hides the whole
        // subtree — the node's children are its *content*, so painting them
        // under a hidden parent draws exactly the thing the author switched
        // off. It already leaves the flow (E-Z) and now it leaves the draw
        // too; before this, a hidden container's children rendered at whatever
        // rect the hidden parent resolved to, which is how a switched-off
        // screen variant left its labels floating over the live one.
        if !ui_visible {
            self.clear_subtree_hit_rects(handle);
            return;
        }

        let children = node.children.clone();
        let child_refs =
            self.resolve_child_references(node, ui_rect, &children, effective_bindings);
        for (child, child_ref) in children.into_iter().zip(child_refs) {
            self.draw_node(
                child,
                &world,
                child_ref,
                time,
                frame,
                effective_bindings,
                state,
            );
        }
    }

    /// A node's live interaction state (E6), which is its own state *or* any
    /// state inherited from an ancestor.
    ///
    /// Inheritance is what makes E6 usable on a real widget. Only one node can
    /// be `hovered`, but the smallest useful button is two nodes — a background
    /// rect and a label — and hovering it has to light up both. Without this,
    /// every composite widget has to push its own per-state colours from host
    /// code each frame, which is the hand-maintained duplication `ui_color_hover`
    /// exists to delete. Matches how every UI toolkit scopes hover: the state
    /// belongs to the widget, not to the one painted box under the cursor.
    fn interaction_state(
        &self,
        handle: NodeHandle2D,
        inherited: crate::scene::data2d::UiInteractionState,
    ) -> crate::scene::data2d::UiInteractionState {
        crate::scene::data2d::UiInteractionState {
            hovered: inherited.hovered || self.hovered.get() == Some(handle),
            pressed: inherited.pressed || self.pressed.get() == Some(handle),
            focused: inherited.focused || self.focused.get() == Some(handle),
        }
    }

    /// Draw the graph's UI primitives directly onto a caller-owned `canvas` (at
    /// whatever z-position the caller is drawing), rather than to per-node frame
    /// layers. Lets a HUD region be authored as a scene and dropped into an
    /// existing immediate-mode render pass. Sprite layers are not drawn here
    /// (they need the frame's sprite batch); use [`draw_at`](Self::draw_at) for
    /// sprite scenes. `time` drives `ui_bob_*` / `ui_sway_*` animations.
    pub fn draw_to_canvas(&self, canvas: &mut Canvas, time: f32) {
        self.draw_to_canvas_with_bindings(canvas, time, &Bindings::new());
    }

    /// Like [`draw_to_canvas`](Self::draw_to_canvas), but every `ui_*`
    /// property may contain `{key}` placeholders resolved against `bindings`
    /// (E2). Empty bindings behave exactly like `draw_to_canvas`.
    pub fn draw_to_canvas_with_bindings(
        &self,
        canvas: &mut Canvas,
        time: f32,
        bindings: &Bindings,
    ) {
        let (sw, sh) = canvas.screen_size();
        let screen = (-(sw as f32) / 2.0, -(sh as f32) / 2.0, sw as f32, sh as f32);
        self.draw_to_canvas_in_with_bindings(canvas, screen, time, bindings);
    }

    /// Like [`draw_to_canvas`](Self::draw_to_canvas), but the root reference
    /// rect (what `ui_stretch_*` / percentage sizes resolve against for the
    /// scene's root nodes) is supplied by the caller instead of being derived
    /// from the whole canvas. Lets a host draw the graph into a sub-rect of a
    /// larger surface — e.g. the rengine editor's viewport panel, which is
    /// smaller than the window it lives in — while keeping every `ui_*`
    /// resolution rule identical to runtime.
    pub fn draw_to_canvas_in(&self, canvas: &mut Canvas, root: (f32, f32, f32, f32), time: f32) {
        self.draw_to_canvas_in_with_bindings(canvas, root, time, &Bindings::new());
    }

    /// [`draw_to_canvas_in`](Self::draw_to_canvas_in) with `{key}` placeholder
    /// bindings (E2), matching [`draw_to_canvas_with_bindings`](Self::draw_to_canvas_with_bindings).
    pub fn draw_to_canvas_in_with_bindings(
        &self,
        canvas: &mut Canvas,
        root: (f32, f32, f32, f32),
        time: f32,
        bindings: &Bindings,
    ) {
        self.text_scale.set(Some(canvas.text_scale()));
        let roots = self.roots.clone();
        for &r in &roots {
            if self.subtree_wants_content_size(r) {
                self.measure_content_size(r, canvas, bindings);
            }
        }
        for r in roots {
            self.draw_node_on_canvas(r, root, time, canvas, bindings, Default::default());
        }
    }

    /// Post-order pass computing `ui_size: "content"` nodes' `(w, h)` from
    /// their children, before the real top-down draw reads it. Draw resolves
    /// a node's own rect before recursing into children (`draw_node`), so a
    /// parent cannot know a child's size when it needs its own — this pass
    /// runs the other direction first and caches the result on
    /// `SceneNode2D::content_size` for `resolve_ui_rect` to read.
    ///
    /// Callers gate this behind `subtree_wants_content_size(root)` so a
    /// scene with none of this authored — every existing scene today — pays
    /// for one cheap check per root and never enters the recursion at all.
    fn measure_content_size(&self, handle: NodeHandle2D, canvas: &Canvas, bindings: &Bindings) {
        let Some(node) = self.get(handle) else {
            return;
        };

        // Borrowed, not cloned. `Bindings` is a `HashMap<String, String>` and a
        // real screen carries a hundred or so entries; cloning it once per node
        // — and again per child visit below — made the measure pass the single
        // most expensive thing in the frame. Measured on Formula R's race
        // screen: 709 nodes x 108 bindings, 12.8ms of a 22ms frame, against
        // 3.6ms for the actual drawing.
        //
        // Only a node carrying its own instance scope needs a merged map, and
        // those are rare. This is the same `let merged; ... &merged` shape the
        // rest of this file already uses for exactly this reason.
        let merged;
        let effective_bindings: &Bindings = match &node.instance_bindings {
            Some(scope) => {
                merged = merge_bindings(bindings, scope);
                &merged
            }
            None => bindings,
        };

        for &child in &node.children {
            self.measure_content_size(child, canvas, effective_bindings);
        }

        if !sizes_to_content(node.property("ui_size")) {
            return;
        }

        let get = |name: &str| {
            node.property(name)
                .map(|v| super::data2d::substitute_bindings(v, effective_bindings).into_owned())
        };
        let scale = canvas.text_scale();
        let prop_f32 = |name: &str| get(name).and_then(|v| super::data2d::parse_length(&v, scale));

        let layout = get("ui_layout");
        let pad_left = prop_f32("ui_pad_left").unwrap_or(0.0);
        let pad_right = prop_f32("ui_pad_right").unwrap_or(0.0);
        let pad_top = prop_f32("ui_pad_top").unwrap_or(0.0);
        let pad_bottom = prop_f32("ui_pad_bottom").unwrap_or(0.0);
        let gap = prop_f32("ui_gap").unwrap_or(0.0);

        // A child's extent for this measurement: its own `content_size` if
        // *it* auto-sizes (set by the post-order recursion above, since
        // children were measured before this node), else its own authored
        // `ui_w`/`ui_h` (or measured text extent for a plain `ui: "text"`
        // leaf with no explicit size) — a non-content-sizing child still
        // has a real size, just not a computed one.
        let child_size = |child: NodeHandle2D| -> (f32, f32) {
            let Some(c) = self.get(child) else {
                return (0.0, 0.0);
            };
            let merged;
            let child_bindings: &Bindings = match &c.instance_bindings {
                Some(scope) => {
                    merged = merge_bindings(effective_bindings, scope);
                    &merged
                }
                None => effective_bindings,
            };
            let own = node_own_extent(c, canvas, child_bindings);
            // `ui_size` is per axis, so a child's measured size counts only on
            // the axis it actually auto-sizes. A `content_h` card measures a
            // width from its children too — that value is never applied to the
            // card itself (`resolve_ui_rect` keeps its authored `ui_w`), so a
            // parent reading it wholesale sized a row of 246px cards to the
            // 222px their blurbs happened to measure.
            match c.content_size.get() {
                Some(measured) => (
                    match c.property("ui_size") {
                        Some("content" | "content_w") => measured.0,
                        _ => own.0,
                    },
                    match c.property("ui_size") {
                        Some("content" | "content_h") => measured.1,
                        _ => own.1,
                    },
                ),
                None => own,
            }
        };

        // Out-of-flow children contribute nothing to their parent's size,
        // exactly as in CSS — an overlay pinned to a panel (`ui_absolute`)
        // must not inflate the panel it overlays, and a hidden variant
        // (`ui_visible: false`, this layer's `display:none`) must not size the
        // container to the branch that is not showing. Same test as the flow
        // itself applies, so a container measures exactly what it lays out.
        let flow_children: Vec<NodeHandle2D> = node
            .children
            .iter()
            .copied()
            .filter(|&c| {
                !self.get(c).is_some_and(|c| {
                    let hidden = c
                        .property("ui_visible")
                        .map(|v| super::data2d::substitute_bindings(v, &effective_bindings))
                        .is_some_and(|v| matches!(v.trim(), "false" | "0" | "no"));
                    hidden
                        || matches!(
                            c.property("ui_absolute").map(str::trim),
                            Some("true" | "1" | "yes")
                        )
                })
            })
            .collect();

        // A child's own `ui_lead` is main-axis space the flow will insert
        // before it, so a container measured without it comes out short by
        // exactly the leads it holds — and then packs its children tighter
        // than the flow places them, overlapping the ones that carry a lead.
        let child_lead = |child: NodeHandle2D| -> f32 {
            let Some(c) = self.get(child) else {
                return 0.0;
            };
            let merged;
            let child_bindings: &Bindings = match &c.instance_bindings {
                Some(scope) => {
                    merged = merge_bindings(effective_bindings, scope);
                    &merged
                }
                None => effective_bindings,
            };
            c.property("ui_lead")
                .map(|v| super::data2d::substitute_bindings(v, child_bindings).into_owned())
                .and_then(|v| super::data2d::parse_length(&v, scale))
                .unwrap_or(0.0)
        };
        let leads: f32 = flow_children.iter().map(|&c| child_lead(c)).sum();

        let (content_w, content_h) = match layout.as_deref() {
            Some("row") => {
                let n = flow_children.len();
                let sum_w: f32 = flow_children.iter().map(|&c| child_size(c).0).sum();
                // `flow_children`, not every child: the cross axis has the same
                // stake in it as the main one — an absolute overlay or a hidden
                // variant must not make the row taller than what it lays out.
                let max_h: f32 = flow_children
                    .iter()
                    .map(|&c| child_size(c).1)
                    .fold(0.0, f32::max);
                let gaps = if n > 1 { gap * (n as f32 - 1.0) } else { 0.0 };
                (sum_w + gaps + leads, max_h)
            }
            Some("column") => {
                let n = flow_children.len();
                let sum_h: f32 = flow_children.iter().map(|&c| child_size(c).1).sum();
                let max_w: f32 = flow_children
                    .iter()
                    .map(|&c| child_size(c).0)
                    .fold(0.0, f32::max);
                let gaps = if n > 1 { gap * (n as f32 - 1.0) } else { 0.0 };
                (max_w, sum_h + gaps + leads)
            }
            Some("grid") => {
                // A content-sized grid is as wide as its columns and as tall
                // as the rows its children wrap onto. Only `ui_cols` can be
                // measured this way: `ui_col_w`'s auto-fill count depends on
                // the width the parent is about to be given, which is the
                // circular case, so it falls back to one row.
                let n = flow_children.len();
                let cell_w: f32 = flow_children
                    .iter()
                    .map(|&c| child_size(c).0)
                    .fold(0.0, f32::max);
                let cell_h = prop_f32("ui_row_h").unwrap_or_else(|| {
                    flow_children
                        .iter()
                        .map(|&c| child_size(c).1)
                        .fold(0.0, f32::max)
                });
                let cols = prop_f32("ui_cols")
                    .filter(|c| *c >= 1.0)
                    .map_or(n.max(1), |c| c as usize);
                let rows = n.div_ceil(cols.max(1)).max(1);
                let gap_y = prop_f32("ui_gap_y").unwrap_or(gap);
                (
                    cols as f32 * cell_w + (cols.saturating_sub(1)) as f32 * gap,
                    rows as f32 * cell_h + (rows.saturating_sub(1)) as f32 * gap_y,
                )
            }
            _ => {
                // No ui_layout: children overlay (Button's implicit single
                // Panel child, say) — bounding box, the max of each axis,
                // starting from this node's own extent (a Panel with both a
                // background image and children sizes to whichever is larger).
                //
                // Its own extent already includes its padding (a text leaf
                // measures the box it paints into), and this pass adds padding
                // to whatever it returns — so take the extent's *content*
                // here, or a padded text leaf counts its padding twice.
                let own = node_own_extent(node, canvas, &effective_bindings);
                let own = (
                    (own.0 - pad_left - pad_right).max(0.0),
                    (own.1 - pad_top - pad_bottom).max(0.0),
                );
                let max_w: f32 = node
                    .children
                    .iter()
                    .map(|&c| child_size(c).0)
                    .fold(own.0, f32::max);
                let max_h: f32 = node
                    .children
                    .iter()
                    .map(|&c| child_size(c).1)
                    .fold(own.1, f32::max);
                (max_w, max_h)
            }
        };

        // The border sits outside the padding, as in CSS: a measured box has no
        // authored size for the border to eat into, so it adds to the box the
        // children asked for rather than being drawn over their padding.
        let (border_x, border_y) = super::data2d::border_extent(&get, node.transform.scale);

        node.content_size.set(Some(clamp_to_min(
            (
                content_w + pad_left + pad_right + border_x,
                content_h + pad_top + pad_bottom + border_y,
            ),
            &prop_f32,
        )));
    }

    /// Whether `handle` or anything under it authors `ui_size: "content"`.
    /// Called once per root per draw, before `measure_content_size`, so a
    /// scene that never authors this — every existing scene today — skips
    /// the measure pass and its recursion entirely rather than descending
    /// through nodes that have nothing to compute. `.any` short-circuits on
    /// the first match, so an early adopter near the top of a deep tree is
    /// also cheap; a scene with none anywhere still visits every node once
    /// to confirm that, same as the measure pass it's guarding would.
    fn subtree_wants_content_size(&self, handle: NodeHandle2D) -> bool {
        let Some(node) = self.get(handle) else {
            return false;
        };
        if sizes_to_content(node.property("ui_size")) {
            return true;
        }
        node.children
            .iter()
            .any(|&c| self.subtree_wants_content_size(c))
    }

    fn draw_node_on_canvas(
        &self,
        handle: NodeHandle2D,
        parent_ui_rect: (f32, f32, f32, f32),
        time: f32,
        canvas: &mut Canvas,
        bindings: &Bindings,
        inherited: crate::scene::data2d::UiInteractionState,
    ) {
        let Some(node) = self.get(handle) else {
            return;
        };
        if !node.visible {
            return;
        }

        // A repeater instance's own scope (E3) overlays the ambient bindings
        // — see the identical comment in `draw_node`.
        let merged_bindings;
        let effective_bindings = match &node.instance_bindings {
            Some(scope) => {
                merged_bindings = merge_bindings(bindings, scope);
                &merged_bindings
            }
            None => bindings,
        };

        let state = self.interaction_state(handle, inherited);
        let (ui_rect, ui_visible) = crate::scene::data2d::draw_ui_node_on_with_bindings(
            canvas,
            parent_ui_rect,
            node.transform.position,
            node.transform.scale,
            time,
            |n| node.property(n),
            effective_bindings,
            node.sprites.first(),
            self.pixel_grid.get(),
            state,
            node.content_size.get(),
            node.flow_size.get(),
            !node.children.is_empty(),
        );
        if ui_visible && node.property("ui").is_some() {
            let (x, y, w, h) = ui_rect;
            node.ui_rect
                .set(Some(Rect::from_pos_size(Vec2::new(x, y), Vec2::new(w, h))));
        } else {
            node.ui_rect.set(None);
        }

        // The generic focus ring: when this node is the focused one AND it
        // authorises a ring (`ui_focus_ring_color`, an explicit opt-in so
        // buttons that already paint their own `_focus` state don't get a
        // second outline), draw an outline around its resolved rect. This is
        // what the controller focus walk points at on the race HUD, where a
        // hover colour is meaningless because there is no cursor.
        if ui_visible
            && self.focused.get() == Some(handle)
            && node.property("ui").is_some()
            && node
                .property("ui_focus_ring_color")
                .map(|c| !c.trim().is_empty())
                .unwrap_or(false)
        {
            let (x, y, w, h) = ui_rect;
            let ring = crate::scene::data2d::parse_srgb_color(
                node.property("ui_focus_ring_color").as_deref(),
                Color::from_srgb8(255, 205, 90, 255),
            );
            let ring_w = node
                .property("ui_focus_ring_w")
                .and_then(|v| v.trim().parse::<f32>().ok())
                .unwrap_or(2.0)
                .max(1.0);
            // Four thin bars (draw_border semantics are the node's own); the
            // ring paints *outside*-ish: the top bar sits just above the rect
            // and the sides straddle its edges so a 2px ring reads as a
            // selection rather than a replacement border.
            let o = ring_w * 0.5;
            canvas.rect(x - o, y + h - o, w + ring_w, ring_w, ring);
            canvas.rect(x - o, y - o, w + ring_w, ring_w, ring);
            canvas.rect(x - o, y - o, ring_w, h + ring_w, ring);
            canvas.rect(x + w - o, y - o, ring_w, h + ring_w, ring);
        }

        // `display:none` hides the subtree — see `draw_node`'s copy of this.
        if !ui_visible {
            self.clear_subtree_hit_rects(handle);
            return;
        }

        // ui_clip: true (E5) confines this node's children to its resolved
        // rect — a scrolling standings/log region can overflow its panel
        // without painting over whatever sits below it. Substituted through
        // bindings first, same as every other ui_* value.
        let clip = node
            .property("ui_clip")
            .map(|v| crate::scene::data2d::substitute_bindings(v, effective_bindings));
        let clipped = matches!(clip.as_deref(), Some("true" | "1" | "yes"));
        if clipped {
            let (x, y, w, h) = ui_rect;
            canvas.push_clip(x, y, w, h);
        }
        let children = node.children.clone();
        let child_refs =
            self.resolve_child_references(node, ui_rect, &children, effective_bindings);
        for (child, child_ref) in children.into_iter().zip(child_refs) {
            self.draw_node_on_canvas(child, child_ref, time, canvas, effective_bindings, state);
        }
        if clipped {
            canvas.pop_clip();
        }
    }

    // --- internal helpers -------------------------------------------------

    /// Drop the cached hit rect of every node under `handle` (inclusive).
    ///
    /// A subtree that `ui_visible: false` skipped never reached its own
    /// bookkeeping, so without this its descendants keep whatever rect they
    /// resolved to on the last frame they *were* shown — and a click would
    /// still land on a button nobody can see.
    fn clear_subtree_hit_rects(&self, handle: NodeHandle2D) {
        let Some(node) = self.get(handle) else {
            return;
        };
        node.ui_rect.set(None);
        for child in node.children.clone() {
            self.clear_subtree_hit_rects(child);
        }
    }

    fn insert_detached(&mut self, node: SceneNode2D) -> NodeHandle2D {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            NodeHandle2D {
                index,
                generation: slot.generation,
            }
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                node: Some(node),
            });
            NodeHandle2D {
                index,
                generation: 0,
            }
        }
    }

    /// Set a node's parent link and add it to the parent's child list. Does not
    /// touch the root set; callers manage that around the link change.
    fn set_parent_link(&mut self, handle: NodeHandle2D, parent: Option<NodeHandle2D>) {
        if let Some(node) = self.get_mut(handle) {
            node.parent = parent;
        }
        if let Some(parent) = parent {
            if let Some(parent_node) = self.get_mut(parent) {
                if !parent_node.children.contains(&handle) {
                    parent_node.children.push(handle);
                }
            }
        }
    }

    fn unlink_from_parent(&mut self, handle: NodeHandle2D) {
        let parent = self.get(handle).and_then(|node| node.parent);
        if let Some(parent) = parent {
            if let Some(parent_node) = self.get_mut(parent) {
                parent_node.children.retain(|child| *child != handle);
            }
        }
        if let Some(node) = self.get_mut(handle) {
            node.parent = None;
        }
    }

    fn is_descendant(&self, maybe_descendant: NodeHandle2D, ancestor: NodeHandle2D) -> bool {
        let mut current = self.parent(maybe_descendant);
        while let Some(node) = current {
            if node == ancestor {
                return true;
            }
            current = self.parent(node);
        }
        false
    }
}

/// Parse a `ui_repeat_items` property into one [`Bindings`] scope per row.
///
/// The property is a JSON array of flat objects, each object one repeated
/// instance's bindings: `[{"pos":"1","name":"REYES"},{"pos":"2",...}]`.
/// Values are coerced to strings (a JSON number or bool is accepted and
/// stringified) because [`Bindings`] is `HashMap<String, String>` and
/// authoring `"1"` vs `1` in the editor shouldn't be a silent failure.
///
/// Returns `None` for anything that isn't a JSON array — including an
/// unresolved `{binding}` placeholder, which a scene may legitimately author
/// here. `Some(vec![])` (an authored empty list) is distinct: it means zero
/// rows, deliberately.
fn parse_repeat_items(raw: &str) -> Option<Vec<Bindings>> {
    let value: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    let rows = value.as_array()?;
    Some(
        rows.iter()
            .map(|row| {
                row.as_object()
                    .map(|fields| {
                        fields
                            .iter()
                            .filter_map(|(key, value)| {
                                let text = match value {
                                    serde_json::Value::String(s) => s.clone(),
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    // null/array/object have no sensible
                                    // single-string form; skip rather than
                                    // inventing one.
                                    _ => return None,
                                };
                                Some((key.clone(), text))
                            })
                            .collect::<Bindings>()
                    })
                    .unwrap_or_default()
            })
            .collect(),
    )
}

/// A node's own natural `(w, h)`, for the content-sizing measure pass
/// (`SceneWorld2D::measure_content_size`): the size it would draw at from
/// its own properties, ignoring any `ui_size: "content"` on itself (its
/// caller reads `content_size` first and only falls back to this for a
/// child that isn't itself auto-sizing). A `text`/`button` leaf with no
/// explicit `ui_w`/`ui_h` measures its rendered extent; anything else with
/// no size at all is `(0, 0)`, same as an unauthored `ui_w`/`ui_h` resolves
/// to for actual drawing.
fn node_own_extent(node: &SceneNode2D, canvas: &Canvas, bindings: &Bindings) -> (f32, f32) {
    let get = |name: &str| {
        node.property(name)
            .map(|v| super::data2d::substitute_bindings(v, bindings).into_owned())
    };
    let prop_f32 = |name: &str| get(name).and_then(|v| super::data2d::parse_length(&v, canvas.text_scale()));

    // A measured axis is the fallback only for whichever of w/h wasn't
    // itself authored — a text node with an explicit ui_w but no ui_h still
    // measures its own line height rather than losing it to a blanket
    // "any literal present wins" check.
    let measured = match node.property("ui") {
        Some("text") | Some("button") => {
            let text = get("ui_text").unwrap_or_default();
            if text.is_empty() {
                (0.0, 0.0)
            } else {
                let size = prop_f32("ui_text_size").unwrap_or(12.0);
                // Measured in the node's *own* font (E-C), not font 0 — a
                // content-sized Silkscreen label measured with the default
                // face's metrics would size every screen slightly wrong, and
                // the error would only show as drift, never as a failure.
                // Tracking widens the run, so a content-sized label that
                // ignored it would size to less than it paints.
                canvas.measure_text_tracked(
                    super::data2d::node_font(&get),
                    &text,
                    size,
                    prop_f32("ui_tracking").unwrap_or(0.0),
                )
            }
        }
        // A run of differently-coloured spans measures as the one line it
        // paints: the widths sum, the height is a single line box. Without
        // this a content-sized `text_spans` measured zero, so a row of them
        // (`BANK $1.9M | LEGACY 25 | SEED …`) stacked at the same x.
        Some("text_spans") => {
            let size = prop_f32("ui_text_size").unwrap_or(12.0);
            let font = super::data2d::node_font(&get);
            let width: f32 = (0..)
                .map_while(|i| get(&format!("ui_span_{i}_text")))
                .map(|text| canvas.measure_text_tracked(font, &text, size, 0.0).0)
                .sum();
            match width > 0.0 {
                true => (width, canvas.line_height_in(font, size)),
                false => (0.0, 0.0),
            }
        }
        // A wrapped block measures to the box its own wrap produces, so a
        // prose panel can be `ui_size: "content"` and be exactly as tall as
        // the text needs. Without this it measured zero and collapsed, which
        // is why every prose block until now carried a hand-counted `ui_h`.
        //
        // Width is the wrap width, not the widest line: `ui_wrap_w` is a
        // constraint the author states, and a block that shrank to its
        // longest line would re-wrap differently the moment it was measured
        // inside a content-sized parent.
        Some("text_block") => {
            let text = get("ui_text").unwrap_or_default();
            let wrap_w = prop_f32("ui_wrap_w").or_else(|| prop_f32("ui_w"));
            match (text.is_empty(), wrap_w) {
                (false, Some(wrap_w)) if wrap_w > 0.0 => {
                    let (_, h) = canvas.measure_text_block_in(
                        super::data2d::node_font(&get),
                        &text,
                        prop_f32("ui_text_size").unwrap_or(12.0),
                        wrap_w,
                        super::data2d::node_leading(&get),
                    );
                    (wrap_w, h)
                }
                // No wrap width to wrap against: the block has no intrinsic
                // width of its own, so it stays zero rather than guessing one.
                _ => (0.0, 0.0),
            }
        }
        _ => (0.0, 0.0),
    };

    // A text leaf's own padding is part of the box it measures to, the same way
    // it is part of the box the text is drawn into (`draw_ui_kind_dyn` insets
    // the line by it). A content-sized card header is `padding:9px 10px` around
    // one line box; measuring only the line would size it to the ink and clip
    // its own padding away.
    let pad = |n: &str| prop_f32(n).unwrap_or(0.0);
    // …and so is its border, for the same reason: a card header measured to its
    // line box plus its padding would have its own 2px rule drawn over that
    // padding rather than around it.
    let (border_x, border_y) = super::data2d::border_extent(&get, node.transform.scale);
    let measured = (
        measured.0 + pad("ui_pad_left") + pad("ui_pad_right") + border_x,
        measured.1 + pad("ui_pad_top") + pad("ui_pad_bottom") + border_y,
    );

    clamp_to_min(
        (
            prop_f32("ui_w").unwrap_or(measured.0),
            prop_f32("ui_h").unwrap_or(measured.1),
        ),
        &prop_f32,
    )
}

/// CSS `min-width`/`min-height` on a measured size — a floor, never a cap.
///
/// Only measured sizes need it: an authored `ui_w` is already whatever the
/// author said. The mockups use it to keep a row of cards the same height when
/// their blurbs wrap to different line counts (`min-height:52px` on the
/// archetype cards), which is otherwise the one thing content sizing cannot
/// express — the shortest card would set its own height and the row would come
/// out ragged.
fn clamp_to_min(size: (f32, f32), prop_f32: &impl Fn(&str) -> Option<f32>) -> (f32, f32) {
    (
        size.0.max(prop_f32("ui_min_w").unwrap_or(0.0)),
        size.1.max(prop_f32("ui_min_h").unwrap_or(0.0)),
    )
}

/// Whether a node measures itself against its children on *either* axis.
///
/// `ui_size` is per axis: `"content"` is both, `"content_w"`/`"content_h"` one
/// each. The measure pass runs whenever any axis wants it and computes the
/// full `(w, h)`; which of the two is actually applied is decided later, in
/// `resolve_ui_rect`. Measuring the unused axis costs nothing and keeps the
/// two passes from needing to agree about anything but this.
fn sizes_to_content(value: Option<&str>) -> bool {
    matches!(value, Some("content" | "content_w" | "content_h"))
}

/// How a flow container spreads leftover main-axis space when no child grows
/// (E-A). Unset — and any unrecognised value — is `Start`, which is the
/// packed-from-the-start flow this layer did before `ui_justify` existed.
fn parse_justify(value: Option<&str>) -> Justify {
    match value {
        Some("center") => Justify::Center,
        Some("end") => Justify::End,
        Some("space_between") => Justify::SpaceBetween,
        Some("space_around") => Justify::SpaceAround,
        Some("space_evenly") => Justify::SpaceEvenly,
        _ => Justify::Start,
    }
}

/// Cross-axis placement of a child within its flow slot (E-B).
#[derive(Clone, Copy, PartialEq, Eq)]
enum CrossAlign {
    /// The slot spans the full cross-axis and the child resolves within it.
    /// The default, and what every scene authored before E-B relies on.
    Stretch,
    Start,
    Center,
    End,
}

fn parse_align(value: Option<&str>) -> CrossAlign {
    match value {
        Some("start") => CrossAlign::Start,
        Some("center") => CrossAlign::Center,
        Some("end") => CrossAlign::End,
        _ => CrossAlign::Stretch,
    }
}

/// Narrow a full-cross-axis flow slot down to `extent` and place it (E-B).
///
/// `Stretch` returns the slot untouched, so the default path adds nothing but
/// a comparison. A child that never declared a cross-axis size measures as
/// zero, which would collapse it — so a zero extent also falls back to the
/// full slot rather than silently vanishing.
fn apply_cross_align(slot: Rect, vertical: bool, align: CrossAlign, extent: f32) -> Rect {
    if align == CrossAlign::Stretch || extent <= 0.0 {
        return slot;
    }
    if vertical {
        // Column: the cross axis is x, which runs left→right.
        let free = (slot.width - extent).max(0.0);
        let x = match align {
            CrossAlign::Start => slot.x,
            CrossAlign::Center => slot.x + free * 0.5,
            CrossAlign::End => slot.x + free,
            CrossAlign::Stretch => unreachable!(),
        };
        Rect::new(x, slot.y, extent, slot.height)
    } else {
        // Row: the cross axis is y, which is **y-up** — `start` is the top
        // edge, matching how `Stack` and `distribute_v` hand out rows in
        // reading order. Getting this backwards would mirror every aligned
        // row in the game and look like a data bug, not a layout one.
        let free = (slot.height - extent).max(0.0);
        let y = match align {
            CrossAlign::Start => slot.top() - extent,
            CrossAlign::Center => slot.y + free * 0.5,
            CrossAlign::End => slot.y,
            CrossAlign::Stretch => unreachable!(),
        };
        Rect::new(slot.x, y, slot.width, extent)
    }
}

/// Overlay a repeater instance's own scope onto the ambient bindings — the
/// item's own fields win on key collision, so `ui_text: "{name}"` inside a
/// repeated row resolves to that row's name even if an outer scope happens
/// to define the same key for something else.
fn merge_bindings(ambient: &Bindings, item: &Bindings) -> Bindings {
    let mut merged = ambient.clone();
    merged.extend(item.iter().map(|(k, v)| (k.clone(), v.clone())));
    merged
}

fn rotate_vec(v: Vec2, radians: f32) -> Vec2 {
    if radians == 0.0 {
        return v;
    }
    let (sin, cos) = radians.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetPack;
    use crate::scene::{Prefab2DDef, Scene2D, Scene2DDef, SceneInstance2DDef};
    use std::path::Path;

    fn props(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn spawn_get_and_mutate_roundtrip() {
        let mut world = SceneWorld2D::new();
        let handle = world.spawn(
            SceneNode2D::new("player")
                .with_name("hero")
                .with_position(Vec2::new(10.0, 20.0)),
        );

        assert_eq!(world.len(), 1);
        assert_eq!(world.get(handle).unwrap().position(), Vec2::new(10.0, 20.0));

        world
            .get_mut(handle)
            .unwrap()
            .translate(Vec2::new(5.0, -5.0));
        assert_eq!(world.get(handle).unwrap().position(), Vec2::new(15.0, 15.0));

        assert_eq!(world.find_by_name("hero"), Some(handle));
    }

    #[test]
    fn despawn_invalidates_handle_and_reuses_slot_safely() {
        let mut world = SceneWorld2D::new();
        let first = world.spawn(SceneNode2D::new("a"));
        assert!(world.contains(first));

        assert!(world.despawn(first));
        assert!(!world.contains(first));
        assert!(world.get(first).is_none());

        // The freed slot is reused, but the new handle's generation differs so
        // the old handle stays invalid (no aliasing).
        let second = world.spawn(SceneNode2D::new("b"));
        assert_eq!(first.index, second.index);
        assert_ne!(first.generation, second.generation);
        assert!(world.contains(second));
        assert!(!world.contains(first));
    }

    #[test]
    fn despawn_removes_entire_subtree() {
        let mut world = SceneWorld2D::new();
        let root = world.spawn(SceneNode2D::new("root"));
        let child = world.spawn_child(root, SceneNode2D::new("child"));
        let grandchild = world.spawn_child(child, SceneNode2D::new("grandchild"));

        assert_eq!(world.len(), 3);
        assert!(world.despawn(root));
        assert_eq!(world.len(), 0);
        assert!(!world.contains(child));
        assert!(!world.contains(grandchild));
        assert!(world.roots().is_empty());
    }

    #[test]
    fn reparent_updates_links_and_rejects_cycles() {
        let mut world = SceneWorld2D::new();
        let a = world.spawn(SceneNode2D::new("a"));
        let b = world.spawn(SceneNode2D::new("b"));

        assert!(world.reparent(b, Some(a)));
        assert_eq!(world.parent(b), Some(a));
        assert_eq!(world.children(a), vec![b]);
        assert_eq!(world.roots(), &[a]);

        // Parenting an ancestor under its descendant must be rejected.
        assert!(!world.reparent(a, Some(b)));
        // Self-parenting must be rejected.
        assert!(!world.reparent(a, Some(a)));

        // Moving back to root restores the root set.
        assert!(world.reparent(b, None));
        assert_eq!(world.parent(b), None);
        assert!(world.children(a).is_empty());
    }

    #[test]
    fn world_transform_composes_parent_chain() {
        let mut world = SceneWorld2D::new();
        let parent = world.spawn(SceneNode2D::new("parent").with_transform(Transform2D {
            position: Vec2::new(100.0, 0.0),
            rotation: std::f32::consts::FRAC_PI_2,
            scale: Vec2::splat(2.0),
        }));
        let child = world.spawn_child(
            parent,
            SceneNode2D::new("child").with_position(Vec2::new(10.0, 0.0)),
        );

        let world_t = world.world_transform(child).unwrap();
        // Child local (10,0) scaled by 2 -> (20,0), rotated 90deg -> (0,20),
        // offset by parent position (100,0) -> (100,20).
        assert!((world_t.position.x - 100.0).abs() < 1e-3);
        assert!((world_t.position.y - 20.0).abs() < 1e-3);
        assert!((world_t.scale.x - 2.0).abs() < 1e-3);
    }

    #[test]
    fn node_bounds_use_authored_size_from_editor_or_explicit_wh() {
        let mut world = SceneWorld2D::new();

        // An editor-authored size (editor_size_x/y) makes a sprite-less node
        // pickable with no extra wiring.
        let a = world.spawn(SceneNode2D::new("a").with_position(Vec2::ZERO));
        {
            let node = world.get_mut(a).unwrap();
            node.set_property("editor_size_x", "60");
            node.set_property("editor_size_y", "40");
        }
        let bounds = world.node_bounds(a).unwrap();
        assert_eq!((bounds.width, bounds.height), (60.0, 40.0));
        assert_eq!(world.hit_test(Vec2::new(30.0, 20.0)), Some(a));

        // Explicit w/h (via the typed setter) overrides the editor size.
        let b = world.spawn(SceneNode2D::new("b").with_position(Vec2::new(200.0, 0.0)));
        {
            let node = world.get_mut(b).unwrap();
            node.set_property("editor_size_x", "10");
            node.set_property("editor_size_y", "10");
            node.set_size(Vec2::new(80.0, 50.0));
        }
        assert_eq!(world.get(b).unwrap().size(), Some(Vec2::new(80.0, 50.0)));
        let bounds_b = world.node_bounds(b).unwrap();
        assert_eq!((bounds_b.width, bounds_b.height), (80.0, 50.0));
    }

    #[test]
    fn em_lengths_ride_the_text_scale_and_plain_numbers_do_not() {
        // T1c: a column whose rows are hand-sized against 8px text needs a
        // unit that grows with the glyphs. `20em` is that unit; `20` is not,
        // and a panel border or a sprite box must stay exactly where it was.
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("list"));
        {
            let node = world.get_mut(container).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w", "100");
            node.set_property("ui_h", "100");
            node.set_property("ui_layout", "column");
        }
        let [scaling, fixed] = [("scaling", "20em"), ("fixed", "20")].map(|(name, h)| {
            let row = world.spawn_child(container, SceneNode2D::new(name));
            let node = world.get_mut(row).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_h", h);
            node.set_property("ui_stretch_x", "true");
            node.set_property("ui_stretch_y", "true");
            row
        });

        let mut canvas = Canvas::new((200, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        assert_eq!(world.resolved_rect(scaling).unwrap().height, 20.0);
        assert_eq!(world.resolved_rect(fixed).unwrap().height, 20.0);

        // Turned up, only the `em` row grows — and it grows in the *flow*
        // measure, not just its own rect, so the row below it moves down.
        canvas.set_text_scale(2.0);
        world.draw_to_canvas(&mut canvas, 0.0);
        let grown = world.resolved_rect(scaling).unwrap();
        let stayed = world.resolved_rect(fixed).unwrap();
        assert_eq!(grown.height, 40.0);
        assert_eq!(stayed.height, 20.0);
        assert_eq!(stayed.top(), grown.bottom());
    }

    #[test]
    fn ui_layout_column_stacks_children_top_to_bottom_with_gap() {
        // E4: ui_layout: "column" on a container flows its children in order
        // via Stack, each taking its own ui_h as the main-axis extent and
        // auto-filling the cross-axis (width) — this is what turns "N
        // hand-placed rows" into "author one row, repeat it" (E3's dependency).
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("list"));
        {
            let node = world.get_mut(container).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w", "100");
            node.set_property("ui_h", "100");
            node.set_property("ui_layout", "column");
            node.set_property("ui_gap", "5");
            node.set_property("ui_pad_top", "10");
        }
        // ui_stretch_x/_y fill the slot Stack hands a child — that slot is
        // only a reference rect, same as any other child's; nothing about
        // being inside a flow container makes a node auto-fill without the
        // same opt-in every other stretch fill already needs. ui_h here
        // drives Stack's main-axis extent (via resolve_child_references'
        // peek), and ui_stretch_y then fills that exact slot height — a
        // child that wants to be *smaller* than its slot would skip
        // ui_stretch_y and use its own ui_h to size within it instead.
        let row_a = world.spawn_child(container, SceneNode2D::new("row_a"));
        world.get_mut(row_a).unwrap().set_property("ui", "rect");
        world.get_mut(row_a).unwrap().set_property("ui_h", "20");
        world
            .get_mut(row_a)
            .unwrap()
            .set_property("ui_stretch_x", "true");
        world
            .get_mut(row_a)
            .unwrap()
            .set_property("ui_stretch_y", "true");
        let row_b = world.spawn_child(container, SceneNode2D::new("row_b"));
        world.get_mut(row_b).unwrap().set_property("ui", "rect");
        world.get_mut(row_b).unwrap().set_property("ui_h", "30");
        world
            .get_mut(row_b)
            .unwrap()
            .set_property("ui_stretch_x", "true");
        world
            .get_mut(row_b)
            .unwrap()
            .set_property("ui_stretch_y", "true");

        let mut canvas = Canvas::new((200, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        // Default anchor "center" against a 200x200 viewport (rx=-100,
        // rw=200 -> anchor point 0) places the container's bottom-left
        // (not its center) at that anchor point: rect (0, 0, 100, 100).
        // Padded top 10 -> content area (0, 0, 100, 90), top at 90.
        // row_a (h=20) takes the first slot: top=90, y = 90-20 = 70.
        let a = world.resolved_rect(row_a).expect("row_a drawn");
        assert!(
            (a.x - 0.0).abs() < 1e-3,
            "ui_stretch_x fills the slot: x={}",
            a.x
        );
        assert!(
            (a.width - 100.0).abs() < 1e-3,
            "ui_stretch_x fills the slot: w"
        );
        assert!((a.y - 70.0).abs() < 1e-3, "row_a y: {}", a.y);
        assert!((a.height - 20.0).abs() < 1e-3, "row_a h");

        // row_b (h=30) starts after row_a + 5px gap: top = 70 - 5 = 65, y = 65-30 = 35.
        let b = world.resolved_rect(row_b).expect("row_b drawn");
        assert!(
            (b.y - 35.0).abs() < 1e-3,
            "row_b y accounts for gap: {}",
            b.y
        );
        assert!((b.height - 30.0).abs() < 1e-3, "row_b h");

        // No overlap: row_b's top sits exactly gap px below row_a's bottom.
        assert!((a.y - (b.y + b.height) - 5.0).abs() < 1e-3);
    }

    fn repeat_source(rows: &[(&str, &str)]) -> Vec<Bindings> {
        rows.iter()
            .map(|&(pos, name)| {
                let mut scope = Bindings::new();
                scope.insert("pos".to_string(), pos.to_string());
                scope.insert("name".to_string(), name.to_string());
                scope
            })
            .collect()
    }

    #[test]
    fn a_button_with_children_defers_its_marker_and_label_but_keeps_its_bar() {
        // A Button with authored children treats them as its content: it
        // draws no marker or label of its own, because those children *are*
        // the content. But its **bar still paints when authored** — a fill
        // and content are not alternatives. The mockups' menu rows are a div
        // with a background that also holds a caret and a label, and the
        // selected row needs both.
        //
        // This originally deferred the bar too, so an authored
        // `ui_bar_color_focus` silently painted nothing and the only way to
        // get a fill under content was a sibling Panel behind it — the
        // stacked-node shape the UI overhaul exists to remove.
        let build = |bar: Option<&str>| {
            let mut world = SceneWorld2D::new();
            let button = world.spawn(SceneNode2D::new("row"));
            {
                let n = world.get_mut(button).unwrap();
                n.set_property("ui", "button");
                n.set_property("ui_w", "80");
                n.set_property("ui_h", "30");
                // A marker and a label that must NOT draw: the children are
                // the content now.
                n.set_property("ui_marker", ">");
                n.set_property("ui_marker_color", "230,178,60,255");
                n.set_property("ui_text", "NEW GAME");
                if let Some(bar) = bar {
                    n.set_property("ui_bar_color", bar);
                }
            }
            let child = world.spawn_child(button, SceneNode2D::new("label"));
            {
                let n = world.get_mut(child).unwrap();
                n.set_property("ui", "rect");
                n.set_property("ui_color", "40,52,78,255");
                n.set_property("ui_w", "40");
                n.set_property("ui_h", "20");
            }
            let mut canvas = Canvas::new((200, 100), std::ptr::null());
            world.draw_to_canvas(&mut canvas, 0.0);
            (canvas.verts.len(), world.resolved_rect(button).is_some())
        };

        // No bar authored: only the child draws. A Button that really is just
        // its children is unchanged — and the built-in marker/label stay
        // deferred, or this would be far more than one quad. (Text needs a
        // font atlas a test Canvas doesn't have, so a drawn label would
        // panic rather than merely add quads.)
        let (verts, has_rect) = build(None);
        assert_eq!(verts, 6, "only the child draws when no bar is authored");
        assert!(has_rect, "Button still contributes its hit rect");

        // Bar authored: the fill draws *under* the child, so two quads.
        let (verts, _) = build(Some("230,178,60,200"));
        assert_eq!(verts, 12, "an authored bar paints beneath the content");

        // A fully transparent bar is nothing to draw, same as everywhere else.
        let (verts, _) = build(Some("230,178,60,0"));
        assert_eq!(verts, 6, "a transparent bar paints nothing");
    }

    #[test]
    fn a_childless_button_still_draws_its_own_built_in_bar() {
        // The other half of the caveat: no children means the simple,
        // built-in path (title_menu_row_* after collapse) is unchanged.
        let mut world = SceneWorld2D::new();
        let button = world.spawn(SceneNode2D::new("row"));
        {
            let n = world.get_mut(button).unwrap();
            n.set_property("ui", "button");
            n.set_property("ui_bar_color", "230,178,60,200");
        }
        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        assert_eq!(canvas.verts.len(), 6, "the built-in bar should have drawn");
    }

    #[test]
    fn content_sized_row_sums_its_childrens_widths() {
        // The mechanism archetype_row_* needs: a container's own w/h comes
        // from its children instead of an authored literal, so a marker
        // sitting next to a variable-width label doesn't need its own
        // hand-matched bound width — it just sits where the row's flow
        // layout puts it, and the row is exactly as wide as its content.
        let mut world = SceneWorld2D::new();
        let row = world.spawn(SceneNode2D::new("row"));
        {
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_size", "content");
            n.set_property("ui_layout", "row");
            n.set_property("ui_gap", "5");
        }
        let marker = world.spawn_child(row, SceneNode2D::new("marker"));
        {
            let n = world.get_mut(marker).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "20");
            n.set_property("ui_h", "10");
        }
        let label = world.spawn_child(row, SceneNode2D::new("label"));
        {
            let n = world.get_mut(label).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "80");
            n.set_property("ui_h", "14");
        }

        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let rect = world.resolved_rect(row).expect("row drew");
        // 20 + 5 (gap) + 80 = 105 wide; tallest child (14) sets the height.
        assert!(
            (rect.width - 105.0).abs() < 1e-3,
            "got width {}",
            rect.width
        );
        assert!(
            (rect.height - 14.0).abs() < 1e-3,
            "got height {}",
            rect.height
        );
    }

    #[test]
    fn a_scene_authoring_no_content_sizing_never_enters_the_measure_pass() {
        // The zero-cost claim: subtree_wants_content_size gates the whole
        // measure pre-pass, so an ordinary literal-sized scene — every
        // existing scene today — skips it. Proven behaviourally: a plain
        // node with no ui_size draws at its authored size, unaffected by
        // the pre-pass existing at all.
        let mut world = SceneWorld2D::new();
        let plain = world.spawn(SceneNode2D::new("plain"));
        {
            let n = world.get_mut(plain).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "50");
            n.set_property("ui_h", "30");
        }
        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        let rect = world.resolved_rect(plain).expect("drew");
        assert_eq!(rect.width, 50.0);
        assert_eq!(rect.height, 30.0);
    }

    #[test]
    fn set_button_text_and_color_override_the_authored_values() {
        // The design point: a button's authored text/colour are starting
        // values, and game code overrides them by name rather than the
        // scene needing several mutually-exclusive label nodes toggled by a
        // `ui_visible` binding for each state.
        let mut world = SceneWorld2D::new();
        let handle = world.spawn(SceneNode2D::new("continue_button"));
        world.get_mut(handle).unwrap().set_property("ui", "button");
        world
            .get_mut(handle)
            .unwrap()
            .set_property("ui_text", "CONTINUE");
        world
            .get_mut(handle)
            .unwrap()
            .set_property("ui_color", "215,220,235,255");

        // `find_by_name` only indexes nodes instantiated from a scene
        // document; a hand-spawned test node is looked up by its own handle
        // instead — real game code reaches the button the same way `find_by_name`
        // does after a scene loads.
        world.set_button_text(handle, "PICK TWO DRIVERS");
        world.set_button_color(handle, "120,128,140,255");

        let node = world.get(handle).unwrap();
        assert_eq!(node.property("ui_text"), Some("PICK TWO DRIVERS"));
        assert_eq!(node.property("ui_color"), Some("120,128,140,255"));
    }

    #[test]
    fn sync_repeaters_materializes_one_instance_per_item() {
        // E3: ui: "repeat" + ui_repeat_source turns the node's one authored
        // template child into N live clones, one per item in the named
        // RepeaterSources collection — "author one row" becomes real,
        // independently hit-testable nodes, not N redraws of the same handle.
        let mut world = SceneWorld2D::new();
        let list = world.spawn(SceneNode2D::new("list"));
        world.get_mut(list).unwrap().set_property("ui", "repeat");
        world
            .get_mut(list)
            .unwrap()
            .set_property("ui_repeat_source", "standings");
        let template = world.spawn_child(list, SceneNode2D::new("row"));
        world.get_mut(template).unwrap().set_property("ui", "text");
        world
            .get_mut(template)
            .unwrap()
            .set_property("ui_text", "P{pos} {name}");

        let mut sources = RepeaterSources::new();
        sources.insert(
            "standings".to_string(),
            repeat_source(&[("1", "Reyes"), ("2", "Voss"), ("3", "Amaro")]),
        );
        world.sync_repeaters(&sources);

        let instances = world.get(list).unwrap().children().to_vec();
        assert_eq!(instances.len(), 3, "one instance per source item");
        // Each instance is a real, distinct handle — not the template reused.
        assert!(!instances.contains(&template));
        for pair in instances.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }

        // Growing the source spawns more instances; shrinking despawns the
        // extras — reconciliation, not a fixed count baked in at first sync.
        sources.insert("standings".to_string(), repeat_source(&[("1", "Reyes")]));
        world.sync_repeaters(&sources);
        assert_eq!(world.get(list).unwrap().children().len(), 1);

        sources.insert(
            "standings".to_string(),
            repeat_source(&[("1", "Reyes"), ("2", "Voss"), ("3", "Amaro"), ("4", "Cole")]),
        );
        world.sync_repeaters(&sources);
        assert_eq!(world.get(list).unwrap().children().len(), 4);
    }

    #[test]
    fn an_instance_gets_exactly_its_templates_children() {
        // `capture_template` clones the template *node*, whose `children` are
        // handles into a world where that template is about to be despawned.
        // Carrying the list forward gave every instance one dangling child per
        // template child, in front of its real ones. Invisible until the
        // instance laid its children out in a flow — then each phantom took a
        // zero-width track and a gap, shifting every real child right by a gap
        // apiece (the sponsor screen's caret column, 14px per phantom).
        let mut world = SceneWorld2D::new();
        let list = world.spawn(SceneNode2D::new("list"));
        world.get_mut(list).unwrap().set_property("ui", "repeat");
        world
            .get_mut(list)
            .unwrap()
            .set_property("ui_repeat_source", "rows");
        let template = world.spawn_child(list, SceneNode2D::new("row"));
        for name in ["caret", "text", "figures"] {
            world.spawn_child(template, SceneNode2D::new(name));
        }

        let mut sources = RepeaterSources::new();
        sources.insert(
            "rows".to_string(),
            repeat_source(&[("1", "a"), ("2", "b"), ("3", "c")]),
        );
        world.sync_repeaters(&sources);

        for instance in world.get(list).unwrap().children() {
            let kids = world.get(*instance).unwrap().children().to_vec();
            assert_eq!(kids.len(), 3, "one child per template child, no phantoms");
            // A phantom is a handle into a despawned slot, so it resolves to
            // nothing — and would have laid out as a zero-width track.
            let prefabs: Vec<_> = kids
                .iter()
                .map(|&k| world.get(k).map(|n| n.prefab().to_string()))
                .collect();
            assert_eq!(
                prefabs,
                vec![
                    Some("caret".to_string()),
                    Some("text".to_string()),
                    Some("figures".to_string())
                ],
                "the template's children, in order, all live"
            );
        }
    }

    /// A repeat node whose rows are authored on the node itself.
    fn authored_repeat_world(items: &str, source: Option<&str>) -> (SceneWorld2D, NodeHandle2D) {
        let mut world = SceneWorld2D::new();
        let list = world.spawn(SceneNode2D::new("list"));
        world.get_mut(list).unwrap().set_property("ui", "repeat");
        if let Some(source) = source {
            world
                .get_mut(list)
                .unwrap()
                .set_property("ui_repeat_source", source);
        }
        world
            .get_mut(list)
            .unwrap()
            .set_property("ui_repeat_items", items);
        let template = world.spawn_child(list, SceneNode2D::new("row"));
        world.get_mut(template).unwrap().set_property("ui", "text");
        world
            .get_mut(template)
            .unwrap()
            .set_property("ui_text", "P{pos} {name}");
        (world, list)
    }

    #[test]
    fn authored_repeat_items_materialize_without_any_supplied_source() {
        // Rows authored in the scene stand on their own: a fixed list needs
        // no Rust supplier, and no ui_repeat_source at all.
        let (mut world, list) = authored_repeat_world(
            r#"[{"pos":"1","name":"REYES"},{"pos":"2","name":"VOSS"}]"#,
            None,
        );

        world.sync_repeaters(&RepeaterSources::new());

        let instances = world.get(list).unwrap().children().to_vec();
        assert_eq!(instances.len(), 2, "one instance per authored row");
        let scope = world.get(instances[0]).unwrap().instance_bindings.clone();
        assert_eq!(
            scope.unwrap().get("name").map(String::as_str),
            Some("REYES"),
            "each instance carries its authored row's scope"
        );
    }

    #[test]
    fn a_supplied_source_wins_over_authored_items() {
        // Authored rows are the fallback for "nobody supplied this", not a
        // default to merge with — otherwise a live standings list could show
        // stale authored rows alongside real ones.
        let (mut world, list) = authored_repeat_world(
            r#"[{"pos":"1","name":"AUTHORED"},{"pos":"2","name":"AUTHORED"}]"#,
            Some("standings"),
        );

        let mut sources = RepeaterSources::new();
        sources.insert("standings".to_string(), repeat_source(&[("9", "LIVE")]));
        world.sync_repeaters(&sources);

        let instances = world.get(list).unwrap().children().to_vec();
        assert_eq!(instances.len(), 1, "the live source decides the count");
        let scope = world.get(instances[0]).unwrap().instance_bindings.clone();
        assert_eq!(scope.unwrap().get("name").map(String::as_str), Some("LIVE"));
    }

    #[test]
    fn a_supplied_empty_source_is_an_answer_not_a_fallback() {
        // The risk the authored-fallback design carries: "the supplier said
        // zero rows" must not read as "the supplier said nothing", or an
        // empty live list would render stale authored rows.
        let (mut world, list) =
            authored_repeat_world(r#"[{"pos":"1","name":"AUTHORED"}]"#, Some("standings"));

        let mut sources = RepeaterSources::new();
        sources.insert("standings".to_string(), Vec::new());
        world.sync_repeaters(&sources);

        assert_eq!(
            world.get(list).unwrap().children().len(),
            0,
            "an empty supplied source yields no rows, not the authored ones"
        );
    }

    #[test]
    fn unparseable_repeat_items_are_ignored_rather_than_erroring() {
        // A scene may author `{some_binding}` here, or simply have a typo.
        // Either way the node degrades to "no authored rows" — consistent
        // with how every other ui_* property handles an unresolved value.
        for raw in [r#"{binding}"#, "not json", r#"{"not":"an array"}"#] {
            let (mut world, list) = authored_repeat_world(raw, None);
            world.sync_repeaters(&RepeaterSources::new());
            assert_eq!(
                world.get(list).unwrap().children().len(),
                0,
                "unparseable ui_repeat_items {raw:?} should yield no rows"
            );
        }
    }

    #[test]
    fn repeat_items_coerce_numbers_and_bools_to_strings() {
        // Bindings are HashMap<String,String>; authoring `1` instead of "1"
        // in the editor's JSON shouldn't silently drop the field.
        let (mut world, list) =
            authored_repeat_world(r#"[{"pos":1,"lapped":true,"name":"REYES"}]"#, None);
        world.sync_repeaters(&RepeaterSources::new());

        let instances = world.get(list).unwrap().children().to_vec();
        let scope = world
            .get(instances[0])
            .unwrap()
            .instance_bindings
            .clone()
            .unwrap();
        assert_eq!(scope.get("pos").map(String::as_str), Some("1"));
        assert_eq!(scope.get("lapped").map(String::as_str), Some("true"));
        assert_eq!(scope.get("name").map(String::as_str), Some("REYES"));
    }

    #[test]
    fn each_repeater_instance_draws_its_own_item_scope() {
        // The whole point: instance 0 and instance 1 are clones of the same
        // template but resolve ui_text differently because sync_repeaters
        // gave each its own Bindings scope.
        let mut world = SceneWorld2D::new();
        let list = world.spawn(SceneNode2D::new("list"));
        world.get_mut(list).unwrap().set_property("ui", "repeat");
        world
            .get_mut(list)
            .unwrap()
            .set_property("ui_repeat_source", "standings");
        // The repeat node's own rect is the reference every instance's
        // ui_w_frac resolves against — same as any other container.
        world
            .get_mut(list)
            .unwrap()
            .set_property("ui_stretch_x", "true");
        world
            .get_mut(list)
            .unwrap()
            .set_property("ui_stretch_y", "true");
        let template = world.spawn_child(list, SceneNode2D::new("row"));
        world.get_mut(template).unwrap().set_property("ui", "rect");
        // ui_w_frac bound to a per-item field: each row's bar width tracks
        // that row's own value, not a shared literal.
        world
            .get_mut(template)
            .unwrap()
            .set_property("ui_w_frac", "{share}");
        world.get_mut(template).unwrap().set_property("ui_h", "10");

        let mut sources = RepeaterSources::new();
        let mut row0 = Bindings::new();
        row0.insert("share".to_string(), "0.25".to_string());
        let mut row1 = Bindings::new();
        row1.insert("share".to_string(), "0.75".to_string());
        sources.insert("standings".to_string(), vec![row0, row1]);
        world.sync_repeaters(&sources);

        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let instances = world.get(list).unwrap().children().to_vec();
        let widths: Vec<f32> = instances
            .iter()
            .map(|&h| world.resolved_rect(h).unwrap().width)
            .collect();
        // Viewport width 200 -> 0.25 => 50, 0.75 => 150.
        assert!(widths.contains(&50.0), "widths: {:?}", widths);
        assert!(widths.contains(&150.0), "widths: {:?}", widths);
    }

    #[test]
    fn repeater_instances_flow_with_ui_layout_like_ordinary_children() {
        // E3 composes with E4 for free: resolve_child_references already
        // lays out whatever `children()` currently is, and sync_repeaters'
        // only job is to make that be N instances instead of the template.
        let mut world = SceneWorld2D::new();
        let list = world.spawn(SceneNode2D::new("list"));
        {
            let node = world.get_mut(list).unwrap();
            node.set_property("ui", "repeat");
            node.set_property("ui_repeat_source", "standings");
            node.set_property("ui_w", "100");
            node.set_property("ui_h", "100");
            node.set_property("ui_layout", "column");
        }
        let template = world.spawn_child(list, SceneNode2D::new("row"));
        {
            let node = world.get_mut(template).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_h", "20");
            node.set_property("ui_stretch_x", "true");
            node.set_property("ui_stretch_y", "true");
        }

        let mut sources = RepeaterSources::new();
        sources.insert(
            "standings".to_string(),
            repeat_source(&[("1", "Reyes"), ("2", "Voss")]),
        );
        world.sync_repeaters(&sources);

        let mut canvas = Canvas::new((200, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let instances = world.get(list).unwrap().children().to_vec();
        let a = world.resolved_rect(instances[0]).unwrap();
        let b = world.resolved_rect(instances[1]).unwrap();
        // Same shape as ui_layout_column_stacks_children_top_to_bottom_with_gap:
        // stacked top-to-bottom, no overlap.
        assert!(a.y > b.y, "first instance sits above the second");
        assert!((a.y - b.height - b.y).abs() < 1e-3, "packed with no gap");
    }

    #[test]
    fn focusable_order_sorts_by_ui_focus_order_then_document_order() {
        let mut world = SceneWorld2D::new();
        let a = world.spawn(SceneNode2D::new("a"));
        let b = world.spawn(SceneNode2D::new("b"));
        let c = world.spawn(SceneNode2D::new("c"));
        for h in [a, b, c] {
            world
                .get_mut(h)
                .unwrap()
                .set_property("ui_focusable", "true");
        }
        // Explicit order overrides spawn order; b (order 0) comes before a
        // (order 1); c has no explicit order (defaults to 0) so ties with b
        // and falls back to document order — b was spawned first.
        world
            .get_mut(a)
            .unwrap()
            .set_property("ui_focus_order", "1");
        world
            .get_mut(b)
            .unwrap()
            .set_property("ui_focus_order", "0");

        assert_eq!(world.focusable_order(), vec![b, c, a]);
    }

    #[test]
    fn focusable_order_excludes_non_focusable_and_invisible_nodes() {
        let mut world = SceneWorld2D::new();
        let visible = world.spawn(SceneNode2D::new("visible"));
        world
            .get_mut(visible)
            .unwrap()
            .set_property("ui_focusable", "true");
        let not_focusable = world.spawn(SceneNode2D::new("not_focusable"));
        let _ = not_focusable;
        let hidden = world.spawn(SceneNode2D::new("hidden"));
        world
            .get_mut(hidden)
            .unwrap()
            .set_property("ui_focusable", "true");
        world.get_mut(hidden).unwrap().visible = false;

        assert_eq!(world.focusable_order(), vec![visible]);
    }

    #[test]
    fn focus_move_wraps_and_seeds_from_unset() {
        let mut world = SceneWorld2D::new();
        let a = world.spawn(SceneNode2D::new("a"));
        let b = world.spawn(SceneNode2D::new("b"));
        for h in [a, b] {
            world
                .get_mut(h)
                .unwrap()
                .set_property("ui_focusable", "true");
        }
        assert_eq!(world.focused(), None);

        // Nothing focused yet: stepping forward lands on the first item.
        assert_eq!(world.focus_move(1), Some(a));
        assert_eq!(world.focused(), Some(a));
        assert_eq!(world.focus_move(1), Some(b));
        // Wraps forward past the end.
        assert_eq!(world.focus_move(1), Some(a));
        // Wraps backward past the start.
        assert_eq!(world.focus_move(-1), Some(b));
    }

    #[test]
    fn focus_move_is_a_noop_with_nothing_focusable() {
        let mut world = SceneWorld2D::new();
        world.spawn(SceneNode2D::new("decorative"));
        assert_eq!(world.focus_move(1), None);
        assert_eq!(world.focused(), None);
    }

    #[test]
    fn ui_color_hover_press_focus_override_the_base_color_at_draw() {
        // E6: pressed beats hovered beats focused beats the base ui_color —
        // a click mid-hover should read as "pressed," not a blend.
        let mut world = SceneWorld2D::new();
        let node = world.spawn(SceneNode2D::new("button"));
        {
            let n = world.get_mut(node).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "10");
            n.set_property("ui_h", "10");
            n.set_property("ui_color", "10,10,10,255");
            n.set_property("ui_color_hover", "20,20,20,255");
            n.set_property("ui_color_press", "30,30,30,255");
            n.set_property("ui_color_focus", "40,40,40,255");
        }

        // Colors are stored linear internally (sRGB decoded on parse), so
        // compare draws against each other rather than hand-computing the
        // linear value of each authored sRGB triplet.
        let draw = |world: &SceneWorld2D| -> [f32; 4] {
            let mut canvas = Canvas::new((200, 200), std::ptr::null());
            world.draw_to_canvas(&mut canvas, 0.0);
            canvas.verts.last().unwrap().color
        };

        let base = draw(&world);

        world.set_focus(Some(node));
        let focused = draw(&world);
        assert_ne!(
            base, focused,
            "ui_color_focus should override the base color"
        );

        world.set_hovered(Some(node));
        let hovered = draw(&world);
        assert_ne!(
            focused, hovered,
            "ui_color_hover should beat ui_color_focus"
        );

        world.set_pressed(Some(node));
        let pressed = draw(&world);
        assert_ne!(
            hovered, pressed,
            "ui_color_press should beat ui_color_hover"
        );
        assert_ne!(base, pressed);
        assert_ne!(focused, pressed);
    }

    #[test]
    fn interaction_state_is_inherited_by_descendants() {
        // E6: hovering a widget lights up the whole widget. Only one node can
        // be `hovered`, but the smallest useful button is a background rect
        // with a label node inside it — if the state stopped at the one node
        // under the cursor, `ui_color_hover` would be unusable on anything
        // real and every host would go back to pushing colours by hand.
        let mut world = SceneWorld2D::new();
        let row = world.spawn(SceneNode2D::new("row"));
        {
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "100");
            n.set_property("ui_h", "20");
            n.set_property("ui_color", "10,10,10,255");
            n.set_property("ui_color_hover", "20,20,20,255");
        }
        let label = world.spawn_child(row, SceneNode2D::new("label"));
        {
            let n = world.get_mut(label).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "40");
            n.set_property("ui_h", "10");
            n.set_property("ui_color", "50,50,50,255");
            n.set_property("ui_color_hover", "60,60,60,255");
        }

        // The label draws last, so the final vert carries its colour.
        let label_color = |world: &SceneWorld2D| -> [f32; 4] {
            let mut canvas = Canvas::new((200, 200), std::ptr::null());
            world.draw_to_canvas(&mut canvas, 0.0);
            canvas.verts.last().unwrap().color
        };

        let base = label_color(&world);
        world.set_hovered(Some(row));
        let inherited = label_color(&world);
        assert_ne!(
            base, inherited,
            "hovering the row should apply the label's own ui_color_hover too"
        );

        // Hovering the label directly must reach the same colour — a widget
        // has one hover appearance however the state got there.
        world.set_hovered(Some(label));
        assert_eq!(label_color(&world), inherited);
    }

    #[test]
    fn pixel_grid_snaps_a_repeater_dividing_rows_into_a_panel() {
        // E9: the exact scenario the plan flags as the reason this exists —
        // a repeater dividing an odd panel height into N rows produces
        // fractional row heights by construction (217 / 22 is not an
        // integer), which must not survive onto a fixed-pixel-art host.
        let mut world = SceneWorld2D::new();
        world.set_pixel_grid(2.0);
        let list = world.spawn(SceneNode2D::new("list"));
        {
            let node = world.get_mut(list).unwrap();
            node.set_property("ui", "repeat");
            node.set_property("ui_repeat_source", "standings");
            node.set_property("ui_w", "100");
            node.set_property("ui_h", "217");
            node.set_property("ui_layout", "column");
        }
        let template = world.spawn_child(list, SceneNode2D::new("row"));
        {
            let node = world.get_mut(template).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_stretch_x", "true");
            node.set_property("ui_stretch_y", "true");
        }
        let rows: Vec<(&str, &str)> = (0..22).map(|_| ("id", "x")).collect();
        let mut sources = RepeaterSources::new();
        sources.insert("standings".to_string(), repeat_source(&rows));
        world.sync_repeaters(&sources);

        let mut canvas = Canvas::new((200, 300), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        for instance in world.get(list).unwrap().children().to_vec() {
            let r = world.resolved_rect(instance).unwrap();
            for v in [r.x, r.y, r.width, r.height] {
                let cells = v / 2.0;
                assert!(
                    (cells - cells.round()).abs() < 1e-4,
                    "{v} is not a multiple of the 2.0 pixel grid"
                );
            }
        }
    }

    #[test]
    fn pixel_grid_is_off_by_default_and_opt_out_per_node_works() {
        let mut world = SceneWorld2D::new();
        let unsnapped = world.spawn(SceneNode2D::new("a"));
        {
            let node = world.get_mut(unsnapped).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w_frac", "1.0");
            node.set_property("ui_h_frac", "1.0");
        }
        let mut canvas = Canvas::new((201, 101), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        // No grid set: an odd viewport dimension passes through unsnapped.
        let r = world.resolved_rect(unsnapped).unwrap();
        assert!((r.width - 201.0).abs() < 1e-3);
        assert!((r.height - 101.0).abs() < 1e-3);

        world.set_pixel_grid(4.0);
        let opted_out = world.spawn(SceneNode2D::new("b"));
        {
            let node = world.get_mut(opted_out).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w_frac", "1.0");
            node.set_property("ui_h_frac", "1.0");
            node.set_property("ui_snap", "false");
        }
        world.draw_to_canvas(&mut canvas, 0.0);
        // Grid on, but this node opted out: still unsnapped.
        let r = world.resolved_rect(opted_out).unwrap();
        assert!((r.width - 201.0).abs() < 1e-3);
        assert!((r.height - 101.0).abs() < 1e-3);
        // The first node has no ui_snap property, so the grid now applies to
        // it too — snapping is on by default once the host sets a grid.
        let r = world.resolved_rect(unsnapped).unwrap();
        assert!((r.width / 4.0 - (r.width / 4.0).round()).abs() < 1e-4);
    }

    #[test]
    fn ui_layout_absent_leaves_every_child_with_the_parents_full_rect() {
        // No ui_layout: the pre-E4 behaviour (every child gets the same
        // reference and resolves its own anchor/position independently)
        // must be unchanged — this is the zero-cost-when-absent guarantee.
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("panel"));
        world.get_mut(container).unwrap().set_property("ui", "rect");
        world.get_mut(container).unwrap().set_property("ui_w", "80");
        world.get_mut(container).unwrap().set_property("ui_h", "60");
        let child = world.spawn_child(container, SceneNode2D::new("child"));
        world.get_mut(child).unwrap().set_property("ui", "rect");
        world
            .get_mut(child)
            .unwrap()
            .set_property("ui_stretch_x", "true");
        world.get_mut(child).unwrap().set_property("ui_h", "10");

        let mut canvas = Canvas::new((200, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let container_rect = world.resolved_rect(container).unwrap();
        let child_rect = world.resolved_rect(child).unwrap();
        // Stretch-x against the container's own rect still works exactly as
        // it did before E4 — the container's reference wasn't touched.
        assert!((child_rect.x - container_rect.x).abs() < 1e-3);
        assert!((child_rect.width - container_rect.width).abs() < 1e-3);
    }

    #[test]
    fn ui_clip_confines_a_childs_scissor_to_the_parent_rect() {
        // E5: ui_clip: true wraps push_clip/pop_clip around the child
        // recursion — a scrolling standings panel can overflow its own
        // bounds without painting over whatever sits below it. Clipping is a
        // GPU scissor recorded per draw segment (Canvas::segments), not
        // vertex culling, so the way to observe it is the segment's scissor
        // field, not the vertex count.
        let mut world = SceneWorld2D::new();
        let panel = world.spawn(SceneNode2D::new("panel"));
        {
            let node = world.get_mut(panel).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w", "40");
            node.set_property("ui_h", "40");
            node.set_property("ui_clip", "true");
        }
        let child = world.spawn_child(panel, SceneNode2D::new("child"));
        {
            let node = world.get_mut(child).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w", "10");
            node.set_property("ui_h", "10");
        }
        // A sibling root with no clipping ancestor draws with no scissor.
        let unclipped = world.spawn(SceneNode2D::new("unclipped"));
        {
            let node = world.get_mut(unclipped).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w", "10");
            node.set_property("ui_h", "10");
        }

        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        canvas.finalize();

        assert!(
            canvas.segments.iter().any(|s| s.scissor.is_some()),
            "the clipped child's segment should carry a scissor rect"
        );
        assert!(
            canvas.segments.iter().any(|s| s.scissor.is_none()),
            "the unrelated root's segment should draw unclipped"
        );
    }

    #[test]
    fn ui_scroll_y_shifts_a_columns_children_and_their_hit_rects() {
        // E-D: the timing tower is more rows than its panel is tall, so Rust
        // hands the column a `ui_scroll_y` and `ui_clip` (E5) trims what falls
        // outside. The value is how far *down the list* the view has scrolled,
        // so rows move up the screen — +y in this y-up space.
        //
        // Built twice, identically but for the scroll, because the absent-
        // property case is the load-bearing half: every scene authored before
        // this must lay out exactly as it did.
        let build = |scroll: Option<&str>| {
            let mut world = SceneWorld2D::new();
            let panel = world.spawn(SceneNode2D::new("panel"));
            {
                let node = world.get_mut(panel).unwrap();
                node.set_property("ui", "rect");
                node.set_property("ui_w", "100");
                node.set_property("ui_h", "100");
                node.set_property("ui_layout", "column");
                node.set_property("ui_clip", "true");
                if let Some(dy) = scroll {
                    node.set_property("ui_scroll_y", dy);
                }
            }
            let mut rows = Vec::new();
            for i in 0..4 {
                let row = world.spawn_child(panel, SceneNode2D::new(&format!("row{i}")));
                let node = world.get_mut(row).unwrap();
                node.set_property("ui", "rect");
                node.set_property("ui_h", "20");
                rows.push(row);
            }
            let mut canvas = Canvas::new((200, 200), std::ptr::null());
            world.draw_to_canvas(&mut canvas, 0.0);
            let ys: Vec<f32> = rows
                .iter()
                .map(|&r| world.resolved_rect(r).unwrap().y)
                .collect();
            let hits: Vec<f32> = rows
                .iter()
                .map(|&r| world.get(r).unwrap().ui_rect.get().unwrap().y)
                .collect();
            (ys, hits)
        };

        let (plain, plain_hits) = build(None);
        let (scrolled, scrolled_hits) = build(Some("30"));

        for (i, (before, after)) in plain.iter().zip(&scrolled).enumerate() {
            assert!(
                (after - before - 30.0).abs() < 1e-3,
                "row {i} should sit 30px higher: {before} -> {after}"
            );
        }
        // The hit rect is the same rect, or a click after a scroll lands on
        // the row that *used* to be there.
        for (i, (drawn, hit)) in scrolled.iter().zip(&scrolled_hits).enumerate() {
            assert!(
                (drawn - hit).abs() < 1e-3,
                "row {i} draws at {drawn} but hit-tests at {hit}"
            );
        }
        assert_eq!(
            plain, plain_hits,
            "an unscrolled column's hit rects match its draw rects"
        );
    }

    #[test]
    fn node_bounds_of_a_ui_node_matches_where_it_was_drawn() {
        // A `ui` node laid out with anchor/stretch (not raw w/h) used to be
        // undetectable to node_bounds/hit_test at all — the ui_* layout math in
        // draw_ui_node and the w/h-based math in node_bounds were two different
        // rects. E1 makes node_bounds return the same rect the node drew at.
        let mut world = SceneWorld2D::new();
        let panel = world.spawn(SceneNode2D::new("panel"));
        {
            let node = world.get_mut(panel).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_anchor", "bottom");
            node.set_property("ui_stretch_x", "true");
            node.set_property("ui_margin_left", "10");
            node.set_property("ui_margin_right", "10");
            node.set_property("ui_h", "50");
        }

        // Before any draw, there is nothing to hit yet.
        assert_eq!(world.node_bounds(panel), None);
        assert_eq!(world.resolved_rect(panel), None);

        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        // Viewport is 200x100 centered at origin: (-100,-50) to (100,50).
        // Stretch-x with 10px margins each side -> x=-90, w=180. Anchor bottom,
        // ui_h=50 -> y=-50, h=50.
        let drawn = world.resolved_rect(panel).expect("drawn ui rect");
        assert!((drawn.x + 90.0).abs() < 1e-3);
        assert!((drawn.y + 50.0).abs() < 1e-3);
        assert!((drawn.width - 180.0).abs() < 1e-3);
        assert!((drawn.height - 50.0).abs() < 1e-3);

        // node_bounds agrees exactly with resolved_rect, and hit_test uses it.
        assert_eq!(world.node_bounds(panel), Some(drawn));
        assert_eq!(world.hit_test(Vec2::new(0.0, -30.0)), Some(panel));
        assert_eq!(world.hit_test(Vec2::new(0.0, 30.0)), None);
    }

    #[test]
    fn ui_w_frac_reads_a_live_binding_not_just_a_literal() {
        // E2: any ui_* value may contain a {key} placeholder substituted from
        // a Bindings scope before it's parsed — this is what lets a progress
        // bar's width track live data (`ui_w_frac: "{fuel}"`) instead of a
        // fixed number baked into the scene file.
        let mut world = SceneWorld2D::new();
        let bar = world.spawn(SceneNode2D::new("bar"));
        world
            .get_mut(bar)
            .unwrap()
            .set_property("ui_w_frac", "{fuel}");
        world.get_mut(bar).unwrap().set_property("ui", "rect");
        world.get_mut(bar).unwrap().set_property("ui_h", "10");

        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        let mut bindings = Bindings::new();
        bindings.insert("fuel".to_string(), "0.25".to_string());
        world.draw_to_canvas_with_bindings(&mut canvas, 0.0, &bindings);
        let quarter = world.resolved_rect(bar).expect("drawn at 0.25 fuel").width;
        assert!((quarter - 50.0).abs() < 1e-3, "200 * 0.25 = 50: {quarter}");

        // Same node, same scene data, different binding — proves the value is
        // read live each draw rather than cached from the first substitution.
        bindings.insert("fuel".to_string(), "1.0".to_string());
        world.draw_to_canvas_with_bindings(&mut canvas, 0.0, &bindings);
        let full = world.resolved_rect(bar).expect("drawn at 1.0 fuel").width;
        assert!((full - 200.0).abs() < 1e-3, "200 * 1.0 = 200: {full}");

        // An empty scope leaves the unresolved placeholder in place, which
        // fails to parse as a float and falls back to 0 rather than panicking
        // or silently reusing a stale value.
        world.draw_to_canvas(&mut canvas, 0.0);
        let no_binding = world.resolved_rect(bar).expect("still drawn, w=0").width;
        assert_eq!(no_binding, 0.0);
    }

    #[test]
    fn ui_visible_binding_hides_the_node_and_clears_its_hit_bounds() {
        // E2's new prop: ui_visible, honoured in both draw (skips painting)
        // and bounds (E1's cache is cleared, so a hidden node stops being
        // hit-testable at a rect nothing was drawn behind).
        let mut world = SceneWorld2D::new();
        let toast = world.spawn(SceneNode2D::new("toast"));
        {
            let node = world.get_mut(toast).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_visible", "{is_dnf}");
            node.set_property("ui_w", "40");
            node.set_property("ui_h", "40");
        }

        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        let mut bindings = Bindings::new();
        bindings.insert("is_dnf".to_string(), "false".to_string());
        world.draw_to_canvas_with_bindings(&mut canvas, 0.0, &bindings);
        assert_eq!(
            world.resolved_rect(toast),
            None,
            "ui_visible: false hides it"
        );
        assert_eq!(world.node_bounds(toast), None);

        bindings.insert("is_dnf".to_string(), "true".to_string());
        world.draw_to_canvas_with_bindings(&mut canvas, 0.0, &bindings);
        assert!(
            world.resolved_rect(toast).is_some(),
            "ui_visible: true shows it"
        );
        assert!(world.node_bounds(toast).is_some());
    }

    #[test]
    fn a_hidden_node_hides_its_children_too() {
        // `ui_visible: false` is `display:none`, so it takes the whole subtree
        // with it. It used to hide only the node itself and then recurse into
        // the children regardless, which meant a switched-off screen variant
        // painted its labels on top of the live one — the driver market's
        // founding seat chips drew over the between-race board that way.
        let mut world = SceneWorld2D::new();
        let panel = world.spawn(SceneNode2D::new("panel").with_name("panel"));
        {
            let node = world.get_mut(panel).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_visible", "{shown}");
            node.set_property("ui_w", "80");
            node.set_property("ui_h", "40");
        }
        let label = world.spawn_child(panel, SceneNode2D::new("label").with_name("label"));
        {
            let node = world.get_mut(label).unwrap();
            node.set_property("ui", "rect");
            node.set_property("ui_w", "20");
            node.set_property("ui_h", "20");
        }

        let mut canvas = Canvas::new((200, 100), std::ptr::null());
        let mut bindings = Bindings::new();
        bindings.insert("shown".to_string(), "true".to_string());
        world.draw_to_canvas_with_bindings(&mut canvas, 0.0, &bindings);
        assert!(world.resolved_rect(label).is_some(), "shown: child draws");

        bindings.insert("shown".to_string(), "false".to_string());
        world.draw_to_canvas_with_bindings(&mut canvas, 0.0, &bindings);
        assert_eq!(
            world.resolved_rect(label),
            None,
            "hidden parent hides the child, and clears its stale hit rect"
        );
    }

    #[test]
    fn hit_test_picks_topmost_visible_node_in_bounds() {
        let mut world = SceneWorld2D::new();

        let a = world.spawn(SceneNode2D::new("a").with_position(Vec2::ZERO));
        {
            let node = world.get_mut(a).unwrap();
            node.set_property("w", "100");
            node.set_property("h", "100");
        }
        // `b` is spawned later, so it draws on top of `a` where they overlap.
        let b = world.spawn(SceneNode2D::new("b").with_position(Vec2::new(50.0, 50.0)));
        {
            let node = world.get_mut(b).unwrap();
            node.set_property("w", "100");
            node.set_property("h", "100");
        }

        // Overlap region resolves to the topmost node.
        assert_eq!(world.hit_test(Vec2::new(75.0, 75.0)), Some(b));
        // Region only covered by `a`.
        assert_eq!(world.hit_test(Vec2::new(10.0, 10.0)), Some(a));
        // Outside everything.
        assert_eq!(world.hit_test(Vec2::new(500.0, 500.0)), None);

        // Hiding the topmost node falls through to the one beneath it.
        world.get_mut(b).unwrap().set_visible(false);
        assert_eq!(world.hit_test(Vec2::new(75.0, 75.0)), Some(a));

        // A node with no sprites and no w/h has no pickable bounds.
        let empty = world.spawn(SceneNode2D::new("empty").with_position(Vec2::ZERO));
        assert_eq!(world.node_bounds(empty), None);
    }

    #[test]
    fn from_scene_reconstructs_hierarchy_and_lookups() {
        let definition = Scene2DDef {
            // An empty sprite list keeps this a pure data test: no texture
            // assets are required, so the world's hierarchy/lookup logic can be
            // exercised without standing up the GPU asset pipeline.
            prefabs: vec![Prefab2DDef {
                name: "marker".to_string(),
                sprites: vec![],
            }],
            instances: vec![
                SceneInstance2DDef {
                    prefab: "marker".to_string(),
                    position: [0.0, 0.0],
                    scale: [1.0, 1.0],
                    properties: props(&[
                        ("editor_node_id", "1"),
                        ("editor_name", "root_node"),
                        ("tags", "spawn, team_a"),
                    ]),
                },
                SceneInstance2DDef {
                    prefab: "marker".to_string(),
                    position: [5.0, 5.0],
                    scale: [1.0, 1.0],
                    properties: props(&[
                        ("editor_node_id", "2"),
                        ("editor_parent_id", "1"),
                        ("editor_name", "child_node"),
                        ("script_path", "scripts/child.rs"),
                    ]),
                },
            ],
            animations: Vec::new(),
        };

        let assets = AssetPack::default();
        let scene =
            Scene2D::from_definition(Path::new("test.scene.json"), definition, &assets).unwrap();
        let world = SceneWorld2D::from_scene(&scene);

        assert_eq!(world.len(), 2);
        let root = world.find_by_editor_id(1).unwrap();
        let child = world.find_by_editor_id(2).unwrap();

        assert_eq!(world.roots(), &[root]);
        assert_eq!(world.parent(child), Some(root));
        assert_eq!(world.children(root), vec![child]);

        assert_eq!(world.find_by_name("child_node"), Some(child));
        assert_eq!(
            world.get(child).unwrap().script_path(),
            Some("scripts/child.rs")
        );

        let tagged = world.by_tag("spawn");
        assert_eq!(tagged, vec![root]);
        assert!(world.get(root).unwrap().has_tag("team_a"));
    }

    fn parent_child_scene() -> Scene2D {
        let definition = Scene2DDef {
            prefabs: vec![Prefab2DDef {
                name: "marker".to_string(),
                sprites: vec![],
            }],
            instances: vec![
                SceneInstance2DDef {
                    prefab: "marker".to_string(),
                    position: [5.0, 0.0],
                    scale: [1.0, 1.0],
                    properties: props(&[("editor_node_id", "1"), ("editor_name", "root")]),
                },
                SceneInstance2DDef {
                    prefab: "marker".to_string(),
                    position: [2.0, 0.0],
                    scale: [1.0, 1.0],
                    properties: props(&[
                        ("editor_node_id", "2"),
                        ("editor_parent_id", "1"),
                        ("editor_name", "child"),
                    ]),
                },
            ],
            animations: Vec::new(),
        };
        Scene2D::from_definition(Path::new("t.scene.json"), definition, &AssetPack::default())
            .unwrap()
    }

    #[test]
    fn instantiate_scene_composes_multiple_offset_subtrees() {
        let scene = parent_child_scene();
        let mut world = SceneWorld2D::new();

        let roots_a = world.instantiate_scene(
            &scene,
            None,
            Transform2D::from_position(Vec2::new(100.0, 0.0)),
        );
        let roots_b = world.instantiate_scene(
            &scene,
            None,
            Transform2D::from_position(Vec2::new(-100.0, 0.0)),
        );

        // Two independent copies of a two-node scene.
        assert_eq!(roots_a.len(), 1);
        assert_eq!(roots_b.len(), 1);
        assert_eq!(world.len(), 4);
        assert_eq!(world.children(roots_a[0]).len(), 1);
        assert_eq!(world.children(roots_b[0]).len(), 1);

        // First-wins name lookup: only the first copy is reachable by name.
        assert_eq!(world.find_by_name("root"), Some(roots_a[0]));

        // The placement offset is composed onto each instance root
        // (root local x = 5.0).
        assert!((world.world_transform(roots_a[0]).unwrap().position.x - 105.0).abs() < 1e-3);
        assert!((world.world_transform(roots_b[0]).unwrap().position.x - (-95.0)).abs() < 1e-3);
    }

    #[test]
    fn instantiate_scene_under_parent_nests_the_subtree() {
        let scene = parent_child_scene();
        let mut world = SceneWorld2D::new();
        let holder = world.spawn(SceneNode2D::new("holder").with_position(Vec2::new(10.0, 10.0)));

        let roots = world.instantiate_scene(&scene, Some(holder), Transform2D::default());

        assert_eq!(roots.len(), 1);
        assert_eq!(world.parent(roots[0]), Some(holder));
        assert_eq!(world.children(holder), roots);
        // The holder is still the only world root.
        assert_eq!(world.roots(), &[holder]);
        // The nested child folds in both the holder and instance-root transforms
        // (holder x 10 + root local 5 + child local 2 = 17).
        let child = world.children(roots[0])[0];
        assert!((world.world_transform(child).unwrap().position.x - 17.0).abs() < 1e-3);
    }

    /// A scene from `(editor id, parent id, name, properties)` rows, so a test
    /// can build a real multi-node document instead of hand-spawning nodes —
    /// which is the only way to exercise anything that runs at *load* time,
    /// nested-scene expansion included.
    fn scene_from_instances(rows: &[(u64, Option<u64>, &str, &[(&str, &str)])]) -> Scene2D {
        let instances = rows
            .iter()
            .map(|(id, parent, name, extra)| {
                let mut properties = props(&[
                    ("editor_node_id", id.to_string().as_str()),
                    ("editor_name", name),
                ]);
                if let Some(parent) = parent {
                    properties.insert("editor_parent_id".to_string(), parent.to_string());
                }
                for (key, value) in *extra {
                    properties.insert(key.to_string(), value.to_string());
                }
                SceneInstance2DDef {
                    prefab: "marker".to_string(),
                    position: [0.0, 0.0],
                    scale: [1.0, 1.0],
                    properties,
                }
            })
            .collect();
        let definition = Scene2DDef {
            prefabs: vec![Prefab2DDef {
                name: "marker".to_string(),
                sprites: vec![],
            }],
            instances,
            animations: Vec::new(),
        };
        Scene2D::from_definition(Path::new("t.scene.json"), definition, &AssetPack::default())
            .unwrap()
    }

    fn single_node_scene(name: &str, extra: &[(&str, &str)]) -> Scene2D {
        let mut properties = props(&[("editor_node_id", "1"), ("editor_name", name)]);
        for (key, value) in extra {
            properties.insert(key.to_string(), value.to_string());
        }
        let definition = Scene2DDef {
            prefabs: vec![Prefab2DDef {
                name: "marker".to_string(),
                sprites: vec![],
            }],
            instances: vec![SceneInstance2DDef {
                prefab: "marker".to_string(),
                position: [0.0, 0.0],
                scale: [1.0, 1.0],
                properties,
            }],
            animations: Vec::new(),
        };
        Scene2D::from_definition(Path::new("t.scene.json"), definition, &AssetPack::default())
            .unwrap()
    }

    #[test]
    fn instantiate_scene_tree_expands_nested_references() {
        let host = single_node_scene("host", &[("nested_scene", "card")]);
        let mut library = SceneLibrary::new();
        library.insert("card", single_node_scene("card_root", &[]));

        let mut world = SceneWorld2D::new();
        let roots = world.instantiate_scene_tree(&host, &library, None, Transform2D::default());

        assert_eq!(roots.len(), 1);
        assert_eq!(world.len(), 2, "host node plus the expanded card node");
        let card_children = world.children(roots[0]);
        assert_eq!(card_children.len(), 1);
        assert_eq!(
            world.get(card_children[0]).unwrap().name(),
            Some("card_root")
        );
    }

    #[test]
    fn instantiate_scene_tree_breaks_reference_cycles() {
        // top -> a -> b -> a (the second a->b would loop; the cycle guard stops it)
        let mut library = SceneLibrary::new();
        library.insert("a", single_node_scene("a_root", &[("nested_scene", "b")]));
        library.insert("b", single_node_scene("b_root", &[("nested_scene", "a")]));
        let top = single_node_scene("top_root", &[("nested_scene", "a")]);

        let mut world = SceneWorld2D::new();
        world.instantiate_scene_tree(&top, &library, None, Transform2D::default());

        // top + a + b, with the cycle back to "a" rejected — no runaway.
        assert_eq!(world.len(), 3);
    }

    #[test]
    fn instantiate_scene_tree_skips_unknown_aliases() {
        let host = single_node_scene("host", &[("nested_scene", "missing")]);
        let library = SceneLibrary::new();

        let mut world = SceneWorld2D::new();
        let roots = world.instantiate_scene_tree(&host, &library, None, Transform2D::default());

        assert_eq!(roots.len(), 1);
        assert_eq!(world.len(), 1, "unknown alias leaves the host node alone");
    }

    #[test]
    fn flow_slot_sizes_read_bindings_including_a_repeat_instances_own_scope() {
        // A child's main-axis size goes through binding substitution the same
        // way the parent's `ui_gap`/`ui_layout` do. Before this, `ui_h:
        // "{row_h}"` measured as zero and every row in a column stacked on top
        // of the one before it.
        let mut world = SceneWorld2D::new();
        let list = world.spawn(SceneNode2D::new("list"));
        {
            let n = world.get_mut(list).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_gap", "0");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        for _ in 0..2 {
            let row = world.spawn_child(list, SceneNode2D::new("row"));
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_h", "{row_h}");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }

        let mut canvas = Canvas::new((400, 400), std::ptr::null());
        let bindings = Bindings::from([("row_h".to_string(), "20".to_string())]);
        world.draw_to_canvas_in_with_bindings(
            &mut canvas,
            (0.0, 0.0, 100.0, 100.0),
            0.0,
            &bindings,
        );

        let rows: Vec<Rect> = world
            .children(list)
            .into_iter()
            .filter_map(|h| world.resolved_rect(h))
            .collect();
        assert_eq!(rows.len(), 2);
        assert!((rows[0].height - 20.0).abs() < 1e-3, "got {:?}", rows[0]);
        // Second row sits exactly one slot below the first, not on top of it.
        assert!((rows[0].y - rows[1].y - 20.0).abs() < 1e-3, "{:?}", rows);
    }

    #[test]
    fn draw_to_canvas_in_resolves_against_the_caller_supplied_root() {
        // Ed3: the editor viewport is a sub-rect of the window, not the whole
        // canvas, so the root reference rect for stretch/anchor resolution
        // has to be a caller-supplied argument rather than always derived
        // from `canvas.screen_size()`. Draw into a root well away from the
        // screen's own (0,0)-centred rect and confirm the node resolved
        // against that rect, not the screen.
        let mut world = SceneWorld2D::new();
        let node = world.spawn(SceneNode2D::new("panel"));
        {
            let n = world.get_mut(node).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }

        let mut canvas = Canvas::new((800, 600), std::ptr::null());
        let root = (120.0, 40.0, 300.0, 150.0);
        world.draw_to_canvas_in(&mut canvas, root, 0.0);

        let r = world.resolved_rect(node).unwrap();
        assert!((r.x - root.0).abs() < 1e-3);
        assert!((r.y - root.1).abs() < 1e-3);
        assert!((r.width - root.2).abs() < 1e-3);
        assert!((r.height - root.3).abs() < 1e-3);
    }

    /// A row container holding children built from `children`, drawn against a
    /// `viewport`-sized canvas. Returns the world so callers can read back each
    /// child's resolved rect.
    ///
    /// The sidebar-and-centre shape every responsive rule in the UI overhaul is
    /// stated in terms of, reduced to the smallest thing that still shows it.
    fn flow_row(
        viewport: (u32, u32),
        container_props: &[(&str, &str)],
        children: &[(&str, &[(&str, &str)])],
    ) -> (SceneWorld2D, Vec<NodeHandle2D>) {
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(container).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "row");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
            for &(k, v) in container_props {
                n.set_property(k, v);
            }
        }
        let handles = children
            .iter()
            .map(|(name, props)| {
                let h = world.spawn_child(container, SceneNode2D::new(*name));
                let n = world.get_mut(h).unwrap();
                n.set_property("ui", "rect");
                // A slot is only a *reference* rect: a child still resolves
                // its own rect within it, so filling the slot on the main
                // axis is opt-in exactly as it is for any other node. These
                // rows want the slot they were given, which is what a real
                // scene authors too — except on the main axis, where ui_grow
                // now implies the fill on its own.
                n.set_property("ui_stretch_x", "true");
                n.set_property("ui_stretch_y", "true");
                for &(k, v) in props.iter() {
                    n.set_property(k, v);
                }
                h
            })
            .collect();
        let mut canvas = Canvas::new(viewport, std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        (world, handles)
    }

    #[test]
    fn ui_grow_shares_leftover_space_by_weight() {
        // E-A. The race screen's main row: two fixed sidebars and a centre
        // that takes whatever is left. Before this, a flow child could only be
        // its literal ui_w, so "the rest" had to be hand-computed against a
        // known viewport — which is exactly what breaks on resize.
        let (world, h) = flow_row(
            (1000, 200),
            &[],
            &[
                ("left", &[("ui_w", "250")]),
                ("centre", &[("ui_grow", "1")]),
                ("right", &[("ui_w", "322")]),
            ],
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("child drawn");
        assert!((r(0).width - 250.0).abs() < 1e-3, "left keeps its width");
        assert!((r(2).width - 322.0).abs() < 1e-3, "right keeps its width");
        // 1000 - 250 - 322 = 428 for the one growing child.
        assert!(
            (r(1).width - 428.0).abs() < 1e-3,
            "centre takes the rest: {}",
            r(1).width
        );
        // Placed in order, no overlap, no gaps.
        assert!((r(0).x - -500.0).abs() < 1e-3);
        assert!((r(1).x - r(0).right()).abs() < 1e-3);
        assert!((r(2).x - r(1).right()).abs() < 1e-3);

        // Weights are proportional, not just "share equally".
        let (world, h) = flow_row(
            (900, 200),
            &[],
            &[("a", &[("ui_grow", "2")]), ("b", &[("ui_grow", "1")])],
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("child drawn");
        assert!(
            (r(0).width - 600.0).abs() < 1e-3,
            "weight 2: {}",
            r(0).width
        );
        assert!(
            (r(1).width - 300.0).abs() < 1e-3,
            "weight 1: {}",
            r(1).width
        );
    }

    #[test]
    fn a_growing_centre_absorbs_the_viewport_change() {
        // The responsive contract, stated as a test: the *same* authored
        // document at two sizes keeps its sidebars fixed and gives every new
        // pixel to the centre. This is the property the plan's per-screen
        // 1920x1200 capture checks by eye.
        let build = || {
            [
                ("left", &[("ui_w", "250")][..]),
                ("centre", &[("ui_grow", "1")][..]),
            ]
        };
        let (w1, h1) = flow_row((1280, 800), &[], &build());
        let (w2, h2) = flow_row((1920, 800), &[], &build());
        let left1 = w1.resolved_rect(h1[0]).unwrap();
        let left2 = w2.resolved_rect(h2[0]).unwrap();
        let mid1 = w1.resolved_rect(h1[1]).unwrap();
        let mid2 = w2.resolved_rect(h2[1]).unwrap();
        assert!(
            (left1.width - left2.width).abs() < 1e-3,
            "sidebar width is viewport-independent"
        );
        assert!(
            (mid2.width - mid1.width - 640.0).abs() < 1e-3,
            "centre absorbs all 640 new pixels: {} -> {}",
            mid1.width,
            mid2.width
        );
        // And it grew on the cross axis for free: a row child spans the full
        // height by default, which is what makes a sidebar fill the viewport.
        assert!((left1.height - 800.0).abs() < 1e-3, "sidebar spans height");
    }

    #[test]
    fn content_sized_child_measures_itself_inside_a_flow() {
        // E-G, the gap that blocked "a sidebar is only as wide as its widest
        // row". A flow child used to take its main-axis extent from a literal
        // ui_w only, so ui_size: "content" measured as zero and collapsed —
        // forcing the hand-tuned constant this overhaul exists to delete.
        let mut world = SceneWorld2D::new();
        let root = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(root).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "row");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        // A sidebar that sizes to its own content: a column whose widest child
        // is 120 wide, plus 8px of padding each side.
        let sidebar = world.spawn_child(root, SceneNode2D::new("sidebar"));
        {
            let n = world.get_mut(sidebar).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_size", "content");
            n.set_property("ui_layout", "column");
            n.set_property("ui_pad_left", "8");
            n.set_property("ui_pad_right", "8");
        }
        for (name, w, h) in [("narrow", "60", "20"), ("widest", "120", "20")] {
            let row = world.spawn_child(sidebar, SceneNode2D::new(name));
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", w);
            n.set_property("ui_h", h);
        }
        let centre = world.spawn_child(root, SceneNode2D::new("centre"));
        {
            let n = world.get_mut(centre).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_grow", "1");
        }

        let mut canvas = Canvas::new((1000, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let bar = world.resolved_rect(sidebar).expect("sidebar drawn");
        assert!(
            (bar.width - 136.0).abs() < 1e-3,
            "sidebar is its widest row + padding, not zero: {}",
            bar.width
        );
        let mid = world.resolved_rect(centre).expect("centre drawn");
        assert!(
            (mid.width - 864.0).abs() < 1e-3,
            "centre takes exactly what the measured sidebar left: {}",
            mid.width
        );
    }

    #[test]
    fn ui_justify_spreads_slack_when_nothing_grows() {
        // E-A's other half: the mockups' 38 space-between bars (a top bar with
        // groups pushed to both ends). Only meaningful when no child grows,
        // since a growing child has already eaten the slack.
        let (world, h) = flow_row(
            (1000, 100),
            &[("ui_justify", "space_between")],
            &[
                ("a", &[("ui_w", "100")]),
                ("b", &[("ui_w", "100")]),
                ("c", &[("ui_w", "100")]),
            ],
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("child drawn");
        assert!((r(0).x - -500.0).abs() < 1e-3, "first is flush left");
        assert!(
            (r(2).right() - 500.0).abs() < 1e-3,
            "last is flush right: {}",
            r(2).right()
        );
        assert!(
            (r(1).x - -50.0).abs() < 1e-3,
            "middle is centred: {}",
            r(1).x
        );
        // Widths are untouched by justification.
        for i in 0..3 {
            assert!((r(i).width - 100.0).abs() < 1e-3);
        }

        // `center` packs the block and centres it, gap included.
        let (world, h) = flow_row(
            (1000, 100),
            &[("ui_justify", "center"), ("ui_gap", "10")],
            &[("a", &[("ui_w", "100")]), ("b", &[("ui_w", "100")])],
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("child drawn");
        // Content is 100 + 10 + 100 = 210, centred in 1000 -> starts at -105.
        assert!((r(0).x - -105.0).abs() < 1e-3, "centred block: {}", r(0).x);
        assert!(
            (r(1).x - (r(0).right() + 10.0)).abs() < 1e-3,
            "gap preserved between centred items"
        );
        assert!(
            (r(1).width - 100.0).abs() < 1e-3,
            "gap is not folded into w"
        );
    }

    #[test]
    fn a_nested_scene_fills_the_slot_its_host_was_given() {
        // A shared component (the game's chrome bar) authored as its own scene
        // and grafted under a host node in a flow: the host takes a 32px track
        // in the column, and the nested root must resolve against *that*, not
        // against the screen.
        let mut library = SceneLibrary::new();
        library.insert(
            "chrome",
            single_node_scene(
                "bar",
                &[
                    ("ui", "rect"),
                    ("ui_stretch_x", "true"),
                    ("ui_stretch_y", "true"),
                ],
            ),
        );

        // The screen, as a real scene document: a column whose first child
        // names the shared scene and whose second grows into the rest. This
        // is the shape `tools/scene_kit.py:screen_root` emits.
        let screen = scene_from_instances(&[
            (
                1,
                None,
                "root",
                &[
                    ("ui", "rect"),
                    ("ui_layout", "column"),
                    ("ui_stretch_x", "true"),
                    ("ui_stretch_y", "true"),
                ][..],
            ),
            (
                2,
                Some(1),
                "host",
                &[
                    (NESTED_SCENE_PROPERTY, "chrome"),
                    ("ui_h", "32"),
                    ("ui_stretch_x", "true"),
                ][..],
            ),
            (3, Some(1), "body", &[("ui_grow", "1")][..]),
        ]);

        let mut world = SceneWorld2D::new();
        world.instantiate_scene_tree(&screen, &library, None, Transform2D::default());
        let host = world.find_by_name("host").expect("host spawned");
        let expanded: Vec<NodeHandle2D> = world.children(host);
        assert_eq!(expanded.len(), 1, "the bar was grafted under its host");

        let mut canvas = Canvas::new((1280, 800), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let host_rect = world.resolved_rect(host);
        let bar = world.resolved_rect(expanded[0]).expect("bar drawn");
        assert!(
            (bar.height - 32.0).abs() < 1e-3,
            "the bar is its authored 32 tall, not half of it: {}",
            bar.height
        );
        assert!(
            (bar.width - 1280.0).abs() < 1e-3,
            "and spans the viewport: {}",
            bar.width
        );
        // Top of the screen, which is where the column put its first child.
        assert!(
            (bar.top() - 400.0).abs() < 1e-3,
            "flush with the top: {}",
            bar.top()
        );
        if let Some(h) = host_rect {
            assert!(
                (bar.top() - h.top()).abs() < 1e-3,
                "bar sits where its host does: bar {} vs host {}",
                bar.top(),
                h.top()
            );
        }
    }

    #[test]
    fn ui_size_content_h_keeps_the_authored_width() {
        // The common mockup panel: "640px wide, as tall as my rows need".
        // `ui_size: "content"` is per node, so before the axis variants the
        // only way to express it was to compute the height in the host and
        // pass it back as a binding.
        let mut world = SceneWorld2D::new();
        let root = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(root).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_align", "center");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        let panel = world.spawn_child(root, SceneNode2D::new("panel"));
        {
            let n = world.get_mut(panel).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_w", "640");
            n.set_property("ui_size", "content_h");
            n.set_property("ui_pad_top", "8");
            n.set_property("ui_pad_bottom", "8");
        }
        // Two 30px rows: the panel should end up 30+30+8+8 = 76 tall.
        for name in ["row_a", "row_b"] {
            let row = world.spawn_child(panel, SceneNode2D::new(name));
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_h", "30");
        }

        let mut canvas = Canvas::new((1000, 600), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let rect = world.resolved_rect(panel).expect("panel drawn");
        assert!(
            (rect.width - 640.0).abs() < 1e-3,
            "width stays authored, not measured: {}",
            rect.width
        );
        assert!(
            (rect.height - 76.0).abs() < 1e-3,
            "height follows its rows: {}",
            rect.height
        );
        // And it is centred on the width it kept, which is what a measured
        // width would have got wrong.
        assert!((rect.x - -320.0).abs() < 1e-3, "centred: {}", rect.x);
    }

    #[test]
    fn a_content_sized_parent_reads_a_childs_measured_size_per_axis() {
        // The archetype cards: a content-sized row of four cards that are
        // `ui_size: "content_h"` with an authored `ui_w`. A `content_h` child
        // still *measures* a width from its own children — a value its own
        // rect never uses — so a parent that read the measured pair wholesale
        // sized the row to the cards' contents rather than to the width they
        // state, and every card after the first landed short.
        let mut world = SceneWorld2D::new();
        let row = world.spawn(SceneNode2D::new("row"));
        {
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "row");
            n.set_property("ui_gap", "12");
            n.set_property("ui_size", "content");
        }
        for i in 0..4 {
            let card = world.spawn_child(row, SceneNode2D::new(&format!("card_{i}")));
            {
                let n = world.get_mut(card).unwrap();
                n.set_property("ui", "rect");
                n.set_property("ui_w", "246");
                n.set_property("ui_layout", "column");
                n.set_property("ui_size", "content_h");
            }
            // Narrower than the card states, which is what used to win.
            let body = world.spawn_child(card, SceneNode2D::new(&format!("body_{i}")));
            let n = world.get_mut(body).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "222");
            n.set_property("ui_h", "86");
        }

        let mut canvas = Canvas::new((1280, 768), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let rect = world.resolved_rect(row).expect("row drawn");
        assert!(
            (rect.width - 1020.0).abs() < 1e-3,
            "four 246px cards and three 12px gaps: {} (222px-wide contents won?)",
            rect.width
        );
        assert!(
            (rect.height - 86.0).abs() < 1e-3,
            "as tall as a card: {}",
            rect.height
        );
    }

    #[test]
    fn a_text_leafs_own_padding_is_part_of_the_box_it_measures() {
        // The archetype card's header: `padding:9px 10px` around one line of
        // text. `ui_pad_*` insets a *flow container's* children, so a leaf that
        // paints its own text used to get no inset at all — it measured to the
        // ink and drew its label flush against its own border.
        let mut world = SceneWorld2D::new();
        let head = world.spawn(SceneNode2D::new("head"));
        {
            let n = world.get_mut(head).unwrap();
            n.set_property("ui", "button");
            n.set_property("ui_text", "Aggressive");
            n.set_property("ui_text_size", "12");
            n.set_property("ui_text_align", "left");
            n.set_property("ui_size", "content");
            n.set_property("ui_pad_left", "10");
            n.set_property("ui_pad_right", "10");
            n.set_property("ui_pad_top", "9");
            n.set_property("ui_pad_bottom", "9");
        }

        let mut canvas = Canvas::new((1000, 600), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let rect = world.resolved_rect(head).expect("header drawn");
        let (ink_w, _) =
            canvas.measure_text_tracked(crate::FontId::DEFAULT, "Aggressive", 12.0, 0.0);
        let line_h = canvas.line_height_in(crate::FontId::DEFAULT, 12.0);
        assert!(
            (rect.width - (ink_w + 20.0)).abs() < 1e-3,
            "measures the run plus both side pads: {} vs {}",
            rect.width,
            ink_w + 20.0
        );
        assert!(
            (rect.height - (line_h + 18.0)).abs() < 1e-3,
            "measures the line box plus both vertical pads: {} vs {}",
            rect.height,
            line_h + 18.0
        );
    }

    #[test]
    fn a_measured_size_grows_by_its_own_border() {
        // E-X. `border_rects` draws a border *inside* the rect, which is right
        // for an authored size — a 640px panel stays 640px. A measured size has
        // no such statement to honour, so the border has to add to it, as CSS's
        // default `content-box` does. Without this the NAME screen's info boxes
        // came out 4px short and painted their 2px rule over their own padding,
        // leaving 8px of the 10px authored.
        let mut world = SceneWorld2D::new();
        let root = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(root).unwrap();
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        // background:#1b1f26;border:2px solid #3f4854;padding:10px 16px — one
        // 18px child, so every term in the expected size is stated.
        let boxed = world.spawn_child(root, SceneNode2D::new("box"));
        {
            let n = world.get_mut(boxed).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_size", "content");
            n.set_property("ui_layout", "column");
            n.set_property("ui_border_w", "2");
            n.set_property("ui_border_color", "63,72,84,255");
            n.set_property("ui_pad_left", "16");
            n.set_property("ui_pad_right", "16");
            n.set_property("ui_pad_top", "10");
            n.set_property("ui_pad_bottom", "10");
        }
        let swatch = world.spawn_child(boxed, SceneNode2D::new("swatch"));
        {
            let n = world.get_mut(swatch).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "18");
            n.set_property("ui_h", "18");
        }

        let mut canvas = Canvas::new((400, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let rect = world.resolved_rect(boxed).expect("box drawn");
        assert!(
            (rect.width - 54.0).abs() < 1e-3,
            "18 content + 32 padding + 4 border: {}",
            rect.width
        );
        assert!(
            (rect.height - 42.0).abs() < 1e-3,
            "18 content + 20 padding + 4 border: {}",
            rect.height
        );
        // The point of the extra 4: with the border outside the padding there
        // is room for both, so the child still clears the box's edge by the
        // full 10px authored rather than by 8.
        let sw = world.resolved_rect(swatch).expect("swatch drawn");
        assert!(
            (sw.y + sw.height - (rect.y + rect.height - 10.0)).abs() < 1e-3,
            "the child's top sits a full pad_top below the box's: {} vs {}",
            sw.y + sw.height,
            rect.y + rect.height - 10.0
        );
    }

    #[test]
    fn ui_min_h_floors_a_content_sized_node_but_never_caps_it() {
        // The archetype cards' `min-height:52px`: four blurbs of different
        // lengths in a row, each as tall as it needs but never shorter than
        // 52 — without which the shortest card sets its own height and the row
        // comes out ragged.
        let mut world = SceneWorld2D::new();
        let root = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(root).unwrap();
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        // A one-row block that measures well under the floor, and a stack of
        // rows that measures well over it. Same `ui_min_h` on both.
        let short = world.spawn_child(root, SceneNode2D::new("short"));
        {
            let n = world.get_mut(short).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_size", "content_h");
            n.set_property("ui_min_h", "52");
        }
        {
            let row = world.spawn_child(short, SceneNode2D::new("short_row"));
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_h", "20");
        }
        let tall = world.spawn_child(root, SceneNode2D::new("tall"));
        {
            let n = world.get_mut(tall).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_size", "content_h");
            n.set_property("ui_min_h", "52");
        }
        for name in ["tall_a", "tall_b", "tall_c"] {
            let row = world.spawn_child(tall, SceneNode2D::new(name));
            let n = world.get_mut(row).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_h", "30");
        }

        let mut canvas = Canvas::new((1000, 600), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let short_h = world.resolved_rect(short).expect("short drawn").height;
        let tall_h = world.resolved_rect(tall).expect("tall drawn").height;
        assert!(
            (short_h - 52.0).abs() < 1e-3,
            "20px of content is floored to the authored 52: {short_h}"
        );
        assert!(
            (tall_h - 90.0).abs() < 1e-3,
            "90px of content is a floor, not a cap: {tall_h}"
        );
    }

    #[test]
    fn text_spans_content_sizes_to_the_line_it_paints() {
        // The chrome bar's readouts: `BANK` in one colour, its value in
        // another, one line box. Content-sized at zero they all stacked at the
        // same x, so the row read as one illegible smear.
        let mut world = SceneWorld2D::new();
        let root = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(root).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "row");
            n.set_property("ui_gap", "12");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        let mut spans = Vec::new();
        for (name, label) in [("bank", "BANK "), ("legacy", "LEGACY ")] {
            let h = world.spawn_child(root, SceneNode2D::new(name));
            let n = world.get_mut(h).unwrap();
            n.set_property("ui", "text_spans");
            n.set_property("ui_text_size", "8");
            n.set_property("ui_span_0_text", label);
            n.set_property("ui_span_1_text", "$1.9M");
            n.set_property("ui_size", "content");
            spans.push(h);
        }

        let mut canvas = Canvas::new((800, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let a = world.resolved_rect(spans[0]).expect("first drawn");
        let b = world.resolved_rect(spans[1]).expect("second drawn");
        assert!(a.width > 0.0, "a span run has width: {}", a.width);
        assert!(
            (a.height - canvas.line_height_in(crate::FontId::DEFAULT, 8.0)).abs() < 1e-3,
            "one line box tall: {}",
            a.height
        );
        // The whole point: the second starts after the first plus the gap,
        // rather than on top of it.
        assert!(
            (b.x - (a.right() + 12.0)).abs() < 1e-3,
            "laid out in sequence: {} then {}",
            a.right(),
            b.x
        );
    }

    #[test]
    fn a_content_sized_flow_child_starts_at_its_cross_edge() {
        // The ninth outing of the centre-anchor rule, and the one that made it
        // an engine bug rather than an authoring trap: under the default
        // `ui_align: "stretch"` the flow decides nothing on the cross axis, so a
        // child sized by `ui_size` fell back to the centre anchor at origin 0 —
        // near edge on the slot's CENTRE, far half hanging outside it.
        //
        // The error is `slotCross/2 - childCross`, so it vanishes when the child
        // happens to be half the slot and is worst when the child is what sized
        // the slot in the first place. That is why 54 green baselines never saw
        // it: one-line rows were out by 8px, and only Formula R's 3-line race-log
        // entry was out far enough (23px) to be drawn through its neighbour.
        //
        // Row (cross = y, y-up) and column (cross = x) both, because getting the
        // row backwards mirrors every aligned row and reads as a data bug.
        let probe = |layout: &str, child: &[(&str, &str)]| {
            let mut world = SceneWorld2D::new();
            let root = world.spawn(SceneNode2D::new("root"));
            {
                let n = world.get_mut(root).unwrap();
                n.set_property("ui", "rect");
                n.set_property("ui_layout", layout);
                n.set_property("ui_stretch_x", "true");
                n.set_property("ui_stretch_y", "true");
            }
            // Fills the cross axis, so it marks where the slot really is.
            let ruler = world.spawn_child(root, SceneNode2D::new("ruler"));
            {
                let n = world.get_mut(ruler).unwrap();
                n.set_property("ui", "rect");
                n.set_property("ui_w", "3");
                n.set_property("ui_h", "3");
                n.set_property("ui_stretch_x", "true");
                n.set_property("ui_stretch_y", "true");
            }
            let sized = world.spawn_child(root, SceneNode2D::new("sized"));
            {
                let n = world.get_mut(sized).unwrap();
                n.set_property("ui", "rect");
                n.set_property("ui_w", "40");
                n.set_property("ui_h", "40");
                for (k, v) in child {
                    n.set_property(*k, *v);
                }
            }
            let mut canvas = Canvas::new((400, 400), std::ptr::null());
            world.draw_to_canvas(&mut canvas, 0.0);
            (
                world.resolved_rect(ruler).expect("ruler"),
                world.resolved_rect(sized).expect("sized"),
            )
        };

        // Row: cross is y, and its start is the TOP edge.
        let (ruler, sized) = probe("row", &[]);
        assert!(
            (sized.top() - ruler.top()).abs() < 1e-3,
            "row child's top sits on the slot's top: {} vs {}",
            sized.top(),
            ruler.top()
        );

        // Column: cross is x, and its start is the LEFT edge.
        let (ruler, sized) = probe("column", &[]);
        assert!(
            (sized.x - ruler.x).abs() < 1e-3,
            "column child's left sits on the slot's left: {} vs {}",
            sized.x,
            ruler.x
        );

        // A child that fills the cross axis is unaffected — it always landed on
        // the slot's edge, and is the workaround every bitten scene reached for.
        let (ruler, sized) = probe("row", &[("ui_stretch_y", "true")]);
        assert!(
            (sized.top() - ruler.top()).abs() < 1e-3 && (sized.height - ruler.height).abs() < 1e-3,
            "an explicitly stretched child is unchanged"
        );

        // ...and an author who *stated* an anchor still gets it. This is a
        // default for scenes that said nothing, not an override of ones that did.
        let (ruler, sized) = probe("row", &[("ui_anchor", "center")]);
        assert!(
            sized.top() < ruler.top() - 1.0,
            "an authored anchor still wins, so this one is NOT at the top: {} vs slot top {}",
            sized.top(),
            ruler.top()
        );
    }

    #[test]
    fn text_block_content_sizes_to_its_wrapped_height() {
        // E-Q. A wrapped prose block used to measure zero and collapse, so
        // every one carried a hand-counted `ui_h`. Now it measures the box its
        // own wrap produces — the mockups' card text, perk descriptions and
        // archetype blurbs are all this shape.
        let mut world = SceneWorld2D::new();
        let root = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(root).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        // Narrow enough that this certainly wraps past one line.
        let block = world.spawn_child(root, SceneNode2D::new("blurb"));
        {
            let n = world.get_mut(block).unwrap();
            n.set_property("ui", "text_block");
            n.set_property(
                "ui_text",
                "High pace, risky calls. Extra command point every lap.",
            );
            n.set_property("ui_text_size", "14");
            n.set_property("ui_wrap_w", "120");
            n.set_property("ui_size", "content");
        }

        let mut canvas = Canvas::new((400, 400), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);

        let rect = world.resolved_rect(block).expect("block drawn");
        let line_h = canvas.line_height_in(crate::FontId::DEFAULT, 14.0);
        assert!(
            (rect.width - 120.0).abs() < 1e-3,
            "block is its wrap width: {}",
            rect.width
        );
        // The point of the feature: taller than one line, and an exact whole
        // number of them rather than some arbitrary measured ink height.
        assert!(
            rect.height > line_h * 1.5,
            "wrapped block is multiple lines tall: {} vs one line {line_h}",
            rect.height
        );
        let lines = (rect.height / line_h).round();
        assert!(
            (rect.height - lines * line_h).abs() < 1e-3,
            "height is a whole number of line boxes: {} / {line_h}",
            rect.height
        );
    }

    #[test]
    fn ui_line_height_leads_a_block_without_moving_its_first_line() {
        // E-R: CSS `line-height:1.35`. Extra leading is space *between* lines,
        // so the first line does not move and the last contributes a full box
        // — a block that leaded the trailing line would carry a phantom gap.
        let measure = |leading: Option<&str>| {
            let mut world = SceneWorld2D::new();
            let root = world.spawn(SceneNode2D::new("root"));
            {
                let n = world.get_mut(root).unwrap();
                n.set_property("ui", "rect");
                n.set_property("ui_layout", "column");
                n.set_property("ui_stretch_x", "true");
                n.set_property("ui_stretch_y", "true");
            }
            let block = world.spawn_child(root, SceneNode2D::new("blurb"));
            {
                let n = world.get_mut(block).unwrap();
                n.set_property("ui", "text_block");
                n.set_property("ui_text", "one two three four five six seven eight");
                n.set_property("ui_text_size", "14");
                n.set_property("ui_wrap_w", "100");
                n.set_property("ui_size", "content");
                if let Some(leading) = leading {
                    n.set_property("ui_line_height", leading);
                }
            }
            let mut canvas = Canvas::new((400, 400), std::ptr::null());
            world.draw_to_canvas(&mut canvas, 0.0);
            let rect = world.resolved_rect(block).expect("block drawn");
            (
                rect.height,
                canvas.line_height_in(crate::FontId::DEFAULT, 14.0),
            )
        };

        let (plain, line_h) = measure(None);
        let (leaded, _) = measure(Some("1.35"));
        let lines = (plain / line_h).round();
        assert!(lines >= 2.0, "test text must wrap to compare leading");
        // lh + (n-1)*lh*1.35, not n*lh*1.35.
        let expected = line_h + (lines - 1.0) * line_h * 1.35;
        assert!(
            (leaded - expected).abs() < 1e-3,
            "leading applies between lines only: {leaded} vs {expected}"
        );
        assert!(leaded > plain, "1.35 is taller than the font's own box");
    }

    #[test]
    fn a_hidden_child_takes_no_slot_and_no_gap() {
        // `ui_visible: false` is CSS `display:none` here, not
        // `visibility:hidden`. The mockups switch between screen variants with
        // `<sc-if>`, which removes the element — so a variant that is not
        // showing must not hold a slot open. Reserving one left every sponsor
        // row's second figure a whole line low, and pushed each of the three
        // heading lines down by the unshown variant above it.
        let children = [
            ("a", &[("ui_w", "40")][..]),
            ("hidden", &[("ui_w", "40"), ("ui_visible", "false")][..]),
            ("b", &[("ui_w", "40")][..]),
        ];
        let (world, h) = flow_row((400, 100), &[("ui_gap", "10")], &children);
        let r = |i: usize| world.resolved_rect(h[i]).expect("child resolved");
        // Packed from the left: a at -200, b immediately after it plus one
        // gap. With the hidden child holding a slot, b sat 50 further right.
        assert!((r(0).x - -200.0).abs() < 1e-3, "first child at the edge");
        assert!(
            (r(2).x - -150.0).abs() < 1e-3,
            "the hidden child costs neither its 40px nor a gap: {}",
            r(2).x
        );

        // And it does not inflate the container that measures its children.
        let mut world = SceneWorld2D::new();
        let col = world.spawn(SceneNode2D::new("col"));
        {
            let n = world.get_mut(col).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_size", "content");
        }
        for (name, visible) in [("shown", "true"), ("gone", "false")] {
            let kid = world.spawn_child(col, SceneNode2D::new(name));
            let n = world.get_mut(kid).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "60");
            n.set_property("ui_h", "20");
            n.set_property("ui_visible", visible);
        }
        let mut canvas = Canvas::new((400, 400), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        let measured = world.resolved_rect(col).expect("column resolved");
        assert!(
            (measured.height - 20.0).abs() < 1e-3,
            "measures only the child it lays out: {}",
            measured.height
        );
    }

    #[test]
    fn ui_align_places_a_child_across_the_cross_axis() {
        // E-B: align-items:center, used 80 times in the mockups. The default
        // stays `stretch` — the slot spans the full cross axis — because every
        // scene authored before this relies on it.
        let children = [("box", &[("ui_w", "50"), ("ui_h", "40")][..])];
        let (world, h) = flow_row((200, 100), &[], &children);
        let d = world.resolved_rect(h[0]).unwrap();
        assert!(
            (d.height - 100.0).abs() < 1e-3,
            "default stretch spans the cross axis: {}",
            d.height
        );

        // Centred: a 40-tall child in a 100-tall row sits 30 from each edge.
        let (world, h) = flow_row((200, 100), &[("ui_align", "center")], &children);
        let c = world.resolved_rect(h[0]).unwrap();
        assert!((c.height - 40.0).abs() < 1e-3, "keeps its own height");
        assert!((c.y - -20.0).abs() < 1e-3, "centred on y: {}", c.y);

        // y-up: `start` is the TOP edge, matching reading order.
        let (world, h) = flow_row((200, 100), &[("ui_align", "start")], &children);
        let s = world.resolved_rect(h[0]).unwrap();
        assert!(
            (s.y + s.height - 50.0).abs() < 1e-3,
            "start pins to the top edge: top={}",
            s.y + s.height
        );
        let (world, h) = flow_row((200, 100), &[("ui_align", "end")], &children);
        let e = world.resolved_rect(h[0]).unwrap();
        assert!(
            (e.y - -50.0).abs() < 1e-3,
            "end pins to the bottom: {}",
            e.y
        );
    }

    #[test]
    fn a_flow_decided_axis_places_the_child_without_a_paired_origin() {
        // `ui_grow` and a non-stretch `ui_align` both hand the child a slot
        // that IS the child: same size, already placed. So the child must
        // take that slot outright. Before this it re-anchored inside it and
        // put its left edge on the slot's centre, which authoring worked
        // around with a paired `ui_origin_x: 0.5` on every such node — the
        // two-property gotcha `flow_size` exists to prevent.
        //
        // Deliberately no `ui_stretch_*` on these children: stretch would
        // mask the bug, which is why every earlier flow test missed it.
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(container).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_align", "center");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        let child = world.spawn_child(container, SceneNode2D::new("box"));
        {
            let n = world.get_mut(child).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "100");
            n.set_property("ui_h", "40");
        }
        let mut canvas = Canvas::new((400, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        let r = world.resolved_rect(child).unwrap();
        assert!((r.width - 100.0).abs() < 1e-3, "keeps its width");
        assert!(
            (r.x - -50.0).abs() < 1e-3,
            "centred on the container, not hung off its centre: x={}",
            r.x
        );

        // The default (`stretch`) decides nothing about *size* on the cross
        // axis — the slot still spans it and filling it is the child's own
        // `ui_stretch_*` opt-in. It does now decide *position*: a child that
        // does not fill the slot starts at the slot's cross edge.
        //
        // This assertion used to read `x == 0.0` — a 100-wide box in a 400-wide
        // column with its LEFT edge on the container's centre and half of it
        // outside the slot. That was the centre-anchor fallback showing through,
        // and it was wrong in the same way for every non-stretching flow child;
        // see `a_content_sized_flow_child_starts_at_its_cross_edge` for what it
        // cost. CSS `align-items: stretch` falls back to `flex-start` for an
        // item with a definite cross size, and so does this.
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(container).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        let child = world.spawn_child(container, SceneNode2D::new("box"));
        {
            let n = world.get_mut(child).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "100");
            n.set_property("ui_h", "40");
        }
        let mut canvas = Canvas::new((400, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        let r = world.resolved_rect(child).unwrap();
        assert!(
            (r.x - -200.0).abs() < 1e-3,
            "starts at the slot's left edge under the default align: x={}",
            r.x
        );
    }

    #[test]
    fn ui_lead_adds_space_a_uniform_gap_cannot() {
        // The mockups' `margin-top:34px` between blocks of a column: space
        // before one child only, which `ui_gap` (uniform) can't express and
        // which would otherwise be an empty spacer node per block.
        let (world, h) = flow_row(
            (400, 100),
            &[],
            &[
                ("a", &[("ui_w", "50")]),
                ("b", &[("ui_w", "50"), ("ui_lead", "30")]),
                ("c", &[("ui_w", "50")]),
            ],
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("child drawn");
        assert!((r(0).x - -200.0).abs() < 1e-3);
        // b starts 30 after a ends, and is still its own 50 wide — the margin
        // is space before it, not extra size.
        assert!(
            (r(1).x - (r(0).right() + 30.0)).abs() < 1e-3,
            "margin precedes b: {} vs {}",
            r(1).x,
            r(0).right()
        );
        assert!((r(1).width - 50.0).abs() < 1e-3, "b keeps its width");
        // c is unaffected beyond being pushed along.
        assert!((r(2).x - r(1).right()).abs() < 1e-3);

        // With a growing sibling the row must still sum to the container:
        // the margin is space already spoken for, not an overflow.
        let (world, h) = flow_row(
            (400, 100),
            &[],
            &[
                ("a", &[("ui_w", "50")]),
                ("grow", &[("ui_grow", "1")]),
                ("c", &[("ui_w", "50"), ("ui_lead", "30")]),
            ],
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("child drawn");
        // 400 - 50 - 50 - 30 = 270 left for the growing child.
        assert!(
            (r(1).width - 270.0).abs() < 1e-3,
            "grower absorbs the margin as spoken-for space: {}",
            r(1).width
        );
        assert!((r(2).x - (r(1).right() + 30.0)).abs() < 1e-3);
        assert!(
            (r(2).right() - 200.0).abs() < 1e-3,
            "row still ends at the container edge: {}",
            r(2).right()
        );
    }

    /// A grid container with `n` equally-sized children, at a given viewport.
    fn grid(
        viewport: (u32, u32),
        container_props: &[(&str, &str)],
        child_props: &[(&str, &str)],
        n: usize,
    ) -> (SceneWorld2D, Vec<NodeHandle2D>) {
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("root"));
        {
            let c = world.get_mut(container).unwrap();
            c.set_property("ui", "rect");
            c.set_property("ui_layout", "grid");
            c.set_property("ui_stretch_x", "true");
            c.set_property("ui_stretch_y", "true");
            for &(k, v) in container_props {
                c.set_property(k, v);
            }
        }
        let handles = (0..n)
            .map(|i| {
                let h = world.spawn_child(container, SceneNode2D::new(&format!("cell{i}")));
                let c = world.get_mut(h).unwrap();
                c.set_property("ui", "rect");
                for &(k, v) in child_props {
                    c.set_property(k, v);
                }
                h
            })
            .collect();
        let mut canvas = Canvas::new(viewport, std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        (world, handles)
    }

    #[test]
    fn grid_places_children_row_major_across_fixed_columns() {
        // `ui_cols: 3` is CSS `repeat(3, 1fr)`: three equal columns that stay
        // three at any width. 300 wide, gap 10 → columns of (300-20)/3 ≈ 93.3.
        let (world, h) = grid(
            (300, 400),
            &[("ui_cols", "3"), ("ui_gap", "10"), ("ui_row_h", "50")],
            &[],
            7,
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("cell drawn");
        let col_w = (300.0 - 20.0) / 3.0;
        for i in 0..7 {
            assert!(
                (r(i).width - col_w).abs() < 1e-3,
                "cell {i} width {} != {col_w}",
                r(i).width
            );
            assert!((r(i).height - 50.0).abs() < 1e-3);
        }
        // Row-major: 0,1,2 across the top; 3 wraps to the second row.
        assert!((r(0).x - -150.0).abs() < 1e-3);
        assert!((r(1).x - (r(0).x + col_w + 10.0)).abs() < 1e-3);
        assert!((r(2).x - (r(1).x + col_w + 10.0)).abs() < 1e-3);
        assert!((r(3).x - r(0).x).abs() < 1e-3, "index 3 starts a new row");
        // First row's top is the container's top; the next row is one row
        // plus one gap below it.
        assert!((r(0).y + r(0).height - 200.0).abs() < 1e-3);
        assert!((r(3).y - (r(0).y - 50.0 - 10.0)).abs() < 1e-3);
        // The last row is partial: 7 items over 3 columns is 3 rows.
        assert!((r(6).y - (r(0).y - 2.0 * 60.0)).abs() < 1e-3);
    }

    #[test]
    fn grid_auto_fills_as_many_fixed_width_columns_as_fit() {
        // `ui_col_w` is CSS `repeat(auto-fill, <px>)` — the deck-view case,
        // where a wider window shows more cards per row without the scene
        // being re-authored.
        let cols_at = |width: u32| {
            let (world, h) = grid(
                (width, 400),
                &[("ui_col_w", "100"), ("ui_gap", "10"), ("ui_row_h", "40")],
                &[],
                12,
            );
            // How many share the first row: count cells at the top row's y.
            let top = world.resolved_rect(h[0]).unwrap().y;
            (0..12)
                .filter(|&i| (world.resolved_rect(h[i]).unwrap().y - top).abs() < 1e-3)
                .count()
        };
        // 340 = 3*100 + 2*10 exactly; 450 fits 4 (4*100 + 3*10 = 430).
        assert_eq!(cols_at(340), 3, "three 100px columns fit exactly");
        assert_eq!(cols_at(450), 4, "a wider window reflows to four");
        assert_eq!(cols_at(90), 1, "never fewer than one column");
    }

    #[test]
    fn a_grid_cell_is_the_child_and_needs_no_stretch() {
        // Same contract as a flow-decided axis: the cell IS the child, so it
        // fills without `ui_stretch_*` and without a paired `ui_origin_*`.
        // These children author no size at all.
        let (world, h) = grid((200, 200), &[("ui_cols", "2"), ("ui_row_h", "60")], &[], 2);
        let a = world.resolved_rect(h[0]).unwrap();
        assert!(
            (a.width - 100.0).abs() < 1e-3,
            "fills its cell: {}",
            a.width
        );
        assert!((a.height - 60.0).abs() < 1e-3);
        assert!((a.x - -100.0).abs() < 1e-3, "at its cell, not hung off it");
    }

    #[test]
    fn a_content_sized_container_includes_its_childrens_leads() {
        // Measuring without `ui_lead` makes the container short by exactly the
        // leads it holds, and it then packs its children tighter than the flow
        // places them — which shows up as the led child overlapping the one
        // before it, not as a wrong size.
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(container).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_size", "content");
        }
        for (name, props) in [
            ("a", &[("ui_h", "40"), ("ui_w", "10")][..]),
            (
                "b",
                &[("ui_h", "40"), ("ui_w", "10"), ("ui_lead", "25")][..],
            ),
        ] {
            let h = world.spawn_child(container, SceneNode2D::new(name));
            let n = world.get_mut(h).unwrap();
            n.set_property("ui", "rect");
            for &(k, v) in props {
                n.set_property(k, v);
            }
        }
        let mut canvas = Canvas::new((200, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        let r = world.resolved_rect(container).unwrap();
        assert!(
            (r.height - 105.0).abs() < 1e-3,
            "40 + 25 lead + 40, not 80: {}",
            r.height
        );
    }

    #[test]
    fn ui_lead_in_a_column_comes_off_the_top() {
        // y-up: a column packs downward, so a child's leading edge is its
        // top and the margin has to move it *down*, not up. Getting this
        // backwards is invisible until two blocks overlap.
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(container).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        let mut handles = Vec::new();
        for (name, props) in [
            ("a", &[("ui_h", "40")][..]),
            ("b", &[("ui_h", "40"), ("ui_lead", "20")][..]),
        ] {
            let h = world.spawn_child(container, SceneNode2D::new(name));
            let n = world.get_mut(h).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
            for &(k, v) in props {
                n.set_property(k, v);
            }
            handles.push(h);
        }
        let mut canvas = Canvas::new((200, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        let a = world.resolved_rect(handles[0]).unwrap();
        let b = world.resolved_rect(handles[1]).unwrap();
        assert!((a.y + a.height - 100.0).abs() < 1e-3, "a starts at the top");
        assert!(
            (b.y + b.height - (a.y - 20.0)).abs() < 1e-3,
            "b's top sits 20 below a's bottom: b_top={} a_bottom={}",
            b.y + b.height,
            a.y
        );
        assert!((b.height - 40.0).abs() < 1e-3, "b keeps its height");
    }

    #[test]
    fn an_auto_lead_pins_a_child_to_the_end_of_the_flow() {
        // CSS `margin-top:auto` — the deck screen's card footer, pinned to the
        // bottom of a fixed-height card whose rows above it size themselves.
        // The slack goes into the auto lead, not after the last child, which is
        // the difference between this and `ui_justify`.
        let mut world = SceneWorld2D::new();
        let container = world.spawn(SceneNode2D::new("root"));
        {
            let n = world.get_mut(container).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_layout", "column");
            n.set_property("ui_stretch_x", "true");
            n.set_property("ui_stretch_y", "true");
        }
        let mut handles = Vec::new();
        for (name, props) in [
            ("head", &[("ui_h", "30")][..]),
            ("foot", &[("ui_h", "20"), ("ui_lead", "auto")][..]),
        ] {
            let h = world.spawn_child(container, SceneNode2D::new(name));
            let n = world.get_mut(h).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_stretch_x", "true");
            for &(k, v) in props {
                n.set_property(k, v);
            }
            handles.push(h);
        }
        let mut canvas = Canvas::new((200, 200), std::ptr::null());
        world.draw_to_canvas(&mut canvas, 0.0);
        let head = world.resolved_rect(handles[0]).unwrap();
        let foot = world.resolved_rect(handles[1]).unwrap();
        // The container is the full 200 tall, centred on the origin: top +100,
        // bottom -100. The header keeps its slot at the top...
        assert!(
            (head.y + head.height - 100.0).abs() < 1e-3,
            "head at the top: {}",
            head.y + head.height
        );
        assert!((head.height - 30.0).abs() < 1e-3, "head keeps its height");
        // ...and the footer sits on the bottom edge, still its own 20 tall —
        // the slack became space *before* it, not extra size.
        assert!((foot.y - -100.0).abs() < 1e-3, "foot on the bottom: {}", foot.y);
        assert!((foot.height - 20.0).abs() < 1e-3, "foot keeps its height");
    }

    #[test]
    fn absent_flow_properties_place_children_exactly_as_before() {
        // The correctness check the plan calls for on the two-pass rewrite:
        // with no ui_grow/ui_justify/ui_align anywhere, placement must match
        // the old packed-from-the-start flow. Every existing scene depends on
        // this and none of them author the new properties.
        let (world, h) = flow_row(
            (400, 100),
            &[("ui_gap", "5"), ("ui_pad_left", "10")],
            &[
                ("a", &[("ui_w", "30")]),
                ("b", &[("ui_w", "40")]),
                ("c", &[("ui_w", "50")]),
            ],
        );
        let r = |i: usize| world.resolved_rect(h[i]).expect("child drawn");
        // Packed from the padded start, each taking its literal width.
        assert!((r(0).x - -190.0).abs() < 1e-3, "a.x: {}", r(0).x);
        assert!((r(1).x - -155.0).abs() < 1e-3, "b.x: {}", r(1).x);
        assert!((r(2).x - -110.0).abs() < 1e-3, "c.x: {}", r(2).x);
        assert!((r(0).width - 30.0).abs() < 1e-3);
        assert!((r(1).width - 40.0).abs() < 1e-3);
        assert!((r(2).width - 50.0).abs() < 1e-3);
        // Full cross axis, as before.
        for i in 0..3 {
            assert!((r(i).height - 100.0).abs() < 1e-3, "spans cross axis");
        }
    }

    /// The editor's white-box bug, as a guard.
    ///
    /// A node authored `"ui_color": "{chalk},255"` and drawn with no ambient
    /// bindings keeps the `{chalk}` literal, which parses as no colour and
    /// falls back to white — every panel in the scene turns into a pale
    /// rectangle and the preview is unreadable. Layout is unaffected, so no
    /// rect assertion can catch this; the drawn colour is the only evidence.
    ///
    /// Both halves matter. Unbound must be white (the bug reproduces) and
    /// bound must be the palette's actual value (the fix resolves), or a
    /// binding table that silently resolved to *some other* wrong colour
    /// would pass.
    #[test]
    fn ambient_bindings_resolve_authored_palette_tokens() {
        let draw = |bindings: &Bindings| {
            let mut world = SceneWorld2D::new();
            let node = world.spawn(SceneNode2D::new("panel"));
            let n = world.get_mut(node).unwrap();
            n.set_property("ui", "rect");
            n.set_property("ui_w", "50");
            n.set_property("ui_h", "50");
            n.set_property("ui_color", "{chalk},255");

            let mut canvas = Canvas::new((200, 200), std::ptr::null());
            world.draw_to_canvas_in_with_bindings(
                &mut canvas,
                (-100.0, -100.0, 200.0, 200.0),
                0.0,
                bindings,
            );
            canvas
                .vertices()
                .first()
                .map(|v| v.color)
                .expect("the rect drew at least one vertex")
        };

        let unbound = draw(&Bindings::new());
        assert!(
            unbound.iter().take(3).all(|c| *c >= 0.99),
            "an unresolved token should fall back to white — that IS the              white-box bug this guards, so if this ever stops being true the              test below is no longer proving anything: {unbound:?}"
        );

        let chalk = crate::Color::from_srgb8(0xe8, 0xe4, 0xd9, 255);
        let bound = draw(&[("chalk".to_string(), "232,228,217".to_string())]
            .into_iter()
            .collect());
        let expected = [chalk.r, chalk.g, chalk.b, chalk.a];
        for (got, want) in bound.iter().zip(expected) {
            assert!(
                (got - want).abs() < 1e-3,
                "bound token drew {bound:?}, palette chalk is {expected:?}"
            );
        }
    }

    #[test]
    fn authored_clips_play_then_restore_their_targets() {
        use crate::scene::{AnimKeyframe, AnimatedProperty, SceneAnimClip, SceneAnimTrack};
        use crate::math::tween::Easing;

        let mut world = SceneWorld2D::new();
        let mut car = SceneNode2D::new("car_a").with_name("car_a");
        car.set_position(Vec2::new(40.0, 20.0));
        world.spawn(car);

        // The clip moves the car 100px right over 1s (linear), then the world
        // plays it on the host clock: t0 = 50, apply at 50.5 => halfway.
        world.animations.push(SceneAnimClip {
            id: "lunge".to_string(),
            duration: 1.0,
            looping: false,
            tracks: vec![SceneAnimTrack {
                target: "car_a".to_string(),
                property: AnimatedProperty::OffsetX,
                keyframes: vec![
                    AnimKeyframe { t: 0.0, value: 0.0, ease: Easing::Linear },
                    AnimKeyframe { t: 1.0, value: 100.0, ease: Easing::Linear },
                ],
            }],
        });

        assert!(world.play_animation("lunge", 50.0));
        world.apply_animations(50.5);
        let x = world.get(world.find_by_name("car_a").unwrap()).unwrap().position().x;
        assert!((x - 90.0).abs() < 0.001, "halfway through the clip the car should sit at rest+50, got {x}");

        // A clip that doesn't exist is a no-op.
        assert!(!world.play_animation("missing", 0.0));

        // Past the end the clip is done and the node is restored.
        world.apply_animations(51.5);
        assert!(!world.is_animation_playing("lunge"));
        let x = world.get(world.find_by_name("car_a").unwrap()).unwrap().position().x;
        assert!((x - 40.0).abs() < 0.001, "a finished clip restores the authored position, got {x}");

        // A looping clip never finishes and (with a flat track) just holds.
        world.animations[0].looping = true;
        world.play_animation("lunge", 0.0);
        world.apply_animations(9.0);
        assert!(world.is_animation_playing("lunge"));
        world.stop_animation("lunge");
        assert!(!world.is_animation_playing("lunge"));
    }

    /// A UI node's rotation is drawn from its `ui_rotation` property, not the
    /// node transform - so a rotation track must drive both, or a UI-authored
    /// scene's cars never visibly turn. Pin that a played rotation clip writes
    /// the property and a finished clip restores the authored value.
    #[test]
    fn a_rotation_track_drives_ui_rotation_for_ui_nodes_and_restores_it() {
        use crate::math::tween::Easing;
        use crate::scene::{AnimKeyframe, AnimatedProperty, SceneAnimClip, SceneAnimTrack};

        let mut world = SceneWorld2D::new();
        let mut badge = SceneNode2D::new("badge").with_name("badge");
        badge.set_property("ui", "image");
        badge.set_property("ui_color", "224,161,60,255");
        world.spawn(badge);

        world.animations.push(SceneAnimClip {
            id: "spin".to_string(),
            duration: 1.0,
            looping: false,
            tracks: vec![SceneAnimTrack {
                target: "badge".to_string(),
                property: AnimatedProperty::RotationDeg,
                keyframes: vec![
                    AnimKeyframe { t: 0.0, value: 0.0, ease: Easing::Linear },
                    AnimKeyframe { t: 1.0, value: 30.0, ease: Easing::Linear },
                ],
            }],
        });

        let badge = world.find_by_name("badge").unwrap();
        assert_eq!(world.get(badge).unwrap().property("ui_rotation"), None);

        world.play_animation("spin", 20.0);
        world.apply_animations(20.5);
        // Midway through a 0->30 degrees linear track.
        assert_eq!(
            world.get(badge).unwrap().property("ui_rotation").map(str::to_string),
            Some("15".to_string()),
            "a UI node's rotation track must write ui_rotation so the draw path sees it"
        );

        world.apply_animations(21.5); // past the end -> clip finishes and restores
        assert_eq!(
            world.get(badge).unwrap().property("ui_rotation"),
            Some("0"),
            "a finished clip returns the UI node to upright"
        );
    }

    /// A `ui: rect` child of a stretched root must emit painted quads with
    /// the authored colour at full alpha - the PiP vignette's panel vanished
    /// from the composed capture despite resolving a full-size rect, and this
    /// pins that the draw itself emits the right verts (so the disappearance
    /// was a compositing/capture issue, not the engine scene paint). Built
    /// through the same loader the host game uses (`from_json_str` + undo the
    /// no-texture fallback difference), so the hierarchy/parenting path is
    /// the shipped one.
    #[test]
    fn a_stretched_root_with_a_rect_child_emits_painted_quads() {
        use crate::canvas::Canvas;
        use crate::scene::Scene2D;
        use crate::AssetPack;

        let json = r#"{
          "name": "probe",
          "version": 1,
          "view": { "window_size": [320.0, 200.0] },
          "animations": [],
          "nodes": [
            { "id": 1, "parent": null, "name": "root", "kind": "Layout",
              "position": [0.0, 0.0], "size": [320.0, 200.0], "visible": true,
              "script_path": "", "runtime_prefab": "ui_node", "asset_alias": "",
              "properties": { "ui": "rect", "ui_color": "0,0,0,0", "ui_stretch_x": "true", "ui_stretch_y": "true" } },
            { "id": 2, "parent": 1, "name": "panel", "kind": "Panel",
              "position": [0.0, 0.0], "size": [160.0, 80.0], "visible": true,
              "script_path": "", "runtime_prefab": "ui_node", "asset_alias": "",
              "properties": { "ui": "rect", "ui_color": "200,0,200,255", "ui_w": "160.0", "ui_h": "80.0" } }
          ]
        }"#;
        let path = std::path::Path::new("scenes/inline_probe.json");
        let scene = Scene2D::from_json_str(path, json, &AssetPack::default()).unwrap();
        let world = SceneWorld2D::from_scene(&scene);

        let mut canvas = Canvas::for_test((640, 400));
        world.draw_to_canvas_in_with_bindings(
            &mut canvas,
            (0.0, 0.0, 320.0, 200.0),
            0.0,
            &crate::Bindings::new(),
        );

        let magenta: Vec<_> = canvas
            .verts
            .iter()
            .filter(|v| (v.color[0] - 200.0 / 255.0).abs() < 1e-3 && v.color[2] - 200.0 / 255.0 < 1e-3)
            .collect();
        // The canvas stores colors in linear space, so the magentas exact
        // numeric value round-trips through sRGB->linear for 200,0,200. What
        // matters is *that* the child's fill reached the canvas: magenta is
        // the one hue nothing else in the probe draws, and it must arrive at
        // full alpha. Compare by hue, not by an absolute linear value, so the
        // guard stays honest to the paint path rather than to one encoding.
        let magenta: Vec<_> = canvas
            .verts
            .iter()
            .filter(|v| {
                v.color[0] > 0.4
                    && v.color[1] < 0.05
                    && (v.color[0] - v.color[2]).abs() < 0.05
            })
            .collect();
        assert!(
            magenta.len() >= 6,
            "the child rect must emit a filled quad (the vignette panel vanishing means this broke): got {} magenta verts",
            magenta.len()
        );
        assert!(
            magenta.iter().all(|v| (v.color[3] - 1.0).abs() < 1e-3),
            "the child rect's verts must be full-alpha; a dimmed composite means the scrim is elsewhere"
        );
        assert!(
            magenta.iter().all(|v| (v.color[3] - 1.0).abs() < 1e-3),
            "the child rect's verts must be full-alpha; a dimmed composite means the scrim is elsewhere"
        );
    }

    /// The generic controller focus ring: a focused node that opts in with
    /// `ui_focus_ring_color` paints four thin bars around its resolved rect.
    /// This is the visible marker the game's controller focus walk points at
    /// on the race HUD, where there is no cursor to hover with.
    #[test]
    fn a_focus_ring_paints_when_the_node_authorises_one() {
        use crate::canvas::Canvas;
        use crate::scene::Scene2D;
        use crate::AssetPack;

        let json = r#"{
          "name": "ring",
          "version": 1,
          "view": { "window_size": [320.0, 200.0] },
          "animations": [],
          "nodes": [
            { "id": 1, "parent": null, "name": "root", "kind": "Layout",
              "position": [0.0, 0.0], "size": [320.0, 200.0], "visible": true,
              "script_path": "", "runtime_prefab": "ui_node", "asset_alias": "",
              "properties": { "ui": "rect", "ui_color": "20,24,30,255", "ui_stretch_x": "true", "ui_stretch_y": "true" } },
            { "id": 2, "parent": 1, "name": "seat", "kind": "Panel",
              "position": [0.0, 0.0], "size": [120.0, 80.0], "visible": true,
              "script_path": "", "runtime_prefab": "ui_node", "asset_alias": "",
              "properties": { "ui": "rect", "ui_color": "40,48,60,255",
                              "ui_focusable": "true", "ui_focus_ring_color": "255,90,90,255" } }
          ]
        }"#;
        let path = std::path::Path::new("scenes/inline_ring.json");
        let scene = Scene2D::from_json_str(path, json, &AssetPack::default()).unwrap();
        let world = SceneWorld2D::from_scene(&scene);

        let seat = world.find_by_name("seat").unwrap();
        let mut canvas = Canvas::for_test((640, 400));
        world.set_focus(Some(seat));
        world.draw_to_canvas_in_with_bindings(
            &mut canvas,
            (0.0, 0.0, 320.0, 200.0),
            0.0,
            &crate::Bindings::new(),
        );

        let ring_verts = canvas
            .verts
            .iter()
            .filter(|v| v.color[0] > 0.9 && v.color[1] < 0.5 && v.color[2] < 0.5)
            .count();
        assert!(ring_verts >= 24, "four 2px bars describe 24 verts, got {ring_verts}");

        let mut canvas2 = Canvas::for_test((640, 400));
        world.set_focus(None);
        world.draw_to_canvas_in_with_bindings(
            &mut canvas2,
            (0.0, 0.0, 320.0, 200.0),
            0.0,
            &crate::Bindings::new(),
        );
        let unfocused_ring = canvas2
            .verts
            .iter()
            .filter(|v| v.color[0] > 0.9 && v.color[1] < 0.5 && v.color[2] < 0.5)
            .count();
        assert_eq!(unfocused_ring, 0, "no ring when nothing is focused");
    }
}
