use rengine_lib::{run, RengineConfig, RengineGame};
use winit::keyboard::KeyCode;
use winit::window::WindowAttributes;

struct Game {
    input_config: Option<rengine_lib::input::InputConfig>,
    should_close: bool,
    sprites: Vec<rengine_lib::sprite::Sprite>,
}

impl Game {
    fn new() -> Self {
        let input_config = Some(rengine_lib::input::InputConfig::new().on_key(
            KeyCode::Escape,
            || {
                println!("Escape pressed! Closing window...");
            },
        ));
        let sprite = rengine_lib::sprite::Sprite::new(
            "./src/resources/image/IMG_6141.jpg",
            (100.0, 100.0),
            (256.0, 256.0),
        );
        Self {
            input_config,
            should_close: false,
            sprites: vec![sprite],
        }
    }
}

impl RengineGame for Game {
    fn input_config(&mut self) -> Option<rengine_lib::input::InputConfig> {
        self.input_config.take()
    }
    fn sprites(&mut self) -> &mut Vec<rengine_lib::sprite::Sprite> {
        &mut self.sprites
    }
    fn update(
        &mut self,
        _wgpu_ctx: &mut rengine_lib::window::WgpuContext,
        input: &rengine_lib::input::InputState,
        _event: &winit::event::Event<()>,
        _window: &winit::window::Window,
    ) {
        let speed = 5.0;
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
        if input.is_just_pressed(KeyCode::Escape) {
            self.should_close = true;
        }
        if dx != 0.0 || dy != 0.0 {
            let sprite = self.sprites.get_mut(0).unwrap();
            sprite.position.0 += dx;
            sprite.position.1 += dy;
        }
    }
    fn on_close(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("Window close requested!");
        event_loop.exit();
    }
}

fn main() {
    let mut attrs = WindowAttributes::default();
    attrs.title = "My Game".to_string();
    let config = RengineConfig {
        window_attributes: attrs,
        on_close: None,
    };
    run(config, Game::new());
}
