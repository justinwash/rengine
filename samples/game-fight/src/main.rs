mod art;
pub mod bot;
mod input;
mod render;
pub mod sim;
pub mod state;

use rengine::{
    Color, Engine, EngineConfig, Frame, Game, OnlineConfig, RollbackConfig, RollbackSession,
    SessionMode,
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
        let body1 = Color::from_rgba8(50, 80, 180, 255);
        let skin = Color::from_rgba8(230, 185, 140, 255);
        let belt1 = Color::from_rgba8(30, 30, 30, 255);

        let (w, h, pix) = art::fighter_idle(body1, skin, belt1);
        let tex_idle_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_punch(body1, skin, belt1);
        let tex_punch_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_kick(body1, skin, belt1);
        let tex_kick_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_block(body1, skin, belt1);
        let tex_block_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_hit(body1, skin, belt1);
        let tex_hit_1 = engine.create_texture(w, h, &pix);

        let body2 = Color::from_rgba8(190, 50, 50, 255);
        let belt2 = Color::from_rgba8(200, 180, 30, 255);

        let (w, h, pix) = art::fighter_idle(body2, skin, belt2);
        let tex_idle_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_punch(body2, skin, belt2);
        let tex_punch_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_kick(body2, skin, belt2);
        let tex_kick_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_block(body2, skin, belt2);
        let tex_block_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = art::fighter_hit(body2, skin, belt2);
        let tex_hit_2 = engine.create_texture(w, h, &pix);

        let (w, h, pix) = art::dojo_floor_tile();
        let floor_tex = engine.create_texture(w, h, &pix);

        let white_tex = engine.white_texture();

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
                idle: tex_idle_1,
                punch: tex_punch_1,
                kick: tex_kick_1,
                block: tex_block_1,
                hit: tex_hit_1,
            },
            p2_tex: FighterTextures {
                idle: tex_idle_2,
                punch: tex_punch_2,
                kick: tex_kick_2,
                block: tex_block_2,
                hit: tex_hit_2,
            },
            floor_tex,
            white_tex,
            demo_mode,
            demo_frame: 0,
            printed_result: false,
        }
    }

    fn update(&mut self, engine: &Engine) {
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
