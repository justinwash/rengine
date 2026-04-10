use rengine::pixelart::PixelCanvas;
use rengine::*;

const CELL_W: u32 = 16;
const CELL_H: u32 = 16;
const COLS: u32 = 4;
const ROWS: u32 = 4;
const DISPLAY_SIZE: f32 = 64.0;

struct AnimationDemo {
    sheet: SpriteSheet,
    walk_right: Animation,
    walk_down: Animation,
    walk_left: Animation,
    walk_up: Animation,
    idle: Animation,
    bounce: Animation,
    spin: Animation,
    pulse: Animation,
}

fn make_sprite_sheet(engine: &mut Engine) -> (TextureId, u32, u32) {
    let tw = COLS * CELL_W;
    let th = ROWS * CELL_H;
    let mut pc = PixelCanvas::new(tw, th);

    let body_colors = [
        Color::new(0.3, 0.6, 1.0, 1.0),
        Color::new(0.35, 0.65, 1.0, 1.0),
        Color::new(0.3, 0.6, 1.0, 1.0),
        Color::new(0.25, 0.55, 0.95, 1.0),
    ];
    for col in 0..COLS {
        let ox = (col * CELL_W) as i32;
        let oy = 0i32;
        let body = body_colors[col as usize];
        pc.fill_circle(ox + 8, oy + 8, 6, body);
        pc.fill_circle(ox + 5, oy + 6, 1, Color::WHITE);
        pc.fill_circle(ox + 11, oy + 6, 1, Color::WHITE);
        let foot_offset = if col % 2 == 0 { 0 } else { 1 };
        pc.fill_rect(ox + 4, oy + 13 + foot_offset, 3, 2, body);
        pc.fill_rect(ox + 9, oy + 13 + (1 - foot_offset), 3, 2, body);
    }

    let down_colors = [
        Color::new(1.0, 0.4, 0.3, 1.0),
        Color::new(1.0, 0.45, 0.35, 1.0),
        Color::new(1.0, 0.4, 0.3, 1.0),
        Color::new(0.95, 0.35, 0.25, 1.0),
    ];
    for col in 0..COLS {
        let ox = (col * CELL_W) as i32;
        let oy = CELL_H as i32;
        let body = down_colors[col as usize];
        pc.fill_circle(ox + 8, oy + 8, 6, body);
        pc.fill_circle(ox + 5, oy + 7, 1, Color::WHITE);
        pc.fill_circle(ox + 11, oy + 7, 1, Color::WHITE);
        let foot_offset = if col % 2 == 0 { 0 } else { 1 };
        pc.fill_rect(ox + 4, oy + 13 + foot_offset, 3, 2, body);
        pc.fill_rect(ox + 9, oy + 13 + (1 - foot_offset), 3, 2, body);
    }

    let green = Color::new(0.3, 0.8, 0.3, 1.0);
    for col in 0..COLS {
        let ox = (col * CELL_W) as i32;
        let oy = (2 * CELL_H) as i32;
        let r = 4 + (col % 3) as i32;
        pc.fill_circle(ox + 8, oy + 8, r, green);
    }

    let colors_spin = [
        Color::new(1.0, 0.8, 0.2, 1.0),
        Color::new(0.2, 1.0, 0.5, 1.0),
        Color::new(0.8, 0.3, 1.0, 1.0),
        Color::new(1.0, 0.5, 0.2, 1.0),
    ];
    for col in 0..COLS {
        let ox = (col * CELL_W) as i32;
        let oy = (3 * CELL_H) as i32;
        pc.fill_rect(ox + 2, oy + 2, 12, 12, colors_spin[col as usize]);
        pc.fill_rect(ox + 4, oy + 4, 8, 8, Color::WHITE);
    }

    let bytes = pc.into_bytes();
    let tex = engine.create_texture(tw, th, &bytes);
    (tex, tw, th)
}

impl Game for AnimationDemo {
    fn new(engine: &mut Engine) -> Self {
        let (tex, tw, th) = make_sprite_sheet(engine);
        let sheet = SpriteSheet::new(tex, tw, th, CELL_W, CELL_H);

        let walk_right = Animation::new(vec![(0, 0), (1, 0), (2, 0), (3, 0)], 8.0);
        let walk_down = Animation::new(vec![(0, 1), (1, 1), (2, 1), (3, 1)], 8.0);
        let walk_left = Animation::new(vec![(3, 0), (2, 0), (1, 0), (0, 0)], 8.0);
        let walk_up = Animation::new(vec![(3, 1), (2, 1), (1, 1), (0, 1)], 8.0);
        let idle = Animation::new(vec![(0, 0), (0, 0), (0, 0), (1, 0)], 2.0);
        let bounce = Animation::new(vec![(0, 2), (1, 2), (2, 2), (3, 2)], 6.0);
        let spin = Animation::new(vec![(0, 3), (1, 3), (2, 3), (3, 3)], 10.0);
        let pulse = Animation::new(vec![(0, 2), (1, 2), (2, 2), (1, 2)], 4.0);

        Self {
            sheet,
            walk_right,
            walk_down,
            walk_left,
            walk_up,
            idle,
            bounce,
            spin,
            pulse,
        }
    }

    fn update(&mut self, engine: &Engine) {
        let dt = engine.dt();
        self.walk_right.update(dt);
        self.walk_down.update(dt);
        self.walk_left.update(dt);
        self.walk_up.update(dt);
        self.idle.update(dt);
        self.bounce.update(dt);
        self.spin.update(dt);
        self.pulse.update(dt);
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.1, 0.1, 0.14, 1.0);
        let screen = engine.window_size();
        let atlas = engine.font_atlas();

        let hud = frame.canvas(0);
        hud.rect(0.0, 0.0, screen.0 as f32, 40.0, Color::new(0.08, 0.07, 0.1, 0.95), screen);
        hud.text(16.0, 10.0, "SpriteSheet + Animation Demo", 18.0, Color::WHITE, screen, atlas);

        let animations: [(&str, &Animation); 8] = [
            ("Walk Right (8fps)", &self.walk_right),
            ("Walk Down (8fps)", &self.walk_down),
            ("Walk Left (8fps)", &self.walk_left),
            ("Walk Up (8fps)", &self.walk_up),
            ("Idle (2fps)", &self.idle),
            ("Bounce (6fps)", &self.bounce),
            ("Spin (10fps)", &self.spin),
            ("Pulse (4fps)", &self.pulse),
        ];

        let cols = 4;
        let x_spacing = 160.0;
        let y_spacing = 130.0;
        let start_x = 60.0;
        let start_y = 70.0;

        for (i, (label, anim)) in animations.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = start_x + col as f32 * x_spacing;
            let y = start_y + row as f32 * y_spacing;

            let (fc, fr) = anim.current_frame();
            let uv = self.sheet.uv_rect(fc, fr);

            frame.draw_sprite(
                DrawParams::new(
                    self.sheet.texture,
                    Vec2::new(x, y),
                    Vec2::new(DISPLAY_SIZE, DISPLAY_SIZE),
                )
                .with_uv_rect(uv),
            );

            let labels = frame.canvas(0);
            labels.text(x, y + DISPLAY_SIZE + 6.0, label, 12.0, Color::new(0.7, 0.8, 0.9, 1.0), screen, atlas);

            let frame_text = format!("frame: ({},{})", fc, fr);
            labels.text(x, y + DISPLAY_SIZE + 22.0, &frame_text, 11.0, Color::new(0.5, 0.6, 0.7, 1.0), screen, atlas);
        }

        let sheet_label = frame.canvas(0);
        sheet_label.text(start_x, start_y + 2.0 * y_spacing + 10.0, "Sprite sheet (4x4 grid, 16x16 cells):", 13.0, Color::new(0.6, 0.7, 0.8, 1.0), screen, atlas);

        let sheet_y = start_y + 2.0 * y_spacing + 30.0;
        let preview_scale = 4.0;
        for row in 0..ROWS {
            for col in 0..COLS {
                let uv = self.sheet.uv_rect(col, row);
                let px = start_x + col as f32 * (CELL_W as f32 * preview_scale + 4.0);
                let py = sheet_y + row as f32 * (CELL_H as f32 * preview_scale + 4.0);
                frame.draw_sprite(
                    DrawParams::new(
                        self.sheet.texture,
                        Vec2::new(px, py),
                        Vec2::new(CELL_W as f32 * preview_scale, CELL_H as f32 * preview_scale),
                    )
                    .with_uv_rect(uv),
                );
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<AnimationDemo>(EngineConfig {
        title: "Feature: SpriteSheet Animation".into(),
        width: 700,
        height: 640,
        ..Default::default()
    })
}
