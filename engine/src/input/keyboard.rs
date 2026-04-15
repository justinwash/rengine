use std::collections::HashSet;
use winit::event::ElementState;
use winit::keyboard::KeyCode;

pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    mouse_delta: (f64, f64),
    mouse_position: (f32, f32),
    mouse_buttons: [bool; 3],
    mouse_buttons_pressed: [bool; 3],
    mouse_buttons_released: [bool; 3],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_delta: (0.0, 0.0),
            mouse_position: (0.0, 0.0),
            mouse_buttons: [false; 3],
            mouse_buttons_pressed: [false; 3],
            mouse_buttons_released: [false; 3],
        }
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    pub fn mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn is_mouse_down(&self, button: usize) -> bool {
        self.mouse_buttons.get(button).copied().unwrap_or(false)
    }

    pub fn is_mouse_pressed(&self, button: usize) -> bool {
        self.mouse_buttons_pressed
            .get(button)
            .copied()
            .unwrap_or(false)
    }

    pub fn is_mouse_released(&self, button: usize) -> bool {
        self.mouse_buttons_released
            .get(button)
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn handle_key_event(&mut self, key: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.keys_down.insert(key) {
                    self.keys_pressed.insert(key);
                }
            }
            ElementState::Released => {
                self.keys_down.remove(&key);
                self.keys_released.insert(key);
            }
        }
    }

    pub(crate) fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        self.mouse_delta.0 += dx;
        self.mouse_delta.1 += dy;
    }

    pub(crate) fn handle_cursor_moved(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    pub(crate) fn handle_mouse_button(&mut self, button: usize, state: ElementState) {
        if button < 3 {
            match state {
                ElementState::Pressed => {
                    if !self.mouse_buttons[button] {
                        self.mouse_buttons_pressed[button] = true;
                    }
                    self.mouse_buttons[button] = true;
                }
                ElementState::Released => {
                    self.mouse_buttons[button] = false;
                    self.mouse_buttons_released[button] = true;
                }
            }
        }
    }

    pub(crate) fn end_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.mouse_buttons_pressed = [false; 3];
        self.mouse_buttons_released = [false; 3];
    }
}
