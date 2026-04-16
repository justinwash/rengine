mod input;
mod level;
mod physics;
mod render;
mod state;

use rengine::*;
use state::FpsGame;

pub const MOVE_SPEED: f32 = 6.0;
pub const MOUSE_SENSITIVITY: f32 = 0.002;
pub const WALL_HEIGHT: f32 = 3.0;
pub const PLAYER_HEIGHT: f32 = 1.7;
pub const PLAYER_RADIUS: f32 = 0.3;
pub const GRAVITY: f32 = 15.0;
pub const JUMP_VEL: f32 = 6.0;
pub const PROJECTILE_SPEED: f32 = 30.0;
pub const PROJECTILE_LIFETIME: f32 = 3.0;
pub const ENEMY_SIZE: f32 = 0.8;
pub const DOOR_OPEN_SPEED: f32 = 3.0;
pub const WORLD_FOV_DEG: f32 = 70.0;
pub const VIEWMODEL_FOV_DEG: f32 = 52.0;

impl Game3D for FpsGame {
    fn new(engine: &mut Engine3D) -> Self {
        let game = level::build(engine);
        engine.set_master_volume(0.85);
        engine.set_audio_bus_volume(AudioBus::Music, 0.3);
        engine.set_audio_bus_volume(AudioBus::Effects, 0.95);
        if let Ok(music) = engine.load_audio("ambient_loop.wav") {
            let _ = engine.play_music_with_volume(&music, 1.0);
        }
        game
    }

    fn update(&mut self, engine: &Engine3D, _frame: &mut Frame3D) {
        let dt = engine.dt();
        let jump_pressed = input::jump_pressed(engine);
        let shoot_pressed = input::shoot_pressed(engine);
        let was_on_ground = self.on_ground;
        let score_before = self.score;

        let (yaw, pitch) = input::mouse_look(engine, self.cam_yaw, self.cam_pitch);
        self.cam_yaw = yaw;
        self.cam_pitch = pitch;

        let dir = input::move_dir(engine, self.cam_yaw);
        physics::move_player(self, dir, dt);

        physics::apply_gravity(self, jump_pressed, dt);

        physics::update_doors(self, dt);

        if shoot_pressed {
            physics::shoot(self);
            let _ = engine.play_sound_on_bus(AudioBus::Effects, &self.shoot_sfx, 0.95);
        }
        physics::update_projectiles(self, dt);

        if jump_pressed && was_on_ground && !self.on_ground {
            let _ = engine.play_sound_on_bus(AudioBus::Effects, &self.jump_sfx, 0.8);
        }
        if self.score > score_before {
            let _ = engine.play_sound_on_bus(AudioBus::Effects, &self.hit_sfx, 0.9);
        }
    }

    fn render(&mut self, engine: &Engine3D, frame: &mut Frame3D) {
        render::draw(self, engine, frame);
    }
}

fn main() {
    rengine::run3d::<FpsGame>(EngineConfig {
        title: "Rengine FPS".into(),
        width: 1024,
        height: 768,
        ..Default::default()
    })
    .unwrap();
}
