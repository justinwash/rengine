use rengine::{MeshId, RollbackSession, Vertex3D};
use serde::{Deserialize, Serialize};

use crate::{MAX_HP, PLAYER_HEIGHT};

pub const LOOK_SCALE: f32 = 4096.0;

#[repr(C)]
#[derive(
    Copy, Clone, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct FpsInput {
    pub flags: u8,
    pub _pad: u8,
    pub look_dx: i16,
    pub look_dy: i16,
    pub _pad2: [u8; 2],
}

impl FpsInput {
    pub const FORWARD: u8 = 1;
    pub const BACK: u8 = 2;
    pub const LEFT: u8 = 4;
    pub const RIGHT: u8 = 8;
    pub const JUMP: u8 = 16;
    pub const SHOOT: u8 = 32;

    pub fn forward(self) -> bool {
        self.flags & Self::FORWARD != 0
    }
    pub fn back(self) -> bool {
        self.flags & Self::BACK != 0
    }
    pub fn left(self) -> bool {
        self.flags & Self::LEFT != 0
    }
    pub fn right(self) -> bool {
        self.flags & Self::RIGHT != 0
    }
    pub fn jump(self) -> bool {
        self.flags & Self::JUMP != 0
    }
    pub fn shoot(self) -> bool {
        self.flags & Self::SHOOT != 0
    }

    pub fn encode_look(dx: f32, dy: f32) -> (i16, i16) {
        (
            (dx * LOOK_SCALE).clamp(-32767.0, 32767.0) as i16,
            (dy * LOOK_SCALE).clamp(-32767.0, 32767.0) as i16,
        )
    }

    pub fn decode_look_dx(self) -> f32 {
        self.look_dx as f32 / LOOK_SCALE
    }
    pub fn decode_look_dy(self) -> f32 {
        self.look_dy as f32 / LOOK_SCALE
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vel_y: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub hp: i32,
    pub score: u32,
    pub respawn_timer: f32,
    pub shoot_cooldown: f32,
}

impl PlayerData {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }

    pub fn new(x: f32, z: f32, yaw: f32) -> Self {
        Self {
            x,
            y: PLAYER_HEIGHT,
            z,
            vel_y: 0.0,
            yaw,
            pitch: 0.0,
            on_ground: true,
            hp: MAX_HP,
            score: 0,
            respawn_timer: 0.0,
            shoot_cooldown: 0.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectileData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub life: f32,
    pub owner: u8,
    pub alive: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DoorState {
    pub offset: f32,
    pub open: bool,
}

#[derive(Clone)]
pub struct DoorDef {
    pub x: f32,
    pub z: f32,
    pub slides_x: bool,
    pub trigger_radius: f32,
    pub wall: CollisionWall,
}

#[derive(Clone)]
pub struct CollisionWall {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
}

impl CollisionWall {
    pub fn new(x0: f32, z0: f32, x1: f32, z1: f32) -> Self {
        Self { x0, z0, x1, z1 }
    }

    pub fn push_out(&self, px: f32, pz: f32, radius: f32) -> (f32, f32) {
        let dx = (self.x1 - self.x0).abs();
        let dz = (self.z1 - self.z0).abs();

        if dx > dz {
            let z_wall = self.z0;
            let x_min = self.x0.min(self.x1);
            let x_max = self.x0.max(self.x1);
            if px >= x_min - radius && px <= x_max + radius {
                let dist = pz - z_wall;
                if dist.abs() < radius {
                    let sign = if dist >= 0.0 { 1.0 } else { -1.0 };
                    return (px, z_wall + sign * radius);
                }
            }
        } else {
            let x_wall = self.x0;
            let z_min = self.z0.min(self.z1);
            let z_max = self.z0.max(self.z1);
            if pz >= z_min - radius && pz <= z_max + radius {
                let dist = px - x_wall;
                if dist.abs() < radius {
                    let sign = if dist >= 0.0 { 1.0 } else { -1.0 };
                    return (x_wall + sign * radius, pz);
                }
            }
        }
        (px, pz)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FpsSnapshot {
    pub players: Vec<PlayerData>,
    pub projectiles: Vec<ProjectileData>,
    pub door_states: Vec<DoorState>,
}

pub struct FpsSim {
    pub players: Vec<PlayerData>,
    pub projectiles: Vec<ProjectileData>,
    pub door_states: Vec<DoorState>,

    pub walls: Vec<CollisionWall>,
    pub door_defs: Vec<DoorDef>,
    pub spawn_points: [[f32; 3]; 2],
}

impl FpsSim {
    pub fn new(
        walls: Vec<CollisionWall>,
        door_defs: Vec<DoorDef>,
        spawn_points: [[f32; 3]; 2],
    ) -> Self {
        let door_states: Vec<DoorState> = door_defs
            .iter()
            .map(|_| DoorState {
                offset: 0.0,
                open: false,
            })
            .collect();

        let p0_yaw = 0.0_f32;
        let p1_yaw = std::f32::consts::PI;

        Self {
            players: vec![
                PlayerData::new(spawn_points[0][0], spawn_points[0][2], p0_yaw),
                PlayerData::new(spawn_points[1][0], spawn_points[1][2], p1_yaw),
            ],
            projectiles: Vec::new(),
            door_states,
            walls,
            door_defs,
            spawn_points,
        }
    }
}

pub struct FpsMpGame {
    pub sim: FpsSim,
    pub session: RollbackSession<FpsInput>,

    pub level_verts: Vec<Vertex3D>,
    pub level_idxs: Vec<u32>,
    pub door_meshes: Vec<MeshId>,
    pub player_mesh: MeshId,
    pub projectile_mesh: MeshId,

    pub demo_mode: bool,
    pub demo_frame: u32,
    pub printed_result: bool,
}
