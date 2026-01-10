use rengine::Vec3;

use crate::state::{FpsGame, Projectile};
use crate::{
    DOOR_OPEN_SPEED, ENEMY_SIZE, GRAVITY, JUMP_VEL, MOVE_SPEED,
    PLAYER_HEIGHT, PLAYER_RADIUS, PROJECTILE_LIFETIME, PROJECTILE_SPEED, WALL_HEIGHT,
};


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
    let cam_forward = Vec3::new(
        game.cam_yaw.sin() * game.cam_pitch.cos(),
        game.cam_pitch.sin(),
        -game.cam_yaw.cos() * game.cam_pitch.cos(),
    )
    .normalize();

    game.projectiles.push(Projectile {
        pos: game.player_pos + cam_forward * 0.5,
        vel: cam_forward * PROJECTILE_SPEED,
        life: PROJECTILE_LIFETIME,
        alive: true,
    });
}


pub fn update_projectiles(game: &mut FpsGame, dt: f32) {
    for proj in &mut game.projectiles {
        if !proj.alive {
            continue;
        }
        proj.pos += proj.vel * dt;
        proj.life -= dt;
        if proj.life <= 0.0 {
            proj.alive = false;
            continue;
        }


        for enemy in &mut game.enemies {
            if !enemy.alive {
                continue;
            }
            let diff = proj.pos - enemy.pos;
            if diff.length() < ENEMY_SIZE * 0.7 {
                enemy.alive = false;
                proj.alive = false;
                game.score += 1;
            }
        }


        if proj.pos.x < -0.5
            || proj.pos.x > 28.5
            || proj.pos.z < -0.5
            || proj.pos.z > 12.5
            || proj.pos.y < 0.0
            || proj.pos.y > WALL_HEIGHT
        {
            proj.alive = false;
        }
    }

    game.projectiles.retain(|p| p.alive);
}
