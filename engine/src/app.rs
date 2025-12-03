use std::sync::Arc;

use winit::dpi::LogicalSize;
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::WindowBuilder;

use crate::assets::Color;
use crate::hud;
use crate::input::{GamepadSystem, InputState};
use crate::math::TimeState;
use crate::renderer::{Frame, Renderer, TextureId};


pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,


    pub vsync: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Rengine Game".into(),
            width: 800,
            height: 600,
            vsync: false,
        }
    }
}


pub struct Engine {
    pub(crate) renderer: Renderer,
    pub(crate) input: InputState,
    pub(crate) time: TimeState,
    pub(crate) window_width: u32,
    pub(crate) window_height: u32,
    pub(crate) gamepads: GamepadSystem,
}

impl Engine {
    pub fn input(&self) -> &InputState {
        &self.input
    }
    pub fn time(&self) -> &TimeState {
        &self.time
    }

    pub fn dt(&self) -> f32 {
        self.time.dt()
    }
    pub fn window_size(&self) -> (u32, u32) {
        (self.window_width, self.window_height)
    }


    pub fn gamepad(&self, player: usize) -> &crate::input::GamepadState {
        self.gamepads.player(player)
    }


    pub fn gamepads_connected(&self) -> usize {
        self.gamepads.connected_count()
    }


    pub fn create_texture(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureId {
        self.renderer.create_texture(width, height, pixels)
    }


    pub fn create_color_texture(&mut self, width: u32, height: u32, color: Color) -> TextureId {
        let r = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
        let a = (color.a.clamp(0.0, 1.0) * 255.0) as u8;
        let pixels: Vec<u8> = [r, g, b, a]
            .iter()
            .copied()
            .cycle()
            .take((width * height * 4) as usize)
            .collect();
        self.renderer.create_texture(width, height, &pixels)
    }


    pub fn white_texture(&self) -> TextureId {
        self.renderer.white_texture
    }
}


pub trait Game: 'static + Sized {


    fn new(engine: &mut Engine) -> Self;


    fn update(&mut self, engine: &Engine);


    fn render(&mut self, engine: &Engine, frame: &mut Frame);
}


pub fn run<G: Game>(config: EngineConfig) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .build(&event_loop)?,
    );

    let present_mode = if config.vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    };
    let renderer = pollster::block_on(Renderer::new(window.clone(), present_mode));

    let mut engine = Engine {
        renderer,
        input: InputState::new(),
        time: TimeState::new(),
        window_width: config.width,
        window_height: config.height,
        gamepads: GamepadSystem::new(),
    };

    let mut game = G::new(&mut engine);

    event_loop.run(move |event, target| {
        target.set_control_flow(winit::event_loop::ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => target.exit(),

                WindowEvent::Resized(new_size) => {
                    engine.window_width = new_size.width;
                    engine.window_height = new_size.height;
                    engine.renderer.resize(new_size.width, new_size.height);
                }

                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(key),
                            state,
                            ..
                        },
                    ..
                } => {
                    engine.input.handle_key_event(key, state);
                }

                WindowEvent::RedrawRequested => {
                    engine.time.tick();
                    engine.gamepads.update();

                    game.update(&engine);

                    let mut frame = Frame::new();
                    game.render(&engine, &mut frame);

                    let screen_size = engine.window_size();
                    hud::push_fps(&mut frame.hud_verts, engine.time.fps(), screen_size);
                    engine.renderer.render_frame(&frame);

                    engine.input.end_frame();
                }

                _ => {}
            },

            Event::AboutToWait => {
                window.request_redraw();
            }

            _ => {}
        }
    })?;

    Ok(())
}

