pub mod bot;
mod input;
mod level;
mod render;
pub mod sim;
pub mod state;

use rengine::{
    Engine3D, EngineConfig, Frame3D, Game3D, OnlineConfig, RollbackConfig, RollbackSession,
    SessionMode,
};
use state::{FpsMpGame, FpsInput, FpsSim};

pub const MOVE_SPEED: f32 = 6.0;
pub const MOUSE_SENSITIVITY: f32 = 0.002;
pub const WALL_HEIGHT: f32 = 3.0;
pub const PLAYER_HEIGHT: f32 = 1.7;
pub const PLAYER_RADIUS: f32 = 0.3;
pub const GRAVITY: f32 = 15.0;
pub const JUMP_VEL: f32 = 6.0;
pub const PROJECTILE_SPEED: f32 = 30.0;
pub const PROJECTILE_LIFETIME: f32 = 3.0;
pub const DOOR_OPEN_SPEED: f32 = 3.0;
pub const MAX_HP: i32 = 100;
pub const RESPAWN_TIME: f32 = 3.0;
pub const SHOOT_COOLDOWN: f32 = 0.3;
pub const HIT_RADIUS: f32 = 0.5;
pub const FIXED_DT: f32 = 1.0 / 60.0;

impl Game3D for FpsMpGame {
    fn new(engine: &mut Engine3D) -> Self {
        let demo_mode = std::env::args().any(|a| a == "--demo");
        let online = std::env::args().any(|a| a == "--online");
        let headless = std::env::args().any(|a| a == "--headless");
        let port: u16 = arg_value("--port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(7000);
        let remote: String =
            arg_value("--remote").unwrap_or_else(|| "127.0.0.1:7001".to_string());
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

        let build = level::build(engine);
        let sim = FpsSim::new(build.walls, build.door_defs, build.spawn_points);

        FpsMpGame {
            sim,
            session,
            level_verts: build.level_verts,
            level_idxs: build.level_idxs,
            door_meshes: build.door_meshes,
            player_mesh: build.player_mesh,
            projectile_mesh: build.projectile_mesh,
            demo_mode,
            demo_frame: 0,
            printed_result: false,
        }
    }

    fn update(&mut self, engine: &Engine3D) {
        let num = self.session.num_players();
        let frame = self.demo_frame;

        let mut inputs = Vec::with_capacity(num);
        for p in 0..num {
            let inp = if self.demo_mode {
                bot::bot_input(&self.sim, frame, p as u32)
            } else if p == self.session.local_player() {
                input::sample_from_engine(engine)
            } else {
                FpsInput::default()
            };
            inputs.push(inp);
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

    fn render(&mut self, engine: &Engine3D, frame: &mut Frame3D) {
        render::draw(self, engine, frame);
    }

    fn should_exit(&self) -> bool {
        self.printed_result
    }
}

fn arg_value(flag: &str) -> Option<String> {
    std::env::args().skip_while(|a| a != flag).nth(1)
}

fn main() {
    let online = std::env::args().any(|a| a == "--online");
    let headless = std::env::args().any(|a| a == "--headless");
    let player: usize = arg_value("--player")
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);

    let title = if online {
        format!("Rengine FPS MP — P{}", player + 1)
    } else {
        "Rengine FPS MP".into()
    };

    rengine::run3d::<FpsMpGame>(EngineConfig {
        title,
        width: 1024,
        height: 768,
        headless,
        ..Default::default()
    })
    .unwrap();
}
