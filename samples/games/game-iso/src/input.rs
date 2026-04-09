use rengine::Engine;
use winit::keyboard::KeyCode;


pub fn movement_dir(engine: &Engine) -> (f32, f32) {
    let input = engine.input();
    let mut dc = 0.0f32;
    let mut dr = 0.0f32;


    if input.is_key_down(KeyCode::ArrowUp) || input.is_key_down(KeyCode::KeyW) {
        dr -= 1.0;
    }

    if input.is_key_down(KeyCode::ArrowDown) || input.is_key_down(KeyCode::KeyS) {
        dr += 1.0;
    }

    if input.is_key_down(KeyCode::ArrowLeft) || input.is_key_down(KeyCode::KeyA) {
        dc -= 1.0;
    }

    if input.is_key_down(KeyCode::ArrowRight) || input.is_key_down(KeyCode::KeyD) {
        dc += 1.0;
    }

    if dc != 0.0 || dr != 0.0 {
        let len = (dc * dc + dr * dr).sqrt();
        dc /= len;
        dr /= len;
    }

    (dc, dr)
}
