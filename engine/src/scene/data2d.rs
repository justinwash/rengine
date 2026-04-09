use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::{AssetError, AssetPack, Color};
use crate::renderer::{DrawParams, Frame};
use crate::{TextureId, Vec2};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab2DDef {
    pub name: String,
    pub sprites: Vec<PrefabSprite2DDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInstance2DDef {
    pub prefab: String,
    pub position: [f32; 2],
    #[serde(default = "default_scale")]
    pub scale: [f32; 2],
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
        let definition: Scene2DDef =
            serde_json::from_str(&text).map_err(|source| AssetError::Json {
                path: path.to_path_buf(),
                source,
            })?;
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
