use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SpriteNodeSettings {
    #[serde(default)]
    pub texture_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

    pub fn translate_subtree(&mut self, node_id: u64, delta: [f32; 2]) {
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
                stack.extend(child_ids.iter().copied());
            }
        }

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
