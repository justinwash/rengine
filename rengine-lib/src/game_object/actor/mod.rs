pub mod character_actor;

use crate::game_object::game_object::GameObject;

pub trait Actor: GameObject {
    fn update(
        &mut self,
        wgpu_ctx: &mut crate::window::WgpuContext,
        input: &crate::input::InputState,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    );
    fn draw(
        &mut self,
        renderer: &mut crate::sprite::SpriteRenderer,
        wgpu_ctx: &mut crate::window::WgpuContext,
    );
    fn as_any(&self) -> &dyn std::any::Any;
}
