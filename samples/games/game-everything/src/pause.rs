use crate::state::*;
use rengine::*;

pub struct PauseOverlay {
    pub demo_frames: u32,
    pub focus: usize,
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
    fn on_enter(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
        println!("[PauseOverlay] on_enter");
        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            demo.log_feature("Scene::on_enter");
            demo.log_feature("Ui (widget system)");
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
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

        let (_sw, sh) = engine.window_size();
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();

        self.ui.begin(-100.0, hh - 40.0, 200.0);
        Self::build_pause_ui(&mut self.ui);
        let resp = self.ui.update(engine.input(), atlas);
        self.focus = resp.focused.unwrap_or(self.focus);

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
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        let overlay = frame.canvas(1);

        overlay.rect(
            -hw,
            -hh,
            sw as f32,
            sh as f32,
            Color::new(0.0, 0.0, 0.0, 0.65),
        );

        self.ui.render(overlay, atlas);

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
