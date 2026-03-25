pub mod app;
pub mod math;

pub mod input;

pub mod canvas;
pub mod renderer;
pub mod renderer3d;
pub mod text;

pub mod world;

pub mod assets;

#[cfg(feature = "rollback")]
pub mod netcode;

pub use app::run;
pub use app::{Engine, EngineConfig, Game};
pub use assets::Color;
pub use input::InputState;
pub use math::Rect;
pub use math::TimeState;
pub use renderer::{Camera2D, DrawParams, Frame, TextureId};

pub use world::tilemap;
pub use world::{aabb_overlap, iso_to_screen, screen_to_iso, TileDef, TileMap};

pub use assets::pixelart;
pub use assets::{Animation, SpriteSheet};

pub use canvas::{screen_to_ndc, Canvas, CanvasVertex};
pub use text::FontAtlas;

pub use gilrs::Button as GamepadButton;
pub use input::{GamepadState, GamepadSystem};

#[cfg(feature = "rollback")]
pub use netcode::OnlineConfig;
#[cfg(feature = "rollback")]
pub use netcode::{fletcher64, RollbackConfig, RollbackSession, Rollbackable, SessionMode};

pub use app::{run3d, Engine3D, Game3D};
pub use renderer3d::{cube_mesh, floor_quad, wall_quad};
pub use renderer3d::{Camera3D, DrawCmd3D, Frame3D, MeshId, Vertex3D};

pub use glam::{Vec2, Vec3};

pub use winit::keyboard::KeyCode;
