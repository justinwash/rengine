mod input;
mod physics;
mod render;
mod state;

use rengine::*;
use state::{Platform, Platformer, Player};

const GRAVITY: f32 = -980.0;
const MOVE_SPEED: f32 = 220.0;
const JUMP_SPEED: f32 = 420.0;
const PLAYER_W: f32 = 28.0;
const PLAYER_H: f32 = 44.0;

impl Game for Platformer {
    fn new(engine: &mut Engine) -> Self {
        let player_tex = engine.create_color_texture(1, 1, Color::from_rgba8(60, 130, 230, 255));
        let eye_tex = engine.create_color_texture(1, 1, Color::WHITE);
        let ground_tex = engine.create_color_texture(1, 1, Color::from_rgba8(72, 140, 54, 255));
        let plat_tex = engine.create_color_texture(1, 1, Color::from_rgba8(139, 90, 43, 255));
        let plat_tex2 = engine.create_color_texture(1, 1, Color::from_rgba8(160, 110, 60, 255));

        let platforms = vec![
            Platform {
                pos: Vec2::new(-400.0, 0.0),
                size: Vec2::new(2000.0, 40.0),
                texture: ground_tex,
            },
            Platform {
                pos: Vec2::new(120.0, 100.0),
                size: Vec2::new(140.0, 18.0),
                texture: plat_tex,
            },
            Platform {
                pos: Vec2::new(320.0, 170.0),
                size: Vec2::new(120.0, 18.0),
                texture: plat_tex2,
            },
            Platform {
                pos: Vec2::new(500.0, 250.0),
                size: Vec2::new(160.0, 18.0),
                texture: plat_tex,
            },
            Platform {
                pos: Vec2::new(350.0, 340.0),
                size: Vec2::new(180.0, 18.0),
                texture: plat_tex2,
            },
            Platform {
                pos: Vec2::new(100.0, 420.0),
                size: Vec2::new(200.0, 18.0),
                texture: plat_tex,
            },
            Platform {
                pos: Vec2::new(-150.0, 160.0),
                size: Vec2::new(100.0, 18.0),
                texture: plat_tex2,
            },
            Platform {
                pos: Vec2::new(600.0, 400.0),
                size: Vec2::new(120.0, 18.0),
                texture: plat_tex,
            },
        ];

        let player = Player {
            pos: Vec2::new(80.0, 200.0),
            vel: Vec2::ZERO,
            on_ground: false,
            facing_right: true,
            texture: player_tex,
            eye_tex,
        };

        Self {
            player,
            platforms,
            bg_color: Color::rgb(0.529, 0.808, 0.922),
        }
    }

    fn update(&mut self, engine: &Engine) {
        let dt = engine.dt();
        input::handle_input(&mut self.player, engine);
        physics::update(&mut self.player, &self.platforms, dt);
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        render::draw(self, engine, frame);
    }
}

fn main() {
    rengine::run::<Platformer>(EngineConfig {
        title: "Rengine Platformer".into(),
        width: 800,
        height: 600,
        ..Default::default()
    })
    .unwrap();
}
