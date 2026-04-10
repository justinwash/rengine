use rengine::Engine;

use crate::state::Player;
use crate::{JUMP_SPEED, MOVE_SPEED};

pub fn handle_input(player: &mut Player, engine: &Engine) {
    let move_dir = engine.axis("move_x");
    player.vel.x = move_dir * MOVE_SPEED;
    if move_dir != 0.0 {
        player.facing_right = move_dir > 0.0;
    }

    if player.on_ground && engine.action_pressed("jump") {
        player.vel.y = JUMP_SPEED;
        player.on_ground = false;
    }
}
