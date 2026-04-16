use rengine::*;

const GAME_W: u32 = 320;
const GAME_H: u32 = 240;
const TILE: f32 = 16.0;

struct ResolutionDemo {
    checker_a: TextureId,
    checker_b: TextureId,
    border: TextureId,
    mode_index: usize,
    modes: [ScaleMode; 3],
    frame_count: u32,
    demo: bool,
    max_frames: Option<u32>,
}

impl Game for ResolutionDemo {
    fn new(engine: &mut Engine) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let demo = args.contains(&"--demo".to_string());
        let max_frames = args
            .windows(2)
            .find(|w| w[0] == "--frames")
            .and_then(|w| w[1].parse().ok());

        Self {
            checker_a: engine.create_color_texture(1, 1, Color::from_rgba8(80, 120, 200, 255)),
            checker_b: engine.create_color_texture(1, 1, Color::from_rgba8(60, 90, 160, 255)),
            border: engine.create_color_texture(1, 1, Color::from_rgba8(255, 200, 50, 255)),
            mode_index: 1,
            modes: [ScaleMode::Stretch, ScaleMode::Letterbox, ScaleMode::PixelPerfect],
            frame_count: 0,
            demo,
            max_frames,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.frame_count += 1;

        if self.demo {
            if self.frame_count == 120 || self.frame_count == 240 {
                self.mode_index = (self.mode_index + 1) % self.modes.len();
            }
        }

        if engine.input().is_key_pressed(KeyCode::Digit1) {
            self.mode_index = 0;
        }
        if engine.input().is_key_pressed(KeyCode::Digit2) {
            self.mode_index = 1;
        }
        if engine.input().is_key_pressed(KeyCode::Digit3) {
            self.mode_index = 2;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        engine.set_scale_mode(self.modes[self.mode_index]);

        frame.clear_color = Color::from_rgba8(30, 30, 40, 255);

        let half_w = GAME_W as f32 / 2.0;
        let half_h = GAME_H as f32 / 2.0;
        for row in 0..(GAME_H as i32 / TILE as i32) {
            for col in 0..(GAME_W as i32 / TILE as i32) {
                let tex = if (row + col) % 2 == 0 {
                    self.checker_a
                } else {
                    self.checker_b
                };
                let x = col as f32 * TILE - half_w;
                let y = row as f32 * TILE - half_h;
                frame.draw(tex, Vec2::new(x, y), Vec2::new(TILE, TILE));
            }
        }

        let b = 2.0;
        frame.draw(self.border, Vec2::new(-half_w, -half_h), Vec2::new(GAME_W as f32, b));
        frame.draw(self.border, Vec2::new(-half_w, half_h - b), Vec2::new(GAME_W as f32, b));
        frame.draw(self.border, Vec2::new(-half_w, -half_h), Vec2::new(b, GAME_H as f32));
        frame.draw(self.border, Vec2::new(half_w - b, -half_h), Vec2::new(b, GAME_H as f32));

        let t = self.frame_count as f32 * 0.02;
        let marker_x = t.sin() * 100.0;
        let marker_y = t.cos() * 60.0;
        frame.draw(self.border, Vec2::new(marker_x - 4.0, marker_y - 4.0), Vec2::new(8.0, 8.0));

        let (ww, wh) = engine.window_size();
        let (gw, gh) = engine.game_size();
        let atlas = engine.font_atlas();
        let hw = ww as f32 / 2.0;
        let hh = wh as f32 / 2.0;

        let mode_name = match self.modes[self.mode_index] {
            ScaleMode::Stretch => "Stretch",
            ScaleMode::Letterbox => "Letterbox",
            ScaleMode::PixelPerfect => "PixelPerfect",
        };
        let info = format!(
            "Mode: {} [1/2/3]  Game: {}x{}  Window: {}x{}",
            mode_name, gw, gh, ww, wh,
        );

        let canvas = frame.canvas(0);
        canvas.rect(-hw + 4.0, hh - 4.0 - 22.0, 400.0, 22.0, Color::from_rgba8(0, 0, 0, 180));
        canvas.text(-hw + 8.0, hh - 8.0, &info, 14.0, Color::WHITE, atlas);
    }

    fn should_exit(&self) -> bool {
        if let Some(max) = self.max_frames {
            self.frame_count >= max
        } else {
            false
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<ResolutionDemo>(EngineConfig {
        title: "Resolution Scaling Demo".into(),
        width: 960,
        height: 720,
        render_width: Some(GAME_W),
        render_height: Some(GAME_H),
        scale_mode: ScaleMode::Letterbox,
        ..Default::default()
    })
}
