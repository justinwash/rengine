use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use std::collections::HashMap;
use winit::keyboard::KeyCode;

use super::keyboard::InputState;

pub const MAX_PLAYERS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadAssignMode {
    OnConnect,
    OnButtonPress,
}

impl Default for GamepadAssignMode {
    fn default() -> Self {
        Self::OnButtonPress
    }
}

/// A rengine-owned handle for \"this pad occupies this slot\", so connectedness
/// is testable without a real gilrs `GamepadId` (which only gilrs can build).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamepadToken(pub(crate) u32);

#[derive(Debug, Clone)]
pub struct GamepadState {
    pub(crate) id: Option<GamepadToken>,

    buttons_down: Vec<Button>,

    buttons_pressed: Vec<Button>,

    buttons_released: Vec<Button>,

    pub left_stick_x: f32,

    pub left_stick_y: f32,

    pub right_stick_x: f32,

    pub right_stick_y: f32,
}

impl GamepadState {
    pub const DEFAULT: Self = Self {
        id: None,
        buttons_down: Vec::new(),
        buttons_pressed: Vec::new(),
        buttons_released: Vec::new(),
        left_stick_x: 0.0,
        left_stick_y: 0.0,
        right_stick_x: 0.0,
        right_stick_y: 0.0,
    };

    pub fn new() -> Self {
        Self::DEFAULT
    }

    pub fn is_button_down(&self, button: Button) -> bool {
        self.buttons_down.contains(&button)
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.buttons_pressed.contains(&button)
    }

    pub fn is_button_released(&self, button: Button) -> bool {
        self.buttons_released.contains(&button)
    }

    pub fn is_connected(&self) -> bool {
        self.id.is_some()
    }
}

pub struct GamepadSystem {
    gilrs: Gilrs,

    pub(crate) slots: Vec<GamepadState>,

    id_to_slot: HashMap<GamepadId, usize>,

    unassigned: Vec<GamepadId>,

    assign_mode: GamepadAssignMode,
}

impl GamepadSystem {
    pub fn new(mode: GamepadAssignMode) -> Self {
        let gilrs = Gilrs::new().expect("Failed to initialise gilrs");
        let mut sys = Self {
            gilrs,
            slots: (0..MAX_PLAYERS).map(|_| GamepadState::new()).collect(),
            id_to_slot: HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: mode,
        };

        let connected: Vec<GamepadId> = sys
            .gilrs
            .gamepads()
            .filter(|(_, gp)| gp.is_connected())
            .map(|(id, _)| id)
            .collect();
        for id in connected {
            sys.track_gamepad(id);
        }
        sys
    }

    pub fn assign_mode(&self) -> GamepadAssignMode {
        self.assign_mode
    }

    pub fn set_assign_mode(&mut self, mode: GamepadAssignMode) {
        self.assign_mode = mode;
        if mode == GamepadAssignMode::OnConnect {
            let pending: Vec<GamepadId> = self.unassigned.drain(..).collect();
            for id in pending {
                self.assign_slot(id);
            }
        }
    }

    pub fn unassigned_count(&self) -> usize {
        self.unassigned.len()
    }

    pub fn player(&self, index: usize) -> &GamepadState {
        &self.slots[index]
    }

    pub fn player_or_default(&self, index: usize) -> &GamepadState {
        static DEFAULT: GamepadState = GamepadState::DEFAULT;
        self.slots.get(index).unwrap_or(&DEFAULT)
    }

    pub fn connected_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_connected()).count()
    }

    /// Merge a connected player-0 pad into the keyboard state the game reads.
    ///
    /// The face/menu buttons map to the universal keys the whole input scheme
    /// already keys off (A→Enter, B→Esc, Start→Space, View→Esc), so **every**
    /// existing `is_key_pressed(Enter)` / `Escape` / `Space` call site becomes
    /// controller-aware with no game-side edit — the same translation a
    /// platform layer (Steam Input, XInput) performs. Direction is *not*
    /// merged: the D-pad means "move the highlight" in menus but "walk the
    /// focus ring" in the race, so the game reads the pad directly where the
    /// two split.
    pub(crate) fn translate_to_keys(&self, input: &mut InputState) {
        let Some(gp) = self.slots.get(0) else {
            return;
        };
        if !gp.is_connected() {
            return;
        }
        if gp.is_button_pressed(Button::South) {
            input.inject_key_press(KeyCode::Enter);
        }
        if gp.is_button_pressed(Button::East) {
            input.inject_key_press(KeyCode::Escape);
        }
        if gp.is_button_pressed(Button::Start) {
            input.inject_key_press(KeyCode::Space);
        }
        // View is the scheme's "deeper out": on screens without a back target
        // it opens the pause menu, on screens with one it goes back — which is
        // exactly what Esc already resolves to per screen.
        if gp.is_button_pressed(Button::Select) {
            input.inject_key_press(KeyCode::Escape);
        }
    }

    pub(crate) fn update(&mut self) {
        for slot in &mut self.slots {
            slot.buttons_pressed.clear();
            slot.buttons_released.clear();
        }

        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    self.track_gamepad(event.id);
                }
                EventType::Disconnected => {
                    self.unassigned.retain(|&id| id != event.id);
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        self.slots[slot_idx].id = None;
                        self.id_to_slot.remove(&event.id);
                    }
                }
                EventType::ButtonPressed(button, _) => {
                    if self.unassigned.contains(&event.id) {
                        self.unassigned.retain(|&id| id != event.id);
                        self.assign_slot(event.id);
                    }
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        let slot = &mut self.slots[slot_idx];
                        if !slot.buttons_down.contains(&button) {
                            slot.buttons_down.push(button);
                            slot.buttons_pressed.push(button);
                        }
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        let slot = &mut self.slots[slot_idx];
                        slot.buttons_down.retain(|&b| b != button);
                        slot.buttons_released.push(button);
                    }
                }
                _ => {}
            }
        }

        // Stick state is refreshed from the live gilrs value for every
        // assigned slot. The `id_to_slot` map is the source of the gilrs id
        // (the slot's own token only says "occupied").
        let assigned: Vec<(usize, GamepadId)> = self
            .id_to_slot
            .iter()
            .map(|(&id, &slot_idx)| (slot_idx, id))
            .collect();
        for (slot_idx, gid) in assigned {
            let Some(gp) = self.gilrs.connected_gamepad(gid) else {
                continue;
            };
            let slot = &mut self.slots[slot_idx];
            slot.left_stick_x = gp.value(Axis::LeftStickX);
            slot.left_stick_y = gp.value(Axis::LeftStickY);
            slot.right_stick_x = gp.value(Axis::RightStickX);
            slot.right_stick_y = gp.value(Axis::RightStickY);

            if gp.is_pressed(Button::DPadLeft) {
                slot.left_stick_x = -1.0;
            } else if gp.is_pressed(Button::DPadRight) {
                slot.left_stick_x = 1.0;
            }
            if gp.is_pressed(Button::DPadUp) {
                slot.left_stick_y = 1.0;
            } else if gp.is_pressed(Button::DPadDown) {
                slot.left_stick_y = -1.0;
            }

            if slot.left_stick_x.abs() < 0.15 {
                slot.left_stick_x = 0.0;
            }
            if slot.left_stick_y.abs() < 0.15 {
                slot.left_stick_y = 0.0;
            }
            if slot.right_stick_x.abs() < 0.15 {
                slot.right_stick_x = 0.0;
            }
            if slot.right_stick_y.abs() < 0.15 {
                slot.right_stick_y = 0.0;
            }
        }
    }

    fn track_gamepad(&mut self, id: GamepadId) {
        if self.id_to_slot.contains_key(&id) || self.unassigned.contains(&id) {
            return;
        }
        match self.assign_mode {
            GamepadAssignMode::OnConnect => self.assign_slot(id),
            GamepadAssignMode::OnButtonPress => {
                log::info!(
                    "Gamepad {:?} detected, waiting for button press to assign",
                    id
                );
                self.unassigned.push(id);
            }
        }
    }

    fn assign_slot(&mut self, id: GamepadId) {
        if self.id_to_slot.contains_key(&id) {
            return;
        }

        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.id.is_none() {
                slot.id = Some(GamepadToken(idx as u32 + 1));
                self.id_to_slot.insert(id, idx);
                log::info!("Gamepad {:?} assigned to player slot {}", id, idx + 1);
                return;
            }
        }
        log::warn!("No free player slot for gamepad {:?}", id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    /// A with a connected pad pushes the universal keys the game keys off
    /// (Enter/Esc/Space), so every existing keyboard read works on the pad.
    #[test]
    fn a_pad_translates_face_buttons_to_universal_keys() {
        let mut slot = GamepadState::DEFAULT;
        // Connected pads carry a token; stub it without a real gilrs event.
        slot.id = Some(GamepadToken(1));
        slot.buttons_pressed.push(Button::South);
        slot.buttons_pressed.push(Button::Start);

        let system = GamepadSystem {
            gilrs: Gilrs::new().expect("gilrs init"),
            slots: vec![slot],
            id_to_slot: std::collections::HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
        };
        // Only the first slot matters; avoid poll (it needs real events).
        let mut input = InputState::new();
        system.translate_to_keys(&mut input);
        assert!(input.is_key_pressed(KeyCode::Enter), "A is confirm");
        assert!(input.is_key_pressed(KeyCode::Space), "Start is pause");
        assert!(!input.is_key_pressed(KeyCode::Escape), "nothing pressed Esc");

        // An idle connected pad injects nothing.
        let idle = GamepadSystem {
            gilrs: Gilrs::new().expect("gilrs init"),
            slots: vec![GamepadState::DEFAULT],
            id_to_slot: std::collections::HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
        };
        let mut input2 = InputState::new();
        idle.translate_to_keys(&mut input2);
        assert!(
            !input2.is_key_pressed(KeyCode::Enter) && !input2.is_key_pressed(KeyCode::Space),
            "a disconnected pad must not fire keys"
        );
    }
}
