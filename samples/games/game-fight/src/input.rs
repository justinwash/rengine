use rengine::Engine;

use crate::state::FightInput;

pub fn sample_from_engine(engine: &Engine, player: usize) -> FightInput {
    let prefix = if player == 0 { "p1" } else { "p2" };
    let mut flags = 0u8;

    let move_x = engine.axis_player(&format!("{prefix}_move_x"), player);
    let move_y = engine.axis_player(&format!("{prefix}_move_y"), player);

    if move_x < -0.5 {
        flags |= FightInput::LEFT;
    }
    if move_x > 0.5 {
        flags |= FightInput::RIGHT;
    }
    if engine.action_pressed_player(&format!("{prefix}_jump"), player) || move_y > 0.7 {
        flags |= FightInput::JUMP;
    }
    if engine.action_down_player(&format!("{prefix}_crouch"), player) || move_y < -0.7 {
        flags |= FightInput::CROUCH;
    }
    if engine.action_pressed_player(&format!("{prefix}_punch"), player) {
        flags |= FightInput::PUNCH;
    }
    if engine.action_pressed_player(&format!("{prefix}_kick"), player) {
        flags |= FightInput::KICK;
    }

    FightInput {
        flags,
        _pad: [0; 3],
    }
}
