use rengine::*;

struct FontsDemo {
    mono: FontId,
    quit: bool,
}

impl Game for FontsDemo {
    fn new(engine: &mut Engine) -> Self {
        let mono_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/mono.ttf");
        let mono = engine.load_font(&mono_path).expect("failed to load mono font");
        Self { mono, quit: false }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;

        let default = engine.font(FontId::DEFAULT);
        let mono = engine.font(self.mono);

        frame.clear_color = Color::from_rgba8(30, 30, 40, 255);
        let canvas = frame.canvas(0);

        let heading = 22.0;
        let body = 16.0;
        let label_color = Color::from_rgba8(180, 200, 255, 255);
        let text_color = Color::WHITE;
        let dim_color = Color::from_rgba8(140, 140, 140, 255);

        let col_x = -hw + 30.0;
        let mut y = hh - 40.0;

        canvas.text(col_x, y, "Multiple Font Support", heading, label_color);
        y -= 35.0;

        canvas.text(col_x, y, "Default font (built-in):", body, dim_color);
        y -= 24.0;
        canvas.text(col_x + 10.0, y, "The quick brown fox jumps over the lazy dog.", body, text_color);
        y -= 24.0;
        canvas.text(col_x + 10.0, y, "ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789", body, text_color);
        y -= 40.0;

        canvas.text(col_x, y, "Mono font (JetBrains Mono, loaded from file):", body, dim_color);
        y -= 24.0;
        canvas.text_with_font(col_x + 10.0, y, "The quick brown fox jumps over the lazy dog.", body, text_color, mono);
        y -= 24.0;
        canvas.text_with_font(col_x + 10.0, y, "ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789", body, text_color, mono);
        y -= 40.0;

        canvas.text(col_x, y, "Mixed fonts on the same canvas:", heading, label_color);
        y -= 30.0;
        canvas.text(col_x + 10.0, y, "This is the default font, ", body, text_color);
        let w = default.measure_text("This is the default font, ", body).0;
        canvas.text_with_font(col_x + 10.0 + w, y, "and this is mono.", body, Color::from_rgba8(120, 255, 120, 255), mono);
        y -= 40.0;

        canvas.text(col_x, y, "Size comparison:", heading, label_color);
        y -= 30.0;
        let sizes = [12.0, 16.0, 24.0, 32.0];
        for &sz in &sizes {
            let label = format!("{}px default", sz as u32);
            canvas.text(col_x + 10.0, y, &label, sz, text_color);
            let dw = default.measure_text(&label, sz).0;
            let mono_label = format!("  |  {}px mono", sz as u32);
            canvas.text_with_font(col_x + 10.0 + dw, y, &mono_label, sz, label_color, mono);
            y -= sz + 8.0;
        }

        y -= 10.0;
        canvas.text(col_x, y, "Measurement:", heading, label_color);
        y -= 30.0;
        let sample = "Hello, World!";
        let sz = 20.0;
        let (dw, dh) = default.measure_text(sample, sz);
        let (mw, mh) = mono.measure_text(sample, sz);
        canvas.text(
            col_x + 10.0, y,
            &format!("Default \"{}\" @ {}px: {:.0}x{:.0}", sample, sz as u32, dw, dh),
            14.0, text_color,
        );
        y -= 20.0;
        canvas.text(
            col_x + 10.0, y,
            &format!("Mono    \"{}\" @ {}px: {:.0}x{:.0}", sample, sz as u32, mw, mh),
            14.0, text_color,
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: Multiple Fonts".into(),
        width: 900,
        height: 700,
        show_fps: true,
        ..Default::default()
    };
    let _ = run::<FontsDemo>(config);
}
