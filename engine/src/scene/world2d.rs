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

use std::collections::{HashMap, HashSet};

use crate::renderer::{DrawParams, Frame};
use crate::{Rect, Vec2};

use super::data2d::{parse_bool_property, PrefabSprite2D, Scene2D};

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
}

impl SceneWorld2D {
    pub fn new() -> Self {
        Self::default()
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
    /// Prefers an explicit interactive size from the node's `w`/`h` properties
    /// (how editor-authored UI/markers carry their box), falling back to the
    /// union of the node's sprite layers. Returns `None` for nodes with no
    /// pickable area. Rotation is not yet folded into the hit rect — the bounds
    /// are the node's axis-aligned extent at its composed world position/scale.
    pub fn node_bounds(&self, handle: NodeHandle2D) -> Option<Rect> {
        let node = self.get(handle)?;
        let world = self.world_transform(handle)?;

        if let (Some(w), Some(h)) = (node.property_f32("w"), node.property_f32("h")) {
            let size = Vec2::new(w, h) * world.scale;
            return Some(Rect::from_pos_size(world.position, size));
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
        let roots = self.roots.clone();
        for root in roots {
            self.draw_node(root, &Transform2D::default(), frame);
        }
    }

    fn draw_node(&self, handle: NodeHandle2D, parent_world: &Transform2D, frame: &mut Frame) {
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

        let children = node.children.clone();
        for child in children {
            self.draw_node(child, &world, frame);
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
