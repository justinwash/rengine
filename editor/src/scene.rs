use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpriteNodeSettings {
    #[serde(default)]
    pub texture_path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera2dNodeSettings {
    #[serde(default = "default_camera_zoom")]
    pub zoom: f32,
    #[serde(default = "default_camera_show_bounds")]
    pub show_bounds: bool,
    #[serde(default = "default_camera_use_scene_view_size")]
    pub use_scene_view_size: bool,
    #[serde(default = "default_scene_window_size")]
    pub view_size: [f32; 2],
}

impl Default for Camera2dNodeSettings {
    fn default() -> Self {
        Self {
            zoom: default_camera_zoom(),
            show_bounds: default_camera_show_bounds(),
            use_scene_view_size: default_camera_use_scene_view_size(),
            view_size: default_scene_window_size(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneNodeKind {
    Group,
    Empty,
    Camera2d,
    Sprite,
    Trigger,
    UiRoot,
}

impl SceneNodeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Group => "Group",
            Self::Empty => "Empty",
            Self::Camera2d => "Camera2D",
            Self::Sprite => "Sprite",
            Self::Trigger => "Trigger",
            Self::UiRoot => "UI Root",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Group => "GRP",
            Self::Empty => "EMP",
            Self::Camera2d => "CAM",
            Self::Sprite => "SPR",
            Self::Trigger => "TRG",
            Self::UiRoot => "UI",
        }
    }

    pub fn default_size(self) -> [f32; 2] {
        match self {
            Self::Group => [120.0, 72.0],
            Self::Empty => [88.0, 56.0],
            Self::Camera2d => [96.0, 68.0],
            Self::Sprite => [112.0, 72.0],
            Self::Trigger => [148.0, 88.0],
            Self::UiRoot => [220.0, 136.0],
        }
    }

    pub fn default_name(self, id: u64) -> String {
        format!("{} {}", self.label(), id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneNodeReorderDirection {
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: u64,
    pub parent: Option<u64>,
    pub name: String,
    pub kind: SceneNodeKind,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub visible: bool,
    pub script_path: String,
    #[serde(default)]
    pub runtime_prefab: String,
    #[serde(default)]
    pub asset_alias: String,
    #[serde(default)]
    pub sprite: SpriteNodeSettings,
    #[serde(default)]
    pub camera2d: Camera2dNodeSettings,
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

impl SceneNode {
    fn new(id: u64, kind: SceneNodeKind, parent: Option<u64>, sibling_index: usize) -> Self {
        let offset = sibling_index as f32 * 28.0;
        let position = if parent.is_some() {
            [56.0 + offset, 48.0 + offset]
        } else {
            [-72.0 + offset, -48.0 + offset]
        };

        Self {
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
            sprite: SpriteNodeSettings::default(),
            camera2d: Camera2dNodeSettings::default(),
            properties: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneDocument {
    pub name: String,
    #[serde(default)]
    pub view: SceneViewSettings,
    pub nodes: Vec<SceneNode>,
    pub next_id: u64,
}

impl SceneDocument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            view: SceneViewSettings::default(),
            nodes: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_node(&mut self, kind: SceneNodeKind, parent: Option<u64>) -> u64 {
        let sibling_index = self
            .nodes
            .iter()
            .filter(|node| node.parent == parent)
            .count();
        let id = self.next_id;
        self.next_id += 1;
        self.nodes
            .push(SceneNode::new(id, kind, parent, sibling_index));
        id
    }

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
        let selected_positions: Vec<usize> = sibling_root_ids
            .iter()
            .enumerate()
            .filter_map(|(index, sibling_id)| root_ids.contains(sibling_id).then_some(index))
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

        let selected_root_set: HashSet<u64> = root_ids.iter().copied().collect();
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
        let new_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node(id: u64) -> SceneNode {
        SceneNode::new(id, SceneNodeKind::Empty, None, 0)
    }

    #[test]
    fn normalize_next_id_raises_loaded_counter_above_existing_ids() {
        let mut document = SceneDocument {
            name: "test".to_string(),
            view: SceneViewSettings::default(),
            nodes: vec![test_node(4), test_node(9)],
            next_id: 3,
        };

        assert!(document.normalize_next_id());
        assert_eq!(document.next_id, 10);
    }

    #[test]
    fn normalize_next_id_preserves_valid_future_counter() {
        let mut document = SceneDocument {
            name: "test".to_string(),
            view: SceneViewSettings::default(),
            nodes: vec![test_node(4), test_node(9)],
            next_id: 25,
        };

        assert!(!document.normalize_next_id());
        assert_eq!(document.next_id, 25);
    }
}
