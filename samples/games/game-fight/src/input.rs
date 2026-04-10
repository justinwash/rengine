use rengine::Engine;

use crate::state::FightInput;

pub fn sample_from_engine(engine: &Engine, player: usize) -> FightInput {
    let (ax, ay, jump, crouch, punch, kick) = match player {
        0 => (
            "p1_move_x",
            "p1_move_y",
            "p1_jump",
            "p1_crouch",
            "p1_punch",
            "p1_kick",
        ),
        _ => (
            "p2_move_x",
            "p2_move_y",
            "p2_jump",
            "p2_crouch",
            "p2_punch",
            "p2_kick",
        ),
    };
    let mut flags = 0u8;

    let move_x = engine.axis_player(ax, player);
    let move_y = engine.axis_player(ay, player);

    if move_x < -0.5 {
        flags |= FightInput::LEFT;
    }
    if move_x > 0.5 {
        flags |= FightInput::RIGHT;
    }
    if engine.action_pressed_player(jump, player) || move_y > 0.7 {
        flags |= FightInput::JUMP;
    }
    if engine.action_down_player(crouch, player) || move_y < -0.7 {
        flags |= FightInput::CROUCH;
    }
    if engine.action_pressed_player(punch, player) {
        flags |= FightInput::PUNCH;
    }
    if engine.action_pressed_player(kick, player) {
        flags |= FightInput::KICK;
    }

    FightInput {
        flags,
        _pad: [0; 3],
    }
}
