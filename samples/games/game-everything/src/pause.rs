use crate::state::*;
use rengine::*;

struct PauseBadge(TextureId);

pub struct PauseOverlay {
    pub demo_frames: u32,
    pub ui: Ui,
    pub badge: Option<TextureId>,
}

impl PauseOverlay {
    fn build_pause_ui(ui: &mut Ui, badge: Option<TextureId>) {
        let badge_animation = UiAnimationOptions::new()
            .with_appear(
                UiAnimation::new(0.28)
                    .with_easing(Easing::OutBack)
                    .with_offset(Vec2::new(0.0, -12.0))
                    .with_alpha(0.0),
            )
            .with_hover(
                UiAnimation::new(0.12)
                    .with_easing(Easing::OutQuad)
                    .with_offset(Vec2::new(0.0, 4.0))
                    .with_scale(1.05),
            );
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
        if let Some(texture) = badge {
            ui.image(texture, Vec2::new(56.0, 56.0));
            ui.animate_with(badge_animation);
            ui.tooltip("Crew badge tooltip: images and other non-interactive widgets can now explain themselves on hover.");
            ui.separator(8.0);
        }
        ui.button(0, "Resume");
        ui.animate_with(button_animation);
        ui.tooltip("Return to the current run without losing any state.");
        ui.button(1, "Quit");
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
            demo.log_feature("Ui::image");
            demo.log_feature("Ui::tooltip");
            demo.log_feature("Ui::animate_with");
        }

        if !globals.contains::<PauseBadge>() {
            let mut icon = pixelart::PixelCanvas::new(24, 24);
            icon.fill(Color::new(0.08, 0.08, 0.12, 0.0));
            let shell = Color::from_rgba8(235, 70, 70, 255);
            let visor = Color::from_rgba8(160, 230, 255, 255);
            for y in 4..20 {
                for x in 4..20 {
                    let dx = x as f32 - 11.5;
                    let dy = y as f32 - 11.5;
                    if dx * dx + dy * dy <= 60.0 {
                        icon.set(x, y, shell);
                    }
                }
            }
            for y in 10..15 {
                for x in 9..19 {
                    icon.set(x, y, visor);
                }
            }
            for x in 5..19 {
                icon.set(x, 19, Color::from_rgba8(40, 40, 55, 255));
            }
            globals.set(PauseBadge(engine.create_texture(
                24,
                24,
                &icon.into_bytes(),
            )));
        }

        self.badge = globals.get::<PauseBadge>().map(|badge| badge.0);

        self.ui.begin(engine, -100.0, 40.0, 200.0);
        Self::build_pause_ui(&mut self.ui, self.badge);
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.ui.begin(engine, -100.0, 40.0, 200.0);
        Self::build_pause_ui(&mut self.ui, self.badge);

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
