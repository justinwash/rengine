use std::collections::HashSet;
use winit::event::{ElementState, Ime};
use winit::keyboard::KeyCode;

pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    /// How many times each key went down this frame, not merely *whether* it
    /// did. A `HashSet` cannot count, and at speed a key really can complete
    /// two full press-release cycles inside one frame — at 60fps a fast double
    /// tap is ~80ms against a 16ms budget, so the two land together often
    /// enough for a player to notice a menu cursor moving one step when they
    /// pressed twice.
    press_counts: Vec<(KeyCode, u32)>,
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
            press_counts: Vec::new(),
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

    /// How many times `key` went down this frame. `is_key_pressed` is this
    /// `> 0`, and is still the right call for anything that acts once per
    /// frame; use this where every tap has to land — a menu cursor, a counter,
    /// a step control.
    pub fn key_press_count(&self, key: KeyCode) -> u32 {
        self.press_counts
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(0, |(_, n)| *n)
    }

    /// Whether *any* key went down this frame — the cheap test the input
    /// device-vote uses to tell "the keyboard is being used" from a pad.
    pub fn any_key_pressed(&self) -> bool {
        !self.keys_pressed.is_empty()
    }

    /// Whether any mouse button was pressed this frame.
    pub fn any_mouse_pressed(&self) -> bool {
        self.mouse_buttons_pressed.iter().any(|&b| b)
    }

    /// Linear scan over a `Vec`: at most a handful of distinct keys go down in
    /// any one frame, so a map would cost more to allocate than it saves.
    fn bump_press(&mut self, key: KeyCode) {
        match self.press_counts.iter_mut().find(|(k, _)| *k == key) {
            Some((_, n)) => *n += 1,
            None => self.press_counts.push((key, 1)),
        }
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

    /// Inject a synthetic key press for this frame. Used by headless playtest
    /// drivers to script input without a window.
    /// ponytail: press-only; add a release variant when a driver needs held keys.
    pub fn inject_key_press(&mut self, key: KeyCode) {
        self.keys_down.insert(key);
        self.keys_pressed.insert(key);
        self.bump_press(key);
    }

    /// Inject committed text for this frame, as a real keyboard would deliver it
    /// alongside the key event. Headless drivers need this to drive text fields.
    pub fn inject_text(&mut self, text: &str) {
        self.handle_committed_text(text);
    }

    /// Move the synthetic cursor for this frame. Used by headless playtest
    /// drivers to position the pointer before a click, in the same engine
    /// coordinate space as [`InputState::mouse_position`].
    pub fn inject_mouse_move(&mut self, x: f32, y: f32) {
        self.handle_cursor_moved(x, y);
    }

    /// Inject a synthetic press-and-release of a mouse button for this frame.
    /// Immediate-mode UI (`Ui::sync_at_with`) reads press/release within the
    /// same frame, so headless drivers don't need to hold the button across
    /// two frames the way a real click-and-release would.
    /// ponytail: press-then-release only; add a hold variant if a driver
    /// needs drag gestures.
    pub fn inject_mouse_click(&mut self, button: usize) {
        self.handle_mouse_button(button, ElementState::Pressed);
        self.handle_mouse_button(button, ElementState::Released);
    }

    pub(crate) fn handle_key_event(&mut self, key: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => {
                // `keys_down.insert` is false while the key is held, which is
                // what stops OS key-repeat from firing a press every frame.
                // It also swallowed the *second* tap of a genuine double tap:
                // press-release-press inside one frame left `keys_pressed`
                // already holding the key, so the set had nothing new to
                // record and the input was gone. Counted separately now — the
                // repeat guard stays, the count is what the drop cost.
                if self.keys_down.insert(key) {
                    self.keys_pressed.insert(key);
                    self.bump_press(key);
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
        self.press_counts.clear();
        self.keys_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.mouse_buttons_pressed = [false; 3];
        self.mouse_buttons_released = [false; 3];
        self.scroll_delta = (0.0, 0.0);
        self.committed_text.clear();
    }
}

#[cfg(test)]
mod double_tap_tests {
    use super::*;

    /// The 2026-08-08 playtest's #44: "pressed Down twice, the cursor moved
    /// once". Filed as possibly a harness artifact; it is not.
    #[test]
    fn a_double_tap_inside_one_frame_counts_twice() {
        let mut input = InputState::new();
        for _ in 0..2 {
            input.handle_key_event(KeyCode::ArrowDown, ElementState::Pressed);
            input.handle_key_event(KeyCode::ArrowDown, ElementState::Released);
        }
        assert_eq!(
            input.key_press_count(KeyCode::ArrowDown),
            2,
            "the second tap was dropped"
        );
        assert!(input.is_key_pressed(KeyCode::ArrowDown));
    }

    /// The guard the count must not break: a *held* key is one press, however
    /// many repeat events the OS sends.
    #[test]
    fn holding_a_key_is_still_one_press() {
        let mut input = InputState::new();
        for _ in 0..5 {
            input.handle_key_event(KeyCode::ArrowDown, ElementState::Pressed);
        }
        assert_eq!(input.key_press_count(KeyCode::ArrowDown), 1);
    }

    #[test]
    fn end_frame_forgets_the_count() {
        let mut input = InputState::new();
        input.inject_key_press(KeyCode::Enter);
        input.end_frame();
        assert_eq!(input.key_press_count(KeyCode::Enter), 0);
        assert!(!input.is_key_pressed(KeyCode::Enter));
    }
}
