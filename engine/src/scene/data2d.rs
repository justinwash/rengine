use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::{AssetError, AssetPack, Color};
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

    pub fn property_u64(&self, name: &str) -> Option<u64> {
        self.property(name)
            .and_then(|value| value.parse::<u64>().ok())
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

    pub fn draw(&self, frame: &mut Frame) {
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
                })
            })
            .collect()
    }

    pub fn draw(&self, frame: &mut Frame) {
        for instance in &self.instances {
            instance.draw(frame);
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

fn parse_bool_property(value: &str) -> Option<bool> {
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
        assert_eq!(instance.script_path(), Some("scripts/pit_wall.rs"));
    }

    #[test]
    fn scene_script_bindings_collect_only_instances_with_scripts() {
        let mut with_script = HashMap::new();
        with_script.insert("script_path".to_string(), "scripts/title.rs".to_string());
        with_script.insert("editor_node_id".to_string(), "1".to_string());
        with_script.insert("editor_name".to_string(), "title_root".to_string());

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
