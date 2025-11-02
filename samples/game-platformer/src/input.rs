use rengine::Engine;
use winit::keyboard::KeyCode;

use crate::state::Player;
use crate::{JUMP_SPEED, MOVE_SPEED};


pub fn handle_input(player: &mut Player, engine: &Engine) {
    let input = engine.input();


    let mut move_dir = 0.0f32;
    if input.is_key_down(KeyCode::ArrowLeft) || input.is_key_down(KeyCode::KeyA) {
        move_dir -= 1.0;
    }
    if input.is_key_down(KeyCode::ArrowRight) || input.is_key_down(KeyCode::KeyD) {
        move_dir += 1.0;
    }
    player.vel.x = move_dir * MOVE_SPEED;
    if move_dir != 0.0 {
        player.facing_right = move_dir > 0.0;
    }


    let jump_pressed = input.is_key_pressed(KeyCode::Space)
        || input.is_key_pressed(KeyCode::ArrowUp)
        || input.is_key_pressed(KeyCode::KeyW);
    if player.on_ground && jump_pressed {
        player.vel.y = JUMP_SPEED;
        player.on_ground = false;
    }
}
