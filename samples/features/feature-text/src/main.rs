use rengine::*;

struct TextDemo {
    quit: bool,
}

impl Game for TextDemo {
    fn new(_engine: &mut Engine) -> Self {
        Self { quit: false }
    }

    fn update(&mut self, engine: &Engine) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(30, 30, 40, 255);

        let canvas = frame.canvas(0);

        let heading = 22.0;
        let body = 14.0;
        let label_color = Color::from_rgba8(180, 200, 255, 255);
        let text_color = Color::WHITE;
        let dim_color = Color::from_rgba8(140, 140, 140, 255);

        let col_x = -hw + 30.0;
        let mut y = hh - 40.0;

        canvas.text(col_x, y, "measure_text()", heading, label_color, (sw, sh), atlas);
        y -= 30.0;
        let sample = "Hello, World!";
        let sizes = [12.0, 18.0, 24.0, 36.0];
        for &sz in &sizes {
            let (w, h) = atlas.measure_text(sample, sz);
            let label = format!("{}px: \"{}\" => {:.0} x {:.0}", sz, sample, w, h);
            canvas.text(col_x + 10.0, y, &label, body, text_color, (sw, sh), atlas);
            y -= 20.0;
        }

        y -= 15.0;
        canvas.text(col_x, y, "TextAlign", heading, label_color, (sw, sh), atlas);
        y -= 30.0;

        let align_x = 0.0;
        let guide_w = 300.0;
        canvas.rect(align_x - guide_w / 2.0, y - 2.0, guide_w, 1.0, dim_color, (sw, sh));

        let aligns = [
            (TextAlign::Left, "Left-aligned text"),
            (TextAlign::Center, "Center-aligned text"),
            (TextAlign::Right, "Right-aligned text"),
        ];
        for (align, txt) in &aligns {
            let ax = match align {
                TextAlign::Left => align_x - guide_w / 2.0,
                TextAlign::Center => align_x,
                TextAlign::Right => align_x + guide_w / 2.0,
            };
            canvas.text_aligned(ax, y, txt, body, text_color, *align, (sw, sh), atlas);
            y -= 22.0;
        }

        canvas.rect(align_x - guide_w / 2.0, y + 18.0, guide_w, 1.0, dim_color, (sw, sh));

        y -= 20.0;
        canvas.text(col_x, y, "Word wrapping", heading, label_color, (sw, sh), atlas);
        y -= 30.0;

        let paragraph = "The quick brown fox jumps over the lazy dog. \
            This paragraph demonstrates automatic word wrapping within a \
            fixed-width region, with each line broken at word boundaries.";
        let wrap_w = 280.0;

        canvas.rect(col_x + 8.0, y + 2.0, wrap_w, 1.0, dim_color, (sw, sh));

        let widths = [280.0, 180.0];
        let labels = ["max_width = 280", "max_width = 180"];
        let offsets = [col_x + 10.0, col_x + 320.0];

        for i in 0..2 {
            let bx = offsets[i];
            let max_w = widths[i];
            let by = y;

            canvas.text(bx, by, labels[i], 11.0, dim_color, (sw, sh), atlas);
            let by = by - 18.0;

            let lines = wrap_text(paragraph, body, max_w, atlas);
            let lh = atlas.line_height(body);
            let block_h = lines.len() as f32 * lh;

            canvas.rect(bx - 2.0, by - block_h + 2.0, max_w + 4.0, block_h + 4.0, Color::from_rgba8(50, 50, 60, 255), (sw, sh));
            canvas.text_block(bx, by, paragraph, body, text_color, max_w, TextAlign::Left, (sw, sh), atlas);
        }

        y -= 120.0;
        canvas.text(col_x, y, "text_block() with alignment", heading, label_color, (sw, sh), atlas);
        y -= 30.0;

        let short_text = "Short lines\nshow alignment\nclearly across\nmultiple lines.";
        let block_w = 200.0;
        let block_aligns = [
            (TextAlign::Left, "Left"),
            (TextAlign::Center, "Center"),
            (TextAlign::Right, "Right"),
        ];
        let spacing = 220.0;
        let start_x = -hw + 40.0;

        for (i, (align, name)) in block_aligns.iter().enumerate() {
            let bx = start_x + i as f32 * spacing;
            canvas.text(bx, y, name, 11.0, dim_color, (sw, sh), atlas);
            let by = y - 18.0;

            canvas.rect(bx - 2.0, by - 70.0, block_w + 4.0, 74.0, Color::from_rgba8(50, 50, 60, 255), (sw, sh));

            let ax = match align {
                TextAlign::Left => bx,
                TextAlign::Center => bx + block_w / 2.0,
                TextAlign::Right => bx + block_w,
            };
            canvas.text_block(ax, by, short_text, body, text_color, block_w, *align, (sw, sh), atlas);
        }

        y -= 120.0;
        let features = "Features: measure_text, line_height, TextAlign, wrap_text, text_aligned, text_block";
        canvas.text(col_x, y, features, 11.0, dim_color, (sw, sh), atlas);
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Text Layout Demo".into(),
        width: 750,
        height: 620,
        ..Default::default()
    };
    let _ = rengine::run::<TextDemo>(config);
}
