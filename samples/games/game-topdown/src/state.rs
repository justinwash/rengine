use rengine::tilemap::TileMap;
use rengine::{Scene2D, TextureId, Vec2};


pub struct Player {
    pub pos: Vec2,
    pub tex: TextureId,
}

pub struct Enemy {
    pub pos: Vec2,
    pub vel: Vec2,
    pub tex: TextureId,
}

pub struct Gem {
    pub pos: Vec2,
    pub tex: TextureId,
    pub collected: bool,
}


pub struct TopDown {
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub gems: Vec<Gem>,
    pub tilemap: TileMap,
    pub score: u32,
    pub scene: Scene2D,
}
