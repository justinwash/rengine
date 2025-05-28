use crate::game_object::actor::Actor;
use crate::graphics::sprite::Sprite;

pub trait CharacterActor: Actor {
    fn sprite(&self) -> &Sprite;
    fn sprite_mut(&mut self) -> &mut Sprite;
    fn collision_enabled(&self) -> bool {
        true
    }
    fn health(&self) -> i32 {
        100
    }
}
