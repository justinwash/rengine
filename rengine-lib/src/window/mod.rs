use wgpu::{Device, Instance, Queue, Surface};
use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

use crate::{Rengine, RengineConfig};

pub struct WgpuContext<'a> {
    pub instance: Instance,
    pub surface: Surface<'a>,
    pub device: Device,
    pub queue: Queue,
    pub config: wgpu::SurfaceConfiguration, // Store the config for resize
}

pub struct RengineApp<'a> {
    pub config: RengineConfig,
    pub engine: Option<Rengine>,
    pub wgpu: Option<WgpuContext<'a>>,
}

impl<'a> ApplicationHandler for RengineApp<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(self.config.window_attributes.clone())
            .unwrap();
        let static_window: &'static Window = Box::leak(Box::new(window));
        let wgpu = pollster::block_on(init_wgpu(static_window));
        self.wgpu = Some(wgpu);
        self.engine = Some(pollster::block_on(Rengine::new(static_window)));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let winit::event::WindowEvent::CloseRequested = event {
            if let Some(cb) = &mut self.config.on_close {
                cb(event_loop);
            }
        }
    }
}

pub async fn init_wgpu(window: &Window) -> WgpuContext<'_> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let surface = instance
        .create_surface(window)
        .expect("Failed to create surface");
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find an appropriate adapter");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::default(),
        })
        .await
        .expect("Failed to create device");
    let size = window.inner_size();
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_capabilities(&adapter).formats[0],
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface.get_capabilities(&adapter).alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2, // Add this field for wgpu 0.19+
    };
    surface.configure(&device, &config);
    WgpuContext {
        instance,
        surface,
        device,
        queue,
        config,
    }
}

pub fn create_window(event_loop: &ActiveEventLoop, attrs: &WindowAttributes) -> Window {
    event_loop.create_window(attrs.clone()).unwrap()
}
