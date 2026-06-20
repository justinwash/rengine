pub mod app;
pub mod math;

pub mod input;

pub mod canvas;
pub mod debug;
pub mod particle;
pub mod renderer;
pub mod renderer3d;
pub mod save;
pub mod scene;
pub mod text;
pub mod ui;

pub mod world;

pub mod assets;

#[cfg(feature = "rollback")]
pub mod netcode;

pub use app::run;
pub use app::run_with_scenes;
pub use app::{Engine, EngineConfig, Game, ScaleMode};
pub use assets::Color;
pub use debug::{DebugLogEntry, DebugLogLevel};
pub use input::InputState;
pub use input::{ActionMap, AxisMapping, Binding, GamepadAxis};
pub use math::Rect;
pub use math::Rng;
pub use math::TimeState;
pub use math::{ease, lerp, Easing, LoopMode, Tween};
pub use math::{EventQueue, Timer};
pub use renderer::{
    Camera2D, CameraBounds, DrawParams, Frame, NineSlice, PostEffect, PostFxChain, RenderTarget,
    Sprite, TextureId,
};

pub use world::tilemap;
pub use world::{
    aabb_overlap, aabb_overlap_layered, iso_to_screen, move_and_collide, move_and_collide_solids,
    screen_to_iso, BodyId, CollisionLayer, Contacts2D, KinematicBody2D, MoveResult2D, OverlapEvent,
    Solid2D, TileDef, TileMap, TriggerSystem, TriggerZone, TriggerZoneId,
};

pub use assets::pixelart;
pub use assets::{
    Animation, AnimationState, AnimationStateMachine, AnimationTransition, AssetBundle, AssetError,
    AssetManifest, AssetPack, AssetSummary, AudioBus, AudioClip, AudioId, FontAsset, MeshAsset,
    SpriteSheet, SpriteSheetAssetDef, TextureAsset,
};

pub use canvas::{screen_to_ndc, wrap_text, Canvas, CanvasVertex, TextAlign};
pub use scene::{
    validate_editor_scene, validate_scene_dir, validate_scene_file, Globals, NodeHandle2D,
    Prefab2D, Prefab2DDef, PrefabSprite2D, PrefabSprite2DDef, Scene, Scene2D, Scene2DDef,
    SceneInstance2D, SceneInstance2DDef, SceneIssueSeverity, SceneLibrary, SceneNode2D, SceneOp,
    SceneScript2D, SceneScriptBinding2D, SceneScriptContext2D, SceneScriptEvent2D,
    SceneScriptHost2D, SceneScriptInputEvent2D, SceneScriptRegistry2D, SceneValidationIssue,
    SceneValidationReport, SceneWorld2D, Transform2D, Transition, CURRENT_EDITOR_SCENE_VERSION,
    NESTED_SCENE_PROPERTY,
};
pub use text::FontAtlas;
pub use text::FontId;
pub use ui::{
    TooltipAnimation, TooltipExpandTrigger, TooltipOptions, TooltipPlacement, Ui, UiAnimation,
    UiAnimationOptions, UiContainerAnimation, UiContainerAnimationOptions, UiResponse, UiStyle,
    UiWidgetStyle,
};

pub use particle::{EmitShape, EmitterConfig, ParticleEmitter, RangeF32};

pub use save::{SaveError, SaveSystem};

pub use gilrs::Button as GamepadButton;
pub use input::{GamepadAssignMode, GamepadState, GamepadSystem};

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
