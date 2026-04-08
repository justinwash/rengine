use rengine::{Engine, Frame, Vec2};

use crate::state::Platformer;
use crate::{PLAYER_H, PLAYER_W};


pub fn draw(game: &Platformer, engine: &Engine, frame: &mut Frame) {
    frame.clear_color = game.bg_color;

    let (_w, h) = engine.window_size();


    let pcx = game.player.pos.x + PLAYER_W / 2.0;
    let pcy = game.player.pos.y + PLAYER_H / 2.0;
    let cam_y = pcy.max(h as f32 / 2.0);
    frame.camera.position = Vec2::new(pcx, cam_y);


    for plat in &game.platforms {
        frame.draw(plat.texture, plat.pos, plat.size);
    }


    frame.draw(
        game.player.texture,
        game.player.pos,
        Vec2::new(PLAYER_W, PLAYER_H),
    );


    let eye_offset_x = if game.player.facing_right {
        PLAYER_W * 0.55
    } else {
        PLAYER_W * 0.15
    };
    let eye_pos = Vec2::new(
        game.player.pos.x + eye_offset_x,
        game.player.pos.y + PLAYER_H * 0.65,
    );
    frame.draw(game.player.eye_tex, eye_pos, Vec2::new(6.0, 6.0));
}
