use crate::input::input::Input;
use glfw::Action::*;
use glfw::{Key, Window};

#[derive(PartialEq, Debug)]
pub enum KeyState {
    Pressed,
    JustPressed,
    Released,
    JustReleased,
}

use KeyState::*;

#[derive(Debug)]
pub struct KeyboardInput {
    key: &'static Key,
    state: KeyState,
}

impl KeyboardInput {
    pub fn new(key: &'static glfw::Key) -> Self {
        Self {
            key: &key,
            state: Released,
        }
    }
}

impl Input for KeyboardInput {
    fn is_just_pressed(&mut self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {
            Press => {
                if self.state != JustPressed && self.state != Pressed {
                    self.state = JustPressed;
                } else {
                    self.state = Pressed;
                }
                self.state == JustPressed
            }
            Release => {
                self.state = Released;
                false
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
