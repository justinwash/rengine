use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub use rengine::EditorSceneNodeKind as SceneNodeKind;
pub use rengine::EditorSceneNode as SceneNode;
pub use rengine::SceneAnimClip;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneViewSettings {
    #[serde(default = "default_scene_window_size")]
    pub window_size: [f32; 2],
}

impl Default for SceneViewSettings {
    fn default() -> Self {
        Self {
            window_size: default_scene_window_size(),
        }
    }
}

pub trait SceneNodeKindExt {
    fn label(self) -> &'static str;
    fn short_label(self) -> &'static str;
    fn default_size(self) -> [f32; 2];
    fn default_name(self, id: u64) -> String;
}

impl SceneNodeKindExt for SceneNodeKind {
    fn label(self) -> &'static str {
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

    fn short_label(self) -> &'static str {
        match self {
            Self::Group => "GRP",
            Self::Empty => "EMP",
            Self::Camera2d => "CAM",
            Self::Sprite => "SPR",
            Self::Trigger => "TRG",
            Self::UiRoot => "UI",
            Self::Label => "TXT",
            Self::Panel => "PNL",
            Self::Image => "IMG",
            Self::Button => "BTN",
            Self::Layout => "LYT",
            Self::List => "LST",
            Self::Shape => "SHP",
            Self::Action => "ACT",
        }
    }

    fn default_size(self) -> [f32; 2] {
        match self {
            Self::Group => [120.0, 72.0],
            Self::Empty => [88.0, 56.0],
            Self::Camera2d => [96.0, 68.0],
            Self::Sprite => [112.0, 72.0],
            Self::Trigger => [148.0, 88.0],
            Self::UiRoot => [220.0, 136.0],
            Self::Label => [160.0, 24.0],
            Self::Panel => [220.0, 136.0],
            Self::Image => [112.0, 72.0],
            Self::Button => [210.0, 38.0],
            Self::Layout => [220.0, 136.0],
            Self::List => [220.0, 136.0],
            Self::Shape => [120.0, 24.0],
            Self::Action => [88.0, 56.0],
        }
    }

    fn default_name(self, id: u64) -> String {
        format!("{} {}", self.label(), id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneNodeReorderDirection {
    Up,
    Down,
}

fn new_scene_node(id: u64, kind: SceneNodeKind, parent: Option<u64>, sibling_index: usize) -> SceneNode {
    let offset = sibling_index as f32 * 28.0;
    let position = if parent.is_some() {
        [56.0 + offset, 48.0 + offset]
    } else {
        [-72.0 + offset, -48.0 + offset]
    };

    SceneNode {
        id,
        parent,
        name: kind.default_name(id),
        kind,
        position,
        size: kind.default_size(),
        visible: true,
        script_path: String::new(),
        runtime_prefab: String::new(),
        asset_alias: String::new(),
        properties: HashMap::new(),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneDocument {
    pub name: String,
    /// Schema version written on every save. Absent (0) in pre-versioning files
    /// which are format-compatible with the current version.
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub view: SceneViewSettings,
    pub nodes: Vec<SceneNode>,
    /// Scene-authored keyframe clips (`animations` at the document root).
    /// Authored in the editor, played back by the engine from the scene data —
    /// never a Rust procedure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<SceneAnimClip>,
    /// Legacy counter kept for backward-compatibility with hand-edited files.
    /// Node IDs are now assigned randomly; this field is no longer used for
    /// allocation but is preserved so older tooling can still read the format.
    #[serde(default)]
    pub next_id: u64,
}

impl SceneDocument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: rengine::CURRENT_EDITOR_SCENE_VERSION,
            view: SceneViewSettings::default(),
            nodes: Vec::new(),
            animations: Vec::new(),
            next_id: 0,
        }
    }

    pub fn add_node(&mut self, kind: SceneNodeKind, parent: Option<u64>) -> u64 {
        let sibling_index = self
            .nodes
            .iter()
            .filter(|node| node.parent == parent)
            .count();
        let id = self.alloc_unique_id();
        self.nodes
            .push(new_scene_node(id, kind, parent, sibling_index));
        id
    }

    /// Repair `next_id` so it sits above every existing node id.
    ///
    /// Called when loading a file that may have been hand-edited or saved by
    /// an older editor. No-op for well-formed documents with random IDs.
    pub fn normalize_next_id(&mut self) -> bool {
        let normalized_next_id = self
            .nodes
            .iter()
            .map(|node| node.id)
            .max()
            .map(|max_id| max_id.saturating_add(1))
            .unwrap_or(1);

        if self.next_id < normalized_next_id {
            self.next_id = normalized_next_id;
            true
        } else {
            false
        }
    }

    /// Return a node id that is not already in this document.
    ///
    /// Uses a thread-local pseudo-random generator seeded from the system
    /// clock so ids are unique across scenes without any coordination.
    fn alloc_unique_id(&self) -> u64 {
        let existing: HashSet<u64> = self.nodes.iter().map(|n| n.id).collect();
        for _ in 0..128 {
            let id = new_node_id();
            if id != 0 && !existing.contains(&id) {
                return id;
            }
        }
        // Astronomical-odds fallback: sequential above the current maximum.
        self.nodes
            .iter()
            .map(|n| n.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    pub fn node(&self, id: u64) -> Option<&SceneNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn node_mut(&mut self, id: u64) -> Option<&mut SceneNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    pub fn node_name(&self, id: u64) -> Option<&str> {
        self.node(id).map(|node| node.name.as_str())
    }

    pub fn root_ids(&self) -> Vec<u64> {
        self.nodes
            .iter()
            .filter(|node| node.parent.is_none())
            .map(|node| node.id)
            .collect()
    }

    pub fn child_ids(&self, parent: u64) -> Vec<u64> {
        self.nodes
            .iter()
            .filter(|node| node.parent == Some(parent))
            .map(|node| node.id)
            .collect()
    }

    pub fn is_descendant_of(&self, node_id: u64, ancestor_id: u64) -> bool {
        let mut current = self.node(node_id).and_then(|node| node.parent);
        while let Some(parent_id) = current {
            if parent_id == ancestor_id {
                return true;
            }
            current = self.node(parent_id).and_then(|node| node.parent);
        }

        false
    }

    pub fn selected_root_ids(&self, node_ids: &[u64]) -> Vec<u64> {
        let selected: HashSet<u64> = node_ids
            .iter()
            .copied()
            .filter(|node_id| self.node(*node_id).is_some())
            .collect();

        self.nodes
            .iter()
            .filter_map(|node| {
                if !selected.contains(&node.id) {
                    return None;
                }

                let mut current = node.parent;
                while let Some(parent_id) = current {
                    if selected.contains(&parent_id) {
                        return None;
                    }
                    current = self.node(parent_id).and_then(|parent| parent.parent);
                }

                Some(node.id)
            })
            .collect()
    }

    pub fn subtree_ids(&self, node_id: u64) -> Vec<u64> {
        let mut children_by_parent: HashMap<u64, Vec<u64>> = HashMap::new();
        for node in &self.nodes {
            if let Some(parent) = node.parent {
                children_by_parent.entry(parent).or_default().push(node.id);
            }
        }

        let mut subtree_ids = HashSet::new();
        let mut stack = vec![node_id];
        while let Some(current_id) = stack.pop() {
            if !subtree_ids.insert(current_id) {
                continue;
            }

            if let Some(child_ids) = children_by_parent.get(&current_id) {
                stack.extend(child_ids.iter().rev().copied());
            }
        }

        self.nodes
            .iter()
            .filter(|node| subtree_ids.contains(&node.id))
            .map(|node| node.id)
            .collect()
    }

    pub fn duplicate_nodes(&mut self, node_ids: &[u64], position_delta: [f32; 2]) -> Vec<u64> {
        let root_ids = self.selected_root_ids(node_ids);
        if root_ids.is_empty() {
            return Vec::new();
        }

        let source_nodes = self.nodes.clone();
        let source_by_id: HashMap<u64, SceneNode> = source_nodes
            .iter()
            .cloned()
            .map(|node| (node.id, node))
            .collect();
        let mut children_by_parent: HashMap<u64, Vec<u64>> = HashMap::new();
        for node in &source_nodes {
            if let Some(parent) = node.parent {
                children_by_parent.entry(parent).or_default().push(node.id);
            }
        }

        let mut duplicated_root_ids = Vec::new();
        for root_id in root_ids {
            if let Some(duplicated_root_id) = self.duplicate_subtree(
                root_id,
                None,
                position_delta,
                &source_by_id,
                &children_by_parent,
            ) {
                duplicated_root_ids.push(duplicated_root_id);
            }
        }

        duplicated_root_ids
    }

    pub fn reparent_nodes(&mut self, node_ids: &[u64], new_parent: Option<u64>) -> bool {
        let root_ids = self.selected_root_ids(node_ids);
        if root_ids.is_empty() {
            return false;
        }

        if root_ids
            .iter()
            .any(|root_id| !self.can_reparent_node(*root_id, new_parent))
        {
            return false;
        }

        let moved_ids: HashSet<u64> = root_ids
            .iter()
            .flat_map(|root_id| self.subtree_ids(*root_id))
            .collect();
        let root_id_set: HashSet<u64> = root_ids.iter().copied().collect();
        let mut moved_nodes = self.extract_nodes_by_ids(&moved_ids);
        for node in &mut moved_nodes {
            if root_id_set.contains(&node.id) {
                node.parent = new_parent;
            }
        }
        self.nodes.extend(moved_nodes);
        true
    }

    pub fn reorder_nodes(
        &mut self,
        node_ids: &[u64],
        direction: SceneNodeReorderDirection,
    ) -> bool {
        let root_ids = self.selected_root_ids(node_ids);
        if root_ids.is_empty() {
            return false;
        }

        let Some(first_root) = root_ids.first().copied() else {
            return false;
        };
        let Some(parent) = self.node(first_root).map(|node| node.parent) else {
            return false;
        };

        if root_ids
            .iter()
            .copied()
            .any(|root_id| self.node(root_id).map(|node| node.parent) != Some(parent))
        {
            return false;
        }

        let sibling_root_ids = if let Some(parent_id) = parent {
            self.child_ids(parent_id)
        } else {
            self.root_ids()
        };
        let selected_root_set: HashSet<u64> = root_ids.iter().copied().collect();
        let selected_positions: Vec<usize> = sibling_root_ids
            .iter()
            .enumerate()
            .filter_map(|(index, sibling_id)| {
                selected_root_set.contains(sibling_id).then_some(index)
            })
            .collect();
        if selected_positions.is_empty() {
            return false;
        }

        let min_index = selected_positions[0];
        let max_index = *selected_positions.last().unwrap_or(&min_index);
        let insert_index = match direction {
            SceneNodeReorderDirection::Up => {
                if min_index == 0 {
                    return false;
                }
                min_index - 1
            }
            SceneNodeReorderDirection::Down => {
                if max_index + 1 >= sibling_root_ids.len() {
                    return false;
                }
                min_index + 1
            }
        };

        let mut reordered_root_ids: Vec<u64> = sibling_root_ids
            .iter()
            .copied()
            .filter(|sibling_id| !selected_root_set.contains(sibling_id))
            .collect();
        reordered_root_ids.splice(insert_index..insert_index, root_ids.iter().copied());

        let mut subtree_root_by_id = HashMap::new();
        for root_id in &sibling_root_ids {
            for subtree_id in self.subtree_ids(*root_id) {
                subtree_root_by_id.insert(subtree_id, *root_id);
            }
        }

        let mut blocks: HashMap<u64, Vec<SceneNode>> = sibling_root_ids
            .iter()
            .copied()
            .map(|root_id| (root_id, Vec::new()))
            .collect();
        let mut insertion_index = None;
        let mut remaining_nodes = Vec::with_capacity(self.nodes.len());

        for node in std::mem::take(&mut self.nodes) {
            if let Some(root_id) = subtree_root_by_id.get(&node.id).copied() {
                if insertion_index.is_none() {
                    insertion_index = Some(remaining_nodes.len());
                }
                blocks.entry(root_id).or_default().push(node);
            } else {
                remaining_nodes.push(node);
            }
        }

        let insert_at = insertion_index.unwrap_or(remaining_nodes.len());
        let mut reordered_nodes = Vec::new();
        for root_id in reordered_root_ids {
            if let Some(mut block) = blocks.remove(&root_id) {
                reordered_nodes.append(&mut block);
            }
        }
        remaining_nodes.splice(insert_at..insert_at, reordered_nodes);
        self.nodes = remaining_nodes;
        true
    }

    pub fn remove_nodes(&mut self, node_ids: &[u64]) {
        let mut ids_to_remove = HashSet::new();
        for &id in node_ids {
            ids_to_remove.extend(self.subtree_ids(id));
        }
        self.nodes.retain(|node| !ids_to_remove.contains(&node.id));
    }

    pub fn translate_subtree(&mut self, node_id: u64, delta: [f32; 2]) {
        let subtree_ids: HashSet<u64> = self.subtree_ids(node_id).into_iter().collect();

        for node in &mut self.nodes {
            if subtree_ids.contains(&node.id) {
                node.position[0] += delta[0];
                node.position[1] += delta[1];
            }
        }
    }

    pub fn pretty_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|error| format!("{{\n  \"error\": \"{}\"\n}}", error))
    }

    fn duplicate_subtree(
        &mut self,
        source_id: u64,
        duplicated_parent: Option<u64>,
        position_delta: [f32; 2],
        source_by_id: &HashMap<u64, SceneNode>,
        children_by_parent: &HashMap<u64, Vec<u64>>,
    ) -> Option<u64> {
        let source = source_by_id.get(&source_id)?.clone();
        let new_id = self.alloc_unique_id();

        let mut duplicate = source;
        duplicate.id = new_id;
        duplicate.parent = duplicated_parent.or(duplicate.parent);
        duplicate.position[0] += position_delta[0];
        duplicate.position[1] += position_delta[1];
        self.nodes.push(duplicate);

        if let Some(child_ids) = children_by_parent.get(&source_id) {
            for child_id in child_ids {
                self.duplicate_subtree(
                    *child_id,
                    Some(new_id),
                    position_delta,
                    source_by_id,
                    children_by_parent,
                );
            }
        }

        Some(new_id)
    }

    fn can_reparent_node(&self, node_id: u64, new_parent: Option<u64>) -> bool {
        let Some(node) = self.node(node_id) else {
            return false;
        };

        if node.parent == new_parent {
            return false;
        }

        if let Some(parent_id) = new_parent {
            if parent_id == node_id || self.is_descendant_of(parent_id, node_id) {
                return false;
            }
        }

        true
    }

    fn extract_nodes_by_ids(&mut self, node_ids: &HashSet<u64>) -> Vec<SceneNode> {
        let mut extracted = Vec::new();
        let mut remaining = Vec::with_capacity(self.nodes.len());

        for node in std::mem::take(&mut self.nodes) {
            if node_ids.contains(&node.id) {
                extracted.push(node);
            } else {
                remaining.push(node);
            }
        }

        self.nodes = remaining;
        extracted
    }
}

/// Generate a pseudo-random non-zero u64 for use as a node id.
///
/// Uses a thread-local xorshift64 generator seeded from the system clock on
/// first call. The 64-bit space makes cross-scene collisions astronomically
/// unlikely without any global coordination.
fn new_node_id() -> u64 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }
    STATE.with(|s| {
        let mut v = s.get();
        if v == 0 {
            v = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0xcafe_babe_dead_beef)
                | 1; // xorshift64 must not be seeded with 0
        }
        v ^= v << 13;
        v ^= v >> 7;
        v ^= v << 17;
        s.set(v);
        v
    })
}

fn default_scene_window_size() -> [f32; 2] {
    [960.0, 720.0]
}

fn default_camera_zoom() -> f32 {
    1.0
}

fn default_camera_show_bounds() -> bool {
    true
}

fn default_camera_use_scene_view_size() -> bool {
    true
}

pub trait SceneNodeCameraExt {
    fn camera_zoom(&self) -> f32;
    fn set_camera_zoom(&mut self, value: f32);
    fn camera_show_bounds(&self) -> bool;
    fn set_camera_show_bounds(&mut self, value: bool);
    fn camera_use_scene_view_size(&self) -> bool;
    fn set_camera_use_scene_view_size(&mut self, value: bool);
    fn camera_view_size(&self) -> [f32; 2];
    fn set_camera_view_size(&mut self, value: [f32; 2]);
}

impl SceneNodeCameraExt for SceneNode {
    fn camera_zoom(&self) -> f32 {
        self.properties
            .get("camera_zoom")
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(default_camera_zoom)
    }

    fn set_camera_zoom(&mut self, value: f32) {
        self.properties
            .insert("camera_zoom".to_string(), value.to_string());
    }

    fn camera_show_bounds(&self) -> bool {
        self.properties
            .get("camera_show_bounds")
            .map(|value| matches!(value.trim(), "true" | "1" | "yes"))
            .unwrap_or_else(default_camera_show_bounds)
    }

    fn set_camera_show_bounds(&mut self, value: bool) {
        self.properties
            .insert("camera_show_bounds".to_string(), value.to_string());
    }

    fn camera_use_scene_view_size(&self) -> bool {
        self.properties
            .get("camera_use_scene_view_size")
            .map(|value| matches!(value.trim(), "true" | "1" | "yes"))
            .unwrap_or_else(default_camera_use_scene_view_size)
    }

    fn set_camera_use_scene_view_size(&mut self, value: bool) {
        self.properties.insert(
            "camera_use_scene_view_size".to_string(),
            value.to_string(),
        );
    }

    fn camera_view_size(&self) -> [f32; 2] {
        let default = default_scene_window_size();
        let w = self
            .properties
            .get("camera_view_w")
            .and_then(|value| value.parse().ok())
            .unwrap_or(default[0]);
        let h = self
            .properties
            .get("camera_view_h")
            .and_then(|value| value.parse().ok())
            .unwrap_or(default[1]);
        [w, h]
    }

    fn set_camera_view_size(&mut self, value: [f32; 2]) {
        self.properties
            .insert("camera_view_w".to_string(), value[0].to_string());
        self.properties
            .insert("camera_view_h".to_string(), value[1].to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node(id: u64) -> SceneNode {
        new_scene_node(id, SceneNodeKind::Empty, None, 0)
    }

    #[test]
    fn a_real_pre_unification_scene_file_still_opens() {
        // scratch/scene-prototype.scene.json predates this change and still
        // has the old per-node `sprite`/`camera2d` JSON objects. Unknown
        // fields must be ignored (not rejected) so existing authored files
        // keep opening.
        let json = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scratch/scene-prototype.scene.json"),
        )
        .expect("read sample scene file");
        let doc: SceneDocument = serde_json::from_str(&json).expect("parse sample scene file");
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn editor_document_validates_with_the_engine_scene_validator() {
        // A serialized editor document is consumable by rengine's scene
        // validator (format compatibility), and authoring issues surface.
        let mut doc = SceneDocument::new("validate_me");
        let group = doc.add_node(SceneNodeKind::Group, None);
        let sprite = doc.add_node(SceneNodeKind::Sprite, Some(group));

        // A freshly added sprite has no asset source yet — it should be flagged.
        let value = serde_json::to_value(&doc).expect("serialize scene document");
        let report = rengine::validate_editor_scene(&value, None, None);
        assert!(
            report
                .issues()
                .iter()
                .any(|issue| issue.node_id == Some(sprite) && issue.message.contains("source")),
            "sourceless sprite should be reported: {:?}",
            report.issues()
        );

        // Giving the sprite an asset alias clears the source warning.
        doc.node_mut(sprite).unwrap().asset_alias = "player_idle".to_string();
        let value = serde_json::to_value(&doc).expect("serialize scene document");
        let report = rengine::validate_editor_scene(&value, None, None);
        assert!(
            !report
                .issues()
                .iter()
                .any(|issue| issue.message.contains("source")),
            "source warning should clear once an asset is set: {:?}",
            report.issues()
        );
    }

    #[test]
    fn arbitrary_ui_properties_survive_a_save_reload_round_trip() {
        // Ed1's whole point: a node.properties entry with no dedicated widget
        // (ui_color, ui_anchor, ...) is not special-cased anywhere in the
        // document model — it's an ordinary HashMap<String,String> entry, so
        // it must serialize/deserialize like any other authored value with no
        // editor-side logic to keep in sync.
        let mut doc = SceneDocument::new("ui_roundtrip");
        let node_id = doc.add_node(SceneNodeKind::UiRoot, None);
        {
            let node = doc.node_mut(node_id).unwrap();
            node.properties.insert("ui".to_string(), "rect".to_string());
            node.properties
                .insert("ui_color".to_string(), "228,44,32,255".to_string());
            node.properties
                .insert("ui_anchor".to_string(), "bottom".to_string());
            // A script param living alongside the free-form properties must
            // not be disturbed by anything Ed1 does to the rest of the map.
            node.properties
                .insert("param_speed".to_string(), "5".to_string());
        }

        let json = serde_json::to_string_pretty(&doc).expect("serialize scene document");
        let reloaded: SceneDocument = serde_json::from_str(&json).expect("reload scene document");

        let node = reloaded.node(node_id).expect("node survives reload");
        assert_eq!(node.properties.get("ui").map(String::as_str), Some("rect"));
        assert_eq!(
            node.properties.get("ui_color").map(String::as_str),
            Some("228,44,32,255")
        );
        assert_eq!(
            node.properties.get("ui_anchor").map(String::as_str),
            Some("bottom")
        );
        assert_eq!(
            node.properties.get("param_speed").map(String::as_str),
            Some("5")
        );
    }

    fn test_node_with_parent(id: u64, parent: Option<u64>) -> SceneNode {
        let mut node = new_scene_node(id, SceneNodeKind::Empty, parent, 0);
        node.name = format!("Node {id}");
        node.position = [id as f32 * 10.0, id as f32 * 20.0];
        node
    }

    #[test]
    fn normalize_next_id_raises_loaded_counter_above_existing_ids() {
        let mut document = SceneDocument {
            nodes: vec![test_node(4), test_node(9)],
            next_id: 3,
            ..SceneDocument::new("test")
        };

        assert!(document.normalize_next_id());
        assert_eq!(document.next_id, 10);
    }

    #[test]
    fn normalize_next_id_preserves_valid_future_counter() {
        let mut document = SceneDocument {
            nodes: vec![test_node(4), test_node(9)],
            next_id: 25,
            ..SceneDocument::new("test")
        };

        assert!(!document.normalize_next_id());
        assert_eq!(document.next_id, 25);
    }

    #[test]
    fn duplicate_nodes_prunes_descendants_and_preserves_subtree_structure() {
        let mut document = SceneDocument {
            nodes: vec![
                test_node_with_parent(1, None),
                test_node_with_parent(2, Some(1)),
                test_node_with_parent(3, None),
            ],
            ..SceneDocument::new("test")
        };

        let duplicated_root_ids = document.duplicate_nodes(&[1, 2], [5.0, 7.0]);

        // Selection of [1, 2] prunes 2 (descendant of 1), so only one root is duplicated.
        assert_eq!(duplicated_root_ids.len(), 1);
        assert_eq!(document.nodes.len(), 5);

        let dup_root_id = duplicated_root_ids[0];
        let dup_root = document.node(dup_root_id).expect("duplicated root exists");
        assert_eq!(dup_root.parent, None);
        assert_eq!(dup_root.position, [15.0, 27.0]); // node 1 pos + delta

        let dup_child = document
            .nodes
            .iter()
            .find(|n| n.parent == Some(dup_root_id))
            .expect("duplicated child exists");
        assert_eq!(dup_child.position, [25.0, 47.0]); // node 2 pos + delta
    }

    #[test]
    fn duplicate_nodes_returns_empty_for_empty_selection() {
        let mut document = SceneDocument {
            nodes: vec![test_node_with_parent(1, None)],
            ..SceneDocument::new("test")
        };

        assert!(document.duplicate_nodes(&[], [0.0, 0.0]).is_empty());
        assert_eq!(document.nodes.len(), 1);
    }

    #[test]
    fn reparent_nodes_rejects_cycles() {
        let original_nodes = vec![
            test_node_with_parent(1, None),
            test_node_with_parent(2, Some(1)),
            test_node_with_parent(3, Some(2)),
        ];
        let mut document = SceneDocument {
            nodes: original_nodes.clone(),
            ..SceneDocument::new("test")
        };

        assert!(!document.reparent_nodes(&[1], Some(3)));
        assert_eq!(document.nodes, original_nodes);
    }

    #[test]
    fn reorder_nodes_moves_selection_up_and_down() {
        let mut document = SceneDocument {
            nodes: vec![
                test_node_with_parent(1, None),
                test_node_with_parent(2, None),
                test_node_with_parent(3, None),
            ],
            ..SceneDocument::new("test")
        };

        assert!(document.reorder_nodes(&[2], SceneNodeReorderDirection::Up));
        assert_eq!(document.root_ids(), vec![2, 1, 3]);

        assert!(document.reorder_nodes(&[2], SceneNodeReorderDirection::Down));
        assert_eq!(document.root_ids(), vec![1, 2, 3]);
    }
}
