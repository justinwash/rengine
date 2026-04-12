use rengine::{Color, DrawParams, Frame, Vec2};

use crate::state::{IsoGame, Tile};
use crate::{MAP_SIZE, TILE_H, TILE_W};

pub fn draw(game: &IsoGame, frame: &mut Frame) {
    frame.clear_color = Color::from_rgba8(30, 30, 50, 255);

    let player_screen =
        iso_to_screen_frac(game.player_col, game.player_row, TILE_W, TILE_H);
    frame.camera.position = player_screen;

    for row in 0..MAP_SIZE {
        for col in 0..MAP_SIZE {
            let tile = game.map[row as usize][col as usize];
            let tex = match tile {
                Tile::Grass => game.grass_tex,
                Tile::Dirt => game.dirt_tex,
                Tile::Water => game.water_tex,
                Tile::Stone => game.stone_tex,
            };
            let uv = match tile {
                Tile::Grass => game.grass_uv,
                Tile::Dirt => game.dirt_uv,
                Tile::Water => game.water_uv,
                Tile::Stone => game.stone_uv,
            };
            let screen = iso_to_screen(col, row, TILE_W, TILE_H);
            let draw_pos = Vec2::new(screen.x - TILE_W / 2.0, screen.y - TILE_H / 2.0);
            frame.draw_sprite(
                DrawParams::new(tex, draw_pos, Vec2::new(TILE_W, TILE_H)).with_uv_rect(uv),
            );
        }
    }

    let mut sorted_trees = game.trees.clone();
    sorted_trees.sort_by_key(|&(_, r)| r);
    for &(col, row) in &sorted_trees {
        let screen = iso_to_screen(col, row, TILE_W, TILE_H);
        let draw_pos = Vec2::new(screen.x - 16.0, screen.y - 8.0);
        frame.draw(game.tree_tex, draw_pos, Vec2::new(32.0, 48.0));
    }

    let player_screen =
        iso_to_screen_frac(game.player_col, game.player_row, TILE_W, TILE_H);
    let draw_pos = Vec2::new(player_screen.x - 8.0, player_screen.y - 8.0);
    frame.draw(game.player_tex, draw_pos, Vec2::new(16.0, 24.0));
}

fn iso_to_screen(col: i32, row: i32, tile_width: f32, tile_height: f32) -> Vec2 {
    iso_to_screen_frac(col as f32, row as f32, tile_width, tile_height)
}

fn iso_to_screen_frac(col: f32, row: f32, tile_width: f32, tile_height: f32) -> Vec2 {
    let x = (col - row) * (tile_width / 2.0);
    let y = (col + row) * (tile_height / 2.0);
    Vec2::new(x, -y)
}
