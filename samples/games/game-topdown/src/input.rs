use rengine::{Engine, Vec2};

pub fn movement_dir(engine: &Engine) -> Vec2 {
    let x = engine.axis("move_x");
    let y = engine.axis("move_y");
    let dir = Vec2::new(x, y);

    if dir != Vec2::ZERO {
        dir.normalize()
    } else {
        dir
    }
}
