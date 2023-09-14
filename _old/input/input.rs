use glfw::Window;

pub trait Input {
    fn is_just_pressed(&mut self, window: &Window) -> bool;
    fn is_held(&mut self, window: &Window) -> bool;
    fn is_just_released(&mut self, window: &Window) -> bool;
}
