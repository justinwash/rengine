mod actors;
use crate::actors::characters::player::Player;
use rengine_lib::{run, RengineConfig, RengineGame};
use winit::keyboard::KeyCode;
use winit::window::WindowAttributes;

struct Game {
    input_config: Option<rengine_lib::input::InputConfig>,
    should_close: bool,
    scene: rengine_lib::Scene,
    sprites_cache: Vec<rengine_lib::graphics::sprite::Sprite>,
}

impl Game {
    fn new() -> Self {
        let input_config = Some(rengine_lib::input::InputConfig::new().on_key(
            KeyCode::Escape,
            || {
                println!("Escape pressed! Closing window...");
            },
        ));
        let mut scene = rengine_lib::Scene::new();
        let character = Player::load_default();
        scene.add_actor(character);
        Self {
            input_config,
            should_close: false,
            scene,
            sprites_cache: Vec::new(),
        }
    }
}

impl RengineGame for Game {
    fn input_config(&mut self) -> Option<rengine_lib::input::InputConfig> {
        self.input_config.take()
    }
    fn sprites(&mut self) -> &mut Vec<rengine_lib::graphics::sprite::Sprite> {
        if !self.sprites_cache.is_empty() {
            self.sprites_cache.clear();
        }
        for actor in &mut self.scene.actors {
            if let Some(player) = actor.as_any().downcast_ref::<Player>() {
                self.sprites_cache.push(player.sprite.clone());
            }
        }
        &mut self.sprites_cache
    }
    fn update(
        &mut self,
        wgpu_ctx: &mut rengine_lib::window::WgpuContext,
        input: &rengine_lib::input::InputState,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    ) {
        self.scene.update(wgpu_ctx, input, event, window);
        if input.is_just_pressed(KeyCode::Escape) {
            self.should_close = true;
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
