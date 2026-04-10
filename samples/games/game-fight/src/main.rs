pub mod bot;
mod input;
mod render;
pub mod sim;
pub mod state;

use std::path::PathBuf;

use rengine::{
    AudioBus, AxisMapping, Binding, Engine, EngineConfig, Frame, Game, GamepadAxis, GamepadButton,
    KeyCode, OnlineConfig, RollbackConfig, RollbackSession, SessionMode,
};
use state::{FightGame, FightSim, FighterTextures};

pub const SCREEN_W: u32 = 800;
pub const SCREEN_H: u32 = 480;

pub const GROUND_Y: f32 = 96.0;
pub const STAGE_LEFT: f32 = 40.0;
pub const STAGE_RIGHT: f32 = 760.0;

pub const FIGHTER_W: f32 = 96.0;
pub const FIGHTER_H: f32 = 144.0;

pub const FIGHTER_FOOT_OFFSET: f32 = 24.0;

pub const WALK_SPEED: f32 = 250.0;
pub const JUMP_VEL: f32 = 600.0;
pub const GRAVITY: f32 = -1500.0;

pub const PUNCH_RANGE: f32 = 90.0;
pub const KICK_RANGE: f32 = 115.0;
pub const PUNCH_DAMAGE: i32 = 8;
pub const KICK_DAMAGE: i32 = 12;
pub const BLOCK_REDUCTION: f32 = 0.25;

pub const ATTACK_DURATION: f32 = 0.25;
pub const HIT_STUN: f32 = 0.30;
pub const KNOCKBACK_SPEED: f32 = 300.0;

pub const MAX_HP: i32 = 100;

pub const ROUND_WIN_PAUSE: f32 = 2.0;

pub const FIXED_DT: f32 = 1.0 / 60.0;

impl Game for FightGame {
    fn new(engine: &mut Engine) -> Self {
        engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

        let actions = engine.actions_mut();
        for (prefix, keys) in [
            ("p1", [KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyF, KeyCode::KeyG]),
            ("p2", [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::KeyK, KeyCode::KeyL]),
        ] {
            actions.bind_axis(
                &format!("{prefix}_move_x"),
                AxisMapping {
                    positive: vec![Binding::Key(keys[1])],
                    negative: vec![Binding::Key(keys[0])],
                    gamepad_axis: Some(GamepadAxis::LeftStickX),
                },
            );
            actions.bind_axis(
                &format!("{prefix}_move_y"),
                AxisMapping {
                    positive: vec![Binding::Key(keys[2])],
                    negative: vec![Binding::Key(keys[3])],
                    gamepad_axis: Some(GamepadAxis::LeftStickY),
                },
            );
            actions.bind(&format!("{prefix}_jump"), Binding::Key(keys[2]));
            actions.bind(&format!("{prefix}_jump"), Binding::GamepadButton(GamepadButton::DPadUp));
            actions.bind(&format!("{prefix}_crouch"), Binding::Key(keys[3]));
            actions.bind(&format!("{prefix}_crouch"), Binding::GamepadButton(GamepadButton::DPadDown));
            actions.bind(&format!("{prefix}_punch"), Binding::Key(keys[4]));
            actions.bind(&format!("{prefix}_punch"), Binding::GamepadButton(GamepadButton::South));
            actions.bind(&format!("{prefix}_kick"), Binding::Key(keys[5]));
            actions.bind(&format!("{prefix}_kick"), Binding::GamepadButton(GamepadButton::West));
        }

        let blue_sheet = engine
            .load_sprite_sheet("fighter_blue.png", 96, 144)
            .expect("failed to load blue fighter sprite sheet");
        let red_sheet = engine
            .load_sprite_sheet("fighter_red.png", 96, 144)
            .expect("failed to load red fighter sprite sheet");
        let floor_tex = engine
            .load_texture("dojo_floor.png")
            .expect("failed to load dojo floor texture")
            .texture();
        let hit_sfx = engine
            .load_audio("fight_hit.wav")
            .expect("failed to load fight hit audio");
        let theme = engine
            .load_audio("fight_theme.wav")
            .expect("failed to load fight theme audio");

        let white_tex = engine.white_texture();

        engine.set_master_volume(0.9);
        engine.set_audio_bus_volume(AudioBus::Music, 0.35);
        engine.set_audio_bus_volume(AudioBus::Effects, 0.9);
        let _ = engine.play_music_with_volume(&theme, 1.0);

        let demo_mode = std::env::args().any(|a| a == "--demo");
        let online = std::env::args().any(|a| a == "--online");
        let headless = std::env::args().any(|a| a == "--headless");
        let port: u16 = arg_value("--port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(7000);
        let remote: String = arg_value("--remote").unwrap_or_else(|| "127.0.0.1:7001".to_string());
        let player: usize = arg_value("--player")
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        let max_frames: Option<u32> = arg_value("--frames").and_then(|f| f.parse().ok());

        let mode = if online {
            SessionMode::Online(OnlineConfig {
                local_port: port,
                remote_addr: remote,
                local_player: player,
            })
        } else if demo_mode {
            SessionMode::SyncTest { check_distance: 7 }
        } else {
            SessionMode::Local
        };

        let session = RollbackSession::new(
            RollbackConfig {
                num_players: 2,
                fps: 60,
                mode,
                max_frames,
                ..Default::default()
            },
            headless,
        )
        .expect("failed to create rollback session");

        FightGame {
            sim: FightSim::new(),
            session,
            p1_tex: FighterTextures {
                texture: blue_sheet.texture,
                idle: blue_sheet.uv_rect(0, 0),
                punch: blue_sheet.uv_rect(1, 0),
                kick: blue_sheet.uv_rect(2, 0),
                block: blue_sheet.uv_rect(3, 0),
                hit: blue_sheet.uv_rect(4, 0),
            },
            p2_tex: FighterTextures {
                texture: red_sheet.texture,
                idle: red_sheet.uv_rect(0, 0),
                punch: red_sheet.uv_rect(1, 0),
                kick: red_sheet.uv_rect(2, 0),
                block: red_sheet.uv_rect(3, 0),
                hit: red_sheet.uv_rect(4, 0),
            },
            floor_tex,
            white_tex,
            demo_mode,
            demo_frame: 0,
            printed_result: false,
            hit_sfx,
        }
    }

    fn update(&mut self, engine: &Engine) {
        let prev_p1_hp = self.sim.p1.hp;
        let prev_p2_hp = self.sim.p2.hp;
        let prev_round_pause = self.sim.round_pause;
        let num = self.session.num_players();
        let frame = self.demo_frame;
        let demo = self.demo_mode;

        let mut inputs = Vec::with_capacity(num);
        for p in 0..num {
            let input = if demo {
                let (me, opp) = if p == 0 {
                    (&self.sim.p1, &self.sim.p2)
                } else {
                    (&self.sim.p2, &self.sim.p1)
                };
                bot::bot_input(me, opp, frame, p as u32)
            } else {
                input::sample_from_engine(engine, p)
            };
            inputs.push(input);
        }

        let ticked = self.session.update(engine.dt(), &inputs, &mut self.sim);
        if ticked && self.demo_mode {
            self.demo_frame += 1;
        }

        if ticked {
            if self.sim.p1.hp < prev_p1_hp || self.sim.p2.hp < prev_p2_hp {
                let _ = engine.play_sound_on_bus(AudioBus::Effects, &self.hit_sfx, 1.0);
            }

            if prev_round_pause <= 0.0 && self.sim.round_pause > 0.0 {
                engine.pause_music();
                let _ = engine.play_sound_on_bus(AudioBus::Ui, &self.hit_sfx, 0.55);
            } else if prev_round_pause > 0.0 && self.sim.round_pause <= 0.0 && self.sim.winner().is_none() {
                engine.resume_music();
            }
        }

        if !self.printed_result && self.session.max_frames_reached() {
            if self.session.desync_detected() {
                println!("DESYNC");
            } else if let Some(cf) = self.session.confirmed_frame() {
                println!("OK {cf}");
            } else {
                let data = self.sim.save();
                let checksum = rengine::fletcher64(&data);
                println!("CHECKSUM {checksum:016x}");
            }
            self.printed_result = true;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        render::draw(self, engine, frame);
    }

    fn should_exit(&self) -> bool {
        self.printed_result || (self.sim.winner().is_some() && self.sim.round_pause <= 0.0)
    }
}

fn arg_value(flag: &str) -> Option<String> {
    std::env::args().skip_while(|a| a != flag).nth(1)
}

fn main() {
    let online = std::env::args().any(|a| a == "--online");
    let demo = std::env::args().any(|a| a == "--demo");
    let headless = std::env::args().any(|a| a == "--headless");
    let player: usize = arg_value("--player")
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);

    let title = if online {
        format!("Rengine Fighter — P{}", player + 1)
    } else if demo {
        "Rengine Fighter — SYNC TEST DEMO".into()
    } else {
        "Rengine Fighter".into()
    };

    rengine::run::<FightGame>(EngineConfig {
        title,
        width: SCREEN_W,
        height: SCREEN_H,
        headless,
        ..Default::default()
    })
    .unwrap();
}
