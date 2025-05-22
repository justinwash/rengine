use std::any::Any;

pub trait Actor {
    fn update(&mut self, wgpu_ctx: &mut crate::window::WgpuContext, input: &crate::input::InputState, event: &winit::event::Event<()>, window: &winit::window::Window);
    fn draw(&mut self, renderer: &mut crate::sprite::SpriteRenderer, wgpu_ctx: &mut crate::window::WgpuContext);
    fn as_any(&self) -> &dyn Any;
}

pub struct Scene {
    pub actors: Vec<Box<dyn Actor>>,
}

impl Scene {
    pub fn new() -> Self {
        Self { actors: Vec::new() }
    }
    pub fn add_actor<A: Actor + 'static>(&mut self, actor: A) {
        self.actors.push(Box::new(actor));
    }
    pub fn update(&mut self, wgpu_ctx: &mut crate::window::WgpuContext, input: &crate::input::InputState, event: &winit::event::Event<()>, window: &winit::window::Window) {
        for actor in self.actors.iter_mut() {
            actor.update(wgpu_ctx, input, event, window);
        }
    }
    pub fn draw(&mut self, renderer: &mut crate::sprite::SpriteRenderer, wgpu_ctx: &mut crate::window::WgpuContext) {
        for actor in self.actors.iter_mut() {
            actor.draw(renderer, wgpu_ctx);
        }
    }
}
