use rengine::tilemap::TileMap;
use rengine::{aabb_overlap, Rect, Vec2};

use crate::state::{Enemy, TopDown};
use crate::{MAP_H, MAP_W, PLAYER_SIZE, PLAYER_SPEED, TILE_SIZE};

pub fn move_player(game: &mut TopDown, dir: Vec2, dt: f32) {

    game.player.pos.x += dir.x * PLAYER_SPEED * dt;
    let rect = Rect::from_pos_size(game.player.pos, Vec2::splat(PLAYER_SIZE));
    if let Some(mtv) = collide_stone(&game.tilemap, &rect) {
        game.player.pos.x += mtv.x;
    }

    game.player.pos.y += dir.y * PLAYER_SPEED * dt;
    let rect = Rect::from_pos_size(game.player.pos, Vec2::splat(PLAYER_SIZE));
    if let Some(mtv) = collide_stone(&game.tilemap, &rect) {
        game.player.pos.y += mtv.y;
    }
}

pub fn collect_gems(game: &mut TopDown) {
    let player_rect = Rect::from_pos_size(game.player.pos, Vec2::splat(PLAYER_SIZE));
    for gem in &mut game.gems {
        if gem.collected {
            continue;
        }
        let gem_rect = Rect::from_pos_size(gem.pos, Vec2::splat(20.0));
        if player_rect.overlaps(&gem_rect) {
            gem.collected = true;
            game.score += 1;
        }
    }
}

pub fn update_enemies(enemies: &mut [Enemy], tilemap: &TileMap, dt: f32) {
    for enemy in enemies.iter_mut() {
        enemy.pos += enemy.vel * dt;
        let r = Rect::from_pos_size(enemy.pos, Vec2::splat(PLAYER_SIZE));
        if let Some(mtv) = collide_stone(tilemap, &r) {
            enemy.pos += mtv;
            if mtv.x.abs() > 0.001 {
                enemy.vel.x = -enemy.vel.x;
            }
            if mtv.y.abs() > 0.001 {
                enemy.vel.y = -enemy.vel.y;
            }
        }
    }
}

pub fn collide_stone(tilemap: &TileMap, rect: &Rect) -> Option<Vec2> {
    let col_min = ((rect.x / TILE_SIZE).floor() as isize).max(0) as usize;
    let col_max = (((rect.x + rect.width) / TILE_SIZE).ceil() as usize).min(MAP_W);
    let row_min = ((rect.y / TILE_SIZE).floor() as isize).max(0) as usize;
    let row_max = (((rect.y + rect.height) / TILE_SIZE).ceil() as usize).min(MAP_H);

    let stone_id = 2;
    let mut total = Vec2::ZERO;
    let mut hit = false;

    for row in row_min..row_max {
        for col in col_min..col_max {
            if tilemap.get(col, row) == Some(stone_id) {
                let tile_rect = Rect::new(
                    col as f32 * TILE_SIZE,
                    row as f32 * TILE_SIZE,
                    TILE_SIZE,
                    TILE_SIZE,
                );
                let adj = Rect::new(
                    rect.x + total.x,
                    rect.y + total.y,
                    rect.width,
                    rect.height,
                );
                if let Some(mtv) = aabb_overlap(&adj, &tile_rect) {
                    total += mtv;
                    hit = true;
                }
            }
        }
    }

    if hit { Some(total) } else { None }
}
