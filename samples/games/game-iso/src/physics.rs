use crate::state::{IsoGame, Tile};
use crate::{MAP_SIZE, PLAYER_SPEED, TILE_W};


pub fn move_player(game: &mut IsoGame, dc: f32, dr: f32, dt: f32) {
    let speed_tiles = PLAYER_SPEED / TILE_W;


    let new_col = game.player_col + dc * speed_tiles * dt;
    if can_walk(&game.map, new_col, game.player_row) {
        game.player_col = new_col;
    }


    let new_row = game.player_row + dr * speed_tiles * dt;
    if can_walk(&game.map, game.player_col, new_row) {
        game.player_row = new_row;
    }
}


pub fn can_walk(map: &[Vec<Tile>], col: f32, row: f32) -> bool {
    let margin = 0.3;
    for &(dc, dr) in &[
        (-margin, -margin),
        (margin, -margin),
        (-margin, margin),
        (margin, margin),
    ] {
        let c = (col + dc).round() as i32;
        let r = (row + dr).round() as i32;
        if c < 0 || r < 0 || c >= MAP_SIZE || r >= MAP_SIZE {
            return false;
        }
        let tile = map[r as usize][c as usize];
        if tile == Tile::Water || tile == Tile::Stone {
            return false;
        }
    }
    true
}
