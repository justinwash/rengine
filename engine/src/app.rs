use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, WindowBuilder};

use crate::assets::{
    AssetError, AssetPack, AssetPipeline, AudioBus, AudioClip, AudioSystem, Color, MeshAsset,
    SpriteSheet, TextureAsset,
};
use crate::canvas;
use crate::input::{ActionMap, GamepadAssignMode, GamepadSystem, InputState};
use crate::math::tween::Easing;
use crate::math::{Rng, TimeState};
use crate::renderer::postfx::PostFxChain;
use crate::renderer::{Frame, Renderer, TextureId};
use crate::renderer3d::{Frame3D, MeshId, Renderer3D, Vertex3D};
use crate::scene::{Globals, Scene, Scene2D, Scene3D, SceneOp, SceneOp3D};
use crate::text;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleMode {
    Stretch,
    Letterbox,
    PixelPerfect,
}

impl Default for ScaleMode {
    fn default() -> Self {
        Self::Letterbox
    }
}

pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,

    pub vsync: bool,
    pub headless: bool,
    pub hot_reload: bool,
    pub show_fps: bool,
    pub fixed_dt: f32,

    pub render_width: Option<u32>,
    pub render_height: Option<u32>,
    pub scale_mode: ScaleMode,
    pub gamepad_assign: GamepadAssignMode,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Rengine Game".into(),
            width: 800,
            height: 600,
            vsync: false,
            headless: false,
            hot_reload: true,
            show_fps: true,
            fixed_dt: 1.0 / 60.0,
            render_width: None,
            render_height: None,
            scale_mode: ScaleMode::default(),
            gamepad_assign: GamepadAssignMode::default(),
        }
    }
}

pub struct Engine {
    pub(crate) renderer: Renderer,
    pub(crate) assets: AssetPipeline,
    pub(crate) audio: AudioSystem,
    pub(crate) input: InputState,
    pub(crate) time: TimeState,
    pub(crate) window_width: u32,
    pub(crate) window_height: u32,
    pub(crate) render_resolution: Option<(u32, u32)>,
    pub(crate) gamepads: GamepadSystem,
    pub(crate) hot_reload_enabled: bool,
    pub(crate) actions: ActionMap,
    pub(crate) rng: RefCell<Rng>,
    pub(crate) postfx_chain: PostFxChain,
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

    pub fn half_size(&self) -> (f32, f32) {
        (
            self.window_width as f32 / 2.0,
            self.window_height as f32 / 2.0,
        )
    }

    pub fn game_size(&self) -> (u32, u32) {
        self.render_resolution
            .unwrap_or((self.window_width, self.window_height))
    }

    pub fn mouse_screen_pos(&self) -> glam::Vec2 {
        let (x, y) = self.input.mouse_position();
        glam::Vec2::new(x, y)
    }

    pub fn set_scale_mode(&self, mode: ScaleMode) {
        self.renderer.set_scale_mode(mode);
    }

    pub fn postfx(&self) -> &PostFxChain {
        &self.postfx_chain
    }

    pub fn gamepad(&self, player: usize) -> &crate::input::GamepadState {
        self.gamepads.player(player)
    }

    pub fn gamepads_connected(&self) -> usize {
        self.gamepads.connected_count()
    }

    pub fn gamepads_unassigned(&self) -> usize {
        self.gamepads.unassigned_count()
    }

    pub fn set_gamepad_assign_mode(&mut self, mode: GamepadAssignMode) {
        self.gamepads.set_assign_mode(mode);
    }

    pub fn actions(&self) -> &ActionMap {
        &self.actions
    }

    pub fn actions_mut(&mut self) -> &mut ActionMap {
        &mut self.actions
    }

    pub fn rng(&self) -> std::cell::RefMut<'_, Rng> {
        self.rng.borrow_mut()
    }

    pub fn action_down(&self, action: &str) -> bool {
        self.actions
            .is_down(action, &self.input, self.gamepads.player(0))
    }

    pub fn action_pressed(&self, action: &str) -> bool {
        self.actions
            .is_pressed(action, &self.input, self.gamepads.player(0))
    }

    pub fn action_released(&self, action: &str) -> bool {
        self.actions
            .is_released(action, &self.input, self.gamepads.player(0))
    }

    pub fn axis(&self, name: &str) -> f32 {
        self.actions
            .axis(name, &self.input, self.gamepads.player(0))
    }

    pub fn action_down_player(&self, action: &str, player: usize) -> bool {
        self.actions
            .is_down(action, &self.input, self.gamepads.player_or_default(player))
    }

    pub fn action_pressed_player(&self, action: &str, player: usize) -> bool {
        self.actions
            .is_pressed(action, &self.input, self.gamepads.player_or_default(player))
    }

    pub fn action_released_player(&self, action: &str, player: usize) -> bool {
        self.actions
            .is_released(action, &self.input, self.gamepads.player_or_default(player))
    }

    pub fn axis_player(&self, name: &str, player: usize) -> f32 {
        self.actions
            .axis(name, &self.input, self.gamepads.player_or_default(player))
    }

    pub fn asset_root(&self) -> &Path {
        self.assets.root()
    }

    pub fn set_asset_root<P: Into<PathBuf>>(&mut self, root: P) {
        self.assets.set_root(root);
    }

    pub fn hot_reload_enabled(&self) -> bool {
        self.hot_reload_enabled
    }

    pub fn set_hot_reload_enabled(&mut self, enabled: bool) {
        self.hot_reload_enabled = enabled;
    }

    pub fn create_texture(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureId {
        self.renderer.create_texture(width, height, pixels)
    }

    pub fn load_bytes<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<[u8]>, AssetError> {
        self.assets.load_bytes(path)
    }

    pub fn load_text<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<str>, AssetError> {
        self.assets.load_text(path)
    }

    pub fn load_resource<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<T, AssetError> {
        let text = self.assets.load_text(&path)?;
        let resolved = self.assets.resolve_path(path.as_ref());
        serde_json::from_str(&text).map_err(|source| AssetError::Json {
            path: resolved,
            source,
        })
    }

    pub fn load_resource_list<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<T>, AssetError> {
        let text = self.assets.load_text(&path)?;
        let resolved = self.assets.resolve_path(path.as_ref());
        serde_json::from_str(&text).map_err(|source| AssetError::Json {
            path: resolved,
            source,
        })
    }

    pub fn load_asset_manifest<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<AssetPack, AssetError> {
        let manifest_path = self.assets.resolve_path(path.as_ref());
        let manifest = self.assets.load_manifest(&manifest_path)?;
        let mut pack = AssetPack::default();
        let mut deps = Vec::new();

        for (alias, rel_path) in manifest.bytes {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_bytes(alias, self.assets.load_bytes(rel_path)?);
        }
        for (alias, rel_path) in manifest.text {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_text(alias, self.assets.load_text(rel_path)?);
        }
        for (alias, rel_path) in manifest.textures {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_texture(alias, self.load_texture(rel_path)?);
        }
        for (alias, sheet) in manifest.sprite_sheets {
            let resolved = self.assets.resolve_path(Path::new(&sheet.path));
            deps.push(resolved);
            pack.insert_sprite_sheet(
                alias,
                self.load_sprite_sheet(sheet.path, sheet.cell_width, sheet.cell_height)?,
            );
        }
        for (alias, rel_path) in manifest.audio {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_audio(alias, self.load_audio(rel_path)?);
        }
        if !manifest.meshes.is_empty() {
            return Err(AssetError::manifest_message(
                self.assets.root(),
                "2D Engine manifest cannot load mesh entries; use Engine3D instead",
            ));
        }

        self.assets.record_manifest_deps(manifest_path, deps);
        Ok(pack)
    }

    pub fn load_texture<P: AsRef<Path>>(&mut self, path: P) -> Result<TextureAsset, AssetError> {
        self.assets.load_texture(path, |width, height, pixels| {
            self.renderer.create_texture(width, height, pixels)
        })
    }

    pub fn load_sprite_sheet<P: AsRef<Path>>(
        &mut self,
        path: P,
        cell_width: u32,
        cell_height: u32,
    ) -> Result<SpriteSheet, AssetError> {
        self.assets
            .load_sprite_sheet(path, cell_width, cell_height, |width, height, pixels| {
                self.renderer.create_texture(width, height, pixels)
            })
    }

    pub fn load_audio<P: AsRef<Path>>(&mut self, path: P) -> Result<AudioClip, AssetError> {
        let resolved = self.assets.resolve_path(path.as_ref());
        let bytes = self.assets.load_bytes(path)?;
        Ok(self.audio.register_clip(resolved, bytes))
    }

    pub fn play_sound(&self, clip: &AudioClip) -> Result<(), AssetError> {
        self.audio.play(clip)
    }

    pub fn play_sound_on_bus(
        &self,
        bus: AudioBus,
        clip: &AudioClip,
        volume: f32,
    ) -> Result<(), AssetError> {
        self.audio.play_on_bus(bus, clip, volume)
    }

    pub fn play_music(&self, clip: &AudioClip) -> Result<(), AssetError> {
        self.audio.play_music(clip)
    }

    pub fn play_music_with_volume(&self, clip: &AudioClip, volume: f32) -> Result<(), AssetError> {
        self.audio.play_music_with_volume(clip, volume)
    }

    pub fn stop_music(&self) {
        self.audio.stop_music();
    }

    pub fn pause_music(&self) {
        self.audio.pause_music();
    }

    pub fn resume_music(&self) {
        self.audio.resume_music();
    }

    pub fn stop_audio_bus(&self, bus: AudioBus) {
        self.audio.stop_bus(bus);
    }

    pub fn set_master_volume(&self, volume: f32) {
        self.audio.set_master_volume(volume);
    }

    pub fn set_audio_bus_volume(&self, bus: AudioBus, volume: f32) {
        self.audio.set_bus_volume(bus, volume);
    }

    pub fn audio_bus_volume(&self, bus: AudioBus) -> f32 {
        self.audio.bus_volume(bus)
    }

    pub fn fade_in_music(
        &self,
        clip: &AudioClip,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio.fade_in_music(clip, duration, easing)
    }

    pub fn fade_in_music_with_volume(
        &self,
        clip: &AudioClip,
        volume: f32,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio
            .fade_in_music_with_volume(clip, volume, duration, easing)
    }

    pub fn fade_out_music(&self, duration: f32, easing: Easing) {
        self.audio.fade_out_music(duration, easing);
    }

    pub fn crossfade_music(
        &self,
        clip: &AudioClip,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio.crossfade_music(clip, duration, easing)
    }

    pub fn crossfade_music_with_volume(
        &self,
        clip: &AudioClip,
        volume: f32,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio
            .crossfade_music_with_volume(clip, volume, duration, easing)
    }

    pub fn fade_bus_volume(&self, bus: AudioBus, target: f32, duration: f32, easing: Easing) {
        self.audio.fade_bus_volume(bus, target, duration, easing);
    }

    pub fn fade_master_volume(&self, target: f32, duration: f32, easing: Easing) {
        self.audio.fade_master_volume(target, duration, easing);
    }

    pub fn is_audio_fading(&self) -> bool {
        self.audio.is_fading()
    }

    pub fn load_scene2d<P: AsRef<Path>>(
        &mut self,
        assets: &AssetPack,
        path: P,
    ) -> Result<Scene2D, AssetError> {
        let resolved = self.assets.resolve_path(path.as_ref());
        Scene2D::load_from_path(&resolved, assets)
    }

    pub fn reload_assets_if_changed(&mut self) {
        if !self.hot_reload_enabled {
            return;
        }

        for result in self
            .assets
            .reload_changed_textures(|id, width, height, pixels| {
                self.renderer.replace_texture(id, width, height, pixels)
            })
        {
            match result {
                Ok(path) => log::info!("Reloaded texture {}", path.display()),
                Err(error) => log::warn!("Texture reload failed: {error}"),
            }
        }

        for path in self.assets.invalidate_changed_manifests() {
            log::info!("Invalidated asset manifest {}", path.display());
        }

        for result in self.audio.reload_changed() {
            match result {
                Ok(path) => log::info!("Reloaded audio {}", path.display()),
                Err(error) => log::warn!("Audio reload failed: {error}"),
            }
        }
    }

    pub fn validate_manifest<P: AsRef<Path>>(&self, path: P) -> Vec<AssetError> {
        let path = path.as_ref();
        let mut errors = self.assets.validate_manifest(path);
        if let Ok(manifest) = self.assets.peek_manifest(path) {
            if !manifest.meshes.is_empty() {
                errors.push(AssetError::manifest_message(
                    &self.assets.resolve_path(path),
                    "2D Engine manifest cannot contain mesh entries; use Engine3D instead",
                ));
            }
        }
        errors
    }

    pub fn loaded_asset_summary(&self) -> crate::assets::AssetSummary {
        self.assets.loaded_asset_summary()
    }

    pub fn manifest_dependencies<P: AsRef<Path>>(&self, path: P) -> Option<Vec<PathBuf>> {
        self.assets
            .manifest_dependencies(path)
            .map(|deps| deps.to_vec())
    }

    pub fn unload_texture<P: AsRef<Path>>(&mut self, path: P) {
        self.assets.unload_texture(path);
    }

    pub fn unload_data<P: AsRef<Path>>(&mut self, path: P) {
        self.assets.unload_data(path);
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

    pub fn font_atlas(&self) -> &text::FontAtlas {
        self.font(text::FontId::DEFAULT)
    }

    pub fn load_font<P: AsRef<Path>>(&mut self, path: P) -> Result<text::FontId, AssetError> {
        let bytes = self.assets.load_bytes(path)?;
        Ok(self.renderer.load_font(&bytes))
    }

    pub fn font(&self, id: text::FontId) -> &text::FontAtlas {
        &self.renderer.fonts[id.0]
    }
}

pub trait Game: 'static + Sized {
    fn new(engine: &mut Engine) -> Self;

    fn update(&mut self, engine: &Engine, frame: &mut Frame);

    fn fixed_update(&mut self, _engine: &Engine) {}

    fn render(&mut self, _engine: &Engine, _frame: &mut Frame) {}

    fn should_exit(&self) -> bool {
        false
    }
}

pub fn run<G: Game>(config: EngineConfig) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let headless = config.headless;
    let show_fps = config.show_fps;
    let fixed_dt = config.fixed_dt;
    let gamepad_assign = config.gamepad_assign;
    assert!(
        config.render_width.is_some() == config.render_height.is_some(),
        "render_width and render_height must both be set or both be None"
    );
    let render_res = config
        .render_width
        .and_then(|w| config.render_height.map(|h| (w, h)));
    if let Some((rw, rh)) = render_res {
        assert!(
            rw >= 1 && rh >= 1,
            "render_width and render_height must both be >= 1"
        );
    }
    let scale_mode = config.scale_mode;

    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_visible(!headless)
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
        assets: AssetPipeline::default(),
        audio: AudioSystem::new(config.headless),
        input: InputState::new(),
        time: TimeState::new(),
        window_width: config.width,
        window_height: config.height,
        render_resolution: render_res,
        gamepads: GamepadSystem::new(gamepad_assign),
        hot_reload_enabled: config.hot_reload,
        actions: ActionMap::new(),
        rng: RefCell::new(Rng::from_time()),
        postfx_chain: PostFxChain::new(),
    };
    engine.time.set_fixed_dt(fixed_dt);
    if let Some((rw, rh)) = render_res {
        engine.renderer.init_offscreen(rw, rh, scale_mode);
    }

    let mut game = G::new(&mut engine);
    let mut frame = Frame::new();

    if headless {
        let mut headless_frame = Frame::new();
        loop {
            engine.time.tick();
            engine.gamepads.update();
            engine.reload_assets_if_changed();
            engine.audio.update(engine.time.dt());
            while engine.time.consume_fixed_step() {
                game.fixed_update(&engine);
            }
            headless_frame.begin(engine.window_size());
            game.update(&engine, &mut headless_frame);
            if game.should_exit() {
                return Ok(());
            }
            engine.input.end_frame();
        }
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

                WindowEvent::CursorMoved { position, .. } => {
                    let x = position.x as f32 - engine.window_width as f32 / 2.0;
                    let y = -(position.y as f32 - engine.window_height as f32 / 2.0);
                    engine.input.handle_cursor_moved(x, y);
                }

                WindowEvent::MouseInput { button, state, .. } => {
                    let idx = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        _ => return,
                    };
                    engine.input.handle_mouse_button(idx, state);
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(pos) => {
                            (pos.x as f32 / 40.0, pos.y as f32 / 40.0)
                        }
                    };
                    engine.input.handle_scroll(dx, dy);
                }

                WindowEvent::RedrawRequested => {
                    engine.time.tick();
                    engine.gamepads.update();
                    engine.reload_assets_if_changed();
                    engine.audio.update(engine.time.dt());

                    while engine.time.consume_fixed_step() {
                        game.fixed_update(&engine);
                    }
                    frame.begin(engine.window_size());
                    game.update(&engine, &mut frame);

                    if game.should_exit() {
                        target.exit();
                        return;
                    }

                    game.render(&engine, &mut frame);

                    if show_fps {
                        let screen_size = engine.window_size();
                        let mut fps_canvas = canvas::Canvas::new(screen_size);
                        canvas::draw_fps(
                            &mut fps_canvas,
                            engine.time.fps(),
                            &engine.renderer.fonts[0],
                        );
                        frame.canvases.push(fps_canvas);
                    }
                    engine
                        .renderer
                        .render_frame(&mut frame, &engine.postfx_chain);

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

pub fn run_with_scenes<F>(config: EngineConfig, init: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut Engine, &mut Globals) -> Box<dyn Scene>,
{
    env_logger::init();

    let headless = config.headless;
    let show_fps = config.show_fps;
    let fixed_dt = config.fixed_dt;
    let gamepad_assign = config.gamepad_assign;
    let render_res = config
        .render_width
        .and_then(|w| config.render_height.map(|h| (w, h)));
    let scale_mode = config.scale_mode;

    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_visible(!headless)
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
        assets: AssetPipeline::default(),
        audio: AudioSystem::new(config.headless),
        input: InputState::new(),
        time: TimeState::new(),
        window_width: config.width,
        window_height: config.height,
        render_resolution: render_res,
        gamepads: GamepadSystem::new(gamepad_assign),
        hot_reload_enabled: config.hot_reload,
        actions: ActionMap::new(),
        rng: RefCell::new(Rng::from_time()),
        postfx_chain: PostFxChain::new(),
    };
    engine.time.set_fixed_dt(fixed_dt);
    if let Some((rw, rh)) = render_res {
        engine.renderer.init_offscreen(rw, rh, scale_mode);
    }

    let mut globals = Globals::new();
    let mut stack: Vec<Box<dyn Scene>> = Vec::new();

    let mut initial = init(&mut engine, &mut globals);
    initial.on_enter(&mut engine, &mut globals);
    stack.push(initial);
    let mut frame = Frame::new();

    if headless {
        loop {
            engine.time.tick();
            engine.gamepads.update();
            engine.reload_assets_if_changed();
            engine.audio.update(engine.time.dt());

            while engine.time.consume_fixed_step() {
                if let Some(scene) = stack.last_mut() {
                    scene.fixed_update(&engine, &mut globals);
                }
            }

            frame.begin(engine.window_size());
            let op = if let Some(scene) = stack.last_mut() {
                scene.update(&engine, &mut globals, &mut frame)
            } else {
                return Ok(());
            };

            apply_scene_op(&mut stack, op, &mut engine, &mut globals);

            if stack.is_empty() {
                return Ok(());
            }

            engine.input.end_frame();
        }
    }

    let mut transition: Option<crate::scene::ActiveTransition> = None;

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

                WindowEvent::CursorMoved { position, .. } => {
                    let x = position.x as f32 - engine.window_width as f32 / 2.0;
                    let y = -(position.y as f32 - engine.window_height as f32 / 2.0);
                    engine.input.handle_cursor_moved(x, y);
                }

                WindowEvent::MouseInput { button, state, .. } => {
                    let idx = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        _ => return,
                    };
                    engine.input.handle_mouse_button(idx, state);
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(pos) => {
                            (pos.x as f32 / 40.0, pos.y as f32 / 40.0)
                        }
                    };
                    engine.input.handle_scroll(dx, dy);
                }

                WindowEvent::RedrawRequested => {
                    engine.time.tick();
                    engine.gamepads.update();
                    engine.reload_assets_if_changed();
                    engine.audio.update(engine.time.dt());

                    while engine.time.consume_fixed_step() {
                        if let Some(scene) = stack.last_mut() {
                            scene.fixed_update(&engine, &mut globals);
                        }
                    }

                    frame.begin(engine.window_size());

                    if transition.is_none() {
                        let op = if let Some(scene) = stack.last_mut() {
                            scene.update(&engine, &mut globals, &mut frame)
                        } else {
                            target.exit();
                            return;
                        };

                        match op {
                            SceneOp::FadePush(new_scene, t) => {
                                transition = Some(crate::scene::ActiveTransition::new(
                                    t,
                                    SceneOp::Push(new_scene),
                                ));
                            }
                            SceneOp::FadeSwitch(new_scene, t) => {
                                transition = Some(crate::scene::ActiveTransition::new(
                                    t,
                                    SceneOp::Switch(new_scene),
                                ));
                            }
                            SceneOp::FadePop(t) => {
                                transition =
                                    Some(crate::scene::ActiveTransition::new(t, SceneOp::Pop));
                            }
                            other => {
                                apply_scene_op(&mut stack, other, &mut engine, &mut globals);
                            }
                        }
                    }

                    if let Some(ref mut t) = transition {
                        t.tick(engine.time.dt());
                        if t.at_midpoint() {
                            if let Some(pending) = t.pending_op.take() {
                                apply_scene_op(&mut stack, pending, &mut engine, &mut globals);
                            }
                        }
                    }

                    if stack.is_empty() {
                        target.exit();
                        return;
                    }

                    for scene in stack.iter() {
                        scene.render(&engine, &globals, &mut frame);
                    }

                    if let Some(ref t) = transition {
                        let alpha = t.alpha();
                        if alpha > 0.001 {
                            let screen_size = engine.window_size();
                            let hw = screen_size.0 as f32 / 2.0;
                            let hh = screen_size.1 as f32 / 2.0;
                            let mut overlay = canvas::Canvas::new(screen_size);
                            let c =
                                crate::assets::Color::new(t.color.r, t.color.g, t.color.b, alpha);
                            overlay.rect(-hw, -hh, screen_size.0 as f32, screen_size.1 as f32, c);
                            frame.canvases.push(overlay);
                        }
                    }

                    if let Some(ref t) = transition {
                        if t.is_done() {
                            transition = None;
                        }
                    }

                    if show_fps {
                        let screen_size = engine.window_size();
                        let mut fps_canvas = canvas::Canvas::new(screen_size);
                        canvas::draw_fps(
                            &mut fps_canvas,
                            engine.time.fps(),
                            &engine.renderer.fonts[0],
                        );
                        frame.canvases.push(fps_canvas);
                    }
                    engine
                        .renderer
                        .render_frame(&mut frame, &engine.postfx_chain);

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

fn apply_scene_op(
    stack: &mut Vec<Box<dyn Scene>>,
    op: SceneOp,
    engine: &mut Engine,
    globals: &mut Globals,
) {
    match op {
        SceneOp::Continue => {}
        SceneOp::Quit => {
            while let Some(mut scene) = stack.pop() {
                scene.on_exit(engine, globals);
            }
        }
        SceneOp::Push(mut new_scene) => {
            if let Some(current) = stack.last_mut() {
                current.on_pause(engine, globals);
            }
            new_scene.on_enter(engine, globals);
            stack.push(new_scene);
        }
        SceneOp::Pop => {
            if let Some(mut old) = stack.pop() {
                old.on_exit(engine, globals);
            }
            if let Some(current) = stack.last_mut() {
                current.on_resume(engine, globals);
            }
        }
        SceneOp::Switch(mut new_scene) => {
            if let Some(mut old) = stack.pop() {
                old.on_exit(engine, globals);
            }
            new_scene.on_enter(engine, globals);
            stack.push(new_scene);
        }
        SceneOp::FadePush(new_scene, _) => {
            apply_scene_op(stack, SceneOp::Push(new_scene), engine, globals);
        }
        SceneOp::FadeSwitch(new_scene, _) => {
            apply_scene_op(stack, SceneOp::Switch(new_scene), engine, globals);
        }
        SceneOp::FadePop(_) => {
            apply_scene_op(stack, SceneOp::Pop, engine, globals);
        }
    }
}

pub struct Engine3D {
    pub(crate) renderer: Renderer3D,
    pub(crate) assets: AssetPipeline,
    pub(crate) audio: AudioSystem,
    input: InputState,
    time: TimeState,
    window_width: u32,
    window_height: u32,
    render_resolution: Option<(u32, u32)>,
    mouse_captured: bool,
    hot_reload_enabled: bool,
    actions: ActionMap,
    no_gamepad: crate::input::GamepadState,
    rng: RefCell<Rng>,
}

impl Engine3D {
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

    pub fn half_size(&self) -> (f32, f32) {
        (
            self.window_width as f32 / 2.0,
            self.window_height as f32 / 2.0,
        )
    }

    pub fn game_size(&self) -> (u32, u32) {
        self.render_resolution
            .unwrap_or((self.window_width, self.window_height))
    }

    pub fn mouse_screen_pos(&self) -> glam::Vec2 {
        let (x, y) = self.input.mouse_position();
        glam::Vec2::new(x, y)
    }

    pub fn set_scale_mode(&self, mode: ScaleMode) {
        self.renderer.set_scale_mode(mode);
    }

    pub fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    pub fn actions(&self) -> &ActionMap {
        &self.actions
    }

    pub fn actions_mut(&mut self) -> &mut ActionMap {
        &mut self.actions
    }

    pub fn rng(&self) -> std::cell::RefMut<'_, Rng> {
        self.rng.borrow_mut()
    }

    pub fn action_down(&self, action: &str) -> bool {
        self.actions.is_down(action, &self.input, &self.no_gamepad)
    }

    pub fn action_pressed(&self, action: &str) -> bool {
        self.actions
            .is_pressed(action, &self.input, &self.no_gamepad)
    }

    pub fn action_released(&self, action: &str) -> bool {
        self.actions
            .is_released(action, &self.input, &self.no_gamepad)
    }

    pub fn axis(&self, name: &str) -> f32 {
        self.actions.axis(name, &self.input, &self.no_gamepad)
    }

    pub fn asset_root(&self) -> &Path {
        self.assets.root()
    }

    pub fn set_asset_root<P: Into<PathBuf>>(&mut self, root: P) {
        self.assets.set_root(root);
    }

    pub fn font_atlas(&self) -> &text::FontAtlas {
        self.font(text::FontId::DEFAULT)
    }

    pub fn load_font<P: AsRef<Path>>(&mut self, path: P) -> Result<text::FontId, AssetError> {
        let bytes = self.assets.load_bytes(path)?;
        Ok(self.renderer.load_font(&bytes))
    }

    pub fn font(&self, id: text::FontId) -> &text::FontAtlas {
        &self.renderer.fonts[id.0]
    }

    pub fn load_bytes<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<[u8]>, AssetError> {
        self.assets.load_bytes(path)
    }

    pub fn load_text<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<str>, AssetError> {
        self.assets.load_text(path)
    }

    pub fn load_resource<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<T, AssetError> {
        let text = self.assets.load_text(&path)?;
        let resolved = self.assets.resolve_path(path.as_ref());
        serde_json::from_str(&text).map_err(|source| AssetError::Json {
            path: resolved,
            source,
        })
    }

    pub fn load_resource_list<T: DeserializeOwned>(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<T>, AssetError> {
        let text = self.assets.load_text(&path)?;
        let resolved = self.assets.resolve_path(path.as_ref());
        serde_json::from_str(&text).map_err(|source| AssetError::Json {
            path: resolved,
            source,
        })
    }

    pub fn load_asset_manifest<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<AssetPack, AssetError> {
        let manifest_path = self.assets.resolve_path(path.as_ref());
        let manifest = self.assets.load_manifest(&manifest_path)?;
        let mut pack = AssetPack::default();
        let mut deps = Vec::new();

        for (alias, rel_path) in manifest.bytes {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_bytes(alias, self.assets.load_bytes(rel_path)?);
        }
        for (alias, rel_path) in manifest.text {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_text(alias, self.assets.load_text(rel_path)?);
        }
        for (alias, rel_path) in manifest.audio {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_audio(alias, self.load_audio(rel_path)?);
        }
        for (alias, rel_path) in manifest.meshes {
            let resolved = self.assets.resolve_path(Path::new(&rel_path));
            deps.push(resolved);
            pack.insert_mesh(alias, self.load_mesh(rel_path)?);
        }
        if !manifest.textures.is_empty() || !manifest.sprite_sheets.is_empty() {
            return Err(AssetError::manifest_message(
                self.assets.root(),
                "3D Engine manifest currently supports meshes, audio, text, and bytes only",
            ));
        }

        self.assets.record_manifest_deps(manifest_path, deps);
        Ok(pack)
    }

    pub fn load_obj_mesh<P: AsRef<Path>>(&mut self, path: P) -> Result<MeshAsset, AssetError> {
        self.assets.load_obj_mesh(path, |vertices, indices| {
            self.renderer.create_mesh(vertices, indices)
        })
    }

    pub fn load_gltf_mesh<P: AsRef<Path>>(&mut self, path: P) -> Result<MeshAsset, AssetError> {
        self.assets.load_gltf_mesh(path, |vertices, indices| {
            self.renderer.create_mesh(vertices, indices)
        })
    }

    pub fn load_mesh<P: AsRef<Path>>(&mut self, path: P) -> Result<MeshAsset, AssetError> {
        self.assets.load_mesh(path, |vertices, indices| {
            self.renderer.create_mesh(vertices, indices)
        })
    }

    pub fn load_audio<P: AsRef<Path>>(&mut self, path: P) -> Result<AudioClip, AssetError> {
        let resolved = self.assets.resolve_path(path.as_ref());
        let bytes = self.assets.load_bytes(path)?;
        Ok(self.audio.register_clip(resolved, bytes))
    }

    pub fn play_sound(&self, clip: &AudioClip) -> Result<(), AssetError> {
        self.audio.play(clip)
    }

    pub fn play_sound_on_bus(
        &self,
        bus: AudioBus,
        clip: &AudioClip,
        volume: f32,
    ) -> Result<(), AssetError> {
        self.audio.play_on_bus(bus, clip, volume)
    }

    pub fn play_music(&self, clip: &AudioClip) -> Result<(), AssetError> {
        self.audio.play_music(clip)
    }

    pub fn play_music_with_volume(&self, clip: &AudioClip, volume: f32) -> Result<(), AssetError> {
        self.audio.play_music_with_volume(clip, volume)
    }

    pub fn stop_music(&self) {
        self.audio.stop_music();
    }

    pub fn pause_music(&self) {
        self.audio.pause_music();
    }

    pub fn resume_music(&self) {
        self.audio.resume_music();
    }

    pub fn stop_audio_bus(&self, bus: AudioBus) {
        self.audio.stop_bus(bus);
    }

    pub fn set_master_volume(&self, volume: f32) {
        self.audio.set_master_volume(volume);
    }

    pub fn set_audio_bus_volume(&self, bus: AudioBus, volume: f32) {
        self.audio.set_bus_volume(bus, volume);
    }

    pub fn audio_bus_volume(&self, bus: AudioBus) -> f32 {
        self.audio.bus_volume(bus)
    }

    pub fn fade_in_music(
        &self,
        clip: &AudioClip,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio.fade_in_music(clip, duration, easing)
    }

    pub fn fade_in_music_with_volume(
        &self,
        clip: &AudioClip,
        volume: f32,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio
            .fade_in_music_with_volume(clip, volume, duration, easing)
    }

    pub fn fade_out_music(&self, duration: f32, easing: Easing) {
        self.audio.fade_out_music(duration, easing);
    }

    pub fn crossfade_music(
        &self,
        clip: &AudioClip,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio.crossfade_music(clip, duration, easing)
    }

    pub fn crossfade_music_with_volume(
        &self,
        clip: &AudioClip,
        volume: f32,
        duration: f32,
        easing: Easing,
    ) -> Result<(), AssetError> {
        self.audio
            .crossfade_music_with_volume(clip, volume, duration, easing)
    }

    pub fn fade_bus_volume(&self, bus: AudioBus, target: f32, duration: f32, easing: Easing) {
        self.audio.fade_bus_volume(bus, target, duration, easing);
    }

    pub fn fade_master_volume(&self, target: f32, duration: f32, easing: Easing) {
        self.audio.fade_master_volume(target, duration, easing);
    }

    pub fn is_audio_fading(&self) -> bool {
        self.audio.is_fading()
    }

    pub fn reload_assets_if_changed(&mut self) {
        if !self.hot_reload_enabled {
            return;
        }

        for result in self.assets.reload_changed_meshes(|id, vertices, indices| {
            self.renderer.replace_mesh(id, vertices, indices)
        }) {
            match result {
                Ok(path) => log::info!("Reloaded mesh {}", path.display()),
                Err(error) => log::warn!("Mesh reload failed: {error}"),
            }
        }

        for path in self.assets.invalidate_changed_manifests() {
            log::info!("Invalidated asset manifest {}", path.display());
        }

        for result in self.audio.reload_changed() {
            match result {
                Ok(path) => log::info!("Reloaded audio {}", path.display()),
                Err(error) => log::warn!("Audio reload failed: {error}"),
            }
        }
    }

    pub fn validate_manifest<P: AsRef<Path>>(&self, path: P) -> Vec<AssetError> {
        let path = path.as_ref();
        let mut errors = self.assets.validate_manifest(path);
        if let Ok(manifest) = self.assets.peek_manifest(path) {
            if !manifest.textures.is_empty() || !manifest.sprite_sheets.is_empty() {
                errors.push(AssetError::manifest_message(
                    &self.assets.resolve_path(path),
                    "3D Engine manifest does not support textures or sprite_sheets",
                ));
            }
        }
        errors
    }

    pub fn loaded_asset_summary(&self) -> crate::assets::AssetSummary {
        self.assets.loaded_asset_summary()
    }

    pub fn manifest_dependencies<P: AsRef<Path>>(&self, path: P) -> Option<Vec<PathBuf>> {
        self.assets
            .manifest_dependencies(path)
            .map(|deps| deps.to_vec())
    }

    pub fn unload_mesh<P: AsRef<Path>>(&mut self, path: P) {
        self.assets.unload_mesh(path);
    }

    pub fn unload_data<P: AsRef<Path>>(&mut self, path: P) {
        self.assets.unload_data(path);
    }

    pub fn create_mesh(&mut self, vertices: Vec<Vertex3D>, indices: Vec<u32>) -> MeshId {
        self.renderer.create_mesh(vertices, indices)
    }
}

pub trait Game3D: 'static + Sized {
    fn new(engine: &mut Engine3D) -> Self;
    fn update(&mut self, engine: &Engine3D, frame: &mut Frame3D);
    fn fixed_update(&mut self, _engine: &Engine3D) {}
    fn render(&mut self, _engine: &Engine3D, _frame: &mut Frame3D) {}
    fn should_exit(&self) -> bool {
        false
    }
}

pub fn run3d<G: Game3D>(config: EngineConfig) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let headless = config.headless;
    let show_fps = config.show_fps;
    let fixed_dt = config.fixed_dt;
    assert!(
        config.render_width.is_some() == config.render_height.is_some(),
        "render_width and render_height must both be set or both be None"
    );
    let render_res = config
        .render_width
        .and_then(|w| config.render_height.map(|h| (w, h)));
    if let Some((rw, rh)) = render_res {
        assert!(
            rw >= 1 && rh >= 1,
            "render_width and render_height must both be >= 1"
        );
    }
    let scale_mode = config.scale_mode;

    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_visible(!headless)
            .build(&event_loop)?,
    );

    let present_mode = if config.vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    };
    let renderer = pollster::block_on(Renderer3D::new(window.clone(), present_mode));

    let mut engine = Engine3D {
        renderer,
        assets: AssetPipeline::default(),
        audio: AudioSystem::new(config.headless),
        input: InputState::new(),
        time: TimeState::new(),
        window_width: config.width,
        window_height: config.height,
        render_resolution: render_res,
        mouse_captured: false,
        hot_reload_enabled: config.hot_reload,
        actions: ActionMap::new(),
        no_gamepad: crate::input::GamepadState::new(),
        rng: RefCell::new(Rng::from_time()),
    };
    engine.time.set_fixed_dt(fixed_dt);
    if let Some((rw, rh)) = render_res {
        engine.renderer.init_offscreen(rw, rh, scale_mode);
    }

    let mut game = G::new(&mut engine);

    if headless {
        loop {
            engine.time.tick();
            engine.reload_assets_if_changed();
            engine.audio.update(engine.time.dt());
            while engine.time.consume_fixed_step() {
                game.fixed_update(&engine);
            }
            let mut headless_frame = Frame3D::new(engine.window_size());
            game.update(&engine, &mut headless_frame);
            if game.should_exit() {
                return Ok(());
            }
            engine.input.end_frame();
        }
    }

    let _ = window
        .set_cursor_grab(CursorGrabMode::Confined)
        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
    window.set_cursor_visible(false);
    engine.mouse_captured = true;

    event_loop.run(move |event, target| {
        target.set_control_flow(winit::event_loop::ControlFlow::Poll);
        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                ..
            } => {
                if engine.mouse_captured {
                    engine.input.handle_mouse_motion(dx, dy);
                }
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => target.exit(),

                WindowEvent::Focused(focused) => {
                    if focused {
                        let _ = window
                            .set_cursor_grab(CursorGrabMode::Confined)
                            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
                        window.set_cursor_visible(false);
                        engine.mouse_captured = true;
                    } else {
                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                        window.set_cursor_visible(true);
                        engine.mouse_captured = false;
                    }
                }

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
                    if key == winit::keyboard::KeyCode::Escape
                        && state == winit::event::ElementState::Pressed
                    {
                        if engine.mouse_captured {
                            let _ = window.set_cursor_grab(CursorGrabMode::None);
                            window.set_cursor_visible(true);
                            engine.mouse_captured = false;
                        } else {
                            target.exit();
                        }
                    }
                    engine.input.handle_key_event(key, state);
                }

                WindowEvent::MouseInput { button, state, .. } => {
                    let idx = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        _ => return,
                    };

                    if !engine.mouse_captured && state == winit::event::ElementState::Pressed {
                        let _ = window
                            .set_cursor_grab(CursorGrabMode::Confined)
                            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
                        window.set_cursor_visible(false);
                        engine.mouse_captured = true;
                    }
                    engine.input.handle_mouse_button(idx, state);
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(pos) => {
                            (pos.x as f32 / 40.0, pos.y as f32 / 40.0)
                        }
                    };
                    engine.input.handle_scroll(dx, dy);
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let x = position.x as f32 - engine.window_width as f32 / 2.0;
                    let y = -(position.y as f32 - engine.window_height as f32 / 2.0);
                    engine.input.handle_cursor_moved(x, y);
                }

                WindowEvent::RedrawRequested => {
                    engine.time.tick();
                    engine.reload_assets_if_changed();
                    engine.audio.update(engine.time.dt());

                    while engine.time.consume_fixed_step() {
                        game.fixed_update(&engine);
                    }
                    let mut frame = Frame3D::new(engine.window_size());
                    game.update(&engine, &mut frame);

                    if game.should_exit() {
                        target.exit();
                        return;
                    }

                    game.render(&engine, &mut frame);

                    if show_fps {
                        let screen_size = engine.window_size();
                        let mut fps_canvas = canvas::Canvas::new(screen_size);
                        canvas::draw_fps(
                            &mut fps_canvas,
                            engine.time.fps(),
                            &engine.renderer.fonts[0],
                        );
                        frame.canvases.push(fps_canvas);
                    }
                    engine.renderer.render_frame(&mut frame);

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

pub fn run3d_with_scenes<F>(config: EngineConfig, init: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut Engine3D, &mut Globals) -> Box<dyn Scene3D>,
{
    env_logger::init();

    let headless = config.headless;
    let show_fps = config.show_fps;
    let fixed_dt = config.fixed_dt;
    assert!(
        config.render_width.is_some() == config.render_height.is_some(),
        "render_width and render_height must both be set or both be None"
    );
    let render_res = config
        .render_width
        .and_then(|w| config.render_height.map(|h| (w, h)));
    if let Some((rw, rh)) = render_res {
        assert!(
            rw >= 1 && rh >= 1,
            "render_width and render_height must both be >= 1"
        );
    }
    let scale_mode = config.scale_mode;

    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_visible(!headless)
            .build(&event_loop)?,
    );

    let present_mode = if config.vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    };
    let renderer = pollster::block_on(Renderer3D::new(window.clone(), present_mode));

    let mut engine = Engine3D {
        renderer,
        assets: AssetPipeline::default(),
        audio: AudioSystem::new(config.headless),
        input: InputState::new(),
        time: TimeState::new(),
        window_width: config.width,
        window_height: config.height,
        render_resolution: render_res,
        mouse_captured: false,
        hot_reload_enabled: config.hot_reload,
        actions: ActionMap::new(),
        no_gamepad: crate::input::GamepadState::new(),
        rng: RefCell::new(Rng::from_time()),
    };
    engine.time.set_fixed_dt(fixed_dt);
    if let Some((rw, rh)) = render_res {
        engine.renderer.init_offscreen(rw, rh, scale_mode);
    }

    let mut globals = Globals::new();
    let mut stack: Vec<Box<dyn Scene3D>> = Vec::new();

    let mut initial = init(&mut engine, &mut globals);
    initial.on_enter(&mut engine, &mut globals);
    stack.push(initial);

    if headless {
        loop {
            engine.time.tick();
            engine.reload_assets_if_changed();
            engine.audio.update(engine.time.dt());

            while engine.time.consume_fixed_step() {
                if let Some(scene) = stack.last_mut() {
                    scene.fixed_update(&engine, &mut globals);
                }
            }

            let mut headless_frame = Frame3D::new(engine.window_size());
            let op = if let Some(scene) = stack.last_mut() {
                scene.update(&engine, &mut globals, &mut headless_frame)
            } else {
                return Ok(());
            };

            apply_scene_op_3d(&mut stack, op, &mut engine, &mut globals);

            if stack.is_empty() {
                return Ok(());
            }

            engine.input.end_frame();
        }
    }

    let _ = window
        .set_cursor_grab(CursorGrabMode::Confined)
        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
    window.set_cursor_visible(false);
    engine.mouse_captured = true;

    event_loop.run(move |event, target| {
        target.set_control_flow(winit::event_loop::ControlFlow::Poll);
        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                ..
            } => {
                if engine.mouse_captured {
                    engine.input.handle_mouse_motion(dx, dy);
                }
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => target.exit(),

                WindowEvent::Focused(focused) => {
                    if focused {
                        let _ = window
                            .set_cursor_grab(CursorGrabMode::Confined)
                            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
                        window.set_cursor_visible(false);
                        engine.mouse_captured = true;
                    } else {
                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                        window.set_cursor_visible(true);
                        engine.mouse_captured = false;
                    }
                }

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
                    if key == winit::keyboard::KeyCode::Escape
                        && state == winit::event::ElementState::Pressed
                    {
                        if engine.mouse_captured {
                            let _ = window.set_cursor_grab(CursorGrabMode::None);
                            window.set_cursor_visible(true);
                            engine.mouse_captured = false;
                        } else {
                            target.exit();
                        }
                    }
                    engine.input.handle_key_event(key, state);
                }

                WindowEvent::MouseInput { button, state, .. } => {
                    let idx = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        _ => return,
                    };

                    if !engine.mouse_captured && state == winit::event::ElementState::Pressed {
                        let _ = window
                            .set_cursor_grab(CursorGrabMode::Confined)
                            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
                        window.set_cursor_visible(false);
                        engine.mouse_captured = true;
                    }
                    engine.input.handle_mouse_button(idx, state);
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(pos) => {
                            (pos.x as f32 / 40.0, pos.y as f32 / 40.0)
                        }
                    };
                    engine.input.handle_scroll(dx, dy);
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let x = position.x as f32 - engine.window_width as f32 / 2.0;
                    let y = -(position.y as f32 - engine.window_height as f32 / 2.0);
                    engine.input.handle_cursor_moved(x, y);
                }

                WindowEvent::RedrawRequested => {
                    engine.time.tick();
                    engine.reload_assets_if_changed();
                    engine.audio.update(engine.time.dt());

                    while engine.time.consume_fixed_step() {
                        if let Some(scene) = stack.last_mut() {
                            scene.fixed_update(&engine, &mut globals);
                        }
                    }

                    let mut frame = Frame3D::new(engine.window_size());

                    let op = if let Some(scene) = stack.last_mut() {
                        scene.update(&engine, &mut globals, &mut frame)
                    } else {
                        target.exit();
                        return;
                    };

                    apply_scene_op_3d(&mut stack, op, &mut engine, &mut globals);

                    if stack.is_empty() {
                        target.exit();
                        return;
                    }

                    for scene in stack.iter() {
                        scene.render(&engine, &globals, &mut frame);
                    }

                    if show_fps {
                        let screen_size = engine.window_size();
                        let mut fps_canvas = canvas::Canvas::new(screen_size);
                        canvas::draw_fps(
                            &mut fps_canvas,
                            engine.time.fps(),
                            &engine.renderer.fonts[0],
                        );
                        frame.canvases.push(fps_canvas);
                    }
                    engine.renderer.render_frame(&mut frame);

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

fn apply_scene_op_3d(
    stack: &mut Vec<Box<dyn Scene3D>>,
    op: SceneOp3D,
    engine: &mut Engine3D,
    globals: &mut Globals,
) {
    match op {
        SceneOp3D::Continue => {}
        SceneOp3D::Quit => {
            while let Some(mut scene) = stack.pop() {
                scene.on_exit(engine, globals);
            }
        }
        SceneOp3D::Push(mut new_scene) => {
            if let Some(current) = stack.last_mut() {
                current.on_pause(engine, globals);
            }
            new_scene.on_enter(engine, globals);
            stack.push(new_scene);
        }
        SceneOp3D::Pop => {
            if let Some(mut old) = stack.pop() {
                old.on_exit(engine, globals);
            }
            if let Some(current) = stack.last_mut() {
                current.on_resume(engine, globals);
            }
        }
        SceneOp3D::Switch(mut new_scene) => {
            if let Some(mut old) = stack.pop() {
                old.on_exit(engine, globals);
            }
            new_scene.on_enter(engine, globals);
            stack.push(new_scene);
        }
    }
}
