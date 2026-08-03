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
use crate::layout::Stack;
use crate::renderer::{DrawParams, Frame};
use crate::{Rect, Vec2};

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

    /// Build a live world from a loaded [`Scene2D`].
    ///
    /// Convenience wrapper over [`SceneWorld2D::instantiate_scene`] into a fresh
    /// world with no parent and no offset.
    pub fn from_scene(scene: &Scene2D) -> Self {
        let mut world = SceneWorld2D::new();
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
        let Some(source_name) = node.property("ui_repeat_source").map(str::to_string) else {
            return;
        };
        let item_count = repeaters.get(&source_name).map_or(0, Vec::len);

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
        if let Some(items) = repeaters.get(&source_name) {
            for (instance, scope) in instances.iter().zip(items.iter()) {
                if let Some(node) = self.get_mut(*instance) {
                    node.instance_bindings = Some(scope.clone());
                }
            }
        }
    }

    /// Deep-copy a live subtree into a detached [`NodeTemplate`] — the node's
    /// own fields (via `SceneNode2D`'s `Clone`) plus each child captured the
    /// same way, recursively. Hierarchy links (`parent`/`children` handles)
    /// on the captured `SceneNode2D` are meaningless once detached from the
    /// live graph; `materialize_template` rebuilds them via `spawn_child`.
    fn capture_template(&self, handle: NodeHandle2D) -> NodeTemplate {
        let node = self
            .get(handle)
            .cloned()
            .expect("capture_template called with a live handle");
        let children = node
            .children
            .iter()
            .map(|&child| self.capture_template(child))
            .collect();
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
        let (sw, sh) = frame.canvas(0).screen_size();
        let screen = (-(sw as f32) / 2.0, -(sh as f32) / 2.0, sw as f32, sh as f32);
        let roots = self.roots.clone();
        for root in roots {
            self.draw_node(root, &Transform2D::default(), screen, time, frame, bindings);
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
    /// Deliberately a single peek-one/place-one pass, not a two-pass
    /// measure-then-place: `Stack::next` only needs one child's size at a
    /// time, so there's no need to resolve every sibling before placing the
    /// first. `Rect::distribute_v/h` (`Track::Even`/`Weight`) and `Justify::*`
    /// both need the *sum* of every sibling's size before placing item 0 —
    /// out of scope until something actually needs proportional/justified
    /// distribution instead of packed-from-the-start flow.
    ///
    /// Main-axis size is read directly here (not via `resolve_ui_rect`, which
    /// also resolves *position* — not wanted yet) — a child on the main axis
    /// only reads plain `ui_w`/`ui_h`; `ui_w_frac`/`ui_stretch_x` on the main
    /// axis are not supported in a flow container for this first pass.
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
            _ => return vec![parent_rect; children.len()],
        };
        let prop_f32 = |name: &str| get(parent, name).and_then(|v| v.trim().parse::<f32>().ok());
        let pad_left = prop_f32("ui_pad_left").unwrap_or(0.0);
        let pad_right = prop_f32("ui_pad_right").unwrap_or(0.0);
        let pad_top = prop_f32("ui_pad_top").unwrap_or(0.0);
        let pad_bottom = prop_f32("ui_pad_bottom").unwrap_or(0.0);
        let gap = prop_f32("ui_gap").unwrap_or(0.0);

        let (px, py, pw, ph) = parent_rect;
        let padded = Rect::new(
            px + pad_left,
            py + pad_bottom,
            (pw - pad_left - pad_right).max(0.0),
            (ph - pad_top - pad_bottom).max(0.0),
        );
        let mut stack = if vertical {
            Stack::vertical(padded, gap)
        } else {
            Stack::horizontal(padded, gap)
        };

        children
            .iter()
            .map(|&child| {
                let Some(child_node) = self.get(child) else {
                    return parent_rect;
                };
                let main_axis_prop = if vertical { "ui_h" } else { "ui_w" };
                let scale = if vertical {
                    child_node.transform.scale.y
                } else {
                    child_node.transform.scale.x
                };
                let extent = child_node.property_f32(main_axis_prop).unwrap_or(0.0) * scale;
                let slot = stack.next(extent);
                (slot.x, slot.y, slot.width, slot.height)
            })
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

        let children = node.children.clone();
        let child_refs =
            self.resolve_child_references(node, ui_rect, &children, effective_bindings);
        for (child, child_ref) in children.into_iter().zip(child_refs) {
            self.draw_node(child, &world, child_ref, time, frame, effective_bindings);
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
        let roots = self.roots.clone();
        for root in roots {
            self.draw_node_on_canvas(root, screen, time, canvas, bindings);
        }
    }

    fn draw_node_on_canvas(
        &self,
        handle: NodeHandle2D,
        parent_ui_rect: (f32, f32, f32, f32),
        time: f32,
        canvas: &mut Canvas,
        bindings: &Bindings,
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

        let (ui_rect, ui_visible) = crate::scene::data2d::draw_ui_node_on_with_bindings(
            canvas,
            parent_ui_rect,
            node.transform.position,
            node.transform.scale,
            time,
            |n| node.property(n).map(str::to_owned),
            effective_bindings,
            node.sprites.first(),
            self.pixel_grid.get(),
        );
        if ui_visible && node.property("ui").is_some() {
            let (x, y, w, h) = ui_rect;
            node.ui_rect
                .set(Some(Rect::from_pos_size(Vec2::new(x, y), Vec2::new(w, h))));
        } else {
            node.ui_rect.set(None);
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
            self.draw_node_on_canvas(child, child_ref, time, canvas, effective_bindings);
        }
        if clipped {
            canvas.pop_clip();
        }
    }

    // --- internal helpers -------------------------------------------------

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
}
