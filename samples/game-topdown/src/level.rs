use rengine::tilemap::{TileDef, TileMap};
use rengine::{pixelart, Color, Engine, Vec2};

use crate::state::{Enemy, Gem, Player, TopDown};
use crate::{ENEMY_SPEED, MAP_H, MAP_W, TILE_SIZE};


pub fn build(engine: &mut Engine) -> TopDown {

    let (w, h, d) = pixelart::grass_tile();
    let grass_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = pixelart::dirt_tile();
    let dirt_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = pixelart::stone_tile();
    let stone_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = pixelart::water_tile();
    let water_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = pixelart::character_topdown(
        Color::from_rgba8(50, 100, 200, 255),
        Color::from_rgba8(220, 180, 140, 255),
        Color::BLACK,
    );
    let player_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = pixelart::enemy_topdown();
    let enemy_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = pixelart::gem_sprite();
    let gem_tex = engine.create_texture(w, h, &d);
    let (w, h, d) = pixelart::tree_top();
    let tree_tex = engine.create_texture(w, h, &d);


    let mut tilemap = TileMap::new(MAP_W, MAP_H, TILE_SIZE);
    let grass_id = tilemap.add_tile(TileDef::solid(grass_tex));
    let dirt_id = tilemap.add_tile(TileDef::solid(dirt_tex));
    let stone_id = tilemap.add_tile(TileDef::solid(stone_tex));
    let _water_id = tilemap.add_tile(TileDef::solid(water_tex));


    for row in 0..MAP_H {
        for col in 0..MAP_W {
            tilemap.set(col, row, Some(grass_id));
        }
    }


    for col in 0..MAP_W {
        tilemap.set(col, 0, Some(stone_id));
        tilemap.set(col, MAP_H - 1, Some(stone_id));
    }
    for row in 0..MAP_H {
        tilemap.set(0, row, Some(stone_id));
        tilemap.set(MAP_W - 1, row, Some(stone_id));
    }


    for col in 1..MAP_W - 1 {
        tilemap.set(col, 9, Some(dirt_id));
        tilemap.set(col, 10, Some(dirt_id));
    }


    for row in 5..15 {
        tilemap.set(15, row, Some(dirt_id));
    }


    for row in 3..6 {
        for col in 5..9 {
            tilemap.set(col, row, Some(_water_id));
        }
    }


    for col in 20..25 {
        tilemap.set(col, 14, Some(stone_id));
    }
    for row in 14..18 {
        tilemap.set(20, row, Some(stone_id));
    }


    for col in 23..28 {
        tilemap.set(col, 3, Some(stone_id));
        tilemap.set(col, 7, Some(stone_id));
    }
    for row in 3..8 {
        tilemap.set(23, row, Some(stone_id));
        tilemap.set(28, row, Some(stone_id));
    }

    tilemap.set(25, 3, Some(dirt_id));


    let trees: Vec<Vec2> = vec![
        Vec2::new(3.0, 14.0),
        Vec2::new(4.0, 16.0),
        Vec2::new(7.0, 15.0),
        Vec2::new(10.0, 3.0),
        Vec2::new(11.0, 5.0),
        Vec2::new(12.0, 2.0),
        Vec2::new(3.0, 6.0),
        Vec2::new(17.0, 16.0),
        Vec2::new(18.0, 17.0),
        Vec2::new(26.0, 15.0),
        Vec2::new(27.0, 13.0),
    ]
    .into_iter()
    .map(|v| v * TILE_SIZE)
    .collect();


    let enemies = vec![
        Enemy {
            pos: Vec2::new(12.0, 6.0) * TILE_SIZE,
            vel: Vec2::new(ENEMY_SPEED, 0.0),
            tex: enemy_tex,
        },
        Enemy {
            pos: Vec2::new(18.0, 12.0) * TILE_SIZE,
            vel: Vec2::new(0.0, ENEMY_SPEED),
            tex: enemy_tex,
        },
        Enemy {
            pos: Vec2::new(8.0, 15.0) * TILE_SIZE,
            vel: Vec2::new(ENEMY_SPEED, ENEMY_SPEED * 0.5),
            tex: enemy_tex,
        },
        Enemy {
            pos: Vec2::new(25.0, 5.0) * TILE_SIZE,
            vel: Vec2::new(0.0, ENEMY_SPEED),
            tex: enemy_tex,
        },
    ];


    let gem_positions = vec![
        Vec2::new(5.0, 12.0),
        Vec2::new(14.0, 7.0),
        Vec2::new(22.0, 5.0),
        Vec2::new(10.0, 16.0),
        Vec2::new(25.0, 16.0),
        Vec2::new(15.0, 12.0),
        Vec2::new(8.0, 8.0),
        Vec2::new(27.0, 10.0),
        Vec2::new(3.0, 3.0),
        Vec2::new(20.0, 2.0),
    ];
    let gems = gem_positions
        .into_iter()
        .map(|p| Gem {
            pos: p * TILE_SIZE,
            tex: gem_tex,
            collected: false,
        })
        .collect();


    let player = Player {
        pos: Vec2::new(3.0, 9.0) * TILE_SIZE,
        tex: player_tex,
    };

    TopDown {
        player,
        enemies,
        gems,
        tilemap,
        score: 0,
        tree_tex,
        trees,
    }
}
