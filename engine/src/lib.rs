pub mod app;
pub mod math;

pub mod input;

pub mod canvas;
pub mod renderer;
pub mod renderer3d;
pub mod scene;
pub mod text;

pub mod world;

pub mod assets;

#[cfg(feature = "rollback")]
pub mod netcode;

pub use app::run;
pub use app::run_with_scenes;
pub use app::{Engine, EngineConfig, Game};
pub use assets::Color;
pub use input::InputState;
pub use input::{ActionMap, AxisMapping, Binding, GamepadAxis};
pub use math::Rect;
pub use math::Rng;
pub use math::TimeState;
pub use renderer::{Camera2D, CameraBounds, DrawParams, Frame, TextureId};

pub use world::tilemap;
pub use world::{
    aabb_overlap, aabb_overlap_layered, iso_to_screen, screen_to_iso, BodyId, CollisionLayer,
    OverlapEvent, TileDef, TileMap, TriggerSystem, TriggerZone, TriggerZoneId,
};

pub use assets::pixelart;
pub use assets::{
    Animation, AssetError, AssetManifest, AssetPack, AssetSummary, AudioBus, AudioClip, AudioId,
    MeshAsset, SpriteSheet, SpriteSheetAssetDef, TextureAsset,
};

pub use canvas::{screen_to_ndc, Canvas, CanvasVertex};
pub use scene::{
    Globals, Prefab2D, Prefab2DDef, PrefabSprite2D, PrefabSprite2DDef, Scene, Scene2D, Scene2DDef,
    SceneInstance2D, SceneInstance2DDef, SceneOp,
};
pub use text::FontAtlas;

pub use gilrs::Button as GamepadButton;
pub use input::{GamepadState, GamepadSystem};

#[cfg(feature = "rollback")]
pub use netcode::OnlineConfig;
#[cfg(feature = "rollback")]
pub use netcode::{fletcher64, RollbackConfig, RollbackSession, Rollbackable, SessionMode};

pub use app::{run3d, run3d_with_scenes, Engine3D, Game3D};
pub use renderer3d::{cube_mesh, floor_quad, wall_quad};
pub use renderer3d::{Camera3D, DrawCmd3D, Frame3D, MeshId, Vertex3D, Viewmodel3D};
pub use scene::{Scene3D, SceneOp3D};

pub use glam::{Quat, Vec2, Vec3};

pub use winit::keyboard::KeyCode;
