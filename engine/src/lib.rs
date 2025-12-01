pub mod app;
pub mod renderer;
pub mod assets;
pub mod input;
pub mod math;
pub mod world;
pub mod hud;

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

pub use hud::HudVertex;

pub use glam::{Vec2, Vec3};
pub use winit::keyboard::KeyCode;
