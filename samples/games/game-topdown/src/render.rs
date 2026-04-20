use rengine::{Color, DrawParams, Frame, TextAlign, Vec2};

use crate::state::TopDown;
use crate::PLAYER_SIZE;

pub fn draw(game: &TopDown, frame: &mut Frame) {
    frame.clear_color = Color::from_rgba8(20, 20, 20, 255);

    frame.camera.position = game.player.pos + Vec2::splat(PLAYER_SIZE / 2.0);

    game.tilemap.draw(frame);

    game.scene.draw(frame);

    for gem in &game.gems {
        if !gem.collected {
            frame
                .draw_sprite(DrawParams::new(gem.tex, gem.pos, Vec2::splat(24.0)).with_z_order(10));
        }
    }

    for enemy in &game.enemies {
        frame.draw_sprite(
            DrawParams::new(enemy.tex, enemy.pos, Vec2::splat(PLAYER_SIZE)).with_z_order(15),
        );
    }

    frame.draw_sprite(
        DrawParams::new(game.player.tex, game.player.pos, Vec2::splat(PLAYER_SIZE))
            .with_z_order(25),
    );

    let hud = frame.canvas(0);
    let (sw, sh) = hud.screen_size();
    let hw = sw as f32 / 2.0;
    let hh = sh as f32 / 2.0;
    hud.rect(
        -hw,
        hh - 40.0,
        sw as f32,
        40.0,
        Color::from_rgba8(10, 12, 18, 220),
    );
    hud.text_block(
        0.0,
        hh - 10.0,
        "WASD / Arrows move. Collect every gem and avoid enemies.",
        12.0,
        Color::WHITE,
        420.0,
        TextAlign::Center,
    );
}
