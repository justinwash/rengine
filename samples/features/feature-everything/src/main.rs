// feature-everything — Kitchen-Sink Rengine Demo
//
// A single cohesive game that exercises every major engine feature.
// See inline comments for which feature each section demonstrates.
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
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        self.blink_timer += engine.dt();

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
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);

        canvas.text(
            200.0,
            100.0,
            "RENGINE KITCHEN SINK",
            32.0,
            Color::YELLOW,
            (sw, sh),
            atlas,
        );

        if (self.blink_timer * 2.0).sin() > 0.0 {
            canvas.text(
                220.0,
                250.0,
                "Press ENTER to start",
                18.0,
                Color::WHITE,
                (sw, sh),
                atlas,
            );
        }

        let transitions = globals.get::<TransitionCounter>().map_or(0, |c| c.0);
        canvas.text(
            10.0,
            550.0,
            &format!("Scene transitions: {}", transitions),
            12.0,
            Color::GREEN,
            (sw, sh),
            atlas,
        );

        if engine.gamepads_connected() > 0 {
            canvas.text(
                220.0,
                300.0,
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
    // Data-driven config loaded from JSON (serializable resources)
    config: Option<GameConfig>,

    // Textures (created procedurally via PixelCanvas)
    player_tex: Option<TextureId>,
    coin_tex: Option<TextureId>,
    ground_tex: Option<TextureId>,
    bg_tex: Option<TextureId>,

    // Sprite sheet and animation
    coin_sheet: Option<SpriteSheet>,
    coin_anim: Animation,

    // Tilemap
    tilemap: Option<TileMap>,

    // Player state
    player_pos: Vec2,
    player_vel: Vec2,
    player_on_ground: bool,
    facing_right: bool,
    player_rotation: f32,
    // Collision layer for the player
    player_layer: CollisionLayer,

    // Coins
    coins: Vec<Vec2>,
    score: u32,

    // Trigger system
    triggers: TriggerSystem,
    zone_checkpoint: TriggerZoneId,
    zone_damage: TriggerZoneId,
    damage_flash: f32,
    checkpoint_flash: f32,
    checkpoint_msg: String,

    // Camera state
    cam_zoom: f32,
    rotation_mode: bool,
    pending_shake: bool,

    // Time tracking
    play_time: f32,
}

impl Default for GameScene {
    fn default() -> Self {
        let mut triggers = TriggerSystem::new();

        // Checkpoint zone — default layer (interacts with everything)
        let zone_checkpoint =
            triggers.add_zone(TriggerZone::new(Rect::new(300.0, 160.0, 64.0, 96.0)));

        // Damage zone — uses TRIGGER layer, only interacts with PLAYER
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
            player_rotation: 0.0,
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
            rotation_mode: false,
            pending_shake: false,
            play_time: 0.0,
        }
    }
}

impl Scene for GameScene {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals) {
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }

        // ── Serializable resource: load game tuning data from JSON ──
        match engine.load_resource::<GameConfig>("game_config.json") {
            Ok(cfg) => {
                self.coin_anim =
                    Animation::new(vec![(0, 0), (1, 0), (2, 0), (3, 0)], cfg.coin_anim_fps);
                self.config = Some(cfg);
            }
            Err(e) => eprintln!("Warning: could not load game_config.json: {e}"),
        }

        // ── Procedural textures via PixelCanvas ──

        // Player texture
        let mut pc = pixelart::PixelCanvas::new(16, 16);
        pc.fill(Color::new(0.0, 0.0, 0.0, 0.0));
        pc.fill_rect(4, 0, 8, 12, Color::new(0.2, 0.5, 1.0, 1.0));
        pc.fill_rect(5, 12, 2, 4, Color::new(0.2, 0.5, 1.0, 1.0));
        pc.fill_rect(9, 12, 2, 4, Color::new(0.2, 0.5, 1.0, 1.0));
        pc.fill_rect(6, 2, 4, 4, Color::new(1.0, 0.85, 0.7, 1.0));
        pc.set(7, 3, Color::BLACK);
        pc.set(9, 3, Color::BLACK);
        self.player_tex = Some(engine.create_texture(16, 16, &pc.into_bytes()));

        // Coin sprite sheet: 4 frames (64×16)
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

        // Ground tile
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

        // Background gradient
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

        // ── Build tilemap ──
        let ground = self.ground_tex.unwrap();
        let mut tilemap = TileMap::new(50, 30, 32.0);
        let ground_tile = tilemap.add_tile(TileDef::solid(ground));
        for col in 0..50 {
            tilemap.set(col, 0, Some(ground_tile));
            tilemap.set(col, 1, Some(ground_tile));
        }
        for col in 5..10 {
            tilemap.set(col, 5, Some(ground_tile));
        }
        for col in 15..22 {
            tilemap.set(col, 8, Some(ground_tile));
        }
        for col in 25..30 {
            tilemap.set(col, 5, Some(ground_tile));
        }
        for col in 8..14 {
            tilemap.set(col, 12, Some(ground_tile));
        }
        for col in 30..40 {
            tilemap.set(col, 10, Some(ground_tile));
        }
        for row in 2..8 {
            tilemap.set(40, row, Some(ground_tile));
        }
        self.tilemap = Some(tilemap);

        self.coins = vec![
            Vec2::new(200.0, 200.0),
            Vec2::new(300.0, 200.0),
            Vec2::new(550.0, 300.0),
            Vec2::new(600.0, 300.0),
            Vec2::new(250.0, 420.0),
            Vec2::new(350.0, 420.0),
            Vec2::new(850.0, 370.0),
            Vec2::new(1050.0, 380.0),
        ];

        // Reset state
        self.player_pos = Vec2::new(100.0, 100.0);
        self.player_vel = Vec2::ZERO;
        self.score = 0;
        self.play_time = 0.0;

        if !globals.contains::<PlayerStats>() {
            globals.set(PlayerStats {
                coins: 0,
                best_height: 0.0,
            });
        }
    }

    // ── Fixed-timestep update — physics runs at fixed_dt (default 1/60) ──
    fn fixed_update(&mut self, engine: &Engine, _globals: &mut Globals) {
        let fixed_dt = engine.time().fixed_dt();
        let cfg_gravity = self.config.as_ref().map_or(-980.0, |c| c.gravity);
        let cfg_speed = self.config.as_ref().map_or(250.0, |c| c.move_speed);
        let cfg_jump = self.config.as_ref().map_or(500.0, |c| c.jump_force);

        // ── Input via action mapping ──
        let move_x = engine.axis("move_x");

        self.player_vel.x = move_x * cfg_speed;
        if move_x > 0.1 {
            self.facing_right = true;
        } else if move_x < -0.1 {
            self.facing_right = false;
        }

        if engine.action_pressed("jump") && self.player_on_ground {
            self.player_vel.y = cfg_jump;
            self.player_on_ground = false;
            self.pending_shake = true;
        }

        // Gravity
        self.player_vel.y += cfg_gravity * fixed_dt;

        // Move and collide with tilemap
        self.player_pos += self.player_vel * fixed_dt;

        if let Some(tilemap) = &self.tilemap {
            let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
            if let Some(mtv) = tilemap.collide_rect(&player_rect) {
                self.player_pos += mtv;
                if mtv.y > 0.0 {
                    self.player_vel.y = 0.0;
                    self.player_on_ground = true;
                } else if mtv.y < 0.0 {
                    self.player_vel.y = 0.0;
                }
                if mtv.x != 0.0 {
                    self.player_vel.x = 0.0;
                }
            } else {
                self.player_on_ground = false;
            }
        }

        // ── Trigger system tick — collision layers filter interactions ──
        let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
        self.triggers
            .tick(&[(PLAYER_BODY_ID, player_rect, self.player_layer)]);

        // Process trigger events
        let events: Vec<_> = self.triggers.events().collect();
        for (zone_id, _body_id, event) in events {
            if zone_id == self.zone_checkpoint && event == OverlapEvent::Enter {
                self.checkpoint_flash = 0.5;
                self.checkpoint_msg = "Checkpoint!".to_string();
            }
            if zone_id == self.zone_damage && event == OverlapEvent::Stay {
                self.damage_flash = 0.15;
            }
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp {
        let dt = engine.dt();
        self.play_time += dt;

        // Airborne rotation (visual flair)
        if !self.player_on_ground {
            self.player_rotation += dt * 5.0 * if self.facing_right { 1.0 } else { -1.0 };
        } else {
            self.player_rotation *= (1.0 - dt * 10.0).max(0.0);
        }

        // ── Coin collection (aabb_overlap) ──
        let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
        self.coins.retain(|coin| {
            let coin_rect = Rect::new(coin.x, coin.y, 16.0, 16.0);
            if aabb_overlap(&player_rect, &coin_rect).is_some() {
                self.score += 1;
                false
            } else {
                true
            }
        });

        // Update globals
        if let Some(stats) = globals.get_mut::<PlayerStats>() {
            stats.coins = self.score;
            if self.player_pos.y > stats.best_height {
                stats.best_height = self.player_pos.y;
            }
        }

        // Animation
        self.coin_anim.update(dt);

        // Decay flashes
        self.damage_flash = (self.damage_flash - dt).max(0.0);
        self.checkpoint_flash = (self.checkpoint_flash - dt).max(0.0);
        self.pending_shake = false;

        // Zoom controls
        if engine.input().is_key_down(KeyCode::Equal) {
            self.cam_zoom *= 1.0 + dt;
        }
        if engine.input().is_key_down(KeyCode::Minus) {
            self.cam_zoom *= 1.0 - dt;
        }
        self.cam_zoom = self.cam_zoom.clamp(0.3, 3.0);

        // Camera rotation toggle
        if engine.input().is_key_pressed(KeyCode::KeyR) {
            self.rotation_mode = !self.rotation_mode;
        }

        // ── Scene management ──
        if engine.action_pressed("pause") {
            return SceneOp::Push(Box::new(PauseOverlay));
        }
        if engine.input().is_key_pressed(KeyCode::KeyT) {
            return SceneOp::Switch(Box::new(TitleScene { blink_timer: 0.0 }));
        }
        if engine.action_pressed("quit") {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        // Damage flash tints the background red
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
            max: Vec2::new(1600.0, 960.0),
        });
        cam.zoom = self.cam_zoom;
        if self.pending_shake {
            // pending_shake is cleared in update(); multiple shake calls just refresh
            cam.shake(4.0, 0.15);
        }
        if self.rotation_mode {
            cam.rotation += engine.dt() * 1.5;
        }
        cam.update(engine.dt());

        // ── Background (parallax-like: large sprite behind everything) ──
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

        // ── Tilemap ──
        if let Some(tilemap) = &self.tilemap {
            tilemap.draw(frame);
        }

        // ── Coins (sprite sheet + animation + uv_rect) ──
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

        // ── Trigger zone debug visualisation ──
        let white = engine.white_texture();

        // Checkpoint zone (green)
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

        // Damage zone (red)
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

        // ── Player (flip_x, rotation, origin, z_order) ──
        if let Some(player_tex) = self.player_tex {
            frame.draw_sprite(
                DrawParams::new(player_tex, self.player_pos, Vec2::new(28.0, 44.0))
                    .with_flip_x(!self.facing_right)
                    .with_rotation(self.player_rotation * 0.05)
                    .with_origin(Vec2::new(14.0, 22.0))
                    .with_z_order(10),
            );
        }

        // ── HUD via Canvas (rect, text) ──
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let hud = frame.canvas(0);

        hud.rect(
            5.0,
            30.0,
            200.0,
            80.0,
            Color::new(0.0, 0.0, 0.0, 0.5),
            (sw, sh),
        );
        hud.text(
            10.0,
            35.0,
            &format!("Coins: {}", self.score),
            18.0,
            Color::YELLOW,
            (sw, sh),
            atlas,
        );
        hud.text(
            10.0,
            55.0,
            &format!("Height: {:.0}", self.player_pos.y),
            14.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );
        hud.text(
            10.0,
            72.0,
            &format!("Time: {:.1}s", self.play_time),
            14.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );

        if self.checkpoint_flash > 0.0 {
            hud.text(
                10.0,
                90.0,
                &self.checkpoint_msg,
                14.0,
                Color::GREEN,
                (sw, sh),
                atlas,
            );
        }

        hud.text(
            sw as f32 - 500.0,
            sh as f32 - 20.0,
            "WASD/Arrows: Move | Space: Jump | P: Pause | T: Title | R: Rotate | +/-: Zoom",
            10.0,
            Color::new(1.0, 1.0, 1.0, 0.6),
            (sw, sh),
            atlas,
        );

        if let Some(stats) = globals.get::<PlayerStats>() {
            hud.text(
                sw as f32 - 200.0,
                35.0,
                &format!("Best height: {:.0}", stats.best_height),
                12.0,
                Color::GREEN,
                (sw, sh),
                atlas,
            );
        }
    }

    fn on_pause(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[GameScene] on_pause — game paused");
    }

    fn on_resume(&mut self, _engine: &mut Engine, _globals: &mut Globals) {
        println!("[GameScene] on_resume — game resumed");
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
// Pause Overlay — Push/Pop, transparent overlay, stack rendering
// ──────────────────────────────────────────────────────────────

struct PauseOverlay;

impl Scene for PauseOverlay {
    fn on_enter(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
        println!("[PauseOverlay] on_enter");
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        if engine.action_pressed("pause") || engine.action_pressed("quit") {
            return SceneOp::Pop;
        }
        if engine.gamepad(0).is_button_pressed(GamepadButton::Start) {
            return SceneOp::Pop;
        }
        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let overlay = frame.canvas(1);

        overlay.rect(
            0.0,
            0.0,
            sw as f32,
            sh as f32,
            Color::new(0.0, 0.0, 0.0, 0.65),
            (sw, sh),
        );
        overlay.text(
            sw as f32 / 2.0 - 80.0,
            sh as f32 / 2.0 - 30.0,
            "PAUSED",
            40.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );
        overlay.text(
            sw as f32 / 2.0 - 120.0,
            sh as f32 / 2.0 + 20.0,
            "Press P or Esc to resume",
            16.0,
            Color::new(0.8, 0.8, 0.8, 1.0),
            (sw, sh),
            atlas,
        );

        if let Some(stats) = globals.get::<PlayerStats>() {
            overlay.text(
                sw as f32 / 2.0 - 100.0,
                sh as f32 / 2.0 + 60.0,
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
// Entry Point
// ──────────────────────────────────────────────────────────────

fn main() {
    rengine::run_with_scenes(
        EngineConfig {
            title: "Rengine Kitchen Sink".into(),
            width: 960,
            height: 720,
            vsync: false,
            headless: false,
            hot_reload: true,
            show_fps: true,
            fixed_dt: 1.0 / 60.0,
            ..Default::default()
        },
        |engine, globals| {
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

            actions.bind("quit", Binding::Key(KeyCode::Escape));

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

            // Globals
            globals.set(TransitionCounter(0));
            globals.set(PlayerStats {
                coins: 0,
                best_height: 0.0,
            });

            Box::new(TitleScene { blink_timer: 0.0 })
        },
    )
    .unwrap();
}
