use rengine::*;

use crate::car::Car;
use crate::state::RacingGame;
use crate::track::Track;
use crate::track_visuals::TrackVisuals;
use crate::track_visuals::TRACK_SCALE;

const CAR_DRAW_SIZE: Vec2 = Vec2::new(28.0, 28.0);

pub fn draw(game: &RacingGame, engine: &Engine, frame: &mut Frame) {
    let screen = engine.window_size();
    let atlas = engine.font_atlas();

    frame.clear_color = Color::from_rgba8(34, 85, 34, 255); // grass green

    // Camera follows the lead car or first car
    let follow_idx = game.camera_target;
    if follow_idx < game.cars.len() {
        let target = game.cars[follow_idx].pos;
        frame.camera.position = target + game.camera_offset;
    }
    frame.camera.zoom = game.camera_zoom;

    // Draw track layers (back to front)
    draw_runoffs(&game.visuals, game.white_tex, frame);
    draw_track_surface(&game.visuals, game.white_tex, frame);
    draw_kerbs(&game.visuals, game.white_tex, frame);
    draw_track_edges(&game.visuals, game.white_tex, frame);

    // Draw racing line
    draw_racing_line(&game.track, game.white_tex, frame);

    // Draw start/finish line
    draw_start_finish(&game.visuals, game.white_tex, frame);

    // Draw cars (sorted by Y for crude depth)
    let mut car_indices: Vec<usize> = (0..game.cars.len()).collect();
    car_indices.sort_by(|&a, &b| {
        game.cars[a]
            .pos
            .y
            .partial_cmp(&game.cars[b].pos.y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for &i in &car_indices {
        draw_car(&game.cars[i], frame);
    }

    // UI overlay
    draw_ui(game, screen, atlas, frame);
}

fn draw_runoffs(visuals: &TrackVisuals, white_tex: TextureId, frame: &mut Frame) {
    let gravel = Color::from_rgba8(140, 130, 100, 255);
    for runoff in &visuals.runoffs {
        frame.draw_world_polygon(white_tex, runoff, gravel);
    }
}

fn draw_track_surface(visuals: &TrackVisuals, white_tex: TextureId, frame: &mut Frame) {
    let asphalt = Color::from_rgba8(60, 60, 65, 255);
    let n = visuals.surface_r.len();
    for i in 0..n {
        let next = (i + 1) % n;
        frame.draw_world_quad(
            white_tex,
            [
                visuals.surface_r[i],
                visuals.surface_l[i],
                visuals.surface_l[next],
                visuals.surface_r[next],
            ],
            asphalt,
        );
    }
}

fn draw_kerbs(visuals: &TrackVisuals, white_tex: TextureId, frame: &mut Frame) {
    let kerb_red = Color::from_rgba8(200, 20, 20, 255);
    let kerb_white = Color::from_rgba8(240, 240, 240, 255);

    for poly in &visuals.kerb_red {
        frame.draw_world_polygon(white_tex, poly, kerb_red);
    }
    for poly in &visuals.kerb_white {
        frame.draw_world_polygon(white_tex, poly, kerb_white);
    }

    // Kerb border lines
    let border_color = Color::from_rgba8(255, 255, 255, 200);
    for line in &visuals.kerb_borders {
        draw_line_strip(frame, white_tex, line, 2.5 * TRACK_SCALE, border_color, false);
    }
}

fn draw_track_edges(visuals: &TrackVisuals, white_tex: TextureId, frame: &mut Frame) {
    let edge_color = Color::WHITE;
    let width = 3.0 * TRACK_SCALE;
    draw_line_strip(
        frame,
        white_tex,
        &visuals.outline_r,
        width,
        edge_color,
        true,
    );
    draw_line_strip(
        frame,
        white_tex,
        &visuals.outline_l,
        width,
        edge_color,
        true,
    );
}

/// Draw a polyline as a strip of thin quads.
fn draw_line_strip(
    frame: &mut Frame,
    tex: TextureId,
    points: &[Vec2],
    width: f32,
    color: Color,
    closed: bool,
) {
    let n = points.len();
    if n < 2 {
        return;
    }
    let limit = if closed { n } else { n - 1 };
    let half_w = width * 0.5;
    for i in 0..limit {
        let next = (i + 1) % n;
        let p0 = points[i];
        let p1 = points[next];
        let dir = (p1 - p0).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);
        frame.draw_world_quad(
            tex,
            [
                p0 + normal * half_w,
                p0 - normal * half_w,
                p1 - normal * half_w,
                p1 + normal * half_w,
            ],
            color,
        );
    }
}

fn draw_racing_line(track: &Track, white_tex: TextureId, frame: &mut Frame) {
    let line_color = Color::new(0.0, 1.0, 0.3, 0.4);
    let dot = 2.0 * TRACK_SCALE;
    let step = 4;
    for i in (0..track.points.len()).step_by(step) {
        let p = track.points[i];
        frame.draw_colored(
            white_tex,
            p - Vec2::splat(dot * 0.5),
            Vec2::splat(dot),
            line_color,
        );
    }
}

fn draw_start_finish(visuals: &TrackVisuals, white_tex: TextureId, frame: &mut Frame) {
    let p0 = visuals.start_line.0;
    let p1 = visuals.start_line.1;

    // Draw checkerboard start line
    let segments = 8;
    for i in 0..segments {
        let color = if i % 2 == 0 {
            Color::WHITE
        } else {
            Color::BLACK
        };
        let t0 = i as f32 / segments as f32;
        let t1 = (i + 1) as f32 / segments as f32;
        let a = p0.lerp(p1, t0);
        let b = p0.lerp(p1, t1);
        let dir = (p1 - p0).normalize_or_zero();
        let normal = Vec2::new(-dir.y, dir.x);
        let half_w = 3.0 * TRACK_SCALE;
        frame.draw_world_quad(
            white_tex,
            [
                a + normal * half_w,
                a - normal * half_w,
                b - normal * half_w,
                b + normal * half_w,
            ],
            color,
        );
    }
}

fn draw_car(car: &Car, frame: &mut Frame) {
    let draw_pos = car.pos - CAR_DRAW_SIZE * 0.5;
    frame
        .draw_sprite(DrawParams::new(car.tex, draw_pos, CAR_DRAW_SIZE).with_rotation(car.rotation));
}

fn draw_ui(game: &RacingGame, screen: (u32, u32), atlas: &FontAtlas, frame: &mut Frame) {
    let canvas = frame.canvas(0);

    // Countdown
    if let Some(num) = game.race.countdown_display() {
        let text = if num == 0 {
            "GO!".to_string()
        } else {
            num.to_string()
        };
        let size = 64.0;
        let x = screen.0 as f32 / 2.0 - 30.0;
        let y = screen.1 as f32 / 3.0;
        canvas.rect(
            x - 10.0,
            y - 10.0,
            80.0,
            size + 20.0,
            Color::from_rgba8(0, 0, 0, 180),
            screen,
        );
        let color = if num == 0 { Color::GREEN } else { Color::RED };
        canvas.text(x, y, &text, size, color, screen, atlas);
    }

    // Leaderboard (right side)
    let lb_x = screen.0 as f32 - 220.0;
    let lb_y = 40.0;
    let row_h = 22.0;
    let header_h = 28.0;

    // Background
    canvas.rect(
        lb_x - 8.0,
        lb_y - 8.0,
        225.0,
        header_h + row_h * game.cars.len() as f32 + 16.0,
        Color::from_rgba8(0, 0, 0, 180),
        screen,
    );

    // Header
    let lap_text = format!(
        "Lap {}/{}",
        game.cars
            .iter()
            .map(|c| c.current_lap)
            .max()
            .unwrap_or(1)
            .min(game.race.total_laps),
        game.race.total_laps
    );
    canvas.text(lb_x, lb_y, &lap_text, 20.0, Color::WHITE, screen, atlas);

    // Entries
    let mut sorted_cars: Vec<&Car> = game.cars.iter().collect();
    sorted_cars.sort_by_key(|c| c.place);

    for (row, car) in sorted_cars.iter().enumerate() {
        let y = lb_y + header_h + row as f32 * row_h;
        let place_str = format!("P{}", car.place);
        let name_str = &car.driver.abbreviation;

        // Highlight the camera target
        if car.index == game.camera_target {
            canvas.rect(
                lb_x - 4.0,
                y - 2.0,
                218.0,
                row_h,
                Color::from_rgba8(255, 255, 255, 30),
                screen,
            );
        }

        canvas.text(lb_x, y, &place_str, 16.0, Color::YELLOW, screen, atlas);
        canvas.text(
            lb_x + 35.0,
            y,
            name_str,
            16.0,
            car.body_color,
            screen,
            atlas,
        );

        // Gap to leader
        if car.place == 1 {
            let time_str = format_time(car.race_time);
            canvas.text(
                lb_x + 100.0,
                y,
                &time_str,
                14.0,
                Color::WHITE,
                screen,
                atlas,
            );
        } else {
            let leader_progress = sorted_cars[0].race_progress;
            let gap = leader_progress - car.race_progress;
            if gap > 1.0 {
                let laps_behind = gap.floor() as u32;
                let gap_str = format!(
                    "+{} lap{}",
                    laps_behind,
                    if laps_behind > 1 { "s" } else { "" }
                );
                canvas.text(
                    lb_x + 100.0,
                    y,
                    &gap_str,
                    14.0,
                    Color::from_rgba8(200, 200, 200, 255),
                    screen,
                    atlas,
                );
            } else {
                // Estimate time gap
                let leader_speed = sorted_cars[0].speed.max(1.0);
                let dist_gap = gap * game.track.length;
                let time_gap = dist_gap / leader_speed;
                let gap_str = format!("+{:.1}s", time_gap);
                canvas.text(
                    lb_x + 100.0,
                    y,
                    &gap_str,
                    14.0,
                    Color::from_rgba8(200, 200, 200, 255),
                    screen,
                    atlas,
                );
            }
        }

        // Position change from grid
        let change = car.start_place as i32 - car.place as i32;
        if change > 0 {
            let s = format!("+{}", change);
            canvas.text(lb_x + 175.0, y, &s, 14.0, Color::GREEN, screen, atlas);
        } else if change < 0 {
            let s = format!("{}", change);
            canvas.text(lb_x + 175.0, y, &s, 14.0, Color::RED, screen, atlas);
        }
    }

    // Bottom-left: followed car info
    if game.camera_target < game.cars.len() {
        let car = &game.cars[game.camera_target];
        let info_y = screen.1 as f32 - 100.0;
        let info_x = 10.0;

        canvas.rect(
            info_x - 5.0,
            info_y - 5.0,
            250.0,
            90.0,
            Color::from_rgba8(0, 0, 0, 180),
            screen,
        );

        canvas.text(
            info_x,
            info_y,
            &car.driver.name,
            18.0,
            car.body_color,
            screen,
            atlas,
        );

        let speed_mph = car.speed * 0.6 / TRACK_SCALE; // arbitrary units-to-mph
        let speed_str = format!("{:.0} MPH", speed_mph);
        canvas.text(
            info_x,
            info_y + 22.0,
            &speed_str,
            16.0,
            Color::WHITE,
            screen,
            atlas,
        );

        let thr_str = format!(
            "THR: {:.0}%  BRK: {:.0}%",
            car.ai_throttle * 100.0,
            car.ai_brake * 100.0
        );
        canvas.text(
            info_x,
            info_y + 42.0,
            &thr_str,
            14.0,
            Color::from_rgba8(180, 180, 180, 255),
            screen,
            atlas,
        );

        if car.best_lap_time < f32::MAX {
            let best_str = format!("Best: {}", format_time(car.best_lap_time));
            canvas.text(
                info_x,
                info_y + 60.0,
                &best_str,
                14.0,
                Color::from_rgba8(160, 100, 255, 255),
                screen,
                atlas,
            );
        }
        if car.last_lap_time > 0.0 {
            let last_str = format!("Last: {}", format_time(car.last_lap_time));
            canvas.text(
                info_x + 130.0,
                info_y + 60.0,
                &last_str,
                14.0,
                Color::from_rgba8(180, 180, 180, 255),
                screen,
                atlas,
            );
        }
    }

    // Race finished banner
    if game.race.finished {
        let banner_y = screen.1 as f32 / 2.0 - 30.0;
        canvas.rect(
            0.0,
            banner_y - 10.0,
            screen.0 as f32,
            70.0,
            Color::from_rgba8(0, 0, 0, 200),
            screen,
        );
        canvas.text(
            screen.0 as f32 / 2.0 - 100.0,
            banner_y,
            "RACE COMPLETE",
            32.0,
            Color::from_rgba8(255, 215, 0, 255),
            screen,
            atlas,
        );
        if let Some(&winner_idx) = game.race.final_order.first() {
            let winner = &game.cars[winner_idx];
            canvas.text(
                screen.0 as f32 / 2.0 - 80.0,
                banner_y + 36.0,
                &format!("Winner: {}", winner.driver.name),
                20.0,
                Color::WHITE,
                screen,
                atlas,
            );
        }
    }

    // Controls hint
    canvas.text(
        10.0,
        screen.1 as f32 - 18.0,
        "1-9: follow car | +/-: zoom | arrows: pan",
        12.0,
        Color::from_rgba8(150, 150, 150, 200),
        screen,
        atlas,
    );
}

fn format_time(seconds: f32) -> String {
    let mins = (seconds / 60.0) as u32;
    let secs = seconds % 60.0;
    if mins > 0 {
        format!("{}:{:05.2}", mins, secs)
    } else {
        format!("{:.2}s", secs)
    }
}
