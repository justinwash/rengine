use rengine::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct GameConfig {
    pub gravity: f32,
    pub jump_force: f32,
    pub move_speed: f32,
    pub coin_anim_fps: f32,
}

pub struct TransitionCounter(pub u32);

pub struct PlayerStats {
    pub coins: u32,
    pub best_height: f32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct CheckpointSave {
    pub coins: u32,
    pub best_height: f32,
    pub times_saved: u32,
}

pub struct DemoConfig {
    pub enabled: bool,
    pub max_frames: u32,
    pub frame: u32,
    pub features_hit: Vec<&'static str>,
}

impl DemoConfig {
    pub fn log_feature(&mut self, name: &'static str) {
        if !self.features_hit.contains(&name) {
            self.features_hit.push(name);
            println!("[FEATURE OK] {name}");
        }
    }
}

pub const PLAYER_BODY_ID: BodyId = 0;
