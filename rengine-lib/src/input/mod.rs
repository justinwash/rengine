use std::collections::HashMap;
use std::collections::HashSet;
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct InputConfig {
    pub key_handlers: HashMap<KeyCode, Box<dyn FnMut() + Send + 'static>>,
}

impl InputConfig {
    pub fn new() -> Self {
        Self {
            key_handlers: HashMap::new(),
        }
    }
    pub fn on_key<F: FnMut() + Send + 'static>(mut self, key: KeyCode, handler: F) -> Self {
        self.key_handlers.insert(key, Box::new(handler));
        self
    }
}

pub struct InputState {
    pub config: InputConfig,
    held_keys: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,
    prev_keys: HashSet<KeyCode>,
}

impl InputState {
    pub fn new(config: InputConfig) -> Self {
        Self {
            config,
            held_keys: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
            prev_keys: HashSet::new(),
        }
    }
    pub fn handle_event(&mut self, event: &winit::event::Event<()>) {
        if let winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::KeyboardInput { event, .. },
            ..
        } = event
        {
            use winit::event::ElementState;
            match event.physical_key {
                PhysicalKey::Code(keycode) => {
                    match event.state {
                        ElementState::Pressed => {
                            if self.held_keys.insert(keycode) {
                                self.just_pressed.insert(keycode);
                            }
                        }
                        ElementState::Released => {
                            self.held_keys.remove(&keycode);
                            self.just_released.insert(keycode);
                        }
                    }
                    if let Some(handler) = self.config.key_handlers.get_mut(&keycode) {
                        handler();
                    }
                }
                _ => {}
            }
        }
    }
    pub fn begin_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
    pub fn end_frame(&mut self) {
        self.prev_keys = self.held_keys.clone();
    }
    pub fn is_held(&self, key: KeyCode) -> bool {
        self.held_keys.contains(&key)
    }
    pub fn is_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }
    pub fn was_just_released(&self, key: KeyCode) -> bool {
        self.just_released.contains(&key)
    }
    pub fn clear_all(&mut self) {
        self.held_keys.clear();
        self.just_pressed.clear();
        self.just_released.clear();
        self.prev_keys.clear();
    }
}
