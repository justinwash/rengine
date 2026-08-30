use std::collections::HashMap;

use gilrs::Button;
use winit::keyboard::KeyCode;

use super::gamepad::GamepadState;
use super::keyboard::InputState;

#[derive(Clone, Debug)]
pub enum Binding {
    Key(KeyCode),
    MouseButton(usize),
    GamepadButton(Button),
}

#[derive(Clone, Copy, Debug)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
}

#[derive(Clone, Debug)]
pub struct AxisMapping {
    pub positive: Vec<Binding>,
    pub negative: Vec<Binding>,
    pub gamepad_axis: Option<GamepadAxis>,
}

pub struct ActionMap {
    actions: HashMap<String, Vec<Binding>>,
    axes: HashMap<String, AxisMapping>,
}

impl ActionMap {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            axes: HashMap::new(),
        }
    }

    pub fn bind(&mut self, action: &str, binding: Binding) {
        self.actions
            .entry(action.to_string())
            .or_default()
            .push(binding);
    }

    pub fn bind_axis(&mut self, name: &str, mapping: AxisMapping) {
        self.axes.insert(name.to_string(), mapping);
    }

    pub fn clear(&mut self) {
        self.actions.clear();
        self.axes.clear();
    }

    pub fn unbind(&mut self, action: &str) {
        self.actions.remove(action);
    }

    pub fn unbind_axis(&mut self, name: &str) {
        self.axes.remove(name);
    }

    pub fn is_down(&self, action: &str, input: &InputState, gamepad: &GamepadState) -> bool {
        let Some(bindings) = self.actions.get(action) else {
            return false;
        };
        bindings.iter().any(|b| binding_down(b, input, gamepad))
    }

    pub fn is_pressed(&self, action: &str, input: &InputState, gamepad: &GamepadState) -> bool {
        let Some(bindings) = self.actions.get(action) else {
            return false;
        };
        bindings.iter().any(|b| match b {
            Binding::Key(k) => input.is_key_pressed(*k),
            Binding::MouseButton(i) => input.is_mouse_pressed(*i),
            Binding::GamepadButton(btn) => gamepad.is_button_pressed(*btn),
        })
    }

    pub fn is_released(&self, action: &str, input: &InputState, gamepad: &GamepadState) -> bool {
        let Some(bindings) = self.actions.get(action) else {
            return false;
        };
        bindings.iter().any(|b| match b {
            Binding::Key(k) => input.is_key_released(*k),
            Binding::MouseButton(i) => input.is_mouse_released(*i),
            Binding::GamepadButton(btn) => gamepad.is_button_released(*btn),
        })
    }

    pub fn axis(&self, name: &str, input: &InputState, gamepad: &GamepadState) -> f32 {
        let Some(mapping) = self.axes.get(name) else {
            return 0.0;
        };

        let mut value = 0.0f32;

        if mapping
            .positive
            .iter()
            .any(|b| binding_down(b, input, gamepad))
        {
            value += 1.0;
        }

        if mapping
            .negative
            .iter()
            .any(|b| binding_down(b, input, gamepad))
        {
            value -= 1.0;
        }

        if let Some(axis) = mapping.gamepad_axis {
            let analog = match axis {
                GamepadAxis::LeftStickX => gamepad.left_stick_x,
                GamepadAxis::LeftStickY => gamepad.left_stick_y,
                GamepadAxis::RightStickX => gamepad.right_stick_x,
                GamepadAxis::RightStickY => gamepad.right_stick_y,
            };
            if analog.abs() > value.abs() {
                value = analog;
            }
        }

        value.clamp(-1.0, 1.0)
    }
}

fn binding_down(b: &Binding, input: &InputState, gamepad: &GamepadState) -> bool {
    match b {
        Binding::Key(k) => input.is_key_down(*k),
        Binding::MouseButton(i) => input.is_mouse_down(*i),
        Binding::GamepadButton(btn) => gamepad.is_button_down(*btn),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scheme drives camera pan/zoom and scroll from the right stick, so
    /// the axis resolver must read the right-stick analog values (not just the
    /// left stick the original engine shipped).
    #[test]
    fn right_stick_axes_resolve_analog() {
        let mut gp = GamepadState::DEFAULT;
        gp.right_stick_x = 0.7;
        gp.right_stick_y = -0.4;

        let map = {
            let mut m = ActionMap::new();
            m.bind_axis(
                "pan_x",
                AxisMapping {
                    positive: vec![],
                    negative: vec![],
                    gamepad_axis: Some(GamepadAxis::RightStickX),
                },
            );
            m.bind_axis(
                "pan_y",
                AxisMapping {
                    positive: vec![],
                    negative: vec![],
                    gamepad_axis: Some(GamepadAxis::RightStickY),
                },
            );
            m
        };

        let input = crate::input::keyboard::InputState::new();
        assert!((map.axis("pan_x", &input, &gp) - 0.7).abs() < 1e-5);
        assert!((map.axis("pan_y", &input, &gp) + 0.4).abs() < 1e-5);
    }
}
