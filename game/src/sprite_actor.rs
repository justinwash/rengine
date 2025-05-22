use rengine_lib::{Actor, sprite::Sprite, window::WgpuContext, input::InputState};
use winit::keyboard::KeyCode;
use winit::event::Event;
use winit::window::Window;

pub struct SpriteActor {
    pub sprite: Sprite,
}

impl SpriteActor {
    pub fn new(sprite: Sprite) -> Self {
        Self { sprite }
    }
}

impl Actor for SpriteActor {
    fn update(&mut self, _wgpu_ctx: &mut WgpuContext, input: &InputState, _event: &Event<()>, _window: &Window) {
        let speed = 1.0;
        let mut dx = 0.0;
        let mut dy = 0.0;
        if input.is_held(KeyCode::KeyW) {
            dy -= speed;
        }
        if input.is_held(KeyCode::KeyS) {
            dy += speed;
        }
        if input.is_held(KeyCode::KeyA) {
            dx -= speed;
        }
        if input.is_held(KeyCode::KeyD) {
            dx += speed;
        }
        let len = f32::sqrt(dx * dx + dy * dy);
        if len != 0.0 {
            dx = dx / len * speed;
            dy = dy / len * speed;
        }
        self.sprite.position.0 += dx;
        self.sprite.position.1 += dy;
    }
    fn draw(&mut self, renderer: &mut rengine_lib::sprite::SpriteRenderer, _wgpu_ctx: &mut WgpuContext) {
        renderer.add_sprite(self.sprite.clone());
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
