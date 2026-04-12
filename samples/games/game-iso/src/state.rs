use rengine::TextureId;

#[derive(Clone, Copy, PartialEq)]
pub enum Tile {
    Grass,
    Dirt,
    Water,
    Stone,
}

pub struct IsoGame {
    pub map: Vec<Vec<Tile>>,
    pub grass_tex: TextureId,
    pub dirt_tex: TextureId,
    pub water_tex: TextureId,
    pub stone_tex: TextureId,
    pub grass_uv: [f32; 4],
    pub dirt_uv: [f32; 4],
    pub water_uv: [f32; 4],
    pub stone_uv: [f32; 4],
    pub tree_tex: TextureId,
    pub player_tex: TextureId,
    pub trees: Vec<(i32, i32)>,

    pub player_col: f32,
    pub player_row: f32,
}
