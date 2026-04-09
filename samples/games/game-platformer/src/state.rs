use rengine::{Color, TextureId, Vec2};


pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    pub facing_right: bool,
    pub texture: TextureId,
    pub eye_tex: TextureId,
}


pub struct Platform {
    pub pos: Vec2,
    pub size: Vec2,
    pub texture: TextureId,
}


pub struct Platformer {
    pub player: Player,
    pub platforms: Vec<Platform>,
    pub bg_color: Color,
}
