use rengine::*;

const SECTIONS: [(Vec2, f32); 6] = [
    (Vec2::new(-290.0, 245.0), 2.5),
    (Vec2::new(-260.0, 175.0), 2.5),
    (Vec2::new(-260.0, 105.0), 2.5),
    (Vec2::new(-260.0, 35.0), 2.5),
    (Vec2::new(200.0, 140.0), 1.8),
    (Vec2::new(-320.0, -90.0), 2.2),
];
const OVERVIEW: (Vec2, f32) = (Vec2::new(-50.0, 60.0), 0.85);

struct SpriteShowcase {
    white: TextureId,
    checker: TextureId,
    time: f32,
    cam_pos: Vec2,
    cam_zoom: f32,
    target_pos: Vec2,
    target_zoom: f32,
}

impl Game for SpriteShowcase {
    fn new(engine: &mut Engine) -> Self {
        let white = engine.create_color_texture(1, 1, Color::WHITE);

        let mut pixels = Vec::with_capacity(4 * 4 * 4);
        for row in 0..4 {
            for col in 0..4 {
                let bright = (row + col) % 2 == 0;
                if bright {
                    pixels.extend_from_slice(&[220, 60, 220, 255]);
                } else {
                    pixels.extend_from_slice(&[40, 40, 40, 255]);
                }
            }
        }
        let checker = engine.create_texture(4, 4, &pixels);

        Self {
            white,
            checker,
            time: 0.0,
            cam_pos: OVERVIEW.0,
            cam_zoom: OVERVIEW.1,
            target_pos: OVERVIEW.0,
            target_zoom: OVERVIEW.1,
        }
    }

    fn update(&mut self, engine: &Engine) {
        self.time += engine.dt();

        let input = engine.input();
        let dt = engine.dt();

        let section_keys = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
        ];
        for (i, key) in section_keys.iter().enumerate() {
            if input.is_key_pressed(*key) {
                self.target_pos = SECTIONS[i].0;
                self.target_zoom = SECTIONS[i].1;
            }
        }

        if input.is_key_pressed(KeyCode::Digit0) || input.is_key_pressed(KeyCode::Space) {
            self.target_pos = OVERVIEW.0;
            self.target_zoom = OVERVIEW.1;
        }

        if input.is_key_down(KeyCode::Equal) {
            self.target_zoom *= 1.0 + dt;
        }
        if input.is_key_down(KeyCode::Minus) {
            self.target_zoom *= 1.0 - dt;
        }

        let speed = 6.0 * dt;
        self.cam_pos += (self.target_pos - self.cam_pos) * speed.min(1.0);
        self.cam_zoom += (self.target_zoom - self.cam_zoom) * speed.min(1.0);
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.12, 0.12, 0.18, 1.0);
        frame.camera.position = self.cam_pos;
        frame.camera.zoom = self.cam_zoom;

        let white = self.white;
        let checker = self.checker;

        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let canvas = frame.canvas(0);
        canvas.text(
            -hw + 8.0,
            hh - 8.0 - 11.0,
            "1:Basic  2:Tint  3:UV  4:Flip  5:Rotation  6:Z-Order  |  0/Space:Overview  +/-:Zoom",
            11.0,
            Color::WHITE,
            (sw, sh),
            engine.font_atlas(),
        );

        let sx = -380.0;
        let sy = 220.0;
        frame.draw(white, Vec2::new(sx, sy), Vec2::new(50.0, 50.0));
        frame.draw(white, Vec2::new(sx + 60.0, sy), Vec2::new(30.0, 50.0));
        frame.draw(white, Vec2::new(sx + 100.0, sy), Vec2::new(50.0, 30.0));
        frame.draw(white, Vec2::new(sx + 160.0, sy), Vec2::new(20.0, 20.0));

        let sy = 150.0;
        frame.draw_colored(
            white,
            Vec2::new(sx, sy),
            Vec2::new(50.0, 50.0),
            Color::new(1.0, 0.3, 0.3, 1.0),
        );
        frame.draw_colored(
            white,
            Vec2::new(sx + 60.0, sy),
            Vec2::new(50.0, 50.0),
            Color::new(0.3, 1.0, 0.3, 1.0),
        );
        frame.draw_colored(
            white,
            Vec2::new(sx + 120.0, sy),
            Vec2::new(50.0, 50.0),
            Color::new(0.3, 0.5, 1.0, 1.0),
        );
        frame.draw_colored(
            white,
            Vec2::new(sx + 180.0, sy),
            Vec2::new(50.0, 50.0),
            Color::new(1.0, 0.9, 0.2, 1.0),
        );
        frame.draw_colored(
            white,
            Vec2::new(sx + 240.0, sy),
            Vec2::new(50.0, 50.0),
            Color::new(1.0, 1.0, 1.0, 0.4),
        );

        let sy = 80.0;
        let sz = Vec2::new(50.0, 50.0);
        frame.draw_sprite(DrawParams::new(checker, Vec2::new(sx, sy), sz));
        frame.draw_sprite(
            DrawParams::new(checker, Vec2::new(sx + 60.0, sy), sz)
                .with_uv_rect([0.0, 0.0, 0.5, 0.5]),
        );
        frame.draw_sprite(
            DrawParams::new(checker, Vec2::new(sx + 120.0, sy), sz)
                .with_uv_rect([0.5, 0.0, 0.5, 0.5]),
        );
        frame.draw_sprite(
            DrawParams::new(checker, Vec2::new(sx + 180.0, sy), sz)
                .with_uv_rect([0.0, 0.5, 1.0, 0.5]),
        );

        let sy = 10.0;
        frame.draw_sprite(DrawParams::new(checker, Vec2::new(sx, sy), sz));
        frame.draw_sprite(DrawParams::new(checker, Vec2::new(sx + 60.0, sy), sz).with_flip_x(true));
        frame
            .draw_sprite(DrawParams::new(checker, Vec2::new(sx + 120.0, sy), sz).with_flip_y(true));
        frame.draw_sprite(
            DrawParams::new(checker, Vec2::new(sx + 180.0, sy), sz)
                .with_flip_x(true)
                .with_flip_y(true),
        );

        let angle = self.time * 1.5;

        frame.draw_sprite(
            DrawParams::new(white, Vec2::new(50.0, 220.0), Vec2::new(70.0, 30.0))
                .with_color(Color::new(1.0, 0.5, 0.2, 1.0))
                .with_rotation(angle),
        );

        frame.draw_sprite(
            DrawParams::new(white, Vec2::new(200.0, 220.0), Vec2::new(70.0, 30.0))
                .with_color(Color::new(0.2, 0.7, 1.0, 1.0))
                .with_rotation(angle)
                .with_centered_origin(),
        );

        frame.draw_sprite(
            DrawParams::new(white, Vec2::new(350.0, 220.0), Vec2::new(70.0, 30.0))
                .with_color(Color::new(0.8, 0.2, 1.0, 1.0))
                .with_rotation(angle)
                .with_origin(Vec2::new(70.0, 30.0)),
        );

        for i in 0..12 {
            let a = (i as f32 / 12.0) * std::f32::consts::TAU + self.time * 0.4;
            let cx = 200.0 + a.cos() * 90.0;
            let cy = 80.0 + a.sin() * 90.0;
            let hue = i as f32 / 12.0;
            let r = (hue * std::f32::consts::TAU).cos() * 0.5 + 0.5;
            let g = (hue * std::f32::consts::TAU + 2.094).cos() * 0.5 + 0.5;
            let b = (hue * std::f32::consts::TAU + 4.189).cos() * 0.5 + 0.5;
            frame.draw_sprite(
                DrawParams::new(white, Vec2::new(cx, cy), Vec2::new(16.0, 16.0))
                    .with_color(Color::new(r, g, b, 0.9))
                    .with_rotation(self.time * 3.0 + i as f32)
                    .with_centered_origin(),
            );
        }

        let zx = -380.0;
        let zy = -120.0;
        frame.draw_sprite(
            DrawParams::new(white, Vec2::new(zx + 50.0, zy), Vec2::new(80.0, 80.0))
                .with_color(Color::new(0.2, 0.2, 0.9, 1.0))
                .with_z_order(2),
        );
        frame.draw_sprite(
            DrawParams::new(white, Vec2::new(zx, zy), Vec2::new(80.0, 80.0))
                .with_color(Color::new(0.9, 0.2, 0.2, 1.0))
                .with_z_order(0),
        );
        frame.draw_sprite(
            DrawParams::new(
                white,
                Vec2::new(zx + 25.0, zy + 15.0),
                Vec2::new(80.0, 80.0),
            )
            .with_color(Color::new(0.2, 0.9, 0.2, 1.0))
            .with_z_order(1),
        );

        frame.draw_sprite(
            DrawParams::new(checker, Vec2::new(50.0, -120.0), Vec2::new(100.0, 100.0))
                .with_color(Color::new(1.0, 0.8, 0.6, 0.85))
                .with_rotation(self.time * 0.7)
                .with_centered_origin()
                .with_z_order(10),
        );
    }
}

fn main() {
    rengine::run::<SpriteShowcase>(EngineConfig {
        title: "Feature: Sprites".into(),
        width: 900,
        height: 700,
        show_fps: false,
        ..Default::default()
    })
    .unwrap();
}
