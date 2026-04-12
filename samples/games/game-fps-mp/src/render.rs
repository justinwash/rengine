use rengine::{Color, Engine3D, Frame3D, Vec3};

use crate::state::FpsMpGame;

pub fn draw(game: &FpsMpGame, engine: &Engine3D, frame: &mut Frame3D) {
    let local = game.session.local_player();
    let player = &game.sim.players[local];

    frame.clear_color = Color::from_rgba8(20, 20, 30, 255);
    frame.light_dir = Vec3::new(0.3, 0.8, 0.4).normalize();
    frame.light_intensity = 0.7;
    frame.ambient_intensity = 0.4;

    frame.camera.position = Vec3::new(player.x, player.y, player.z);
    frame.camera.yaw = player.yaw;
    frame.camera.pitch = player.pitch;

    frame.draw_raw(&game.level_verts, &game.level_idxs);

    for (i, door_def) in game.sim.door_defs.iter().enumerate() {
        let door_state = &game.sim.door_states[i];
        let slide = if door_def.slides_x {
            Vec3::new(door_state.offset, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, door_state.offset)
        };
        let pos = Vec3::new(door_def.x, 1.1, door_def.z) + slide;
        frame.draw_mesh(game.door_meshes[i], pos);
    }

    for (i, p) in game.sim.players.iter().enumerate() {
        if i == local || !p.alive() {
            continue;
        }
        frame.draw_mesh(game.player_mesh, Vec3::new(p.x, p.y - 0.85, p.z));
    }

    for proj in &game.sim.projectiles {
        frame.draw_mesh(game.projectile_mesh, Vec3::new(proj.x, proj.y, proj.z));
    }

    let screen_size = engine.window_size();
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    let size = 10.0_f32;
    let thickness = 2.0_f32;
    let crosshair = frame.canvas(0);
    crosshair.rect(
        -size,
        -thickness / 2.0,
        size * 2.0,
        thickness,
        Color::WHITE,
        screen_size,
    );
    crosshair.rect(
        -thickness / 2.0,
        -size,
        thickness,
        size * 2.0,
        Color::WHITE,
        screen_size,
    );

    let hp_frac = player.hp.max(0) as f32 / crate::MAX_HP as f32;
    let bar_w = 200.0_f32;
    let bar_h = 16.0_f32;
    let bar_x = -hw + 20.0;
    let bar_y = -hh + 40.0 - bar_h;

    let hud = frame.canvas(1);
    hud.rect(
        bar_x,
        bar_y,
        bar_w,
        bar_h,
        Color::from_rgba8(60, 60, 60, 200),
        screen_size,
    );
    let hp_color = if hp_frac > 0.5 {
        Color::from_rgba8(50, 200, 50, 255)
    } else if hp_frac > 0.25 {
        Color::from_rgba8(220, 180, 30, 255)
    } else {
        Color::from_rgba8(220, 40, 40, 255)
    };
    hud.rect(bar_x, bar_y, bar_w * hp_frac, bar_h, hp_color, screen_size);

    let score_y = bar_y + bar_h + 8.0;
    let my_score = player.score;
    let opp_score = game.sim.players[1 - local].score;
    let font = engine.font_atlas();
    hud.text(
        bar_x,
        score_y,
        &format!("Score: {my_score} — {opp_score}"),
        16.0,
        Color::WHITE,
        screen_size,
        font,
    );
}
