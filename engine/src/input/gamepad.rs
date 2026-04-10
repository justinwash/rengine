use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use std::collections::HashMap;


pub const MAX_PLAYERS: usize = 4;


#[derive(Debug, Clone)]
pub struct GamepadState {

    pub(crate) id: Option<GamepadId>,

    buttons_down: Vec<Button>,

    buttons_pressed: Vec<Button>,

    buttons_released: Vec<Button>,

    pub left_stick_x: f32,

    pub left_stick_y: f32,
}

impl GamepadState {
    pub fn new() -> Self {
        Self {
            id: None,
            buttons_down: Vec::new(),
            buttons_pressed: Vec::new(),
            buttons_released: Vec::new(),
            left_stick_x: 0.0,
            left_stick_y: 0.0,
        }
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
}

impl GamepadSystem {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().expect("Failed to initialise gilrs");
        let mut sys = Self {
            gilrs,
            slots: (0..MAX_PLAYERS).map(|_| GamepadState::new()).collect(),
            id_to_slot: HashMap::new(),
        };

        let connected: Vec<GamepadId> = sys.gilrs
            .gamepads()
            .filter(|(_, gp)| gp.is_connected())
            .map(|(id, _)| id)
            .collect();
        for id in connected {
            sys.assign_slot(id);
        }
        sys
    }


    pub fn player(&self, index: usize) -> &GamepadState {
        &self.slots[index]
    }

    pub fn player_or_default(&self, index: usize) -> &GamepadState {
        static DEFAULT: GamepadState = GamepadState {
            id: None,
            buttons_down: Vec::new(),
            buttons_pressed: Vec::new(),
            buttons_released: Vec::new(),
            left_stick_x: 0.0,
            left_stick_y: 0.0,
        };
        self.slots.get(index).unwrap_or(&DEFAULT)
    }


    pub fn connected_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_connected()).count()
    }


    pub(crate) fn update(&mut self) {

        for slot in &mut self.slots {
            slot.buttons_pressed.clear();
            slot.buttons_released.clear();
        }


        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    self.assign_slot(event.id);
                }
                EventType::Disconnected => {
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        self.slots[slot_idx].id = None;
                        self.id_to_slot.remove(&event.id);
                    }
                }
                EventType::ButtonPressed(button, _) => {
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


        for slot in &mut self.slots {
            if let Some(id) = slot.id {
                if let Some(gp) = self.gilrs.connected_gamepad(id) {
                    slot.left_stick_x = gp.value(Axis::LeftStickX);
                    slot.left_stick_y = gp.value(Axis::LeftStickY);


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
                }
            }
        }
    }

    fn assign_slot(&mut self, id: GamepadId) {
        if self.id_to_slot.contains_key(&id) {
            return;
        }

        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.id.is_none() {
                slot.id = Some(id);
                self.id_to_slot.insert(id, idx);
                log::info!("Gamepad {:?} assigned to player slot {}", id, idx + 1);
                return;
            }
        }
        log::warn!("No free player slot for gamepad {:?}", id);
    }
}
