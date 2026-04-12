use rengine::*;

const PLAYER_SPEED: f32 = 200.0;
const PLAYER_SIZE: f32 = 20.0;
const PLAYER_BODY_ID: BodyId = 0;

struct TriggerDemo {
    white: TextureId,
    player_pos: Vec2,
    triggers: TriggerSystem,
    zone_checkpoint: TriggerZoneId,
    zone_pickup: TriggerZoneId,
    zone_damage: TriggerZoneId,
    zone_layered: TriggerZoneId,
    score: u32,
    checkpoint_flash: f32,
    damage_flash: f32,
    pickup_collected: bool,
    layered_overlap: bool,
}

impl Game for TriggerDemo {
    fn new(engine: &mut Engine) -> Self {
        let white = engine.create_color_texture(1, 1, Color::WHITE);

        let mut triggers = TriggerSystem::new();

        // Checkpoint zone (green) — detects any body
        let zone_checkpoint =
            triggers.add_zone(TriggerZone::new(Rect::new(-300.0, -50.0, 80.0, 100.0)));

        // Pickup zone (yellow) — one-time collection
        let zone_pickup = triggers.add_zone(TriggerZone::new(Rect::new(100.0, 100.0, 40.0, 40.0)));

        // Damage zone (red) — continuous overlap
        let zone_damage =
            triggers.add_zone(TriggerZone::new(Rect::new(200.0, -150.0, 120.0, 60.0)));

        // Layered zone (purple) — only interacts with PLAYER layer
        let zone_layered = triggers.add_zone(
            TriggerZone::new(Rect::new(-100.0, -200.0, 100.0, 100.0)).with_layer(
                CollisionLayer::new(CollisionLayer::TRIGGER, CollisionLayer::PLAYER),
            ),
        );

        Self {
            white,
            player_pos: Vec2::ZERO,
            triggers,
            zone_checkpoint,
            zone_pickup,
            zone_damage,
            zone_layered,
            score: 0,
            checkpoint_flash: 0.0,
            damage_flash: 0.0,
            pickup_collected: false,
            layered_overlap: false,
        }
    }

    fn update(&mut self, engine: &Engine) {
        let input = engine.input();
        let dt = engine.dt();

        // Move player
        let mut dir = Vec2::ZERO;
        if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if input.is_key_down(KeyCode::KeyS) || input.is_key_down(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if dir != Vec2::ZERO {
            self.player_pos += dir.normalize() * PLAYER_SPEED * dt;
        }

        // Player body: center the rect on player_pos
        let hs = PLAYER_SIZE / 2.0;
        let player_rect = Rect::new(
            self.player_pos.x - hs,
            self.player_pos.y - hs,
            PLAYER_SIZE,
            PLAYER_SIZE,
        );
        let player_layer = CollisionLayer::new(
            CollisionLayer::PLAYER,
            CollisionLayer::PLAYER | CollisionLayer::TRIGGER,
        );

        // Tick trigger system
        self.triggers
            .tick(&[(PLAYER_BODY_ID, player_rect, player_layer)]);

        // Process events
        let events: Vec<_> = self.triggers.events().collect();
        for (zone_id, _body_id, event) in events {
            if zone_id == self.zone_checkpoint && event == OverlapEvent::Enter {
                self.checkpoint_flash = 1.0;
                self.score += 10;
            }

            if zone_id == self.zone_pickup && event == OverlapEvent::Enter && !self.pickup_collected
            {
                self.pickup_collected = true;
                self.score += 50;
                // Disable the zone after collection
                self.triggers.zone_mut(self.zone_pickup).enabled = false;
            }

            if zone_id == self.zone_damage && event == OverlapEvent::Stay {
                self.damage_flash = 0.5;
            }

            if zone_id == self.zone_layered {
                self.layered_overlap = event != OverlapEvent::Exit;
            }
        }

        // Decay flashes
        self.checkpoint_flash = (self.checkpoint_flash - dt * 2.0).max(0.0);
        self.damage_flash = (self.damage_flash - dt * 2.0).max(0.0);
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.08, 0.08, 0.12, 1.0);
        let white = self.white;

        // Draw trigger zones
        let zones: &[(TriggerZoneId, Color)] = &[
            (self.zone_checkpoint, Color::new(0.1, 0.5, 0.2, 0.4)),
            (self.zone_pickup, Color::new(0.8, 0.7, 0.1, 0.4)),
            (self.zone_damage, Color::new(0.7, 0.1, 0.1, 0.4)),
            (self.zone_layered, Color::new(0.5, 0.2, 0.7, 0.4)),
        ];

        for &(zone_id, base_color) in zones {
            let zone = self.triggers.zone(zone_id);
            if !zone.enabled {
                continue;
            }
            let r = &zone.rect;

            // Brighten on overlap
            let color = if self.triggers.overlapping(zone_id, PLAYER_BODY_ID) {
                Color::new(
                    (base_color.r * 2.0).min(1.0),
                    (base_color.g * 2.0).min(1.0),
                    (base_color.b * 2.0).min(1.0),
                    0.7,
                )
            } else {
                base_color
            };

            frame.draw_sprite(
                DrawParams::new(white, Vec2::new(r.x, r.y), Vec2::new(r.width, r.height))
                    .with_color(color),
            );
        }

        // Draw player
        let hs = PLAYER_SIZE / 2.0;
        let player_color = if self.damage_flash > 0.0 {
            Color::new(1.0, 0.3, 0.3, 1.0)
        } else if self.checkpoint_flash > 0.0 {
            Color::new(0.3, 1.0, 0.5, 1.0)
        } else {
            Color::new(0.2, 0.8, 1.0, 1.0)
        };

        frame.draw_sprite(
            DrawParams::new(
                white,
                Vec2::new(self.player_pos.x - hs, self.player_pos.y - hs),
                Vec2::new(PLAYER_SIZE, PLAYER_SIZE),
            )
            .with_color(player_color)
            .with_z_order(10),
        );

        // HUD
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let canvas = frame.canvas(0);
        canvas.text(
            -hw + 8.0,
            hh - 8.0 - 11.0,
            &format!(
                "WASD:Move  Score:{}  {}  {}",
                self.score,
                if self.pickup_collected {
                    "Pickup:Collected"
                } else {
                    "Pickup:Active"
                },
                if self.layered_overlap {
                    "Layered:Inside"
                } else {
                    "Layered:Outside"
                }
            ),
            11.0,
            Color::WHITE,
            (sw, sh),
            engine.font_atlas(),
        );

        // Zone labels
        canvas.text(
            -hw + 8.0,
            hh - 24.0 - 9.0,
            "Green=Checkpoint  Yellow=Pickup  Red=Damage  Purple=Layered(PLAYER only)",
            9.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
            (sw, sh),
            engine.font_atlas(),
        );
    }
}

fn main() {
    rengine::run::<TriggerDemo>(EngineConfig {
        title: "Feature: Trigger Volumes".into(),
        width: 800,
        height: 600,
        show_fps: false,
        ..Default::default()
    })
    .unwrap();
}
