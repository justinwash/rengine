use rengine::Vec3;

use crate::state::{FpsGame, Projectile};
use crate::{
    DOOR_OPEN_SPEED, ENEMY_SIZE, GRAVITY, JUMP_VEL, MOVE_SPEED, PLAYER_HEIGHT, PLAYER_RADIUS,
    PROJECTILE_LIFETIME, PROJECTILE_SPEED, WALL_HEIGHT,
};

const PROJECTILE_RADIUS: f32 = 0.12;
const RAY_STEP: f32 = 0.1;
const VIEWMODEL_MUZZLE_OFFSET: Vec3 = Vec3::new(0.42, -0.22, -1.22);

enum ProjectileHit {
    Enemy(usize),
    World,
}

pub fn move_player(game: &mut FpsGame, move_dir: Vec3, dt: f32) {
    let mut new_x = game.player_pos.x + move_dir.x * MOVE_SPEED * dt;
    let mut new_z = game.player_pos.z + move_dir.z * MOVE_SPEED * dt;

    for wall in &game.walls {
        let (px, pz) = wall.push_out(new_x, new_z, PLAYER_RADIUS);
        new_x = px;
        new_z = pz;
    }

    for door in &game.doors {
        if door.offset < 1.5 {
            let (px, pz) = door.wall.push_out(new_x, new_z, PLAYER_RADIUS);
            new_x = px;
            new_z = pz;
        }
    }

    game.player_pos.x = new_x;
    game.player_pos.z = new_z;
}

pub fn apply_gravity(game: &mut FpsGame, jump: bool, dt: f32) {
    if jump && game.on_ground {
        game.player_vel_y = JUMP_VEL;
        game.on_ground = false;
    }

    game.player_vel_y -= GRAVITY * dt;
    game.player_pos.y += game.player_vel_y * dt;

    if game.player_pos.y <= PLAYER_HEIGHT {
        game.player_pos.y = PLAYER_HEIGHT;
        game.player_vel_y = 0.0;
        game.on_ground = true;
    }
}

pub fn update_doors(game: &mut FpsGame, dt: f32) {
    for door in &mut game.doors {
        let dx = game.player_pos.x - door.x;
        let dz = game.player_pos.z - door.z;
        let dist = (dx * dx + dz * dz).sqrt();
        if dist < door.trigger_radius {
            door.open = true;
        }
        if door.open && door.offset < 2.2 {
            door.offset += DOOR_OPEN_SPEED * dt;
            if door.offset > 2.2 {
                door.offset = 2.2;
            }
        }
    }
}

pub fn shoot(game: &mut FpsGame) {
    let cam_forward = camera_forward(game.cam_yaw, game.cam_pitch);
    let camera_origin = game.player_pos;
    let target_point = trace_aim_point(game, camera_origin, cam_forward);
    let pair_id = game.next_projectile_pair_id;
    game.next_projectile_pair_id += 1;

    let actual_origin = camera_origin;
    let actual_delta = target_point - actual_origin;
    let actual_distance = actual_delta.length().max(0.001);
    let actual_dir = actual_delta / actual_distance;
    let actual_life = (actual_distance / PROJECTILE_SPEED).min(PROJECTILE_LIFETIME);

    let dummy_origin = viewmodel_muzzle_world(game);
    let dummy_delta = target_point - dummy_origin;
    let dummy_distance = dummy_delta.length().max(0.001);
    let dummy_vel = dummy_delta / actual_life.max(0.001);

    game.projectiles.push(Projectile {
        pos: actual_origin,
        vel: actual_dir * PROJECTILE_SPEED,
        life: actual_life,
        alive: true,
        visible: false,
        collides: true,
        pair_id,
    });
    game.projectiles.push(Projectile {
        pos: dummy_origin,
        vel: if dummy_distance > 0.0 { dummy_vel } else { cam_forward * PROJECTILE_SPEED },
        life: actual_life,
        alive: true,
        visible: true,
        collides: false,
        pair_id,
    });
}

pub fn update_projectiles(game: &mut FpsGame, dt: f32) {
    let mut pairs_to_kill = Vec::new();

    for index in 0..game.projectiles.len() {
        let projectile = &game.projectiles[index];
        if !projectile.alive {
            continue;
        }

        let mut pos = projectile.pos;
        let vel = projectile.vel;
        let life = projectile.life - dt;
        let mut alive = true;
        let collides = projectile.collides;
        let pair_id = projectile.pair_id;

        let step_count = ((vel.length() * dt) / RAY_STEP).ceil().max(1.0) as u32;
        let step_dt = dt / step_count as f32;
        let mut hit = None;

        for _ in 0..step_count {
            pos += vel * step_dt;
            if collides {
                hit = projectile_hit(game, pos);
                if hit.is_some() {
                    alive = false;
                    break;
                }
            }
        }

        if alive && life <= 0.0 {
            alive = false;
            if collides {
                pairs_to_kill.push(pair_id);
            }
        }

        {
            let projectile = &mut game.projectiles[index];
            projectile.pos = pos;
            projectile.life = life;
            projectile.alive = alive;
        }

        match hit {
            Some(ProjectileHit::Enemy(enemy_index)) => {
                if game.enemies[enemy_index].alive {
                    game.enemies[enemy_index].alive = false;
                    game.score += 1;
                }
                pairs_to_kill.push(pair_id);
            }
            Some(ProjectileHit::World) => {
                pairs_to_kill.push(pair_id);
            }
            None => {}
        }
    }

    game.projectiles.retain(|projectile| {
        projectile.alive && !pairs_to_kill.contains(&projectile.pair_id)
    });
}

fn camera_forward(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    )
    .normalize()
}

fn camera_right(yaw: f32, pitch: f32) -> Vec3 {
    camera_forward(yaw, pitch).cross(Vec3::Y).normalize()
}

fn camera_up(yaw: f32, pitch: f32) -> Vec3 {
    camera_right(yaw, pitch)
        .cross(camera_forward(yaw, pitch))
        .normalize()
}

fn viewmodel_muzzle_world(game: &FpsGame) -> Vec3 {
    let forward = camera_forward(game.cam_yaw, game.cam_pitch);
    let right = camera_right(game.cam_yaw, game.cam_pitch);
    let up = camera_up(game.cam_yaw, game.cam_pitch);
    game.player_pos
        + right * VIEWMODEL_MUZZLE_OFFSET.x
        + up * VIEWMODEL_MUZZLE_OFFSET.y
        - forward * VIEWMODEL_MUZZLE_OFFSET.z
}

fn trace_aim_point(game: &FpsGame, origin: Vec3, direction: Vec3) -> Vec3 {
    let max_distance = PROJECTILE_SPEED * PROJECTILE_LIFETIME;
    let step_count = (max_distance / RAY_STEP).ceil() as u32;
    let mut point = origin;

    for _ in 0..step_count {
        point += direction * RAY_STEP;
        if projectile_hit(game, point).is_some() {
            return point;
        }
    }

    origin + direction * max_distance
}

fn projectile_hit(game: &FpsGame, point: Vec3) -> Option<ProjectileHit> {
    for (index, enemy) in game.enemies.iter().enumerate() {
        if !enemy.alive {
            continue;
        }
        if (point - enemy.pos).length() < ENEMY_SIZE * 0.7 {
            return Some(ProjectileHit::Enemy(index));
        }
    }

    if point.y < 0.0 || point.y > WALL_HEIGHT {
        return Some(ProjectileHit::World);
    }

    if point.x < -0.5 || point.x > 28.5 || point.z < -0.5 || point.z > 12.5 {
        return Some(ProjectileHit::World);
    }

    for wall in &game.walls {
        if collides_with_wall(wall, point, PROJECTILE_RADIUS) {
            return Some(ProjectileHit::World);
        }
    }

    for door in &game.doors {
        if door.offset < 1.5 && collides_with_wall(&door.wall, point, PROJECTILE_RADIUS) {
            return Some(ProjectileHit::World);
        }
    }

    None
}

fn collides_with_wall(wall: &crate::state::CollisionWall, point: Vec3, radius: f32) -> bool {
    let dx = (wall.x1 - wall.x0).abs();
    let dz = (wall.z1 - wall.z0).abs();

    if dx > dz {
        let x_min = wall.x0.min(wall.x1) - radius;
        let x_max = wall.x0.max(wall.x1) + radius;
        point.x >= x_min && point.x <= x_max && (point.z - wall.z0).abs() <= radius
    } else {
        let z_min = wall.z0.min(wall.z1) - radius;
        let z_max = wall.z0.max(wall.z1) + radius;
        point.z >= z_min && point.z <= z_max && (point.x - wall.x0).abs() <= radius
    }
}
