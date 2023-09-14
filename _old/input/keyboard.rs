use crate::input::input::Input;
use glfw::Action::*;
use glfw::{Key, Window};

#[derive(PartialEq, Debug)]
pub enum KeyState {
    Pressed,
    JustPressed,
    Released,
    JustReleased,
    Unknown,
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
            _ => false,
        }
    }

    fn is_held(&mut self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {
            Repeat => true,
            Press => true,
            _ => false,
        }
    }

    fn is_just_released(&mut self, window: &Window) -> bool {
        match Window::get_key(window, *self.key) {
            Release => {
                if self.state != JustReleased && self.state != Released {
                    self.state = JustReleased;
                } else {
                    self.state = Released;
                }
                self.state == JustReleased
            }
            _ => false,
        }
    }
}
