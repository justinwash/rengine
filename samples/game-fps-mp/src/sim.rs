use rengine::Vec3;

use crate::state::{FpsInput, FpsSim, FpsSnapshot, ProjectileData};
use crate::{
    DOOR_OPEN_SPEED, FIXED_DT, GRAVITY, HIT_RADIUS, JUMP_VEL, MAX_HP, MOVE_SPEED, PLAYER_HEIGHT,
    PLAYER_RADIUS, PROJECTILE_LIFETIME, PROJECTILE_SPEED, RESPAWN_TIME, SHOOT_COOLDOWN,
    WALL_HEIGHT,
};

impl FpsSim {
    pub fn advance(&mut self, inputs: &[FpsInput]) {
        let dt = FIXED_DT;

        for i in 0..self.players.len() {
            let sp = self.spawn_points[i];

            if !self.players[i].alive() {
                self.players[i].respawn_timer -= dt;
                if self.players[i].respawn_timer <= 0.0 {
                    self.players[i].x = sp[0];
                    self.players[i].y = sp[1];
                    self.players[i].z = sp[2];
                    self.players[i].vel_y = 0.0;
                    self.players[i].hp = MAX_HP;
                    self.players[i].on_ground = true;
                    self.players[i].shoot_cooldown = 0.0;
                    self.players[i].yaw = if i == 0 { 0.0 } else { std::f32::consts::PI };
                    self.players[i].pitch = 0.0;
                }
                continue;
            }

            let input = inputs[i];

            self.players[i].yaw += input.decode_look_dx();
            let max_pitch = 89.0_f32.to_radians();
            self.players[i].pitch =
                (self.players[i].pitch + input.decode_look_dy()).clamp(-max_pitch, max_pitch);

            let yaw = self.players[i].yaw;
            let forward = Vec3::new(yaw.sin(), 0.0, -yaw.cos());
            let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());
            let mut dir = Vec3::ZERO;
            if input.forward() {
                dir += forward;
            }
            if input.back() {
                dir -= forward;
            }
            if input.right() {
                dir += right;
            }
            if input.left() {
                dir -= right;
            }
            if dir.length_squared() > 0.0 {
                dir = dir.normalize();
            }

            let mut new_x = self.players[i].x + dir.x * MOVE_SPEED * dt;
            let mut new_z = self.players[i].z + dir.z * MOVE_SPEED * dt;

            for wall in &self.walls {
                let (px, pz) = wall.push_out(new_x, new_z, PLAYER_RADIUS);
                new_x = px;
                new_z = pz;
            }
            for (di, door_def) in self.door_defs.iter().enumerate() {
                if self.door_states[di].offset < 1.5 {
                    let (px, pz) = door_def.wall.push_out(new_x, new_z, PLAYER_RADIUS);
                    new_x = px;
                    new_z = pz;
                }
            }
            self.players[i].x = new_x;
            self.players[i].z = new_z;

            if input.jump() && self.players[i].on_ground {
                self.players[i].vel_y = JUMP_VEL;
                self.players[i].on_ground = false;
            }
            self.players[i].vel_y -= GRAVITY * dt;
            self.players[i].y += self.players[i].vel_y * dt;
            if self.players[i].y <= PLAYER_HEIGHT {
                self.players[i].y = PLAYER_HEIGHT;
                self.players[i].vel_y = 0.0;
                self.players[i].on_ground = true;
            }

            self.players[i].shoot_cooldown -= dt;
            if self.players[i].shoot_cooldown < 0.0 {
                self.players[i].shoot_cooldown = 0.0;
            }

            if input.shoot() && self.players[i].shoot_cooldown <= 0.0 {
                let p_yaw = self.players[i].yaw;
                let p_pitch = self.players[i].pitch;
                let cam_fwd = Vec3::new(
                    p_yaw.sin() * p_pitch.cos(),
                    p_pitch.sin(),
                    -p_yaw.cos() * p_pitch.cos(),
                )
                .normalize();

                self.projectiles.push(ProjectileData {
                    x: self.players[i].x + cam_fwd.x * 0.5,
                    y: self.players[i].y + cam_fwd.y * 0.5,
                    z: self.players[i].z + cam_fwd.z * 0.5,
                    vx: cam_fwd.x * PROJECTILE_SPEED,
                    vy: cam_fwd.y * PROJECTILE_SPEED,
                    vz: cam_fwd.z * PROJECTILE_SPEED,
                    life: PROJECTILE_LIFETIME,
                    owner: i as u8,
                    alive: true,
                });
                self.players[i].shoot_cooldown = SHOOT_COOLDOWN;
            }
        }

        for proj in &mut self.projectiles {
            if !proj.alive {
                continue;
            }
            proj.x += proj.vx * dt;
            proj.y += proj.vy * dt;
            proj.z += proj.vz * dt;
            proj.life -= dt;
            if proj.life <= 0.0
                || proj.x < -0.5
                || proj.x > 28.5
                || proj.z < -0.5
                || proj.z > 12.5
                || proj.y < 0.0
                || proj.y > WALL_HEIGHT
            {
                proj.alive = false;
            }
        }

        let mut hits: Vec<(usize, usize)> = Vec::new();
        for (pi, proj) in self.projectiles.iter().enumerate() {
            if !proj.alive {
                continue;
            }
            for (pli, player) in self.players.iter().enumerate() {
                if pli as u8 == proj.owner || !player.alive() {
                    continue;
                }
                let dx = proj.x - player.x;
                let dy = proj.y - player.y;
                let dz = proj.z - player.z;
                let dist_sq = dx * dx + dy * dy + dz * dz;
                if dist_sq < HIT_RADIUS * HIT_RADIUS {
                    hits.push((pi, pli));
                    break;
                }
            }
        }

        for (proj_idx, player_idx) in hits {
            self.projectiles[proj_idx].alive = false;
            let owner = self.projectiles[proj_idx].owner as usize;
            self.players[player_idx].hp -= 25;
            if self.players[player_idx].hp <= 0 {
                self.players[player_idx].respawn_timer = RESPAWN_TIME;
                self.players[owner].score += 1;
            }
        }

        self.projectiles.retain(|p| p.alive);

        for (di, door_def) in self.door_defs.iter().enumerate() {
            let door_state = &mut self.door_states[di];
            for player in &self.players {
                if !player.alive() {
                    continue;
                }
                let dx = player.x - door_def.x;
                let dz = player.z - door_def.z;
                let dist = (dx * dx + dz * dz).sqrt();
                if dist < door_def.trigger_radius {
                    door_state.open = true;
                }
            }
            if door_state.open && door_state.offset < 2.2 {
                door_state.offset += DOOR_OPEN_SPEED * dt;
                if door_state.offset > 2.2 {
                    door_state.offset = 2.2;
                }
            }
        }
    }

    pub fn save(&self) -> Vec<u8> {
        let snapshot = FpsSnapshot {
            players: self.players.clone(),
            projectiles: self.projectiles.clone(),
            door_states: self.door_states.clone(),
        };
        bincode::serialize(&snapshot).expect("serialize fps state")
    }

    pub fn load(&mut self, data: &[u8]) {
        let snapshot: FpsSnapshot = bincode::deserialize(data).expect("deserialize fps state");
        self.players = snapshot.players;
        self.projectiles = snapshot.projectiles;
        self.door_states = snapshot.door_states;
    }
}

impl rengine::Rollbackable for FpsSim {
    type Input = FpsInput;

    fn advance(&mut self, inputs: &[FpsInput]) {
        self.advance(inputs);
    }

    fn save(&self) -> Vec<u8> {
        self.save()
    }

    fn load(&mut self, data: &[u8]) {
        self.load(data);
    }
}
