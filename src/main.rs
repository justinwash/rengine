use glfw::{Action, Context as _, Key, WindowEvent};
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use rengine::graphics::{renderer::*, sprite::*};
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
    renderer.sprites.push(sprite);
    let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 200, y: 200 },
        Size { w: 512, h: 1024 },
    );
    renderer.sprites.push(sprite);
    let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 700, y: 400 },
        Size { w: 600, h: 300 },
    );
    renderer.sprites.push(sprite);
    // sprites for testing

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

        let render = renderer.render(&mut surface, &mut back_buffer);

        if render.is_ok() {
            surface.window.swap_buffers();
        } else {
            break 'app;
        }
    }
}
