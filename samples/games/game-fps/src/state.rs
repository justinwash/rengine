use rengine::{AudioClip, MeshId, Vec3, Vertex3D};

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

pub struct Door {
    pub x: f32,
    pub z: f32,
    pub slides_x: bool,
    pub slide_sign: f32,
    pub offset: f32,
    pub open: bool,
    pub mesh: MeshId,
    pub trigger_radius: f32,
    pub wall: CollisionWall,
}

pub struct Projectile {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub alive: bool,
    pub visible: bool,
    pub collides: bool,
    pub pair_id: u32,
}

pub struct Enemy {
    pub pos: Vec3,
    pub alive: bool,
    pub mesh: MeshId,
}

pub struct FpsGame {
    pub level_verts: Vec<Vertex3D>,
    pub level_idxs: Vec<u32>,
    pub walls: Vec<CollisionWall>,
    pub doors: Vec<Door>,

    pub cam_yaw: f32,
    pub cam_pitch: f32,
    pub player_pos: Vec3,
    pub player_vel_y: f32,
    pub on_ground: bool,

    pub projectiles: Vec<Projectile>,
    pub next_projectile_pair_id: u32,
    pub projectile_mesh: MeshId,
    pub viewmodel_mesh: MeshId,

    pub enemies: Vec<Enemy>,
    pub score: u32,
    pub shoot_sfx: AudioClip,
    pub hit_sfx: AudioClip,
    pub jump_sfx: AudioClip,
}
