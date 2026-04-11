use rengine::pixelart::{darken, lighten, PixelCanvas};
use rengine::*;

struct PixelArtDemo {
    textures: Vec<(TextureId, &'static str)>,
}

fn make_smiley(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(16, 16);
    pc.fill_circle(8, 8, 7, Color::YELLOW);
    pc.fill_circle(5, 6, 1, Color::new(0.2, 0.1, 0.0, 1.0));
    pc.fill_circle(11, 6, 1, Color::new(0.2, 0.1, 0.0, 1.0));
    for x in 5..=11 {
        let y = if x >= 7 && x <= 9 { 12 } else { 11 };
        pc.set(x, y, Color::new(0.2, 0.1, 0.0, 1.0));
    }
    let bytes = pc.into_bytes();
    engine.create_texture(16, 16, &bytes)
}

fn make_tree(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(16, 24);
    let brown = Color::new(0.4, 0.25, 0.1, 1.0);
    pc.fill_rect(6, 16, 4, 8, brown);
    let green = Color::new(0.2, 0.6, 0.15, 1.0);
    let light = lighten(green, 1.4);
    pc.fill_circle(8, 10, 6, green);
    pc.fill_circle(6, 7, 3, light);
    pc.fill_circle(10, 8, 3, light);
    let bytes = pc.into_bytes();
    engine.create_texture(16, 24, &bytes)
}

fn make_gem(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(16, 16);
    pc.fill_diamond(Color::new(0.3, 0.6, 1.0, 1.0));
    pc.stroke_diamond(Color::new(0.5, 0.8, 1.0, 1.0), 2.0);
    let highlight = Color::new(0.9, 0.95, 1.0, 0.8);
    pc.set(6, 5, highlight);
    pc.set(7, 4, highlight);
    pc.set(7, 5, highlight);
    let bytes = pc.into_bytes();
    engine.create_texture(16, 16, &bytes)
}

fn make_gradient(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(32, 32);
    let base = Color::new(1.0, 0.3, 0.1, 1.0);
    for y in 0..32 {
        let t = y as f32 / 31.0;
        let c = if t < 0.5 {
            lighten(base, 1.0 + (0.5 - t))
        } else {
            darken(base, 1.0 - (t - 0.5) * 0.8)
        };
        for x in 0..32 {
            pc.set(x, y, c);
        }
    }
    let bytes = pc.into_bytes();
    engine.create_texture(32, 32, &bytes)
}

fn make_checkerboard(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(16, 16);
    let c1 = Color::new(0.9, 0.9, 0.85, 1.0);
    let c2 = Color::new(0.3, 0.3, 0.35, 1.0);
    for y in 0..16 {
        for x in 0..16 {
            let check = ((x / 4) + (y / 4)) % 2 == 0;
            pc.set(x, y, if check { c1 } else { c2 });
        }
    }
    let bytes = pc.into_bytes();
    engine.create_texture(16, 16, &bytes)
}

fn make_circle_pattern(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(24, 24);
    pc.fill(Color::new(0.1, 0.05, 0.2, 1.0));
    pc.fill_circle(12, 12, 10, Color::new(0.6, 0.2, 0.8, 1.0));
    pc.fill_circle(12, 12, 6, Color::new(0.8, 0.4, 1.0, 1.0));
    pc.fill_circle(12, 12, 2, Color::WHITE);
    let bytes = pc.into_bytes();
    engine.create_texture(24, 24, &bytes)
}

impl Game for PixelArtDemo {
    fn new(engine: &mut Engine) -> Self {
        Self {
            textures: vec![
                (make_smiley(engine), "Smiley (circles)"),
                (make_tree(engine), "Tree (circle+rect)"),
                (make_gem(engine), "Gem (diamond)"),
                (make_gradient(engine), "Gradient (darken/lighten)"),
                (make_checkerboard(engine), "Checkerboard (set pixel)"),
                (make_circle_pattern(engine), "Rings (nested circles)"),
            ],
        }
    }

    fn update(&mut self, _engine: &Engine) {}

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.12, 0.1, 0.15, 1.0);
        let screen = engine.window_size();
        let atlas = engine.font_atlas();

        let hud = frame.canvas(0);
        hud.rect(0.0, 0.0, screen.0 as f32, 40.0, Color::new(0.08, 0.07, 0.1, 0.95), screen);
        hud.text(16.0, 10.0, "PixelCanvas Procedural Textures", 18.0, Color::WHITE, screen, atlas);

        let cols = 3;
        let spacing = 180.0;
        let scale = 6.0;
        let start_x = 80.0;
        let start_y = 80.0;

        for (i, (tex, label)) in self.textures.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = start_x + col as f32 * spacing;
            let y = start_y + row as f32 * 220.0;

            let size = if i == 1 {
                Vec2::new(16.0 * scale, 24.0 * scale)
            } else if i == 3 || i == 5 {
                Vec2::new(32.0 * scale * 0.75, 32.0 * scale * 0.75)
            } else {
                Vec2::new(16.0 * scale, 16.0 * scale)
            };

            frame.draw(*tex, Vec2::new(x, y), size);

            let labels = frame.canvas(0);
            labels.text(x, y + size.y + 8.0, label, 13.0, Color::new(0.7, 0.8, 0.9, 1.0), screen, atlas);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<PixelArtDemo>(EngineConfig {
        title: "Feature: PixelArt Procedural Textures".into(),
        width: 640,
        height: 560,
        ..Default::default()
    })
}
