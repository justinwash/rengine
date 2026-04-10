mod input;
mod physics;
mod render;
mod state;

use std::path::PathBuf;

use rengine::*;
use state::{Platform, Platformer, Player};

const GRAVITY: f32 = -980.0;
const MOVE_SPEED: f32 = 220.0;
const JUMP_SPEED: f32 = 420.0;
const PLAYER_W: f32 = 28.0;
const PLAYER_H: f32 = 44.0;

impl Game for Platformer {
    fn new(engine: &mut Engine) -> Self {
        engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

        let actions = engine.actions_mut();
        actions.bind_axis(
            "move_x",
            AxisMapping {
                positive: vec![
                    Binding::Key(KeyCode::KeyD),
                    Binding::Key(KeyCode::ArrowRight),
                ],
                negative: vec![
                    Binding::Key(KeyCode::KeyA),
                    Binding::Key(KeyCode::ArrowLeft),
                ],
                gamepad_axis: Some(GamepadAxis::LeftStickX),
            },
        );
        actions.bind("jump", Binding::Key(KeyCode::Space));
        actions.bind("jump", Binding::Key(KeyCode::ArrowUp));
        actions.bind("jump", Binding::Key(KeyCode::KeyW));
        actions.bind("jump", Binding::GamepadButton(GamepadButton::South));

        let player_tex = engine
            .load_texture("player.ppm")
            .expect("failed to load platformer player texture")
            .texture();
        let eye_tex = engine
            .load_texture("eye.ppm")
            .expect("failed to load platformer eye texture")
            .texture();
        let ground_tex = engine
            .load_texture("ground.ppm")
            .expect("failed to load platformer ground texture")
            .texture();
        let plat_tex = engine
            .load_texture("platform.ppm")
            .expect("failed to load platformer platform texture")
            .texture();
        let plat_tex2 = engine
            .load_texture("platform_alt.ppm")
            .expect("failed to load platformer alternate platform texture")
            .texture();

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
