use rengine::Engine;


pub fn movement_dir(engine: &Engine) -> (f32, f32) {
    let dc = engine.axis("move_col");
    let dr = engine.axis("move_row");

    if dc != 0.0 || dr != 0.0 {
        let len = (dc * dc + dr * dr).sqrt();
        (dc / len, dr / len)
    } else {
        (0.0, 0.0)
    }
}
