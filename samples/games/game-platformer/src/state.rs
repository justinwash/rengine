use rengine::{Color, Sprite, Vec2};

pub struct Player {
    pub sprite: Sprite,
    pub eye: Sprite,
    pub vel: Vec2,
    pub on_ground: bool,
    pub facing_right: bool,
}

pub struct Platform {
    pub sprite: Sprite,
}

pub struct Platformer {
    pub player: Player,
    pub platforms: Vec<Platform>,
    pub bg_color: Color,
}
