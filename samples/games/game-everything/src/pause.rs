use crate::state::*;
use rengine::*;

pub struct PauseOverlay {
    pub demo_frames: u32,
    pub ui: Ui,
}

impl PauseOverlay {
    fn build_pause_ui(ui: &mut Ui) {
        ui.label_centered("PAUSED", 40.0, Color::WHITE);
        ui.separator(12.0);
        ui.button(0, "Resume");
        ui.button(1, "Quit");
    }
}

impl Scene for PauseOverlay {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals) {
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
        println!("[PauseOverlay] on_enter");
        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            demo.log_feature("Scene::on_enter");
            demo.log_feature("Ui (widget system)");
        }
        self.ui.begin(engine, -100.0, 40.0, 200.0);
        Self::build_pause_ui(&mut self.ui);
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.ui.begin(engine, -100.0, 40.0, 200.0);
        Self::build_pause_ui(&mut self.ui);

        let is_demo = globals.get::<DemoConfig>().map_or(false, |d| d.enabled);

        if is_demo {
            self.demo_frames += 1;
            if self.demo_frames >= 10 {
                println!("[PauseOverlay] demo: auto-popping after 10 frames");
                if let Some(demo) = globals.get_mut::<DemoConfig>() {
                    demo.log_feature("SceneOp::Pop (Unpause)");
                }
                return SceneOp::Pop;
            }
            return SceneOp::Continue;
        }

        let resp = self.ui.update(engine);

        if let Some(id) = resp.activated {
            match id {
                0 => return SceneOp::Pop,
                1 => return SceneOp::Quit,
                _ => {}
            }
        }

        if engine.action_pressed("pause") {
            return SceneOp::Pop;
        }
        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        let (hw, hh) = engine.half_size();
        let atlas = engine.font_atlas();
        let overlay = frame.canvas(1);

        overlay.rect(
            -hw,
            -hh,
            hw * 2.0,
            hh * 2.0,
            Color::new(0.0, 0.0, 0.0, 0.65),
        );

        self.ui.render(overlay, engine);

        if let Some(stats) = globals.get::<PlayerStats>() {
            overlay.text(
                -100.0,
                -60.0,
                &format!(
                    "Coins: {} | Best Height: {:.0}",
                    stats.coins, stats.best_height
                ),
                14.0,
                Color::YELLOW,
                atlas,
            );
        }
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[PauseOverlay] on_exit");
    }
}
