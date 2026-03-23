use rengine::{Canvas, Color, DrawParams, Engine, FontAtlas, Frame, Vec2};

use crate::state::{Facing, FightGame, FighterData, FighterState, FighterTextures};
use crate::{FIGHTER_H, FIGHTER_W, GROUND_Y, MAX_HP, SCREEN_H, SCREEN_W};

pub fn draw(game: &FightGame, engine: &Engine, frame: &mut Frame) {
    let screen = engine.window_size();
    frame.clear_color = Color::from_rgba8(10, 8, 20, 255);

    frame.camera.position = Vec2::new(SCREEN_W as f32 / 2.0, SCREEN_H as f32 / 2.0);
    frame.camera.zoom = 1.0;

    let w = SCREEN_W as f32;
    let wt = game.white_tex;

    let sky_bands: &[(f32, f32, [u8; 4])] = &[
        (440.0, 40.0, [8, 10, 30, 255]),
        (400.0, 40.0, [12, 18, 50, 255]),
        (360.0, 40.0, [18, 28, 70, 255]),
        (320.0, 40.0, [25, 40, 90, 255]),
        (280.0, 40.0, [35, 55, 110, 255]),
        (240.0, 40.0, [50, 70, 120, 255]),
        (200.0, 40.0, [70, 85, 130, 255]),
    ];
    for &(y, h, rgba) in sky_bands {
        frame.draw_colored(
            wt,
            Vec2::new(0.0, y),
            Vec2::new(w, h),
            Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]),
        );
    }

    let bldg_color = Color::from_rgba8(20, 25, 50, 255);
    let bldg_hi = Color::from_rgba8(30, 38, 65, 255);
    let buildings: &[(f32, f32, f32, bool)] = &[
        (0.0, 80.0, 90.0, false),
        (70.0, 50.0, 140.0, true),
        (110.0, 90.0, 70.0, false),
        (200.0, 40.0, 160.0, true),
        (230.0, 70.0, 80.0, false),
        (310.0, 60.0, 110.0, true),
        (360.0, 90.0, 60.0, false),
        (440.0, 45.0, 170.0, true),
        (475.0, 80.0, 75.0, false),
        (550.0, 65.0, 120.0, true),
        (600.0, 90.0, 65.0, false),
        (680.0, 50.0, 130.0, true),
        (720.0, 80.0, 85.0, false),
    ];
    let horizon_y = 200.0;
    for &(bx, bw, bh, lighter) in buildings {
        let col = if lighter { bldg_hi } else { bldg_color };
        frame.draw_colored(wt, Vec2::new(bx, horizon_y), Vec2::new(bw, bh), col);
    }

    frame.draw_colored(
        wt,
        Vec2::new(0.0, horizon_y - 2.0),
        Vec2::new(w, 4.0),
        Color::from_rgba8(100, 80, 60, 100),
    );

    let wall_bands: &[(f32, f32, [u8; 4])] = &[
        (170.0, 30.0, [55, 42, 35, 255]),
        (140.0, 30.0, [65, 50, 40, 255]),
        (GROUND_Y, 140.0 - GROUND_Y, [75, 58, 45, 255]),
    ];
    for &(y, h, rgba) in wall_bands {
        frame.draw_colored(
            wt,
            Vec2::new(0.0, y),
            Vec2::new(w, h),
            Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]),
        );
    }

    let pillar_color = Color::from_rgba8(50, 38, 30, 255);
    for i in 0..6 {
        let px = 30.0 + i as f32 * 155.0;
        frame.draw_colored(
            wt,
            Vec2::new(px, GROUND_Y),
            Vec2::new(12.0, 100.0),
            pillar_color,
        );
        frame.draw_colored(
            wt,
            Vec2::new(px - 3.0, 196.0),
            Vec2::new(18.0, 6.0),
            Color::from_rgba8(80, 65, 50, 255),
        );
    }

    frame.draw_colored(
        wt,
        Vec2::new(0.0, GROUND_Y - 1.0),
        Vec2::new(w, 3.0),
        Color::from_rgba8(120, 100, 70, 200),
    );

    let tile_size = 48.0;
    let num_tiles = (w / tile_size).ceil() as i32 + 1;
    for i in 0..num_tiles {
        let tx = i as f32 * tile_size;
        frame.draw(
            game.floor_tex,
            Vec2::new(tx, GROUND_Y - tile_size),
            Vec2::new(tile_size, tile_size),
        );
        frame.draw(
            game.floor_tex,
            Vec2::new(tx, GROUND_Y - tile_size * 2.0),
            Vec2::new(tile_size, tile_size),
        );
    }

    for fighter in [&game.sim.p1, &game.sim.p2] {
        let shadow_w = FIGHTER_W * 0.7;
        let shadow_h = 6.0;
        frame.draw_colored(
            wt,
            Vec2::new(fighter.x - shadow_w / 2.0, GROUND_Y - shadow_h / 2.0),
            Vec2::new(shadow_w, shadow_h),
            Color::from_rgba8(0, 0, 0, 90),
        );
    }

    draw_fighter(game, &game.sim.p1, &game.p1_tex, frame);
    draw_fighter(game, &game.sim.p2, &game.p2_tex, frame);

    for spark in &game.sim.sparks {
        let alpha = (spark.life * 4.0).min(1.0);
        let color = Color::new(1.0, 1.0, 0.3, alpha);
        frame.draw_colored(
            game.white_tex,
            Vec2::new(spark.x - 3.0, spark.y - 3.0),
            Vec2::new(6.0, 6.0),
            color,
        );
    }

    draw_hud(game, frame.canvas(0), screen, engine.font_atlas());

    if game.demo_mode {
        draw_demo_overlay(game, frame.canvas(1), screen, engine.font_atlas());
    }
}

fn draw_fighter(game: &FightGame, fighter: &FighterData, tex: &FighterTextures, frame: &mut Frame) {
    let t = match fighter.state {
        FighterState::Punching => tex.punch,
        FighterState::Kicking => tex.kick,
        FighterState::Blocking => tex.block,
        FighterState::HitStun => tex.hit,
        _ => tex.idle,
    };

    let flip = fighter.facing == Facing::Left;

    frame.draw_sprite(
        DrawParams::new(
            t,
            Vec2::new(fighter.rect_x(), fighter.rect_y()),
            Vec2::new(FIGHTER_W, FIGHTER_H),
        )
        .with_flip_x(flip),
    );

    if !fighter.is_on_ground() {
        let shadow_w = FIGHTER_W * 0.6;
        let shadow_h = 8.0;
        let shadow_alpha = ((fighter.y - GROUND_Y) / 200.0).clamp(0.1, 0.5);
        frame.draw_colored(
            game.white_tex,
            Vec2::new(fighter.x - shadow_w / 2.0, GROUND_Y - shadow_h / 2.0),
            Vec2::new(shadow_w, shadow_h),
            Color::new(0.0, 0.0, 0.0, shadow_alpha),
        );
    }
}

fn draw_hud(game: &FightGame, hud: &mut Canvas, screen: (u32, u32), atlas: &FontAtlas) {
    let bar_w = 300.0;
    let bar_h = 24.0;
    let bar_y = 16.0;

    let p1_bar_x = 20.0;
    hud.rect(
        p1_bar_x - 2.0,
        bar_y - 2.0,
        bar_w + 4.0,
        bar_h + 4.0,
        Color::WHITE,
        screen,
    );
    hud.rect(
        p1_bar_x,
        bar_y,
        bar_w,
        bar_h,
        Color::from_rgba8(60, 20, 20, 255),
        screen,
    );
    let p1_fill = (game.sim.p1.hp.max(0) as f32 / MAX_HP as f32) * bar_w;
    let p1_color = hp_color(game.sim.p1.hp);
    hud.rect(p1_bar_x, bar_y, p1_fill, bar_h, p1_color, screen);

    let p2_bar_x = SCREEN_W as f32 - 20.0 - bar_w;
    hud.rect(
        p2_bar_x - 2.0,
        bar_y - 2.0,
        bar_w + 4.0,
        bar_h + 4.0,
        Color::WHITE,
        screen,
    );
    hud.rect(
        p2_bar_x,
        bar_y,
        bar_w,
        bar_h,
        Color::from_rgba8(60, 20, 20, 255),
        screen,
    );
    let p2_fill = (game.sim.p2.hp.max(0) as f32 / MAX_HP as f32) * bar_w;
    let p2_color = hp_color(game.sim.p2.hp);
    hud.rect(
        p2_bar_x + bar_w - p2_fill,
        bar_y,
        p2_fill,
        bar_h,
        p2_color,
        screen,
    );

    hud.text(
        p1_bar_x,
        bar_y + bar_h + 6.0,
        "1",
        16.0,
        Color::from_rgba8(80, 120, 255, 255),
        screen,
        atlas,
    );
    hud.text(
        p2_bar_x + bar_w - 10.0,
        bar_y + bar_h + 6.0,
        "2",
        16.0,
        Color::from_rgba8(255, 80, 80, 255),
        screen,
        atlas,
    );

    let marker_size = 10.0;
    for i in 0..game.sim.p1.wins {
        hud.rect(
            p1_bar_x + 30.0 + i as f32 * 16.0,
            bar_y + bar_h + 4.0,
            marker_size,
            marker_size,
            Color::YELLOW,
            screen,
        );
    }
    for i in 0..game.sim.p2.wins {
        hud.rect(
            p2_bar_x + bar_w - 40.0 - i as f32 * 16.0,
            bar_y + bar_h + 4.0,
            marker_size,
            marker_size,
            Color::YELLOW,
            screen,
        );
    }

    let round_x = SCREEN_W as f32 / 2.0 - 15.0;
    hud.text(
        round_x,
        bar_y,
        &game.sim.round_number.to_string(),
        20.0,
        Color::WHITE,
        screen,
        atlas,
    );

    if game.sim.round_pause > 0.0 {
        let winner = if game.sim.p1.hp <= 0 { 2u32 } else { 1 };
        let ko_x = SCREEN_W as f32 / 2.0 - 30.0;
        let ko_y = SCREEN_H as f32 / 2.0 - 30.0;
        hud.rect(
            ko_x - 10.0,
            ko_y - 10.0,
            80.0,
            50.0,
            Color::from_rgba8(0, 0, 0, 200),
            screen,
        );
        hud.text(
            ko_x,
            ko_y,
            &winner.to_string(),
            36.0,
            Color::YELLOW,
            screen,
            atlas,
        );
    }
}

fn draw_demo_overlay(game: &FightGame, hud: &mut Canvas, screen: (u32, u32), atlas: &FontAtlas) {
    let elapsed_secs = game.demo_frame / 60;
    let elapsed_mins = elapsed_secs / 60;
    let elapsed_rem = elapsed_secs % 60;

    let banner_h = 36.0;
    let banner_y = SCREEN_H as f32 - banner_h;
    hud.rect(
        0.0,
        banner_y,
        SCREEN_W as f32,
        banner_h,
        Color::from_rgba8(0, 60, 0, 220),
        screen,
    );

    hud.text(
        8.0,
        banner_y + 4.0,
        &game.demo_frame.to_string(),
        14.0,
        Color::from_rgba8(100, 255, 100, 255),
        screen,
        atlas,
    );
    hud.text(
        200.0,
        banner_y + 4.0,
        &elapsed_mins.to_string(),
        14.0,
        Color::WHITE,
        screen,
        atlas,
    );
    hud.text(
        240.0,
        banner_y + 4.0,
        &elapsed_rem.to_string(),
        14.0,
        Color::WHITE,
        screen,
        atlas,
    );
    hud.text(
        SCREEN_W as f32 - 40.0,
        banner_y + 4.0,
        "7",
        14.0,
        Color::from_rgba8(255, 200, 50, 255),
        screen,
        atlas,
    );

    let state_y = banner_y + 20.0;
    hud.text(
        8.0,
        state_y,
        &game.sim.p1.hp.max(0).to_string(),
        12.0,
        Color::from_rgba8(80, 120, 255, 255),
        screen,
        atlas,
    );
    hud.text(
        80.0,
        state_y,
        &game.sim.p2.hp.max(0).to_string(),
        12.0,
        Color::from_rgba8(255, 80, 80, 255),
        screen,
        atlas,
    );
}

pub fn hp_color(hp: i32) -> Color {
    let t = hp.max(0) as f32 / MAX_HP as f32;
    if t > 0.5 {
        let s = (t - 0.5) * 2.0;
        Color::new(1.0 - s, 1.0, 0.0, 1.0)
    } else {
        let s = t * 2.0;
        Color::new(1.0, s, 0.0, 1.0)
    }
}
