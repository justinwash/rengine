use std::collections::HashSet;
use winit::event::{ElementState, Ime};
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
    scroll_delta: (f32, f32),
    committed_text: String,
    ime_preedit: Option<(String, Option<(usize, usize)>)>,
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
            scroll_delta: (0.0, 0.0),
            committed_text: String::new(),
            ime_preedit: None,
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

    /// Cursor position in the engine's 2D coordinate space: origin at the window
    /// centre, `+x` right and `+y` **up** — the same space as the `Canvas` and
    /// `SceneWorld2D` node positions. A point is over a rect when
    /// `Rect::from_pos_size(pos, size).contains_point(Vec2::new(x, y))`, and it
    /// can be passed straight to `SceneWorld2D::hit_test` with no conversion.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Whether a mouse button is held. Button indices: `0` = left, `1` = right,
    /// `2` = middle.
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

    pub fn scroll_delta(&self) -> (f32, f32) {
        self.scroll_delta
    }

    pub fn committed_text(&self) -> &str {
        &self.committed_text
    }

    pub fn ime_preedit(&self) -> Option<(&str, Option<(usize, usize)>)> {
        self.ime_preedit
            .as_ref()
            .map(|(text, cursor)| (text.as_str(), *cursor))
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

    pub(crate) fn handle_scroll(&mut self, dx: f32, dy: f32) {
        self.scroll_delta.0 += dx;
        self.scroll_delta.1 += dy;
    }

    pub(crate) fn handle_committed_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\u{7f}' || (ch.is_control() && ch != '\n' && ch != '\t') {
                continue;
            }
            if ch == '\r' || ch == '\n' || ch == '\t' {
                continue;
            }
            self.committed_text.push(ch);
        }
    }

    pub(crate) fn handle_ime_event(&mut self, ime: Ime) {
        match ime {
            Ime::Enabled => {}
            Ime::Preedit(text, cursor) => {
                if text.is_empty() {
                    self.ime_preedit = None;
                } else {
                    self.ime_preedit = Some((text, cursor));
                }
            }
            Ime::Commit(text) => {
                self.ime_preedit = None;
                self.handle_committed_text(&text);
            }
            Ime::Disabled => {
                self.ime_preedit = None;
            }
        }
    }

    pub(crate) fn end_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.mouse_buttons_pressed = [false; 3];
        self.mouse_buttons_released = [false; 3];
        self.scroll_delta = (0.0, 0.0);
        self.committed_text.clear();
    }
}
