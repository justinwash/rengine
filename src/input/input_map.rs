use std::collections::HashMap;
use crate::input::input::Input;
use crate::input::keyboard::*;
use glfw::Window;

#[derive(Debug)]
pub struct InputMap<S> {
    pub actions: HashMap<String, S>
}

impl InputMap<KeyboardInput> {
    pub fn new() -> Self { 
        InputMap { 
            actions: HashMap::new() 
        } 
    }

    pub fn add_action(mut self, name: &str, input: KeyboardInput) -> Self {
        self.actions.insert(String::from(name), input);
        self
    }

    pub fn is_action_just_pressed(&self, action_name: &str, window: &Window) -> bool {
        self.actions.get(&String::from(action_name)).unwrap().is_just_pressed(window)
    }

    pub fn is_action_held(&self, action_name: &str, window: &Window) -> bool {
        self.actions.get(&String::from(action_name)).unwrap().is_held(window)
     }

    pub fn is_action_just_released(&self, action_name: &str, window: &Window) -> bool {
        self.actions.get(&String::from(action_name)).unwrap().is_just_released(window)
     }
}