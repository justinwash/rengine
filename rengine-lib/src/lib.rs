pub mod graphics;
pub mod input;
pub mod scene;
pub mod util;
pub mod window;

use std::time::{Duration, Instant};
use winit::window::{Window, WindowAttributes};

pub struct Rengine;

pub struct RengineConfig {
    pub window_attributes: WindowAttributes,
    pub on_close: Option<Box<dyn FnMut(&winit::event_loop::ActiveEventLoop) + Send + 'static>>,
}

impl Default for RengineConfig {
    fn default() -> Self {
        Self {
            window_attributes: Window::default_attributes(),
            on_close: None,
        }
    }
}

impl Rengine {
    pub async fn new(_window: &Window) -> Self {
        Self
    }
}

pub trait RengineGame {
    fn input_config(&mut self) -> Option<crate::input::InputConfig> {
        None
    }
    fn sprites(&mut self) -> &mut Vec<crate::sprite::Sprite>;
    fn update(
        &mut self,
        wgpu_ctx: &mut window::WgpuContext,
        input: &crate::input::InputState,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    );
    fn on_close(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}
}

pub fn run<G: RengineGame + 'static>(config: RengineConfig, game: G) {
    use crate::input::InputState;
    use window::WgpuContext;
    use winit::event_loop::{ActiveEventLoop, EventLoop};

    struct RengineApp<G: RengineGame + 'static> {
        config: RengineConfig,
        game: G,
        window: Option<&'static Window>,
        wgpu_ctx: Option<WgpuContext<'static>>,
        input: Option<InputState>,
        sprite_renderer: Option<crate::graphics::sprite::SpriteRenderer>,
        last_update: Option<Instant>,
        accumulator: Duration,
    }

    impl<G: RengineGame + 'static> winit::application::ApplicationHandler for RengineApp<G> {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window = event_loop
                .create_window(self.config.window_attributes.clone())
                .unwrap();
            let static_window: &'static Window = Box::leak(Box::new(window));
            let wgpu_ctx = pollster::block_on(window::init_wgpu(static_window));
            self.window = Some(static_window);
            self.wgpu_ctx = Some(wgpu_ctx);
            if self.input.is_none() {
                if let Some(cfg) = self.game.input_config() {
                    self.input = Some(InputState::new(cfg));
                }
            }
            if self.sprite_renderer.is_none() {
                self.sprite_renderer = Some(crate::graphics::sprite::SpriteRenderer::new());
            }
            self.last_update = Some(Instant::now());
            self.accumulator = Duration::ZERO;
            if let Some(input) = self.input.as_mut() {
                input.clear_all();
            }
        }
        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            const DT: Duration = Duration::from_nanos(16_666_667);
            if let (Some(window), Some(wgpu_ctx), Some(input)) =
                (self.window, self.wgpu_ctx.as_mut(), self.input.as_mut())
            {
                let now = Instant::now();
                let last = self.last_update.get_or_insert(now);
                let mut accumulator = self.accumulator + now.duration_since(*last);
                while accumulator >= DT {
                    input.begin_frame();
                    let event = winit::event::Event::AboutToWait;
                    self.game.update(wgpu_ctx, input, &event, window);
                    if let Some(renderer) = self.sprite_renderer.as_mut() {
                        renderer.sprites = self.game.sprites().clone();
                        renderer.render(wgpu_ctx);
                    }
                    input.end_frame();
                    accumulator -= DT;
                }
                self.last_update = Some(now);
                self.accumulator = accumulator;
                window.request_redraw();
            }
        }
        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
            if let winit::event::WindowEvent::CloseRequested = event {
                self.game.on_close(event_loop);
                event_loop.exit();
            }
            if let (Some(wgpu_ctx), Some(_window)) = (self.wgpu_ctx.as_mut(), self.window) {
                if let winit::event::WindowEvent::Resized(new_size) = event {
                    if new_size.width > 0 && new_size.height > 0 {
                        wgpu_ctx.config.width = new_size.width;
                        wgpu_ctx.config.height = new_size.height;
                        wgpu_ctx
                            .surface
                            .configure(&wgpu_ctx.device, &wgpu_ctx.config);
                    }
                }
            }
            if let Some(input) = self.input.as_mut() {
                let event = winit::event::Event::WindowEvent {
                    window_id: self.window.unwrap().id(),
                    event: event.clone(),
                };
                input.handle_event(&event);
            }
            if let (Some(window), Some(wgpu_ctx), Some(input)) =
                (self.window, self.wgpu_ctx.as_mut(), self.input.as_mut())
            {
                let event = winit::event::Event::WindowEvent {
                    window_id: window.id(),
                    event: event.clone(),
                };
                self.game.update(wgpu_ctx, input, &event, window);
                if let Some(renderer) = self.sprite_renderer.as_mut() {
                    renderer.sprites = self.game.sprites().clone();
                    if let Some(wgpu_ctx) = self.wgpu_ctx.as_mut() {
                        renderer.render(wgpu_ctx);
                    }
                }
            }
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let mut app = RengineApp {
        config,
        game,
        window: None,
        wgpu_ctx: None,
        input: None,
        sprite_renderer: None,
        last_update: None,
        accumulator: Duration::ZERO,
    };
    event_loop.run_app(&mut app).unwrap();
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

pub use crate::scene::*;
pub use util::*;
pub mod game_object;
pub use game_object::actor::character_actor::CharacterActor;
pub use game_object::actor::Actor;
pub use game_object::GameObject;
pub use graphics::sprite;
