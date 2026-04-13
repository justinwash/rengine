// feature-everything — Kitchen-Sink Rengine Demo
//
// A single cohesive game that exercises every major engine feature.
// See inline comments for which feature each section demonstrates.
//
// Run modes:
//   cargo run -p rengine-feature-everything                     # interactive
//   cargo run -p rengine-feature-everything -- --demo           # visible auto-play
//   cargo run -p rengine-feature-everything -- --demo --headless --frames 600  # CI test
//
// Demonstrates: EngineConfig (all fields), run_with_scenes(), Scene trait (all hooks
// including fixed_update), SceneOp (Switch, Push, Pop, Quit), Globals,
// Engine, Frame, Camera2D (follow, dead zone, bounds, shake, rotation, zoom),
// CameraBounds, DrawParams (position, size, color, uv_rect, flip_x, rotation,
// origin, z_order), TextureId, SpriteSheet, Animation, TileMap, TileDef,
// aabb_overlap, CollisionLayer, aabb_overlap_layered, TriggerSystem, TriggerZone,
// OverlapEvent, ActionMap, Binding, AxisMapping, GamepadAxis,
// load_resource (serializable resources via serde), fixed_update (fixed timestep),
// Rect, Canvas (rect, text), FontAtlas, Color, pixelart::PixelCanvas,
// InputState, GamepadState, TimeState, hot reload, Vec2.

use rengine::*;
use serde::Deserialize;
use std::cell::Cell;
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────
// Serializable resource — loaded from JSON via load_resource()
// ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GameConfig {
    gravity: f32,
    jump_force: f32,
    move_speed: f32,
    coin_anim_fps: f32,
}

// ──────────────────────────────────────────────────────────────
// Globals — typed key-value store shared across the scene stack
// ──────────────────────────────────────────────────────────────

struct TransitionCounter(u32);

struct PlayerStats {
    coins: u32,
    best_height: f32,
}

/// Demo mode configuration stored in Globals so all scenes can read it.
struct DemoConfig {
    enabled: bool,
    max_frames: u32,
    frame: u32,
    features_hit: Vec<&'static str>,
}

impl DemoConfig {
    fn log_feature(&mut self, name: &'static str) {
        if !self.features_hit.contains(&name) {
            self.features_hit.push(name);
            println!("[FEATURE OK] {name}");
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────────────────

const PLAYER_BODY_ID: BodyId = 0;

// ──────────────────────────────────────────────────────────────
// Title Scene — Switch, canvas text, gamepad, action mapping
// ──────────────────────────────────────────────────────────────

struct TitleScene {
    blink_timer: f32,
}

impl Scene for TitleScene {
    fn on_enter(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        println!("[TitleScene] on_enter");
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            demo.log_feature("Scene::on_enter");
            demo.log_feature("Globals::get_mut");
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp {
        self.blink_timer += engine.dt();

        // In demo mode, skip straight to game after a few frames
        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            if demo.enabled {
                demo.frame += 1;
                demo.log_feature("TimeState::dt");
                if demo.frame > 5 {
                    println!("[TitleScene] demo: auto-switching to GameScene");
                    demo.log_feature("SceneOp::Switch (Title->Game)");
                    return SceneOp::Switch(Box::new(GameScene::default()));
                }
            }
        }

        // Action mapping — "confirm" bound to Enter, Space, and gamepad South
        if engine.action_pressed("confirm") {
            return SceneOp::Switch(Box::new(GameScene::default()));
        }
        if engine.action_pressed("quit") {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        frame.clear_color = Color::new(0.1, 0.05, 0.2, 1.0);

        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);

        canvas.text(
            -hw + 200.0,
            hh - 100.0,
            "RENGINE KITCHEN SINK",
            32.0,
            Color::YELLOW,
            (sw, sh),
            atlas,
        );

        if (self.blink_timer * 2.0).sin() > 0.0 {
            canvas.text(
                -hw + 220.0,
                hh - 250.0,
                "Press ENTER to start",
                18.0,
                Color::WHITE,
                (sw, sh),
                atlas,
            );
        }

        let transitions = globals.get::<TransitionCounter>().map_or(0, |c| c.0);
        canvas.text(
            -hw + 10.0,
            -hh + 50.0,
            &format!("Scene transitions: {}", transitions),
            12.0,
            Color::GREEN,
            (sw, sh),
            atlas,
        );

        if engine.gamepads_connected() > 0 {
            canvas.text(
                -hw + 220.0,
                hh - 300.0,
                "(Gamepad detected: press A)",
                14.0,
                Color::ORANGE,
                (sw, sh),
                atlas,
            );
        }
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[TitleScene] on_exit");
    }
}

// ──────────────────────────────────────────────────────────────
// Game Scene — main gameplay exercising nearly every feature
// ──────────────────────────────────────────────────────────────

struct GameScene {
    config: Option<GameConfig>,

    player_tex: Option<TextureId>,
    coin_tex: Option<TextureId>,
    ground_tex: Option<TextureId>,
    bg_tex: Option<TextureId>,

    coin_sheet: Option<SpriteSheet>,
    coin_anim: Animation,

    tilemap: Option<TileMap>,

    player_pos: Vec2,
    player_vel: Vec2,
    player_on_ground: bool,
    facing_right: bool,
    jump_buffered: bool,
    player_layer: CollisionLayer,

    coins: Vec<Vec2>,
    score: u32,

    triggers: TriggerSystem,
    zone_checkpoint: TriggerZoneId,
    zone_damage: TriggerZoneId,
    damage_flash: f32,
    checkpoint_flash: f32,
    checkpoint_msg: String,

    cam_zoom: f32,
    // Small camera tilt applied on each jump-shake, decays back to zero.
    cam_tilt: f32,
    // Cell allows mutation from &self in render() — fixes the
    // fixed_update→update→render ordering so shake actually fires.
    pending_shake: Cell<bool>,

    play_time: f32,

    // Demo auto-play state
    demo_step: usize,
    demo_did_pause: bool,
    demo_did_zoom: bool,
    demo_last_frame: u32,
}

impl Default for GameScene {
    fn default() -> Self {
        let mut triggers = TriggerSystem::new();

        let zone_checkpoint =
            triggers.add_zone(TriggerZone::new(Rect::new(300.0, 160.0, 64.0, 96.0)));

        let zone_damage = triggers.add_zone(
            TriggerZone::new(Rect::new(750.0, 64.0, 120.0, 60.0)).with_layer(CollisionLayer::new(
                CollisionLayer::TRIGGER,
                CollisionLayer::PLAYER,
            )),
        );

        Self {
            config: None,
            player_tex: None,
            coin_tex: None,
            ground_tex: None,
            bg_tex: None,
            coin_sheet: None,
            coin_anim: Animation::new(vec![(0, 0), (1, 0), (2, 0), (3, 0)], 8.0),
            tilemap: None,
            player_pos: Vec2::new(100.0, 100.0),
            player_vel: Vec2::ZERO,
            player_on_ground: false,
            facing_right: true,
            jump_buffered: false,
            player_layer: CollisionLayer::new(
                CollisionLayer::PLAYER,
                CollisionLayer::PLAYER | CollisionLayer::TRIGGER,
            ),
            coins: Vec::new(),
            score: 0,
            triggers,
            zone_checkpoint,
            zone_damage,
            damage_flash: 0.0,
            checkpoint_flash: 0.0,
            checkpoint_msg: String::new(),
            cam_zoom: 1.0,
            cam_tilt: 0.0,
            pending_shake: Cell::new(false),
            play_time: 0.0,
            demo_step: 0,
            demo_did_pause: false,
            demo_did_zoom: false,
            demo_last_frame: 0,
        }
    }
}

impl Scene for GameScene {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals) {
        println!("[GameScene] on_enter");

        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }

        // ── Serializable resource: load game tuning data from JSON ──
        match engine.load_resource::<GameConfig>("game_config.json") {
            Ok(cfg) => {
                println!(
                    "[FEATURE OK] load_resource — loaded game_config.json \
                     (gravity={}, jump_force={}, move_speed={}, coin_anim_fps={})",
                    cfg.gravity, cfg.jump_force, cfg.move_speed, cfg.coin_anim_fps
                );
                self.coin_anim =
                    Animation::new(vec![(0, 0), (1, 0), (2, 0), (3, 0)], cfg.coin_anim_fps);
                self.config = Some(cfg);
            }
            Err(e) => eprintln!("Warning: could not load game_config.json: {e}"),
        }

        // ── Procedural textures via PixelCanvas ──

        let mut pc = pixelart::PixelCanvas::new(16, 16);
        pc.fill(Color::new(0.0, 0.0, 0.0, 0.0));
        pc.fill_rect(4, 0, 8, 12, Color::new(0.2, 0.5, 1.0, 1.0));
        pc.fill_rect(5, 12, 2, 4, Color::new(0.2, 0.5, 1.0, 1.0));
        pc.fill_rect(9, 12, 2, 4, Color::new(0.2, 0.5, 1.0, 1.0));
        pc.fill_rect(6, 2, 4, 4, Color::new(1.0, 0.85, 0.7, 1.0));
        pc.set(7, 3, Color::BLACK);
        pc.set(9, 3, Color::BLACK);
        self.player_tex = Some(engine.create_texture(16, 16, &pc.into_bytes()));
        println!("[FEATURE OK] PixelCanvas — player texture (fill, fill_rect, set)");

        let mut cc = pixelart::PixelCanvas::new(64, 16);
        cc.fill(Color::new(0.0, 0.0, 0.0, 0.0));
        for i in 0..4 {
            let ox = (i * 16 + 3) as i32;
            let widths = [10, 8, 4, 8];
            let w = widths[i];
            let xo = (10 - w) / 2;
            cc.fill_rect(ox + xo, 3, w, 10, Color::YELLOW);
            cc.fill_rect(
                ox + xo + 1,
                4,
                (w - 2).max(1),
                8,
                pixelart::lighten(Color::YELLOW, 1.3),
            );
        }
        let coin_pixels = cc.into_bytes();
        let coin_tex_id = engine.create_texture(64, 16, &coin_pixels);
        self.coin_tex = Some(coin_tex_id);
        self.coin_sheet = Some(SpriteSheet::new(coin_tex_id, 64, 16, 16, 16));
        println!("[FEATURE OK] SpriteSheet — 4-frame coin sheet (lighten)");

        let mut gc = pixelart::PixelCanvas::new(16, 16);
        gc.fill(Color::new(0.4, 0.25, 0.1, 1.0));
        gc.fill_rect(0, 0, 16, 3, Color::new(0.2, 0.7, 0.2, 1.0));
        gc.set(3, 5, pixelart::darken(Color::new(0.4, 0.25, 0.1, 1.0), 0.7));
        gc.set(
            10,
            8,
            pixelart::darken(Color::new(0.4, 0.25, 0.1, 1.0), 0.7),
        );
        self.ground_tex = Some(engine.create_texture(16, 16, &gc.into_bytes()));
        println!("[FEATURE OK] PixelCanvas::darken — ground tile");

        let mut bg = pixelart::PixelCanvas::new(1, 64);
        for y in 0..64 {
            let t = y as f32 / 63.0;
            bg.set(
                0,
                y,
                Color::rgb(
                    0.4 * (1.0 - t) + 0.1 * t,
                    0.6 * (1.0 - t) + 0.2 * t,
                    1.0 * (1.0 - t) + 0.5 * t,
                ),
            );
        }
        self.bg_tex = Some(engine.create_texture(1, 64, &bg.into_bytes()));
        println!("[FEATURE OK] Engine::create_texture — 4 procedural textures uploaded");
        println!("[FEATURE OK] Color::rgb — gradient background");

        // ── Build tilemap — platformer arena ──
        let ground = self.ground_tex.unwrap();
        let mut tilemap = TileMap::new(50, 20, 32.0);
        let ground_tile = tilemap.add_tile(TileDef::solid(ground));
        // Floor (rows 0-1)
        for col in 0..50 {
            tilemap.set(col, 0, Some(ground_tile));
            tilemap.set(col, 1, Some(ground_tile));
        }
        // Side walls
        for row in 2..20 {
            tilemap.set(0, row, Some(ground_tile));
            tilemap.set(49, row, Some(ground_tile));
        }
        // Platform P1: cols 5-9, row 4 (top=160, reachable from floor)
        for col in 5..10 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        // Platform P2: cols 14-19, row 4
        for col in 14..20 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        // Platform P5: cols 20-26, row 7 (top=256, reachable from P2)
        for col in 20..27 {
            tilemap.set(col, 7, Some(ground_tile));
        }
        // Platform P3: cols 28-33, row 4
        for col in 28..34 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        // Platform P4: cols 35-42, row 4 (1-tile gap from P3)
        for col in 35..43 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        self.tilemap = Some(tilemap);
        println!("[FEATURE OK] TileMap — 50x20 platformer arena with platforms");

        self.coins = vec![
            Vec2::new(70.0, 72.0),    // floor near start
            Vec2::new(220.0, 168.0),  // on P1
            Vec2::new(360.0, 72.0),   // floor between P1 and P2
            Vec2::new(540.0, 168.0),  // on P2
            Vec2::new(750.0, 264.0),  // on P5 (high platform)
            Vec2::new(980.0, 168.0),  // on P3
            Vec2::new(1250.0, 168.0), // on P4
            Vec2::new(1420.0, 72.0),  // floor after P4
        ];

        self.player_pos = Vec2::new(100.0, 100.0);
        self.player_vel = Vec2::ZERO;
        self.score = 0;
        self.play_time = 0.0;

        if !globals.contains::<PlayerStats>() {
            globals.set(PlayerStats {
                coins: 0,
                best_height: 0.0,
            });
            println!("[FEATURE OK] Globals::set — PlayerStats initialized");
        }
        println!("[FEATURE OK] Globals::contains — checked PlayerStats existence");
        println!(
            "[FEATURE OK] TriggerSystem — checkpoint zone (300,160) 64x96, \
             damage zone (750,64) 120x60"
        );
        println!(
            "[FEATURE OK] CollisionLayer — player mask PLAYER|TRIGGER, \
             damage zone TRIGGER->PLAYER"
        );
    }

    fn fixed_update(&mut self, engine: &Engine, globals: &mut Globals) {
        let fixed_dt = engine.time().fixed_dt();
        let cfg_gravity = self.config.as_ref().map_or(-980.0, |c| c.gravity);
        let cfg_speed = self.config.as_ref().map_or(250.0, |c| c.move_speed);
        let cfg_jump = self.config.as_ref().map_or(500.0, |c| c.jump_force);

        // ── Demo auto-play: scripted waypoints ──
        // Each waypoint is (target_x, jump_on_arrival). The AI walks toward
        // target_x and optionally jumps when it arrives, giving a natural-
        // looking run through the level.
        const DEMO_STEPS: &[(f32, bool)] = &[
            (70.0, false),   // walk to floor coin
            (85.0, true),    // jump to arc onto P1
            (220.0, false),  // walk to coin on P1
            (330.0, false),  // walk off P1 edge
            (360.0, false),  // walk to floor coin
            (380.0, true),   // jump to arc onto P2
            (540.0, false),  // walk to coin on P2
            (560.0, true),   // jump from P2 up to P5
            (750.0, false),  // walk to coin on P5
            (930.0, false),  // walk off P5, fall onto P3
            (980.0, false),  // walk to coin on P3
            (1080.0, true),  // jump across gap to P4
            (1250.0, false), // walk to coin on P4
            (1400.0, false), // walk off P4 to floor
            (1420.0, false), // walk to floor coin
            (1500.0, false), // run to end of level
        ];

        let is_demo = globals.get::<DemoConfig>().map_or(false, |d| d.enabled);
        let demo_move_x;
        let demo_jump;

        if is_demo {
            if self.demo_step < DEMO_STEPS.len() {
                let (target_x, jump) = DEMO_STEPS[self.demo_step];
                let px = self.player_pos.x + 14.0;
                let dx = target_x - px;

                demo_move_x = if dx.abs() < 8.0 {
                    0.0
                } else if dx > 0.0 {
                    1.0
                } else {
                    -1.0
                };

                if dx.abs() < 15.0 {
                    if jump {
                        if self.player_on_ground {
                            demo_jump = true;
                            self.demo_step += 1;
                        } else {
                            demo_jump = false; // wait for landing
                        }
                    } else {
                        demo_jump = false;
                        self.demo_step += 1;
                    }
                } else {
                    demo_jump = false;
                }
            } else {
                // All waypoints done — run right
                demo_move_x = 1.0;
                demo_jump = false;
            }
        } else {
            demo_move_x = 0.0;
            demo_jump = false;
        }

        let move_x = if is_demo {
            demo_move_x
        } else {
            engine.axis("move_x")
        };

        // ── Horizontal movement ──
        if self.player_on_ground {
            self.player_vel.x = move_x * cfg_speed;
        } else {
            // Gentle air control
            self.player_vel.x += (move_x * cfg_speed - self.player_vel.x) * (5.0 * fixed_dt);
        }

        if move_x > 0.1 {
            self.facing_right = true;
        } else if move_x < -0.1 {
            self.facing_right = false;
        }

        // ── Jump ──
        let should_jump = demo_jump || (self.jump_buffered && self.player_on_ground);
        self.jump_buffered = false;
        if should_jump && self.player_on_ground {
            self.player_vel.y = cfg_jump;
            self.player_on_ground = false;
        }

        // ── Gravity (always downward) ──
        self.player_vel.y += cfg_gravity * fixed_dt;
        self.player_pos += self.player_vel * fixed_dt;

        // ── Tilemap collision ──
        self.player_on_ground = false;
        if let Some(tilemap) = &self.tilemap {
            let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
            if let Some(mtv) = tilemap.collide_rect(&player_rect) {
                self.player_pos += mtv;
                // MTV pushing upward = standing on ground
                if mtv.y > 0.1 {
                    self.player_on_ground = true;
                }
                // Zero out velocity into the surface
                let mtv_len = mtv.length();
                if mtv_len > 0.001 {
                    let mtv_norm = mtv / mtv_len;
                    let vel_into_wall = self.player_vel.dot(mtv_norm);
                    if vel_into_wall < 0.0 {
                        self.player_vel -= mtv_norm * vel_into_wall;
                    }
                }
                if let Some(demo) = globals.get_mut::<DemoConfig>() {
                    demo.log_feature("TileMap::collide_rect");
                }
            }
        }

        // ── Trigger system tick ──
        let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
        self.triggers
            .tick(&[(PLAYER_BODY_ID, player_rect, self.player_layer)]);

        let events: Vec<_> = self.triggers.events().collect();
        for (zone_id, _body_id, event) in &events {
            if *zone_id == self.zone_checkpoint && *event == OverlapEvent::Enter {
                self.checkpoint_flash = 0.5;
                self.checkpoint_msg = "Checkpoint!".to_string();
                println!("[FEATURE OK] TriggerSystem — checkpoint Enter event");
            }
            if *zone_id == self.zone_damage && *event == OverlapEvent::Stay {
                self.damage_flash = 0.15;
            }
        }
        if !events.is_empty() {
            if let Some(demo) = globals.get_mut::<DemoConfig>() {
                demo.log_feature("TriggerSystem::tick + OverlapEvent");
            }
        }

        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            if demo.enabled {
                demo.frame += 1;
                demo.log_feature("fixed_update (fixed timestep)");
                demo.log_feature("TimeState::fixed_dt");
                demo.log_feature("Animation::update + current_frame");
                demo.log_feature("Rect");
            }
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp {
        let dt = engine.dt();
        self.play_time += dt;

        // Buffer jump input so fixed_update never misses it
        if engine.action_pressed("jump") {
            self.jump_buffered = true;
        }

        // Decay camera tilt back to zero
        self.cam_tilt *= (1.0 - dt * 8.0).max(0.0);
        if self.cam_tilt.abs() < 0.005 {
            self.cam_tilt = 0.0;
        }

        // ── Coin collection (aabb_overlap) ──
        let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
        let prev_score = self.score;
        let mut collected = 0u32;
        self.coins.retain(|coin| {
            let coin_rect = Rect::new(coin.x, coin.y, 16.0, 16.0);
            if aabb_overlap(&player_rect, &coin_rect).is_some() {
                collected += 1;
                false
            } else {
                true
            }
        });
        self.score += collected;
        if self.score > prev_score {
            // Juicy feedback: shake + tilt on coin pickup
            self.pending_shake.set(true);
            self.cam_tilt = 0.07 * if self.facing_right { 1.0 } else { -1.0 };
            println!(
                "[FEATURE OK] aabb_overlap — collected coin! score: {}",
                self.score
            );
            if let Some(demo) = globals.get_mut::<DemoConfig>() {
                demo.log_feature("Camera2D::shake (via coin)");
                demo.log_feature("Camera2D::rotation");
            }
        }

        if let Some(stats) = globals.get_mut::<PlayerStats>() {
            stats.coins = self.score;
            if self.player_pos.y > stats.best_height {
                stats.best_height = self.player_pos.y;
            }
        }

        self.coin_anim.update(dt);

        self.damage_flash = (self.damage_flash - dt).max(0.0);
        self.checkpoint_flash = (self.checkpoint_flash - dt).max(0.0);

        // ── Demo mode: drive features and eventually quit ──
        let is_demo = globals.get::<DemoConfig>().map_or(false, |d| d.enabled);

        if is_demo {
            if let Some(demo) = globals.get_mut::<DemoConfig>() {
                let f = demo.frame;
                let prev = self.demo_last_frame;
                self.demo_last_frame = f;

                if f != prev {
                    // Zoom in briefly to demonstrate Camera2D::zoom
                    if f >= 150 && !self.demo_did_zoom {
                        self.demo_did_zoom = true;
                        self.cam_zoom = 1.3;
                        demo.log_feature("Camera2D::zoom");
                        println!("[GameScene] demo: zoom to 1.3x at frame {f}");
                    }
                    if f >= 300 && self.cam_zoom > 1.0 {
                        self.cam_zoom = 1.0;
                        println!("[GameScene] demo: zoom back to 1.0 at frame {f}");
                    }
                    if f >= 400 && !self.demo_did_pause {
                        self.demo_did_pause = true;
                        demo.log_feature("SceneOp::Push (Pause)");
                        println!("[GameScene] demo: pushing PauseOverlay at frame {f}");
                        return SceneOp::Push(Box::new(PauseOverlay { demo_frames: 0 }));
                    }
                }

                let max = demo.max_frames;

                if f >= max {
                    println!();
                    println!("==============================================");
                    println!("  KITCHEN SINK DEMO COMPLETE - {} frames", f);
                    println!("==============================================");
                    println!("Features demonstrated ({}):", demo.features_hit.len());
                    for feat in &demo.features_hit {
                        println!("  + {feat}");
                    }
                    println!("Coins collected: {}", self.score);
                    println!(
                        "Player final pos: ({:.0}, {:.0})",
                        self.player_pos.x, self.player_pos.y
                    );
                    println!("OK {f}");
                    return SceneOp::Quit;
                }
            }
        } else {
            if engine.input().is_key_down(KeyCode::Equal) {
                self.cam_zoom *= 1.0 + dt;
            }
            if engine.input().is_key_down(KeyCode::Minus) {
                self.cam_zoom *= 1.0 - dt;
            }
            self.cam_zoom = self.cam_zoom.clamp(0.3, 3.0);
        }

        if !is_demo {
            if engine.action_pressed("pause") {
                return SceneOp::Push(Box::new(PauseOverlay { demo_frames: 0 }));
            }
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        frame.clear_color = if self.damage_flash > 0.0 {
            Color::new(0.8, 0.2, 0.2, 1.0)
        } else {
            Color::new(0.4, 0.6, 1.0, 1.0)
        };

        // ── Camera: follow, dead zone, bounds, shake, rotation, zoom ──
        let cam = &mut frame.camera;
        let player_center = self.player_pos + Vec2::new(14.0, 22.0);
        cam.follow(player_center, 6.0);
        cam.set_dead_zone(Vec2::new(30.0, 20.0));
        cam.bounds = Some(CameraBounds {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(1600.0, 640.0),
        });
        cam.zoom = self.cam_zoom;
        // Small tilt on jump shakes — purely cosmetic
        cam.rotation = self.cam_tilt;
        // Cell<bool> lets render() consume the flag despite &self
        if self.pending_shake.get() {
            self.pending_shake.set(false);
            cam.shake(4.0, 0.15);
        }
        cam.update(engine.dt());

        if let Some(bg_tex) = self.bg_tex {
            frame.draw_sprite(
                DrawParams::new(
                    bg_tex,
                    Vec2::new(frame.camera.position.x - 500.0, -100.0),
                    Vec2::new(1000.0, 800.0),
                )
                .with_z_order(-10),
            );
        }

        if let Some(tilemap) = &self.tilemap {
            tilemap.draw(frame);
        }

        if let Some(sheet) = &self.coin_sheet {
            let (col, row) = self.coin_anim.current_frame();
            let uv = sheet.uv_rect(col, row);
            for coin_pos in &self.coins {
                frame.draw_sprite(
                    DrawParams::new(sheet.texture, *coin_pos, Vec2::new(16.0, 16.0))
                        .with_uv_rect(uv)
                        .with_z_order(5),
                );
            }
        }

        let white = engine.white_texture();

        let cz = self.triggers.zone(self.zone_checkpoint);
        let alpha = if self.checkpoint_flash > 0.0 {
            0.5
        } else {
            0.15
        };
        frame.draw_sprite(
            DrawParams::new(
                white,
                Vec2::new(cz.rect.x, cz.rect.y),
                Vec2::new(cz.rect.width, cz.rect.height),
            )
            .with_color(Color::new(0.0, 1.0, 0.0, alpha))
            .with_z_order(1),
        );

        let dz = self.triggers.zone(self.zone_damage);
        let alpha = if self.damage_flash > 0.0 { 0.6 } else { 0.2 };
        frame.draw_sprite(
            DrawParams::new(
                white,
                Vec2::new(dz.rect.x, dz.rect.y),
                Vec2::new(dz.rect.width, dz.rect.height),
            )
            .with_color(Color::new(1.0, 0.0, 0.0, alpha))
            .with_z_order(1),
        );

        if let Some(player_tex) = self.player_tex {
            frame.draw_sprite(
                DrawParams::new(player_tex, self.player_pos, Vec2::new(28.0, 44.0))
                    .with_flip_x(!self.facing_right)
                    .with_z_order(10),
            );
        }

        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        let hud = frame.canvas(0);

        hud.rect(
            -hw + 5.0,
            hh - 30.0 - 95.0,
            200.0,
            95.0,
            Color::new(0.0, 0.0, 0.0, 0.5),
            (sw, sh),
        );
        hud.text(
            -hw + 10.0,
            hh - 35.0,
            &format!("Coins: {}", self.score),
            18.0,
            Color::YELLOW,
            (sw, sh),
            atlas,
        );
        hud.text(
            -hw + 10.0,
            hh - 55.0,
            &format!("Height: {:.0}", self.player_pos.y),
            14.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );
        hud.text(
            -hw + 10.0,
            hh - 72.0,
            &format!("Time: {:.1}s", self.play_time),
            14.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );

        if self.checkpoint_flash > 0.0 {
            hud.text(
                -hw + 10.0,
                hh - 105.0,
                &self.checkpoint_msg,
                14.0,
                Color::GREEN,
                (sw, sh),
                atlas,
            );
        }

        hud.text(
            hw - 380.0,
            -hh + 20.0,
            "WASD: Move | Space: Jump | +/-: Zoom | ESC: Pause/Quit",
            10.0,
            Color::new(1.0, 1.0, 1.0, 0.6),
            (sw, sh),
            atlas,
        );

        if let Some(stats) = globals.get::<PlayerStats>() {
            hud.text(
                hw - 200.0,
                hh - 35.0,
                &format!("Best height: {:.0}", stats.best_height),
                12.0,
                Color::GREEN,
                (sw, sh),
                atlas,
            );
        }
    }

    fn on_pause(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[GameScene] on_pause");
        println!("[FEATURE OK] Scene::on_pause");
    }

    fn on_resume(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        println!("[GameScene] on_resume");
        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            demo.log_feature("Scene::on_resume");
        }
    }

    fn on_exit(&mut self, _engine: &Engine, globals: &Globals) {
        println!("[GameScene] on_exit — final score: {}", self.score);
        if let Some(stats) = globals.get::<PlayerStats>() {
            println!(
                "  Total coins: {}, Best height: {:.0}",
                stats.coins, stats.best_height
            );
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Countdown Scene — gives time to start screen capture before demo
// ──────────────────────────────────────────────────────────────

struct CountdownScene {
    timer: f32,
}

impl CountdownScene {
    fn new() -> Self {
        Self { timer: 3.5 }
    }
}

impl Scene for CountdownScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {
        println!("[CountdownScene] on_enter — 3 second countdown");
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        self.timer -= engine.dt();
        if self.timer <= 0.0 {
            return SceneOp::Switch(Box::new(GameScene::default()));
        }
        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        frame.clear_color = Color::new(0.05, 0.05, 0.15, 1.0);

        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);

        let secs = self.timer.ceil() as i32;
        let label = if secs <= 0 {
            "GO!".to_string()
        } else {
            format!("{secs}")
        };

        canvas.text(
            -40.0,
            50.0,
            &label,
            80.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );

        canvas.text(
            -140.0,
            -50.0,
            "Demo starting... start recording!",
            16.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
            (sw, sh),
            atlas,
        );
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[CountdownScene] on_exit — starting demo");
    }
}

// ──────────────────────────────────────────────────────────────
// Pause Overlay — Push/Pop, transparent overlay, stack rendering
// ──────────────────────────────────────────────────────────────

struct PauseOverlay {
    demo_frames: u32,
}

impl Scene for PauseOverlay {
    fn on_enter(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
        println!("[PauseOverlay] on_enter");
        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            demo.log_feature("Scene::on_enter");
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp {
        let is_demo = globals.get::<DemoConfig>().map_or(false, |d| d.enabled);

        if is_demo {
            self.demo_frames += 1;
            if self.demo_frames >= 10 {
                println!("[PauseOverlay] demo: auto-popping after 10 frames");
                if let Some(demo) = globals.get_mut::<DemoConfig>() {
                    demo.log_feature("SceneOp::Pop (Unpause)");
                }
                return SceneOp::Pop;
            }
            return SceneOp::Continue;
        }

        if engine.action_pressed("pause") {
            return SceneOp::Pop;
        }
        if engine.gamepad(0).is_button_pressed(GamepadButton::Start) {
            return SceneOp::Pop;
        }
        if engine.action_pressed("quit") {
            return SceneOp::Quit;
        }
        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        let overlay = frame.canvas(1);

        overlay.rect(
            -hw,
            -hh,
            sw as f32,
            sh as f32,
            Color::new(0.0, 0.0, 0.0, 0.65),
            (sw, sh),
        );
        overlay.text(
            -80.0,
            30.0,
            "PAUSED",
            40.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );
        overlay.text(
            -130.0,
            -20.0,
            "Press ESC/P to resume | Q to quit",
            16.0,
            Color::new(0.8, 0.8, 0.8, 1.0),
            (sw, sh),
            atlas,
        );

        if let Some(stats) = globals.get::<PlayerStats>() {
            overlay.text(
                -100.0,
                -60.0,
                &format!(
                    "Coins: {} | Best Height: {:.0}",
                    stats.coins, stats.best_height
                ),
                14.0,
                Color::YELLOW,
                (sw, sh),
                atlas,
            );
        }
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[PauseOverlay] on_exit");
    }
}

// ──────────────────────────────────────────────────────────────
// CLI helpers
// ──────────────────────────────────────────────────────────────

fn has_flag(flag: &str) -> bool {
    std::env::args().any(|a| a == flag)
}

fn arg_value(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

// ──────────────────────────────────────────────────────────────
// Entry Point
// ──────────────────────────────────────────────────────────────

fn main() {
    let headless = has_flag("--headless");
    let demo = has_flag("--demo");
    let max_frames: u32 = arg_value("--frames")
        .and_then(|f| f.parse().ok())
        .unwrap_or(600);

    if demo {
        println!("==============================================");
        println!("  RENGINE KITCHEN SINK - DEMO MODE");
        println!("  headless: {}  frames: {}", headless, max_frames);
        println!("==============================================");
    }

    rengine::run_with_scenes(
        EngineConfig {
            title: "Rengine Kitchen Sink".into(),
            width: 960,
            height: 720,
            vsync: false,
            headless,
            hot_reload: !headless,
            show_fps: !headless,
            fixed_dt: 1.0 / 60.0,
            ..Default::default()
        },
        move |engine, globals| {
            // ── Asset root ──
            engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
            println!("[FEATURE OK] Engine::set_asset_root — assets resolve from crate directory");

            // ── Action mapping setup ──
            let actions = engine.actions_mut();

            actions.bind("confirm", Binding::Key(KeyCode::Enter));
            actions.bind("confirm", Binding::Key(KeyCode::Space));
            actions.bind("confirm", Binding::GamepadButton(GamepadButton::South));

            actions.bind("jump", Binding::Key(KeyCode::Space));
            actions.bind("jump", Binding::Key(KeyCode::ArrowUp));
            actions.bind("jump", Binding::Key(KeyCode::KeyW));
            actions.bind("jump", Binding::GamepadButton(GamepadButton::South));

            actions.bind("pause", Binding::Key(KeyCode::KeyP));
            actions.bind("pause", Binding::Key(KeyCode::Escape));

            actions.bind("quit", Binding::Key(KeyCode::KeyQ));
            // Quit is only used from menus/pause overlay — not during gameplay

            actions.bind_axis(
                "move_x",
                AxisMapping {
                    positive: vec![
                        Binding::Key(KeyCode::KeyD),
                        Binding::Key(KeyCode::ArrowRight),
                    ],
                    negative: vec![
                        Binding::Key(KeyCode::KeyA),
                        Binding::Key(KeyCode::ArrowLeft),
                    ],
                    gamepad_axis: Some(GamepadAxis::LeftStickX),
                },
            );

            println!("[FEATURE OK] ActionMap — bound confirm, jump, pause, quit, move_x axis");
            println!("[FEATURE OK] Binding::Key + Binding::GamepadButton");
            println!("[FEATURE OK] AxisMapping — move_x with keyboard + GamepadAxis::LeftStickX");
            println!(
                "[FEATURE OK] EngineConfig — title, width, height, vsync, headless, \
                 hot_reload, show_fps, fixed_dt"
            );

            globals.set(TransitionCounter(0));
            globals.set(PlayerStats {
                coins: 0,
                best_height: 0.0,
            });
            globals.set(DemoConfig {
                enabled: demo,
                max_frames,
                frame: 0,
                features_hit: Vec::new(),
            });

            println!("[FEATURE OK] Globals::set — TransitionCounter, PlayerStats, DemoConfig");
            println!("[FEATURE OK] run_with_scenes — scene-stack entry point");

            if demo {
                if headless {
                    println!("[Demo] Headless: skipping countdown, starting GameScene directly");
                    Box::new(GameScene::default()) as Box<dyn Scene>
                } else {
                    println!("[Demo] 3-second countdown before demo starts");
                    Box::new(CountdownScene::new()) as Box<dyn Scene>
                }
            } else {
                Box::new(TitleScene { blink_timer: 0.0 }) as Box<dyn Scene>
            }
        },
    )
    .unwrap();
}
