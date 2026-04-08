use rengine::{Engine, Vec2};
use winit::keyboard::KeyCode;


pub fn movement_dir(engine: &Engine) -> Vec2 {
    let input = engine.input();
    let mut dir = Vec2::ZERO;

    if input.is_key_down(KeyCode::ArrowLeft) || input.is_key_down(KeyCode::KeyA) {
        dir.x -= 1.0;
    }
    if input.is_key_down(KeyCode::ArrowRight) || input.is_key_down(KeyCode::KeyD) {
        dir.x += 1.0;
    }
    if input.is_key_down(KeyCode::ArrowDown) || input.is_key_down(KeyCode::KeyS) {
        dir.y -= 1.0;
    }
    if input.is_key_down(KeyCode::ArrowUp) || input.is_key_down(KeyCode::KeyW) {
        dir.y += 1.0;
    }

    if dir != Vec2::ZERO {
        dir = dir.normalize();
    }
    dir
}
