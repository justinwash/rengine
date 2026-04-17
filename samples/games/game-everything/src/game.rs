use crate::pause::PauseOverlay;
use crate::state::*;
use rengine::*;

pub struct GameScene {
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
    cam_tilt: f32,
    pending_shake: bool,

    play_time: f32,

    score_popup: Tween,

    coin_particles: ParticleEmitter,

    music_a: Option<AudioClip>,
    music_b: Option<AudioClip>,
    coin_sfx: Option<AudioClip>,
    did_crossfade: bool,

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
            pending_shake: false,
            play_time: 0.0,
            score_popup: Tween::new(0.0, 1.0, 0.6, Easing::OutElastic),

            coin_particles: ParticleEmitter::new(
                EmitterConfig::default()
                    .with_emit_rate(0.0)
                    .with_burst_count(12)
                    .with_lifetime((0.3, 0.6))
                    .with_speed((40.0, 120.0))
                    .with_angle((0.0, std::f32::consts::TAU))
                    .with_size_start((3.0, 6.0))
                    .with_size_end((0.0, 1.0))
                    .with_color_start(Color::YELLOW)
                    .with_color_end(Color::new(1.0, 0.8, 0.0, 0.0))
                    .with_damping(3.0)
                    .with_looping(false)
                    .with_z_order(5)
                    .with_max_particles(64),
            ),

            music_a: None,
            music_b: None,
            coin_sfx: None,
            did_crossfade: false,

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

        if let Some(saves) = globals.get::<SaveSystem>() {
            match saves.load::<CheckpointSave>("checkpoint") {
                Ok(cs) => {
                    if let Some(stats) = globals.get_mut::<PlayerStats>() {
                        stats.coins = cs.coins;
                        stats.best_height = cs.best_height;
                    }
                    self.score = cs.coins;
                    println!(
                        "[FEATURE OK] SaveSystem::load — restored checkpoint (coins={}, saves={})",
                        cs.coins, cs.times_saved
                    );
                }
                Err(_) => println!("[SaveSystem] No checkpoint save found, starting fresh"),
            }
        }

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

        let ground = self.ground_tex.unwrap();
        let mut tilemap = TileMap::new(50, 20, 32.0);
        let ground_tile = tilemap.add_tile(TileDef::solid(ground));
        for col in 0..50 {
            tilemap.set(col, 0, Some(ground_tile));
            tilemap.set(col, 1, Some(ground_tile));
        }
        for row in 2..20 {
            tilemap.set(0, row, Some(ground_tile));
            tilemap.set(49, row, Some(ground_tile));
        }
        for col in 5..10 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        for col in 14..20 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        for col in 20..27 {
            tilemap.set(col, 7, Some(ground_tile));
        }
        for col in 28..34 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        for col in 35..43 {
            tilemap.set(col, 4, Some(ground_tile));
        }
        self.tilemap = Some(tilemap);
        println!("[FEATURE OK] TileMap — 50x20 platformer arena with platforms");

        self.coins = vec![
            Vec2::new(70.0, 72.0),
            Vec2::new(220.0, 168.0),
            Vec2::new(360.0, 72.0),
            Vec2::new(540.0, 168.0),
            Vec2::new(750.0, 264.0),
            Vec2::new(980.0, 168.0),
            Vec2::new(1250.0, 168.0),
            Vec2::new(1420.0, 72.0),
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

        self.music_a = engine.load_audio("music_a.wav").ok();
        self.music_b = engine.load_audio("music_b.wav").ok();
        self.coin_sfx = engine.load_audio("coin_sfx.wav").ok();
        if let Some(ref clip) = self.music_a {
            let _ = engine.fade_in_music(clip, 2.0, Easing::OutQuad);
            if let Some(demo) = globals.get_mut::<DemoConfig>() {
                demo.log_feature("fade_in_music");
                demo.log_feature("load_audio");
            }
        }
    }

    fn fixed_update(&mut self, engine: &Engine, globals: &mut Globals) {
        let fixed_dt = engine.time().fixed_dt();
        let cfg_gravity = self.config.as_ref().map_or(-980.0, |c| c.gravity);
        let cfg_speed = self.config.as_ref().map_or(250.0, |c| c.move_speed);
        let cfg_jump = self.config.as_ref().map_or(500.0, |c| c.jump_force);

        const DEMO_STEPS: &[(f32, bool)] = &[
            (70.0, false),
            (85.0, true),
            (220.0, false),
            (330.0, false),
            (360.0, false),
            (380.0, true),
            (540.0, false),
            (560.0, true),
            (750.0, false),
            (930.0, false),
            (980.0, false),
            (1080.0, true),
            (1250.0, false),
            (1400.0, false),
            (1420.0, false),
            (1500.0, false),
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
                            demo_jump = false;
                        }
                    } else {
                        demo_jump = false;
                        self.demo_step += 1;
                    }
                } else {
                    demo_jump = false;
                }
            } else {
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

        if self.player_on_ground {
            self.player_vel.x = move_x * cfg_speed;
        } else {
            self.player_vel.x += (move_x * cfg_speed - self.player_vel.x) * (5.0 * fixed_dt);
        }

        if move_x > 0.1 {
            self.facing_right = true;
        } else if move_x < -0.1 {
            self.facing_right = false;
        }

        let should_jump = demo_jump || (self.jump_buffered && self.player_on_ground);
        self.jump_buffered = false;
        if should_jump && self.player_on_ground {
            self.player_vel.y = cfg_jump;
            self.player_on_ground = false;
        }

        self.player_vel.y += cfg_gravity * fixed_dt;
        self.player_pos += self.player_vel * fixed_dt;

        self.player_on_ground = false;
        if let Some(tilemap) = &self.tilemap {
            let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
            if let Some(mtv) = tilemap.collide_rect(&player_rect) {
                self.player_pos += mtv;
                if mtv.y > 0.1 {
                    self.player_on_ground = true;
                }
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

        let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
        self.triggers
            .tick(&[(PLAYER_BODY_ID, player_rect, self.player_layer)]);

        let events: Vec<_> = self.triggers.events().collect();
        for (zone_id, _body_id, event) in &events {
            if *zone_id == self.zone_checkpoint && *event == OverlapEvent::Enter {
                self.checkpoint_flash = 0.5;
                self.checkpoint_msg = "Checkpoint saved!".to_string();
                println!("[FEATURE OK] TriggerSystem — checkpoint Enter event");

                if let Some(saves) = globals.get::<SaveSystem>() {
                    let stats = globals.get::<PlayerStats>();
                    let cs = CheckpointSave {
                        coins: stats.map_or(0, |s| s.coins),
                        best_height: stats.map_or(0.0, |s| s.best_height),
                        times_saved: saves
                            .load::<CheckpointSave>("checkpoint")
                            .map_or(0, |prev| prev.times_saved)
                            + 1,
                    };
                    match saves.save("checkpoint", &cs) {
                        Ok(()) => println!(
                            "[FEATURE OK] SaveSystem::save — checkpoint slot (save #{})",
                            cs.times_saved
                        ),
                        Err(e) => eprintln!("Warning: save failed: {e}"),
                    }
                }
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

    fn update(&mut self, engine: &Engine, globals: &mut Globals, frame: &mut Frame) -> SceneOp {
        let dt = engine.dt();
        self.play_time += dt;

        if engine.action_pressed("jump") {
            self.jump_buffered = true;
        }

        self.cam_tilt *= (1.0 - dt * 8.0).max(0.0);
        if self.cam_tilt.abs() < 0.005 {
            self.cam_tilt = 0.0;
        }

        let player_rect = Rect::new(self.player_pos.x, self.player_pos.y, 28.0, 44.0);
        let prev_score = self.score;
        let mut collected = 0u32;
        let mut collected_positions = Vec::new();
        self.coins.retain(|coin| {
            let coin_rect = Rect::new(coin.x, coin.y, 16.0, 16.0);
            if aabb_overlap(&player_rect, &coin_rect).is_some() {
                collected += 1;
                collected_positions.push(*coin + Vec2::new(8.0, 8.0));
                false
            } else {
                true
            }
        });
        self.score += collected;
        if self.score > prev_score {
            let mut rng = engine.rng();
            for pos in &collected_positions {
                self.coin_particles.set_position(*pos);
                self.coin_particles.burst(&mut rng);
            }
            println!(
                "[FEATURE OK] ParticleEmitter::burst — coin pickup particles (score: {})",
                self.score
            );
            self.pending_shake = true;
            self.cam_tilt = 0.07 * if self.facing_right { 1.0 } else { -1.0 };
            self.score_popup.reset();
            if let Some(ref clip) = self.coin_sfx {
                let _ = engine.play_sound_on_bus(AudioBus::Effects, clip, 0.7);
            }
            println!(
                "[FEATURE OK] aabb_overlap — collected coin! score: {}",
                self.score
            );
            if let Some(demo) = globals.get_mut::<DemoConfig>() {
                demo.log_feature("Camera2D::shake (via coin)");
                demo.log_feature("Camera2D::rotation");
                demo.log_feature("Tween + Easing (score popup)");
                demo.log_feature("TextAlign::Center (text_aligned)");
                demo.log_feature("text_block (word wrapping)");
            }
        }

        if let Some(stats) = globals.get_mut::<PlayerStats>() {
            stats.coins = self.score;
            if self.player_pos.y > stats.best_height {
                stats.best_height = self.player_pos.y;
            }
        }

        self.coin_anim.update(dt);
        self.score_popup.update(dt);
        {
            let mut rng = engine.rng();
            self.coin_particles.update(dt, &mut rng);
        }

        self.damage_flash = (self.damage_flash - dt).max(0.0);
        self.checkpoint_flash = (self.checkpoint_flash - dt).max(0.0);

        let is_demo = globals.get::<DemoConfig>().map_or(false, |d| d.enabled);

        if is_demo {
            if let Some(demo) = globals.get_mut::<DemoConfig>() {
                let f = demo.frame;
                let prev = self.demo_last_frame;
                self.demo_last_frame = f;

                if f != prev {
                    if f >= 250 && !self.did_crossfade {
                        self.did_crossfade = true;
                        if let Some(ref clip) = self.music_b {
                            let _ = engine.crossfade_music(clip, 2.0, Easing::InOutSine);
                            demo.log_feature("crossfade_music");
                        }
                    }
                    if f >= 150 && !self.demo_did_zoom {
                        self.demo_did_zoom = true;
                        self.cam_zoom = 1.3;
                        demo.log_feature("Camera2D::zoom");
                        demo.log_feature("Camera2D::world_to_screen");
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
                        return SceneOp::Push(Box::new(PauseOverlay {
                            demo_frames: 0,
                            ui: Ui::default(),
                            badge: None,
                        }));
                    }
                    if prev < 500 && f >= 500 {
                        engine.fade_out_music(1.5, Easing::InQuad);
                        demo.log_feature("fade_out_music");
                    }
                    if prev < 200 && f >= 200 {
                        engine.postfx().push(PostEffect::Vignette {
                            intensity: 0.7,
                            radius: 0.5,
                            softness: 0.4,
                        });
                        demo.log_feature("PostEffect::Vignette");
                        println!("[GameScene] demo: postfx vignette enabled at frame {f}");
                    }
                    if prev < 350 && f >= 350 {
                        engine.postfx().clear();
                        engine.postfx().push(PostEffect::Crt {
                            scanline_intensity: 0.3,
                            curvature: 0.1,
                        });
                        demo.log_feature("PostEffect::Crt");
                        println!("[GameScene] demo: postfx switched to CRT at frame {f}");
                    }
                    if prev < 450 && f >= 450 {
                        engine.postfx().clear();
                        demo.log_feature("PostFxChain::clear");
                        println!("[GameScene] demo: postfx cleared at frame {f}");
                    }
                    if self.score > 0 {
                        demo.log_feature("play_sound_on_bus (coin sfx)");
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
                return SceneOp::Push(Box::new(PauseOverlay {
                    demo_frames: 0,
                    ui: Ui::default(),
                    badge: None,
                }));
            }
        }

        // Camera setup — game logic that belongs in update(), not render()
        frame.clear_color = if self.damage_flash > 0.0 {
            Color::new(0.8, 0.2, 0.2, 1.0)
        } else {
            Color::new(0.4, 0.6, 1.0, 1.0)
        };

        let cam = &mut frame.camera;
        let player_center = self.player_pos + Vec2::new(14.0, 22.0);
        cam.follow(player_center, 6.0);
        cam.set_dead_zone(Vec2::new(30.0, 20.0));
        cam.bounds = Some(CameraBounds {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(1600.0, 640.0),
        });
        cam.zoom = self.cam_zoom;
        cam.rotation = self.cam_tilt;
        if self.pending_shake {
            self.pending_shake = false;
            cam.shake(4.0, 0.15);
        }
        cam.update(engine.dt());

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
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

        self.coin_particles.draw(frame, engine.white_texture());

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
        let hud = frame.canvas(0);

        hud.rect(
            -hw + 5.0,
            hh - 30.0 - 95.0,
            200.0,
            95.0,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
        let popup_scale = 18.0
            + 10.0
                * self.score_popup.value()
                * if self.score_popup.is_finished() {
                    0.0
                } else {
                    1.0
                };
        hud.text(
            -hw + 10.0,
            hh - 35.0,
            &format!("Coins: {}", self.score),
            popup_scale,
            Color::YELLOW,
        );
        hud.text(
            -hw + 10.0,
            hh - 55.0,
            &format!("Height: {:.0}", self.player_pos.y),
            14.0,
            Color::WHITE,
        );
        hud.text(
            -hw + 10.0,
            hh - 72.0,
            &format!("Time: {:.1}s", self.play_time),
            14.0,
            Color::WHITE,
        );

        if self.checkpoint_flash > 0.0 {
            hud.text(
                -hw + 10.0,
                hh - 105.0,
                &self.checkpoint_msg,
                14.0,
                Color::GREEN,
            );
        }

        hud.text(
            hw - 380.0,
            -hh + 20.0,
            "WASD: Move | Space: Jump | +/-: Zoom | ESC: Pause/Quit",
            10.0,
            Color::new(1.0, 1.0, 1.0, 0.6),
        );

        hud.text_aligned(
            0.0,
            -hh + 50.0,
            "Kitchen Sink Demo",
            12.0,
            Color::new(0.7, 0.8, 1.0, 0.8),
            TextAlign::Center,
        );

        hud.text_block(
            hw - 200.0,
            hh - 75.0,
            "Collect coins to earn points. Reach checkpoints to save progress.",
            10.0,
            Color::new(1.0, 1.0, 1.0, 0.5),
            190.0,
            TextAlign::Left,
        );

        if let Some(stats) = globals.get::<PlayerStats>() {
            hud.text(
                hw - 200.0,
                hh - 35.0,
                &format!("Best height: {:.0}", stats.best_height),
                12.0,
                Color::GREEN,
            );
        }

        let player_top = self.player_pos + Vec2::new(14.0, 52.0);
        let sp = frame.camera.world_to_screen(player_top);
        let world_labels = frame.canvas(2);
        world_labels.text(
            sp.x - 20.0,
            sp.y,
            "Player",
            10.0,
            Color::new(1.0, 1.0, 1.0, 0.7),
        );
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
