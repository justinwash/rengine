use rengine::*;

/// Demonstrates nine-slice rendering for resizable UI panels.
///
/// Shows:
/// - NineSlice::new() and NineSlice::uniform() constructors
/// - draw_nine_slice() at various sizes
/// - Corners stay fixed, edges stretch, center fills
/// - Color tinting via with_color()
/// - Animated panel resizing

struct NineSliceDemo {
    panel: NineSlice,
    quit: bool,
    time: f32,
}

/// Build a 32×32 panel texture using PixelCanvas:
///
/// ```text
///  ┌──────────────────────────┐
///  │ ░░░░░░░░░░░░░░░░░░░░░░░ │  <- 1px dark border
///  │ ░┌────────────────────┐░ │
///  │ ░│  8px corner = dark │░ │  <- corners are distinct
///  │ ░│                    │░ │
///  │ ░│    center = mid    │░ │  <- center is lighter fill
///  │ ░│                    │░ │
///  │ ░└────────────────────┘░ │
///  │ ░░░░░░░░░░░░░░░░░░░░░░░ │
///  └──────────────────────────┘
/// ```
fn make_panel_texture(engine: &mut Engine) -> TextureId {
    let size = 32u32;
    let border = 8u32;
    let mut canvas = pixelart::PixelCanvas::new(size, size);

    // Fill entire texture with corner color (dark blue-gray)
    let corner_color = Color::from_rgba8(40, 45, 65, 255);
    canvas.fill(corner_color);

    // Fill the edge strips (mid-tone)
    let edge_color = Color::from_rgba8(55, 60, 85, 255);
    // Top edge (between corners)
    canvas.fill_rect(
        border as i32,
        0,
        (size - border * 2) as i32,
        border as i32,
        edge_color,
    );
    // Bottom edge
    canvas.fill_rect(
        border as i32,
        (size - border) as i32,
        (size - border * 2) as i32,
        border as i32,
        edge_color,
    );
    // Left edge
    canvas.fill_rect(
        0,
        border as i32,
        border as i32,
        (size - border * 2) as i32,
        edge_color,
    );
    // Right edge
    canvas.fill_rect(
        (size - border) as i32,
        border as i32,
        border as i32,
        (size - border * 2) as i32,
        edge_color,
    );

    // Fill center (lighter)
    let center_color = Color::from_rgba8(70, 78, 110, 255);
    canvas.fill_rect(
        border as i32,
        border as i32,
        (size - border * 2) as i32,
        (size - border * 2) as i32,
        center_color,
    );

    // 1px bright border around the outside
    let outline = Color::from_rgba8(120, 130, 180, 255);
    for i in 0..size as i32 {
        canvas.set(i, 0, outline);
        canvas.set(i, (size - 1) as i32, outline);
        canvas.set(0, i, outline);
        canvas.set((size - 1) as i32, i, outline);
    }

    // Inner highlight (1px inside, top-left edges)
    let highlight = Color::from_rgba8(90, 100, 145, 255);
    for i in 1..(size - 1) as i32 {
        canvas.set(i, 1, highlight);
        canvas.set(1, i, highlight);
    }

    engine.create_texture(size, size, &canvas.into_bytes())
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Game for NineSliceDemo {
    fn new(engine: &mut Engine) -> Self {
        let tex = make_panel_texture(engine);
        let panel = NineSlice::uniform(tex, 32, 32, 8);

        Self {
            panel,
            quit: false,
            time: 0.0,
        }
    }

    fn update(&mut self, engine: &Engine) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }
        self.time += engine.time().dt();
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let t = self.time;

        // Animated sizes — smooth ping-pong using sin
        let anim_w1 = lerp(60.0, 200.0, (t * 0.8).sin() * 0.5 + 0.5);
        let anim_h1 = lerp(40.0, 150.0, (t * 1.2).sin() * 0.5 + 0.5);
        let anim_w2 = lerp(100.0, 400.0, (t * 0.6).sin() * 0.5 + 0.5);

        // Layout: two rows with generous spacing
        // Row 1 (y=90): static panels — Small, Medium, Wide
        // Row 2 (y=320): Tall, Tinted, comparison textures
        // Row 3 (y=530): animated panels

        let label_color = Color::from_rgba8(200, 200, 255, 255);
        let label_size = 14.0;
        let row1_y = 90.0;
        let row2_y = 310.0;
        let row3_y = 540.0;

        // --- Static panels ---
        let static_panels: &[(&str, f32, f32, f32, f32)] = &[
            ("Small (60x40)", 30.0, row1_y, 60.0, 40.0),
            ("Medium (200x120)", 130.0, row1_y, 200.0, 120.0),
            ("Wide (400x50)", 370.0, row1_y, 400.0, 50.0),
            ("Tall (80x250)", 30.0, row2_y, 80.0, 250.0),
            ("Large (500x200)", 150.0, row2_y, 500.0, 200.0),
        ];

        for &(_label, x, y, w, h) in static_panels {
            frame.draw_nine_slice(&self.panel, Vec2::new(x, y), Vec2::new(w, h));
        }

        // Tinted panel
        let tinted = self
            .panel
            .clone()
            .with_color(Color::from_rgba8(255, 180, 100, 255));
        frame.draw_nine_slice(&tinted, Vec2::new(700.0, row2_y), Vec2::new(180.0, 150.0));

        // Source texture at 1:1 for comparison
        frame.draw(
            self.panel.texture,
            Vec2::new(700.0, row1_y),
            Vec2::new(32.0, 32.0),
        );

        // Naive stretch for comparison
        frame.draw(
            self.panel.texture,
            Vec2::new(760.0, row1_y),
            Vec2::new(120.0, 60.0),
        );

        // --- Animated panels ---
        let anim_color = Color::from_rgba8(100, 220, 180, 200);
        let anim_panel = self.panel.clone().with_color(anim_color);
        frame.draw_nine_slice(
            &anim_panel,
            Vec2::new(30.0, row3_y),
            Vec2::new(anim_w1, anim_h1),
        );
        frame.draw_nine_slice(
            &anim_panel,
            Vec2::new(280.0, row3_y),
            Vec2::new(anim_w2, 80.0),
        );
        // Breathing square
        let breath = lerp(60.0, 160.0, (t * 1.5).sin() * 0.5 + 0.5);
        frame.draw_nine_slice(
            &anim_panel,
            Vec2::new(730.0, row3_y),
            Vec2::new(breath, breath),
        );

        // --- Canvas text overlay ---
        frame.clear_color = Color::from_rgba8(15, 15, 25, 255);
        let canvas = frame.canvas(0);

        canvas.text(
            20.0,
            20.0,
            "NineSlice Feature Demo",
            28.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );
        canvas.text(
            20.0,
            52.0,
            "Same 32x32 texture drawn at different sizes - corners stay sharp",
            16.0,
            Color::from_rgba8(180, 180, 180, 255),
            (sw, sh),
            atlas,
        );

        // Static panel labels
        for &(label, x, y, _w, _h) in static_panels {
            canvas.text(x, y - 18.0, label, label_size, label_color, (sw, sh), atlas);
        }

        canvas.text(
            700.0,
            row2_y - 18.0,
            "Tinted (180x150)",
            label_size,
            Color::from_rgba8(255, 200, 150, 255),
            (sw, sh),
            atlas,
        );

        // Comparison labels
        canvas.text(
            700.0,
            row1_y - 18.0,
            "Source (1:1)",
            label_size,
            label_color,
            (sw, sh),
            atlas,
        );
        canvas.text(
            760.0,
            row1_y - 18.0,
            "Naive stretch",
            label_size,
            label_color,
            (sw, sh),
            atlas,
        );

        // Animated panel labels
        let anim_label_color = Color::from_rgba8(150, 255, 220, 255);
        canvas.text(
            30.0,
            row3_y - 18.0,
            "Animated (resizing)",
            label_size,
            anim_label_color,
            (sw, sh),
            atlas,
        );
        canvas.text(
            280.0,
            row3_y - 18.0,
            "Animated (width stretch)",
            label_size,
            anim_label_color,
            (sw, sh),
            atlas,
        );
        canvas.text(
            730.0,
            row3_y - 18.0,
            "Animated (breathing)",
            label_size,
            anim_label_color,
            (sw, sh),
            atlas,
        );

        canvas.text(
            20.0,
            sh as f32 - 30.0,
            "ESC to quit",
            14.0,
            Color::from_rgba8(120, 120, 120, 255),
            (sw, sh),
            atlas,
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: Nine-Slice".into(),
        width: 960,
        height: 760,
        show_fps: false,
        ..Default::default()
    };
    let _ = run::<NineSliceDemo>(config);
}
