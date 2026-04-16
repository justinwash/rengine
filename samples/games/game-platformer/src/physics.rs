use rengine::{aabb_overlap, Rect, Vec2};

use crate::state::{Platform, Player};
use crate::{GRAVITY, PLAYER_H, PLAYER_W};

pub fn update(player: &mut Player, platforms: &[Platform], dt: f32) {
    player.vel.y += GRAVITY * dt;

    player.sprite.position += player.vel * dt;

    player.on_ground = false;
    let player_rect = Rect::from_pos_size(player.sprite.position, Vec2::new(PLAYER_W, PLAYER_H));

    for plat in platforms {
        let plat_rect = Rect::from_pos_size(plat.sprite.position, plat.sprite.size);
        if let Some(mtv) = aabb_overlap(&player_rect, &plat_rect) {
            player.sprite.position += mtv;

            if mtv.y > 0.0 {
                player.vel.y = 0.0;
                player.on_ground = true;
            } else if mtv.y < 0.0 {
                player.vel.y = 0.0;
            }
        }
    }
}
