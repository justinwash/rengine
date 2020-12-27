use glfw::{Window};

pub trait Input {
    fn is_just_pressed(self, window: &Window) -> bool;
    fn is_held(self, window: &Window) -> bool;
    fn is_just_released(self, window: &Window) -> bool;
}