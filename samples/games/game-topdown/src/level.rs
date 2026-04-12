use std::path::PathBuf;

use rengine::tilemap::{TileDef, TileMap};
use rengine::{Engine, Vec2};

use crate::state::{Enemy, Gem, Player, TopDown};
use crate::{ENEMY_SPEED, MAP_H, MAP_W, TILE_SIZE};

pub fn build(engine: &mut Engine) -> TopDown {
    engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

    let assets = engine
        .load_asset_manifest("topdown.assets.json")
        .expect("failed to load topdown asset manifest");
    let world_sheet = assets
        .sprite_sheet("world_tiles")
        .expect("manifest missing world_tiles sprite sheet");
    let grass_uv = world_sheet.uv_rect(0, 0);
    let dirt_uv = world_sheet.uv_rect(1, 0);
    let stone_uv = world_sheet.uv_rect(2, 0);
    let water_uv = world_sheet.uv_rect(3, 0);

    let player_tex = assets.texture_id("player").expect("manifest missing player texture");
    let enemy_tex = assets.texture_id("enemy").expect("manifest missing enemy texture");
    let gem_tex = assets.texture_id("gem").expect("manifest missing gem texture");

    let scene = engine
        .load_scene2d(&assets, "world.scene.json")
        .expect("failed to load topdown scene");

    let mut tilemap = TileMap::new(MAP_W, MAP_H, TILE_SIZE);
    let grass_id = tilemap.add_tile(TileDef::solid(world_sheet.texture).with_uv(grass_uv));
    let dirt_id = tilemap.add_tile(TileDef::solid(world_sheet.texture).with_uv(dirt_uv));
    let stone_id = tilemap.add_tile(TileDef::solid(world_sheet.texture).with_uv(stone_uv));
    let water_id = tilemap.add_tile(TileDef::solid(world_sheet.texture).with_uv(water_uv));

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
            tilemap.set(col, row, Some(water_id));
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

    let enemy_directions = [
        Vec2::new(ENEMY_SPEED, 0.0),
        Vec2::new(0.0, ENEMY_SPEED),
        Vec2::new(ENEMY_SPEED, ENEMY_SPEED * 0.5),
        Vec2::new(0.0, ENEMY_SPEED),
    ];
    let enemies = scene
        .by_prefab("enemy_spawn")
        .enumerate()
        .map(|(index, instance)| Enemy {
            pos: instance.position,
            vel: enemy_directions[index % enemy_directions.len()],
            tex: enemy_tex,
        })
        .collect();

    let gems = scene
        .by_prefab("gem_spawn")
        .map(|instance| Gem {
            pos: instance.position,
            tex: gem_tex,
            collected: false,
        })
        .collect();

    let player = Player {
        pos: scene
            .by_prefab("player_spawn")
            .next()
            .map(|instance| instance.position)
            .unwrap_or(Vec2::new(3.0, 9.0) * TILE_SIZE),
        tex: player_tex,
    };

    TopDown {
        player,
        enemies,
        gems,
        tilemap,
        score: 0,
        scene,
    }
}
