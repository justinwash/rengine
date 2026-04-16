use rengine::{Color, Engine3D, Frame3D, Vec3};

use crate::state::FpsGame;
use crate::{VIEWMODEL_FOV_DEG, WORLD_FOV_DEG};

pub fn draw(game: &FpsGame, _engine: &Engine3D, frame: &mut Frame3D) {
    frame.clear_color = Color::from_rgba8(20, 20, 30, 255);
    frame.light_dir = Vec3::new(0.3, 0.8, 0.4).normalize();
    frame.light_intensity = 0.7;
    frame.ambient_intensity = 0.4;

    frame.camera.position = game.player_pos;
    frame.camera.yaw = game.cam_yaw;
    frame.camera.pitch = game.cam_pitch;
    frame.camera.fov_y = WORLD_FOV_DEG.to_radians();

    frame.viewmodel.camera.position = game.player_pos;
    frame.viewmodel.camera.yaw = game.cam_yaw;
    frame.viewmodel.camera.pitch = game.cam_pitch;
    frame.viewmodel.camera.fov_y = VIEWMODEL_FOV_DEG.to_radians();
    frame.viewmodel.camera.z_near = 0.01;
    frame.viewmodel.camera.z_far = 8.0;
    frame.draw_viewmodel_mesh(game.viewmodel_mesh, Vec3::ZERO);

    frame.draw_raw(&game.level_verts, &game.level_idxs);

    for door in &game.doors {
        let slide = if door.slides_x {
            Vec3::new(door.offset * door.slide_sign, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, door.offset * door.slide_sign)
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
        if proj.visible {
            frame.draw_mesh(game.projectile_mesh, proj.pos);
        }
    }

    let size = 10.0_f32;
    let thickness = 2.0_f32;
    let crosshair = frame.canvas(0);
    crosshair.rect(-size, -thickness / 2.0, size * 2.0, thickness, Color::WHITE);
    crosshair.rect(-thickness / 2.0, -size, thickness, size * 2.0, Color::WHITE);
}
