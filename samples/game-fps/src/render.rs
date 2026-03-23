use rengine::{Color, Engine3D, Frame3D, Vec3};

use crate::state::FpsGame;


pub fn draw(game: &FpsGame, engine: &Engine3D, frame: &mut Frame3D) {
    frame.clear_color = Color::from_rgba8(20, 20, 30, 255);
    frame.light_dir = Vec3::new(0.3, 0.8, 0.4).normalize();
    frame.light_intensity = 0.7;
    frame.ambient_intensity = 0.4;


    frame.camera.position = game.player_pos;
    frame.camera.yaw = game.cam_yaw;
    frame.camera.pitch = game.cam_pitch;


    frame.draw_raw(&game.level_verts, &game.level_idxs);


    for door in &game.doors {
        let slide = if door.slides_x {
            Vec3::new(door.offset, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, door.offset)
        };
        let pos = Vec3::new(door.x, 1.1, door.z) + slide;
        frame.draw_mesh(door.mesh, pos);
    }


    for enemy in &game.enemies {
        if enemy.alive {
            frame.draw_mesh(enemy.mesh, enemy.pos);
        }
    }


    for proj in &game.projectiles {
        frame.draw_mesh(game.projectile_mesh, proj.pos);
    }


    let screen_size = engine.window_size();
    let cx = screen_size.0 as f32 / 2.0;
    let cy = screen_size.1 as f32 / 2.0;
    let size = 10.0_f32;
    let thickness = 2.0_f32;
    let crosshair = frame.canvas(0);
    crosshair.rect(cx - size, cy - thickness / 2.0, size * 2.0, thickness, Color::WHITE, screen_size);
    crosshair.rect(cx - thickness / 2.0, cy - size, thickness, size * 2.0, Color::WHITE, screen_size);
}
