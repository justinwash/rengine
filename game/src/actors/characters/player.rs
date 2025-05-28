use image::GenericImageView;
use rengine_lib::game_object::GameObject;
use rengine_lib::graphics::sprite::{Sprite, SpriteRenderer};
use rengine_lib::input::InputState;
use rengine_lib::window::WgpuContext;
use rengine_lib::Actor;
use rengine_lib::CharacterActor;
use winit::event::Event;
use winit::keyboard::KeyCode;
use winit::window::Window;

pub struct Player {
    pub sprite: Sprite,
    pub collision_enabled: bool,
    pub health: i32,
}

impl Player {
    pub fn new(sprite: Sprite) -> Self {
        Self {
            sprite,
            collision_enabled: true,
            health: 100,
        }
    }

    pub fn load_default() -> Self {
        let image_path = rengine_lib::resource_path("resources/image/mario.png");
        let img = image::open(&image_path).expect("Failed to open sprite image");
        let (width, height) = img.dimensions();
        let sprite = Sprite::new(&image_path, (100.0, 100.0), (width as f32, height as f32));
        Self::new(sprite)
    }
}

impl GameObject for Player {
    fn position(&self) -> (f32, f32) {
        self.sprite.position
    }
    fn set_position(&mut self, pos: (f32, f32)) {
        self.sprite.position = pos;
    }
}

impl Actor for Player {
    fn update(
        &mut self,
        _wgpu_ctx: &mut WgpuContext,
        input: &InputState,
        _event: &Event<()>,
        _window: &Window,
    ) {
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
        let (x, y) = self.position();
        self.set_position((x + dx, y + dy));
    }
    fn draw(&mut self, renderer: &mut SpriteRenderer, _wgpu_ctx: &mut WgpuContext) {
        renderer.add_sprite(self.sprite.clone());
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl CharacterActor for Player {
    fn sprite(&self) -> &Sprite {
        &self.sprite
    }
    fn sprite_mut(&mut self) -> &mut Sprite {
        &mut self.sprite
    }
    fn collision_enabled(&self) -> bool {
        self.collision_enabled
    }
    fn health(&self) -> i32 {
        self.health
    }
}
