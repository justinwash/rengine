use rengine::{RollbackSession, TextureId};
use serde::{Deserialize, Serialize};

use crate::{FIGHTER_FOOT_OFFSET, FIGHTER_W, GROUND_Y, MAX_HP};

#[repr(C)]
#[derive(
    Copy, Clone, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct FightInput {
    pub flags: u8,
    pub _pad: [u8; 3],
}

impl FightInput {
    pub const LEFT: u8 = 0b00_0001;
    pub const RIGHT: u8 = 0b00_0010;
    pub const JUMP: u8 = 0b00_0100;
    pub const CROUCH: u8 = 0b00_1000;
    pub const PUNCH: u8 = 0b01_0000;
    pub const KICK: u8 = 0b10_0000;

    pub fn left(self) -> bool {
        self.flags & Self::LEFT != 0
    }
    pub fn right(self) -> bool {
        self.flags & Self::RIGHT != 0
    }
    pub fn jump(self) -> bool {
        self.flags & Self::JUMP != 0
    }
    pub fn crouch(self) -> bool {
        self.flags & Self::CROUCH != 0
    }
    pub fn punch(self) -> bool {
        self.flags & Self::PUNCH != 0
    }
    pub fn kick(self) -> bool {
        self.flags & Self::KICK != 0
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FighterState {
    Idle,
    Walking,
    Jumping,
    Punching,
    Kicking,
    Blocking,
    HitStun,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Facing {
    Right,
    Left,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FighterData {
    pub x: f32,
    pub y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub hp: i32,
    pub facing: Facing,
    pub state: FighterState,
    pub state_timer: f32,
    pub attack_hit_connected: bool,
    pub wins: u32,
}

impl FighterData {
    pub fn rect_x(&self) -> f32 {
        self.x - FIGHTER_W / 2.0
    }
    pub fn rect_y(&self) -> f32 {
        self.y - FIGHTER_FOOT_OFFSET
    }

    pub fn is_on_ground(&self) -> bool {
        self.y <= GROUND_Y + 1.0
    }

    pub fn can_act(&self) -> bool {
        matches!(self.state, FighterState::Idle | FighterState::Walking)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Spark {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FightSnapshot {
    pub p1: FighterData,
    pub p2: FighterData,
    pub round_pause: f32,
    pub round_number: u32,
    pub sparks: Vec<Spark>,
}

pub struct FightSim {
    pub p1: FighterData,
    pub p2: FighterData,
    pub round_pause: f32,
    pub round_number: u32,
    pub sparks: Vec<Spark>,
}

impl FightSim {
    pub fn new() -> Self {
        Self {
            p1: FighterData {
                x: 250.0,
                y: GROUND_Y,
                vel_x: 0.0,
                vel_y: 0.0,
                hp: MAX_HP,
                facing: Facing::Right,
                state: FighterState::Idle,
                state_timer: 0.0,
                attack_hit_connected: false,
                wins: 0,
            },
            p2: FighterData {
                x: 550.0,
                y: GROUND_Y,
                vel_x: 0.0,
                vel_y: 0.0,
                hp: MAX_HP,
                facing: Facing::Left,
                state: FighterState::Idle,
                state_timer: 0.0,
                attack_hit_connected: false,
                wins: 0,
            },
            round_pause: 0.0,
            round_number: 1,
            sparks: Vec::new(),
        }
    }
}

pub struct FighterTextures {
    pub idle: TextureId,
    pub punch: TextureId,
    pub kick: TextureId,
    pub block: TextureId,
    pub hit: TextureId,
}

pub struct FightGame {
    pub sim: FightSim,
    pub session: RollbackSession<FightInput>,
    pub p1_tex: FighterTextures,
    pub p2_tex: FighterTextures,
    pub floor_tex: TextureId,
    pub white_tex: TextureId,
    pub demo_mode: bool,
    pub demo_frame: u32,
    pub printed_result: bool,
}
