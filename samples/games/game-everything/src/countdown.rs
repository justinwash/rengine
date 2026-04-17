use crate::game::GameScene;
use rengine::*;

pub struct CountdownScene {
    timer: f32,
}

impl CountdownScene {
    pub fn new() -> Self {
        Self { timer: 3.5 }
    }
}

impl Scene for CountdownScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {
        println!("[CountdownScene] on_enter — 3 second countdown");
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, frame: &mut Frame) -> SceneOp {
        self.timer -= engine.dt();
        if self.timer <= 0.0 {
            return SceneOp::Switch(Box::new(GameScene::default()));
        }

        frame.clear_color = Color::new(0.05, 0.05, 0.15, 1.0);

        let canvas = frame.canvas(0);

        let secs = self.timer.ceil() as i32;
        let label = if secs <= 0 {
            "GO!".to_string()
        } else {
            format!("{secs}")
        };

        canvas.text(-40.0, 50.0, &label, 80.0, Color::WHITE);

        canvas.text(
            -140.0,
            -50.0,
            "Demo starting... start recording!",
            16.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
        );

        SceneOp::Continue
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[CountdownScene] on_exit — starting demo");
    }
}
