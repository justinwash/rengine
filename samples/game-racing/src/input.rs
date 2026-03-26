use rengine::{Engine, KeyCode, Vec2};

use crate::state::RacingGame;

pub fn handle_input(game: &mut RacingGame, engine: &Engine) {
    let input = engine.input();

    // Camera follow: number keys 1-9 select car to follow (and 0 for car 10)
    for (key, idx) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
        (KeyCode::Digit0, 9),
    ] {
        if input.is_key_pressed(key) && idx < game.cars.len() {
            game.camera_target = idx;
        }
    }

    // Tab cycles to next car
    if input.is_key_pressed(KeyCode::Tab) {
        game.camera_target = (game.camera_target + 1) % game.cars.len();
    }

    // Zoom with +/- or scroll-like keys
    if input.is_key_down(KeyCode::Equal) || input.is_key_down(KeyCode::NumpadAdd) {
        game.camera_zoom = (game.camera_zoom * 1.02).min(4.0);
    }
    if input.is_key_down(KeyCode::Minus) || input.is_key_down(KeyCode::NumpadSubtract) {
        game.camera_zoom = (game.camera_zoom * 0.98).max(0.3);
    }

    // Pan with arrow keys
    let pan_speed = 200.0 / game.camera_zoom;
    let dt = engine.dt();
    if input.is_key_down(KeyCode::ArrowLeft) {
        game.camera_offset.x -= pan_speed * dt;
    }
    if input.is_key_down(KeyCode::ArrowRight) {
        game.camera_offset.x += pan_speed * dt;
    }
    if input.is_key_down(KeyCode::ArrowUp) {
        game.camera_offset.y += pan_speed * dt;
    }
    if input.is_key_down(KeyCode::ArrowDown) {
        game.camera_offset.y -= pan_speed * dt;
    }

    // Reset pan with Home
    if input.is_key_pressed(KeyCode::Home) {
        game.camera_offset = Vec2::ZERO;
    }
}
