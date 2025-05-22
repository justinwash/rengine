use image::GenericImageView;
use rengine_lib::{run, RengineConfig, RengineGame};
use winit::keyboard::KeyCode;
use winit::window::WindowAttributes;

mod actors;
use actors::character_actor::CharacterActor;
use rengine_lib::Scene;

struct Game {
    input_config: Option<rengine_lib::input::InputConfig>,
    should_close: bool,
    scene: Scene,
}

impl Game {
    fn new() -> Self {
        let input_config = Some(rengine_lib::input::InputConfig::new().on_key(
            KeyCode::Escape,
            || {
                println!("Escape pressed! Closing window...");
            },
        ));
        let image_path = rengine_lib::resource_path("game/resources/image/mario.png");
        let img = image::open(&image_path).expect("Failed to open sprite image");
        let (width, height) = img.dimensions();
        let sprite = rengine_lib::sprite::Sprite::new(
            &image_path,
            (100.0, 100.0),
            (width as f32, height as f32),
        );
        let mut scene = Scene::new();
        scene.add_actor(CharacterActor::new(sprite));
        Self {
            input_config,
            should_close: false,
            scene,
        }
    }
}

impl RengineGame for Game {
    fn input_config(&mut self) -> Option<rengine_lib::input::InputConfig> {
        self.input_config.take()
    }
    fn sprites(&mut self) -> &mut Vec<rengine_lib::sprite::Sprite> {
        // Collect all sprites from actors for rendering
        static mut SPRITES: Option<Vec<rengine_lib::sprite::Sprite>> = None;
        let sprites = unsafe { SPRITES.get_or_insert_with(Vec::new) };
        sprites.clear();
        for actor in &mut self.scene.actors {
            if let Some(character_actor) = actor.as_any().downcast_ref::<CharacterActor>() {
                sprites.push(character_actor.sprite.clone());
            }
        }
        sprites
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
