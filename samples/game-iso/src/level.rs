use rengine::{Color, Engine};

use crate::art;
use crate::state::{IsoGame, Tile};
use crate::MAP_SIZE;


pub fn build(engine: &mut Engine) -> IsoGame {

    let (w, h, d) = art::iso_grass_tile();
    let grass_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = art::iso_dirt_tile();
    let dirt_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = art::iso_water_tile();
    let water_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = art::iso_stone_tile();
    let stone_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = art::iso_tree();
    let tree_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = art::iso_character(Color::from_rgba8(50, 100, 200, 255));
    let player_tex = engine.create_texture(w, h, &d);


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
        tree_tex,
        player_tex,
        trees,
        player_col: 7.0,
        player_row: 7.0,
    }
}
