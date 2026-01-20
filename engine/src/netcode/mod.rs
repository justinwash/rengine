use std::sync::Arc;

use winit::dpi::LogicalSize;
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::WindowBuilder;

use crate::app::{Engine, EngineConfig};
use crate::hud;
use crate::input::{GamepadSystem, InputState};
use crate::math::TimeState;
use crate::renderer::{Frame, Renderer};

pub trait InputT:
    Copy
    + Clone
    + PartialEq
    + Default
    + bytemuck::Pod
    + bytemuck::Zeroable
    + serde::Serialize
    + serde::de::DeserializeOwned
    + 'static
{
}
impl<T> InputT for T where
    T: Copy
        + Clone
        + PartialEq
        + Default
        + bytemuck::Pod
        + bytemuck::Zeroable
        + serde::Serialize
        + serde::de::DeserializeOwned
        + 'static
{
}

struct GgrsConfig<I: InputT>(std::marker::PhantomData<I>);

impl<I: InputT> ggrs::Config for GgrsConfig<I> {
    type Input = I;
    type State = Vec<u8>;
    type Address = String;
}

pub enum SessionMode {
    Local,

    SyncTest { check_distance: usize },
}

pub struct RollbackConfig {
    pub num_players: usize,

    pub input_delay: usize,

    pub max_prediction: usize,

    pub fps: u32,

    pub mode: SessionMode,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            num_players: 2,
            input_delay: 2,
            max_prediction: 8,
            fps: 60,
            mode: SessionMode::Local,
        }
    }
}

pub trait RollbackGame: 'static + Sized {
    type Input: InputT;

    fn new(engine: &mut Engine) -> Self;

    fn sample_local_input(&self, engine: &Engine, player: usize) -> Self::Input;

    fn advance(&mut self, inputs: &[Self::Input]);

    fn save(&self) -> Vec<u8>;

    fn load(&mut self, data: &[u8]);

    fn render(&self, engine: &Engine, frame: &mut Frame);
}

pub fn run_rollback<G: RollbackGame>(
    config: EngineConfig,
    rb: RollbackConfig,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let fixed_dt = 1.0 / rb.fps as f32;
    let mut accumulator = 0.0_f32;

    let mut sync_test: Option<ggrs::SyncTestSession<GgrsConfig<G::Input>>> = None;

    if let SessionMode::SyncTest { check_distance } = rb.mode {
        let mut builder = ggrs::SessionBuilder::<GgrsConfig<G::Input>>::new()
            .with_num_players(rb.num_players)
            .with_max_prediction_window(rb.max_prediction)
            .with_input_delay(rb.input_delay)
            .with_check_distance(check_distance);

        for p in 0..rb.num_players {
            builder = builder.add_player(ggrs::PlayerType::Local, p)?;
        }

        sync_test = Some(builder.start_synctest_session()?);
    }

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

                    accumulator += engine.dt();

                    if accumulator >= fixed_dt {
                        let inputs: Vec<G::Input> = (0..rb.num_players)
                            .map(|p| game.sample_local_input(&engine, p))
                            .collect();

                        match sync_test.as_mut() {
                            Some(sess) => {
                                for (p, &inp) in inputs.iter().enumerate() {
                                    sess.add_local_input(p, inp).expect("ggrs: add_local_input");
                                }
                                match sess.advance_frame() {
                                    Ok(requests) => {
                                        for req in requests {
                                            handle_request(&mut game, req);
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("GGRS SyncTest error: {e:?}");
                                    }
                                }
                            }
                            None => {
                                game.advance(&inputs);
                            }
                        }

                        accumulator -= fixed_dt;
                        if accumulator > fixed_dt {
                            accumulator = 0.0;
                        }
                    }

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

fn handle_request<G: RollbackGame>(game: &mut G, request: ggrs::GgrsRequest<GgrsConfig<G::Input>>) {
    match request {
        ggrs::GgrsRequest::SaveGameState { cell, frame } => {
            let data = game.save();
            let checksum = fletcher64(&data);
            cell.save(frame, Some(data), Some(checksum as u128));
        }
        ggrs::GgrsRequest::LoadGameState { cell, .. } => {
            let data = cell.load().expect("ggrs: loaded state should be Some");
            game.load(&data);
        }
        ggrs::GgrsRequest::AdvanceFrame { inputs } => {
            let plain: Vec<G::Input> = inputs.iter().map(|(i, _status)| *i).collect();
            game.advance(&plain);
        }
    }
}

fn fletcher64(data: &[u8]) -> u64 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = s1.wrapping_add(b as u32);
        s2 = s2.wrapping_add(s1);
    }
    ((s2 as u64) << 32) | s1 as u64
}
