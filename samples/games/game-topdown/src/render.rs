use rengine::{Color, Frame, Vec2};

use crate::state::TopDown;
use crate::PLAYER_SIZE;

pub fn draw(game: &TopDown, frame: &mut Frame) {
    frame.clear_color = Color::from_rgba8(20, 20, 20, 255);

    frame.camera.position = game.player.pos + Vec2::splat(PLAYER_SIZE / 2.0);

    game.tilemap.draw(frame);

    game.scene.draw(frame);

    for gem in &game.gems {
        if !gem.collected {
            frame.draw(gem.tex, gem.pos, Vec2::splat(24.0));
        }
    }

    for enemy in &game.enemies {
        frame.draw(enemy.tex, enemy.pos, Vec2::splat(PLAYER_SIZE));
    }

    frame.draw(game.player.tex, game.player.pos, Vec2::splat(PLAYER_SIZE));
}
