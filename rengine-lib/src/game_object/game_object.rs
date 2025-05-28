pub trait GameObject {
    fn position(&self) -> (f32, f32);
    fn set_position(&mut self, pos: (f32, f32));
}
