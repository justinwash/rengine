use rengine::*;

struct ImagesDemo {
    ui: Ui,
    card: TextureId,
    icons: TextureId,
    time: f32,
    accent_index: usize,
}

impl ImagesDemo {
    fn create_card(engine: &mut Engine) -> TextureId {
        let width = 96;
        let height = 128;
        let mut canvas = pixelart::PixelCanvas::new(width, height);
        let navy = Color::from_rgba8(24, 30, 52, 255);
        let gold = Color::from_rgba8(214, 163, 56, 255);
        let red = Color::from_rgba8(214, 77, 63, 255);
        let sky = Color::from_rgba8(148, 216, 255, 255);

        canvas.fill(navy);
        for y in 0..height as i32 {
            let t = y as f32 / (height - 1) as f32;
            let stripe = Color::new(0.10 + 0.18 * t, 0.14 + 0.22 * t, 0.25 + 0.28 * t, 1.0);
            for x in 0..width as i32 {
                canvas.set(x, y, stripe);
            }
        }
        for x in 5..(width as i32 - 5) {
            canvas.set(x, 5, gold);
            canvas.set(x, height as i32 - 6, gold);
        }
        for y in 5..(height as i32 - 5) {
            canvas.set(5, y, gold);
            canvas.set(width as i32 - 6, y, gold);
        }
        for y in 16..52 {
            for x in 30..66 {
                let dx = x as f32 - 48.0;
                let dy = y as f32 - 34.0;
                if dx * dx + dy * dy <= 290.0 {
                    canvas.set(x, y, red);
                }
            }
        }
        for y in 28..40 {
            for x in 46..78 {
                canvas.set(x, y, sky);
            }
        }
        for y in 62..104 {
            for x in 24..72 {
                if (x + y) % 7 < 3 {
                    canvas.set(x, y, Color::from_rgba8(32, 40, 70, 255));
                }
            }
        }
        for y in 78..84 {
            for x in 24..72 {
                canvas.set(x, y, gold);
            }
        }

        engine.create_texture(width, height, &canvas.into_bytes())
    }

    fn create_icons(engine: &mut Engine) -> TextureId {
        let width = 64;
        let height = 32;
        let mut canvas = pixelart::PixelCanvas::new(width, height);
        canvas.fill(Color::new(0.0, 0.0, 0.0, 0.0));

        let amber = Color::from_rgba8(255, 191, 83, 255);
        let teal = Color::from_rgba8(92, 224, 202, 255);
        let dark = Color::from_rgba8(28, 34, 52, 255);

        for y in 4..28 {
            for x in 4..28 {
                if (x - 16) * (x - 16) + (y - 16) * (y - 16) <= 120 {
                    canvas.set(x, y, amber);
                }
            }
        }
        for y in 10..22 {
            for x in 11..21 {
                canvas.set(x, y, dark);
            }
        }
        for y in 5..27 {
            for x in 37..59 {
                if x >= 48 - (y - 5) / 2 && x <= 48 + (y - 5) / 2 {
                    canvas.set(x, y, teal);
                }
            }
        }
        for y in 18..24 {
            for x in 43..53 {
                canvas.set(x, y, dark);
            }
        }

        engine.create_texture(width, height, &canvas.into_bytes())
    }

    fn build_ui(&mut self, engine: &Engine) {
        self.ui.begin(engine, -360.0, 56.0, 240.0);
        self.ui
            .label_centered("UI Image Widget", 24.0, Color::WHITE);
        self.ui.separator(8.0);
        self.ui.panel(7);
        self.ui.image(self.card, Vec2::new(120.0, 160.0));
        self.ui.separator(6.0);
        self.ui
            .label_centered("Profile Card", 14.0, Color::from_rgba8(220, 220, 220, 255));
        self.ui.row(2);
        self.ui
            .image_region(self.icons, Vec2::new(44.0, 44.0), [0.0, 0.0, 0.5, 1.0]);
        self.ui
            .image_region(self.icons, Vec2::new(44.0, 44.0), [0.5, 0.0, 0.5, 1.0]);
        self.ui.button(0, "Cycle Tint");
    }
}

impl Game for ImagesDemo {
    fn new(engine: &mut Engine) -> Self {
        Self {
            ui: Ui::default(),
            card: Self::create_card(engine),
            icons: Self::create_icons(engine),
            time: 0.0,
            accent_index: 0,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.time += engine.dt();
        self.build_ui(engine);
        if self.ui.update(engine).was_activated(0) {
            self.accent_index = (self.accent_index + 1) % 3;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(16, 20, 30, 255);
        let (_, hh) = engine.half_size();
        let accent = match self.accent_index {
            0 => Color::from_rgba8(255, 191, 83, 255),
            1 => Color::from_rgba8(92, 224, 202, 255),
            _ => Color::from_rgba8(247, 108, 122, 255),
        };
        let pulse = 0.9 + 0.1 * (self.time * 2.0).sin();

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);

        canvas.text(20.0, hh - 48.0, "Canvas Images", 26.0, Color::WHITE);
        canvas.text_block(
            20.0,
            hh - 78.0,
            "Screen-space textures can sit beside UI widgets without leaving the same 2D canvas pass.",
            14.0,
            Color::from_rgba8(190, 190, 205, 255),
            420.0,
            TextAlign::Left,
        );

        canvas.rect(
            20.0,
            hh - 430.0,
            420.0,
            320.0,
            Color::from_rgba8(24, 28, 44, 235),
        );
        canvas.image(self.card, 48.0, hh - 384.0, 156.0, 208.0);
        canvas.text(236.0, hh - 152.0, "Tinted icon", 13.0, accent);
        canvas.image_colored(self.icons, 252.0, hh - 214.0, 64.0, 64.0, accent);
        canvas.text(
            236.0,
            hh - 232.0,
            "Sprite region",
            13.0,
            Color::from_rgba8(160, 195, 255, 255),
        );
        canvas.image_region(
            self.icons,
            252.0,
            hh - 302.0,
            64.0,
            64.0,
            [0.5, 0.0, 0.5, 1.0],
            Color::new(pulse, pulse, pulse, 1.0),
        );
        canvas.text_block(
            236.0,
            hh - 330.0,
            "Full textures, sprite regions, and tinting all reuse the same screen-space image API.",
            13.0,
            Color::from_rgba8(220, 220, 220, 255),
            168.0,
            TextAlign::Left,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<ImagesDemo>(EngineConfig {
        title: "Feature: Screen-Space Images".into(),
        width: 1040,
        height: 700,
        ..Default::default()
    })
}
