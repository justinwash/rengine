use rengine::*;

struct RenderTargetDemo {
    monitor: RenderTarget,
    quit: bool,
    phase: f32,
}

impl RenderTargetDemo {
    fn dashboard_color() -> Color {
        Color::from_rgba8(17, 22, 32, 255)
    }
}

impl Game for RenderTargetDemo {
    fn new(engine: &mut Engine) -> Self {
        Self {
            monitor: engine.create_render_target(256, 144),
            quit: false,
            phase: 0.0,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.phase += engine.dt();
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let white = engine.white_texture();
        let monitor_texture = self.monitor.texture_id();
        let target_frame = frame.render_target(&self.monitor);
        target_frame.clear_color = Color::new(0.0, 0.0, 0.0, 0.0);

        let road_y = -12.0;
        let car_x = (self.phase * 1.35).sin() * 78.0;
        target_frame.draw_colored(
            white,
            Vec2::new(-112.0, road_y - 4.0),
            Vec2::new(224.0, 8.0),
            Color::from_rgba8(82, 92, 112, 255),
        );
        target_frame.draw_colored(
            white,
            Vec2::new(-24.0, 30.0),
            Vec2::new(48.0, 72.0),
            Color::from_rgba8(40, 52, 66, 235),
        );
        target_frame.draw_colored(
            white,
            Vec2::new(car_x, road_y - 10.0),
            Vec2::new(30.0, 18.0),
            Color::from_rgba8(232, 96, 72, 255),
        );
        target_frame.draw_colored(
            white,
            Vec2::new(-14.0, 14.0),
            Vec2::new(20.0, 18.0),
            Color::from_rgba8(118, 236, 255, 255),
        );

        for index in 0..3 {
            let height = 14.0 + index as f32 * 12.0;
            let width = 20.0 + ((self.phase * (1.2 + index as f32 * 0.25)).sin() + 1.0) * 16.0;
            target_frame.draw_colored(
                white,
                Vec2::new(-12.0 + index as f32 * 16.0, -58.0),
                Vec2::new(width, height),
                Color::from_rgba8(80 + index as u8 * 30, 180, 120 + index as u8 * 28, 255),
            );
        }

        let canvas = target_frame.canvas(0);
        canvas.text(
            -118.0,
            60.0,
            "Telemetry Feed",
            16.0,
            Color::from_rgba8(230, 236, 244, 255),
        );
        canvas.text(
            -118.0,
            42.0,
            &format!("lap delta  {:+.2}s", (self.phase * 0.7).sin() * 0.42),
            12.0,
            Color::from_rgba8(120, 235, 160, 255),
        );
        canvas.text(
            -118.0,
            26.0,
            &format!("track temp  {:.0}C", 31.0 + (self.phase * 0.45).cos() * 4.0),
            12.0,
            Color::from_rgba8(245, 193, 92, 255),
        );

        frame.clear_color = Self::dashboard_color();
        frame.draw_colored(
            white,
            Vec2::new(-228.0, -116.0),
            Vec2::new(456.0, 250.0),
            Color::from_rgba8(28, 34, 44, 255),
        );
        frame.draw_colored(
            white,
            Vec2::new(-204.0, -92.0),
            Vec2::new(408.0, 202.0),
            Color::from_rgba8(10, 12, 18, 255),
        );
        frame.draw(
            monitor_texture,
            Vec2::new(-192.0, -84.0),
            Vec2::new(384.0, 216.0),
        );

        frame.draw_colored(
            white,
            Vec2::new(124.0, 46.0),
            Vec2::new(166.0, 108.0),
            Color::from_rgba8(24, 30, 38, 245),
        );
        frame.draw_colored(
            monitor_texture,
            Vec2::new(132.0, 54.0),
            Vec2::new(150.0, 84.0),
            Color::new(1.0, 1.0, 1.0, 0.88),
        );

        let canvas = frame.canvas(0);
        canvas.text_aligned(
            0.0,
            286.0,
            "Render Targets And Offscreen Textures",
            24.0,
            Color::from_rgba8(228, 234, 242, 255),
            TextAlign::Center,
        );
        canvas.text_block(
            0.0,
            262.0,
            "The telemetry scene renders into a target texture first, then gets reused as the main monitor and a smaller preview in the final frame.",
            12.0,
            Color::from_rgba8(158, 168, 186, 255),
            760.0,
            TextAlign::Center,
        );
        canvas.text(
            126.0,
            166.0,
            "Preview Panel",
            12.0,
            Color::from_rgba8(214, 220, 232, 255),
        );
        canvas.text(
            -202.0,
            -156.0,
            "Press Esc to quit.",
            11.0,
            Color::from_rgba8(132, 142, 160, 255),
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let _ = run::<RenderTargetDemo>(EngineConfig {
        title: "Feature: Render Targets".into(),
        width: 960,
        height: 640,
        show_fps: false,
        ..Default::default()
    });
}
