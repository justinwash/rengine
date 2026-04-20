use crate::state::*;
use rengine::*;

pub struct PauseOverlay {
    pub demo_frames: u32,
    pub ui: Ui,
}

impl PauseOverlay {
    fn build_pause_ui(ui: &mut Ui) {
        let button_animation = UiAnimationOptions::new()
            .with_appear(
                UiAnimation::new(0.22)
                    .with_easing(Easing::OutQuad)
                    .with_offset(Vec2::new(0.0, -8.0))
                    .with_alpha(0.0),
            )
            .with_focus(
                UiAnimation::new(0.12)
                    .with_easing(Easing::OutQuad)
                    .with_offset(Vec2::new(0.0, 3.0))
                    .with_scale(1.03),
            )
            .with_press(
                UiAnimation::new(0.1)
                    .with_easing(Easing::OutQuad)
                    .with_scale(0.97)
                    .with_alpha(0.92),
            );

        ui.label_centered("PAUSED", 40.0, Color::WHITE);
        ui.separator(12.0);
        ui.button(0, "Resume");
        ui.style_with(
            UiWidgetStyle::new()
                .with_button_colors(
                    Color::from_rgba8(32, 112, 84, 235),
                    Color::from_rgba8(48, 148, 108, 255),
                    Color::from_rgba8(28, 88, 68, 255),
                )
                .with_button_text_colors(Color::from_rgba8(235, 255, 244, 255), Color::WHITE),
        );
        ui.animate_with(button_animation);
        ui.tooltip("Return to the current run without losing any state.");
        ui.button(1, "Quit");
        ui.style_with(
            UiWidgetStyle::new()
                .with_button_colors(
                    Color::from_rgba8(136, 42, 50, 235),
                    Color::from_rgba8(180, 56, 66, 255),
                    Color::from_rgba8(96, 24, 32, 255),
                )
                .with_button_text_colors(Color::from_rgba8(255, 236, 236, 255), Color::WHITE),
        );
        ui.animate_with(button_animation);
        ui.tooltip("Exit the kitchen-sink demo immediately.");
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
            demo.log_feature("Ui::run");
            demo.log_feature("Ui::tooltip");
            demo.log_feature("Ui::animate_with");
            demo.log_feature("Ui::style_with");
        }
        self.ui.begin(engine, -100.0, 40.0, 200.0);
        Self::build_pause_ui(&mut self.ui);
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let resp = self
            .ui
            .run(engine, -100.0, 40.0, 200.0, |ui| Self::build_pause_ui(ui));

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
            );
        }
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[PauseOverlay] on_exit");
    }
}
