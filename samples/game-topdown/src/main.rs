mod art;
mod input;
mod level;
mod physics;
mod render;
mod state;

use rengine::*;
use state::TopDown;


pub const TILE_SIZE: f32 = 32.0;
pub const MAP_W: usize = 30;
pub const MAP_H: usize = 20;
pub const PLAYER_SPEED: f32 = 150.0;
pub const PLAYER_SIZE: f32 = 28.0;
pub const ENEMY_SPEED: f32 = 60.0;


impl Game for TopDown {
    fn new(engine: &mut Engine) -> Self {
        level::build(engine)
    }

    fn update(&mut self, engine: &Engine) {
        let dt = engine.dt();
        let dir = input::movement_dir(engine);

        physics::move_player(self, dir, dt);
        physics::collect_gems(self);
        physics::update_enemies(&mut self.enemies, &self.tilemap, dt);
    }

    fn render(&mut self, _engine: &Engine, frame: &mut Frame) {
        render::draw(self, frame);
    }
}


fn main() {
    rengine::run::<TopDown>(EngineConfig {
        title: "Rengine Top-Down Adventure".into(),
        width: 800,
        height: 600,
        ..Default::default()
    })
    .unwrap();
}
