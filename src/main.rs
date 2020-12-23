use glfw::{Action, Context as _, Key, WindowEvent};
use luminance::context::GraphicsContext;
use luminance::pipeline::PipelineState;
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use std::process::exit;

mod graphics;
use crate::graphics::sprite;
use crate::graphics::sprite::*;

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
    let back_buffer = surface.back_buffer().unwrap();

    let mut sprites: Vec<Sprite> = Vec::new();
    let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 0, y: 0 },
        Size { w: 512, h: 512 },
    );
    sprites.push(sprite);
    let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 200, y: 200 },
        Size { w: 512, h: 1024 },
    );
    sprites.push(sprite);
    let sprite = Sprite::new(
        "test_texture.png".to_string(),
        Position { x: 700, y: 400 },
        Size { w: 600, h: 300 },
    );
    sprites.push(sprite);

    let mut sprite_renderer = SpriteRenderer::new(&mut surface);

    for sprite in &sprites {
        let sprite_cl = sprite.clone();
        sprite_renderer.textures.insert(
            sprite.id,
            sprite::load_from_disk(&mut surface, sprite_cl.image),
        );
    }

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

        let render = surface
            .new_pipeline_gate()
            .pipeline(
                &back_buffer,
                &PipelineState::default().set_clear_color([0.0, 0.0, 0.0, 1.0]),
                |pipeline, mut shading_gate| {
                    sprite_renderer.render(&pipeline, &mut shading_gate, &mut sprites)
                },
            )
            .assume();

        if render.is_ok() {
            surface.window.swap_buffers();
        } else {
            break 'app;
        }
    }
}
