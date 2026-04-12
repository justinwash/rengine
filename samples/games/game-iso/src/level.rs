use std::path::PathBuf;

use rengine::Engine;

use crate::state::{IsoGame, Tile};
use crate::MAP_SIZE;

pub fn build(engine: &mut Engine) -> IsoGame {
    engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

    let tile_sheet = engine
        .load_sprite_sheet("tiles.png", 64, 32)
        .expect("failed to load iso tile sheet");
    let grass_tex = tile_sheet.texture;
    let dirt_tex = tile_sheet.texture;
    let water_tex = tile_sheet.texture;
    let stone_tex = tile_sheet.texture;
    let tree_tex = engine
        .load_texture("tree.png")
        .expect("failed to load iso tree texture")
        .texture();
    let player_tex = engine
        .load_texture("player.png")
        .expect("failed to load iso player texture")
        .texture();

    let mut map = vec![vec![Tile::Grass; MAP_SIZE as usize]; MAP_SIZE as usize];

    for col in 0..MAP_SIZE as usize {
        map[7][col] = Tile::Dirt;
        map[8][col] = Tile::Dirt;
    }
    for row in 3..12 {
        map[row][7] = Tile::Dirt;
    }

    for row in 2..5 {
        for col in 2..5 {
            map[row][col] = Tile::Water;
        }
    }

    for col in 10..14 {
        map[3][col] = Tile::Stone;
        map[6][col] = Tile::Stone;
    }
    map[4][10] = Tile::Stone;
    map[5][10] = Tile::Stone;
    map[4][13] = Tile::Stone;
    map[5][13] = Tile::Stone;

    for i in 0..MAP_SIZE as usize {
        map[0][i] = Tile::Stone;
        map[MAP_SIZE as usize - 1][i] = Tile::Stone;
        map[i][0] = Tile::Stone;
        map[i][MAP_SIZE as usize - 1] = Tile::Stone;
    }

    let trees = vec![
        (3, 10),
        (5, 11),
        (8, 12),
        (1, 5),
        (9, 1),
        (12, 9),
        (11, 11),
        (6, 2),
        (4, 6),
    ];

    IsoGame {
        map,
        grass_tex,
        dirt_tex,
        water_tex,
        stone_tex,
        grass_uv: tile_sheet.uv_rect(0, 0),
        dirt_uv: tile_sheet.uv_rect(1, 0),
        water_uv: tile_sheet.uv_rect(2, 0),
        stone_uv: tile_sheet.uv_rect(3, 0),
        tree_tex,
        player_tex,
        trees,
        player_col: 7.0,
        player_row: 7.0,
    }
}
