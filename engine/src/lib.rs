pub mod app;
pub mod math;


pub mod input;


pub mod hud;
pub mod renderer;
pub mod renderer3d;


pub mod world;


pub mod assets;


pub mod netcode;


pub use app::run;
pub use app::{Engine, EngineConfig, Game};
pub use assets::Color;
pub use input::InputState;
pub use math::Rect;
pub use math::TimeState;
pub use renderer::{Camera2D, DrawParams, Frame, TextureId};


pub use world::{aabb_overlap, iso_to_screen, screen_to_iso, TileDef, TileMap};
pub use world::tilemap;


pub use assets::pixelart;
pub use assets::{Animation, SpriteSheet};


pub use hud::{push_shape, push_rect, push_text, screen_to_ndc, HudVertex};


pub use input::{GamepadState, GamepadSystem};
pub use gilrs::Button as GamepadButton;


pub use netcode::{run_rollback, RollbackConfig, RollbackGame, SessionMode};


pub use app::{run3d, Engine3D, Game3D};
pub use renderer3d::{cube_mesh, floor_quad, wall_quad};
pub use renderer3d::{Camera3D, DrawCmd3D, Frame3D, MeshId, Vertex3D};


pub use glam::{Vec2, Vec3};


pub use winit::keyboard::KeyCode;
