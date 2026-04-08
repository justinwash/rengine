use rengine::Engine3D;
use winit::keyboard::KeyCode;

use crate::state::FpsInput;
use crate::MOUSE_SENSITIVITY;

pub fn sample_from_engine(engine: &Engine3D) -> FpsInput {
    let input = engine.input();
    let mut flags = 0u8;

    if input.is_key_down(KeyCode::KeyW) {
        flags |= FpsInput::FORWARD;
    }
    if input.is_key_down(KeyCode::KeyS) {
        flags |= FpsInput::BACK;
    }
    if input.is_key_down(KeyCode::KeyA) {
        flags |= FpsInput::LEFT;
    }
    if input.is_key_down(KeyCode::KeyD) {
        flags |= FpsInput::RIGHT;
    }
    if input.is_key_pressed(KeyCode::Space) {
        flags |= FpsInput::JUMP;
    }
    if input.is_mouse_down(0) {
        flags |= FpsInput::SHOOT;
    }

    let (dx, dy) = input.mouse_delta();
    let (look_dx, look_dy) = FpsInput::encode_look(
        dx as f32 * MOUSE_SENSITIVITY,
        -(dy as f32) * MOUSE_SENSITIVITY,
    );

    FpsInput {
        flags,
        _pad: 0,
        look_dx,
        look_dy,
        _pad2: [0; 2],
    }
}
