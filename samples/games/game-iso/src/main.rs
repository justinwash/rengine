mod input;
mod level;
mod physics;
mod render;
mod state;

use rengine::*;
use state::IsoGame;

pub const TILE_W: f32 = 64.0;
pub const TILE_H: f32 = 32.0;
pub const MAP_SIZE: i32 = 15;
pub const PLAYER_SPEED: f32 = 120.0;

impl Game for IsoGame {
    fn new(engine: &mut Engine) -> Self {
        let actions = engine.actions_mut();
        actions.bind_axis(
            "move_col",
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
        actions.bind_axis(
            "move_row",
            AxisMapping {
                positive: vec![
                    Binding::Key(KeyCode::KeyS),
                    Binding::Key(KeyCode::ArrowDown),
                ],
                negative: vec![
                    Binding::Key(KeyCode::KeyW),
                    Binding::Key(KeyCode::ArrowUp),
                ],
                gamepad_axis: None,
            },
        );

        level::build(engine)
    }

    fn update(&mut self, engine: &Engine) {
        let dt = engine.dt();
        let (dc, dr) = input::movement_dir(engine);
        physics::move_player(self, dc, dr, dt);
    }

    fn render(&mut self, _engine: &Engine, frame: &mut Frame) {
        render::draw(self, frame);
    }
}

fn main() {
    rengine::run::<IsoGame>(EngineConfig {
        title: "Rengine Isometric Explorer".into(),
        width: 900,
        height: 700,
        ..Default::default()
    })
    .unwrap();
}
