use glfw::{Action, Context as _, Key, WindowEvent};
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use rengine::graphics::{renderer::*, sprite::*};
use rengine::input::input_map::*;
use rengine::input::keyboard::*;
use rengine::utils::transform::*;
use std::process::exit;

fn main() {
    let dim = WindowDim::Windowed {
        width: 1280,
        height: 720,
    };
    let surface = GlfwSurface::new_gl33("Hello, world!", WindowOpt::default().set_dim(dim));

    match surface {
        Ok(surface) => {
            eprintln!("graphics surface created");
            main_loop(surface);
        }

        Err(e) => {
            eprintln!("cannot create graphics surface:\n{}", e);
            exit(1);
        }
    }
}

fn main_loop(mut surface: GlfwSurface) {
    let mut back_buffer = surface.back_buffer().unwrap();
    let mut renderer = Renderer::new(&mut surface);

    // sprites for testing
    let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 0, y: 0 },
        Size { w: 512, h: 512 },
    );
    renderer.sprite_renderer.sprites.insert(sprite.id, sprite.clone());
    renderer.sprite_renderer.load_texture(&mut surface, sprite);
        let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 200, y: 200 },
        Size { w: 512, h: 1024 },
    );
    renderer.sprite_renderer.sprites.insert(sprite.id, sprite.clone());
    renderer.sprite_renderer.load_texture(&mut surface, sprite);
        let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 700, y: 400 },
        Size { w: 600, h: 300 },
    );

    let test_sprite_id = sprite.id;
    renderer.sprite_renderer.sprites.insert(sprite.id, sprite.clone());
    renderer.sprite_renderer.load_texture(&mut surface, sprite);
    // sprites for testing

    // input map for testing
    let mut controls: InputMap<KeyboardInput> = InputMap::new()
        .add_action("up", KeyboardInput::new(&Key::W))
        .add_action("down", KeyboardInput::new(&Key::S))
        .add_action("left", KeyboardInput::new(&Key::A))
        .add_action("right", KeyboardInput::new(&Key::D))
        .add_action("ok", KeyboardInput::new(&Key::Enter))
        .add_action("cancel", KeyboardInput::new(&Key::Escape));
    //input map for testing

    println!("{:?}", controls);

    'app: loop {
        surface.window.glfw.poll_events();
        for (_, event) in surface.events_rx.try_iter() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }
                _ => (),
            }
        }

        if controls.is_action_just_pressed("up", &surface.window) {
            println!("up just pressed")
        }
        if controls.is_action_just_released("up", &surface.window) {
            println!("up just released")
        }
        if controls.is_action_held("up", &surface.window) {
            println!("up held");
            let mut sprite = renderer
            .sprite_renderer
                .sprites
                .get_mut(&test_sprite_id)
                .unwrap();
            sprite.position.y -= 1;
        }

        let render = renderer.render(&mut surface, &mut back_buffer);

        if render.is_ok() {
            surface.window.swap_buffers();
        } else {
            break 'app;
        }
    }
}
