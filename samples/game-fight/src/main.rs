pub mod bot;
mod input;
mod render;
pub mod sim;
pub mod state;

use rengine::pixelart;
use rengine::{
    Color, Engine, EngineConfig, Frame, RollbackConfig, RollbackGame,
    SessionMode,
};
use state::{FightGame, FightInput, FightSim, FighterTextures};


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


impl RollbackGame for FightGame {
    type Input = FightInput;

    fn new(engine: &mut Engine) -> Self {

        let body1 = Color::from_rgba8(50, 80, 180, 255);
        let skin = Color::from_rgba8(230, 185, 140, 255);
        let belt1 = Color::from_rgba8(30, 30, 30, 255);

        let (w, h, pix) = pixelart::fighter_idle(body1, skin, belt1);
        let tex_idle_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_punch(body1, skin, belt1);
        let tex_punch_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_kick(body1, skin, belt1);
        let tex_kick_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_block(body1, skin, belt1);
        let tex_block_1 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_hit(body1, skin, belt1);
        let tex_hit_1 = engine.create_texture(w, h, &pix);


        let body2 = Color::from_rgba8(190, 50, 50, 255);
        let belt2 = Color::from_rgba8(200, 180, 30, 255);

        let (w, h, pix) = pixelart::fighter_idle(body2, skin, belt2);
        let tex_idle_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_punch(body2, skin, belt2);
        let tex_punch_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_kick(body2, skin, belt2);
        let tex_kick_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_block(body2, skin, belt2);
        let tex_block_2 = engine.create_texture(w, h, &pix);
        let (w, h, pix) = pixelart::fighter_hit(body2, skin, belt2);
        let tex_hit_2 = engine.create_texture(w, h, &pix);


        let (w, h, pix) = pixelart::dojo_floor_tile();
        let floor_tex = engine.create_texture(w, h, &pix);

        let white_tex = engine.white_texture();

        let demo_mode = std::env::args().any(|a| a == "--demo");

        FightGame {
            sim: FightSim::new(),
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
        }
    }

    fn sample_local_input(&self, engine: &Engine, player: usize) -> FightInput {
        input::sample(self, engine, player)
    }

    fn advance(&mut self, inputs: &[FightInput]) {
        self.sim.advance(inputs);
        if self.demo_mode {
            self.demo_frame += 1;
        }
    }

    fn save(&self) -> Vec<u8> {
        self.sim.save()
    }

    fn load(&mut self, data: &[u8]) {
        self.sim.load(data);
    }

    fn render(&self, engine: &Engine, frame: &mut Frame) {
        render::draw(self, engine, frame);
    }
}


fn main() {
    let demo = std::env::args().any(|a| a == "--demo");

    rengine::run_rollback::<FightGame>(
        EngineConfig {
            title: if demo {
                "Rengine Fighter — SYNC TEST DEMO".into()
            } else {
                "Rengine Fighter".into()
            },
            width: SCREEN_W,
            height: SCREEN_H,
            ..Default::default()
        },
        RollbackConfig {
            num_players: 2,
            fps: 60,
            mode: if demo {
                SessionMode::SyncTest { check_distance: 7 }
            } else {
                SessionMode::Local
            },
            ..Default::default()
        },
    )
    .unwrap();
}
