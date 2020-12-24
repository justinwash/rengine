use crate::input::input::Input;
use glfw::{Window,  Key};
use glfw::Action::*;

#[derive(Copy, Clone, Debug)]
pub struct KeyboardInput {
    key: &'static Key,
}

impl KeyboardInput {
    pub fn new(key: &'static glfw::Key) -> Self {
        Self {
            key: &key
        }
    }
}

impl Input for KeyboardInput {
    fn is_just_pressed(self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {
            Press => true,
            _ => false
        }
    }

    fn is_held(self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {            Repeat => true,
            _ => false
        }
    }

    fn is_just_released(self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {            Release => true,
            _ => false
        }
    }
}