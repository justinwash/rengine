use crate::input::input::Input;
use glfw::Action::*;
use glfw::{Key, Window};

#[derive(Debug)]
pub struct KeyboardInput {
    key: &'static Key,
    state: String,
}

impl KeyboardInput {
    pub fn new(key: &'static glfw::Key) -> Self {
        Self {
            key: &key,
            state: "released".to_string(),
        }
    }
}

impl Input for KeyboardInput {
    fn is_just_pressed(&mut self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {
            Press => {
                println!("Window reports: {:?}, self state is {}", Press, self.state);
                true
            }
            _ => false,
        }
    }

    fn is_held(self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {
            Repeat => true,
            _ => false,
        }
    }

    fn is_just_released(self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {
            Release => true,
            _ => false,
        }
    }
}
