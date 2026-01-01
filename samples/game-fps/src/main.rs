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


impl Game3D for FpsGame {
    fn new(engine: &mut Engine3D) -> Self {
        level::build(engine)
    }

    fn update(&mut self, engine: &Engine3D) {
        let dt = engine.dt();


        let (yaw, pitch) = input::mouse_look(engine, self.cam_yaw, self.cam_pitch);
        self.cam_yaw = yaw;
        self.cam_pitch = pitch;


        let dir = input::move_dir(engine, self.cam_yaw);
        physics::move_player(self, dir, dt);


        physics::apply_gravity(self, input::jump_pressed(engine), dt);


        physics::update_doors(self, dt);


        if input::shoot_pressed(engine) {
            physics::shoot(self);
        }
        physics::update_projectiles(self, dt);
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
