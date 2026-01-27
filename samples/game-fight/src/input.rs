use rengine::{Engine, KeyCode};

use crate::bot;
use crate::state::{FightGame, FightInput};


pub fn sample(game: &FightGame, engine: &Engine, player: usize) -> FightInput {

    if game.demo_mode {
        let (me, opp) = if player == 0 {
            (&game.sim.p1, &game.sim.p2)
        } else {
            (&game.sim.p2, &game.sim.p1)
        };
        return bot::bot_input(me, opp, game.demo_frame, player as u32);
    }

    let kb = engine.input();
    let gp = engine.gamepad(player);
    let mut flags = 0u8;

    match player {
        0 => {
            if kb.is_key_down(KeyCode::KeyA) || gp.left_stick_x < -0.5 {
                flags |= FightInput::LEFT;
            }
            if kb.is_key_down(KeyCode::KeyD) || gp.left_stick_x > 0.5 {
                flags |= FightInput::RIGHT;
            }
            if kb.is_key_pressed(KeyCode::KeyW)
                || gp.is_button_pressed(rengine::GamepadButton::DPadUp)
                || gp.left_stick_y > 0.7
            {
                flags |= FightInput::JUMP;
            }
            if kb.is_key_down(KeyCode::KeyS)
                || gp.is_button_down(rengine::GamepadButton::DPadDown)
                || gp.left_stick_y < -0.7
            {
                flags |= FightInput::CROUCH;
            }
            if kb.is_key_pressed(KeyCode::KeyF)
                || gp.is_button_pressed(rengine::GamepadButton::South)
            {
                flags |= FightInput::PUNCH;
            }
            if kb.is_key_pressed(KeyCode::KeyG)
                || gp.is_button_pressed(rengine::GamepadButton::West)
            {
                flags |= FightInput::KICK;
            }
        }
        1 => {
            if kb.is_key_down(KeyCode::ArrowLeft) || gp.left_stick_x < -0.5 {
                flags |= FightInput::LEFT;
            }
            if kb.is_key_down(KeyCode::ArrowRight) || gp.left_stick_x > 0.5 {
                flags |= FightInput::RIGHT;
            }
            if kb.is_key_pressed(KeyCode::ArrowUp)
                || gp.is_button_pressed(rengine::GamepadButton::DPadUp)
                || gp.left_stick_y > 0.7
            {
                flags |= FightInput::JUMP;
            }
            if kb.is_key_down(KeyCode::ArrowDown)
                || gp.is_button_down(rengine::GamepadButton::DPadDown)
                || gp.left_stick_y < -0.7
            {
                flags |= FightInput::CROUCH;
            }
            if kb.is_key_pressed(KeyCode::KeyK)
                || gp.is_button_pressed(rengine::GamepadButton::South)
            {
                flags |= FightInput::PUNCH;
            }
            if kb.is_key_pressed(KeyCode::KeyL)
                || gp.is_button_pressed(rengine::GamepadButton::West)
            {
                flags |= FightInput::KICK;
            }
        }
        _ => {}
    }

    FightInput {
        flags,
        _pad: [0; 3],
    }
}
