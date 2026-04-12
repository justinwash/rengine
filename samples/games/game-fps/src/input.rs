use rengine::{Engine3D, Vec3};
use winit::keyboard::KeyCode;

use crate::MOUSE_SENSITIVITY;

pub fn mouse_look(engine: &Engine3D, yaw: f32, pitch: f32) -> (f32, f32) {
    if !engine.is_mouse_captured() {
        return (yaw, pitch);
    }
    let (dx, dy) = engine.input().mouse_delta();
    let new_yaw = yaw + dx as f32 * MOUSE_SENSITIVITY;
    let mut new_pitch = pitch - dy as f32 * MOUSE_SENSITIVITY;
    let max_pitch = 89.0f32.to_radians();
    new_pitch = new_pitch.clamp(-max_pitch, max_pitch);
    (new_yaw, new_pitch)
}

pub fn move_dir(engine: &Engine3D, yaw: f32) -> Vec3 {
    let input = engine.input();
    let forward = Vec3::new(yaw.sin(), 0.0, -yaw.cos()).normalize();
    let right = Vec3::new(yaw.cos(), 0.0, yaw.sin()).normalize();

    let mut dir = Vec3::ZERO;
    if input.is_key_down(KeyCode::KeyW) { dir += forward; }
    if input.is_key_down(KeyCode::KeyS) { dir -= forward; }
    if input.is_key_down(KeyCode::KeyD) { dir += right; }
    if input.is_key_down(KeyCode::KeyA) { dir -= right; }

    if dir.length_squared() > 0.0 {
        dir = dir.normalize();
    }
    dir
}

pub fn jump_pressed(engine: &Engine3D) -> bool {
    engine.input().is_key_pressed(KeyCode::Space)
}

pub fn shoot_pressed(engine: &Engine3D) -> bool {
    engine.input().is_mouse_pressed(0)
}
