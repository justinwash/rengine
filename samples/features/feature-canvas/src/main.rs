use rengine::*;

struct CanvasDemo {
    time: f32,
}

impl Game for CanvasDemo {
    fn new(_engine: &mut Engine) -> Self {
        Self { time: 0.0 }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.time += engine.dt();
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.08, 0.08, 0.12, 1.0);
        let screen = engine.window_size();
        let sw = screen.0 as f32;
        let sh = screen.1 as f32;
        let hw = sw / 2.0;
        let hh = sh / 2.0;

        let hud = frame.canvas(0);

        hud.rect(
            -hw + 20.0,
            hh - 20.0 - 120.0,
            200.0,
            120.0,
            Color::new(0.15, 0.15, 0.22, 0.9),
        );
        hud.text(-hw + 30.0, hh - 30.0, "Canvas Demo", 20.0, Color::WHITE);
        hud.text(
            -hw + 30.0,
            hh - 56.0,
            "Rectangles, text,",
            14.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
        );
        hud.text(
            -hw + 30.0,
            hh - 74.0,
            "and custom shapes",
            14.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
        );

        let colors = [
            Color::RED,
            Color::ORANGE,
            Color::YELLOW,
            Color::GREEN,
            Color::BLUE,
            Color::INDIGO,
            Color::VIOLET,
        ];
        for (i, color) in colors.iter().enumerate() {
            let x = -hw + 260.0 + i as f32 * 50.0;
            hud.rect(x, hh - 30.0 - 40.0, 40.0, 40.0, *color);
        }
        hud.text(
            -hw + 260.0,
            hh - 80.0,
            "Color palette",
            14.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
        );

        let bar_w = 300.0;
        let bar_h = 24.0;
        let bar_x = -hw + 260.0;
        let bar_y = hh - 110.0 - bar_h;
        hud.rect(bar_x, bar_y, bar_w, bar_h, Color::new(0.2, 0.2, 0.2, 1.0));
        let fill = ((self.time * 0.3).sin() * 0.5 + 0.5) * bar_w;
        hud.rect(bar_x, bar_y, fill, bar_h, Color::new(0.3, 0.8, 0.4, 1.0));
        hud.text(
            bar_x + 8.0,
            bar_y + 18.0,
            "Animated bar",
            14.0,
            Color::WHITE,
        );

        let shapes = frame.canvas(1);

        let cx = -0.3 * hw;
        let cy = -0.2 * hh;
        let r = 60.0;
        let segments = 24;
        for i in 0..segments {
            let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            let t = i as f32 / segments as f32;
            let color = Color::new(t, 1.0 - t, 0.5 + t * 0.5, 0.8);
            let c = color.to_array();

            let p0 = screen_to_ndc(cx, cy, screen);
            let p1 = screen_to_ndc(cx + a0.cos() * r, cy + a0.sin() * r, screen);
            let p2 = screen_to_ndc(cx + a1.cos() * r, cy + a1.sin() * r, screen);

            let uv = [0.0, 0.0];
            shapes.shape(&[
                CanvasVertex {
                    position: p0,
                    color: c,
                    uv,
                },
                CanvasVertex {
                    position: p1,
                    color: c,
                    uv,
                },
                CanvasVertex {
                    position: p2,
                    color: c,
                    uv,
                },
            ]);
        }

        let label = frame.canvas(2);
        label.text(
            cx - 70.0,
            cy - r - 10.0,
            "Custom triangle fan",
            14.0,
            Color::WHITE,
        );

        let dx = 0.3 * hw;
        let dy = -0.1 * hh;
        let spin = self.time * 1.5;
        let size = 90.0;
        let corners: [(f32, f32); 4] = [
            (dx + spin.cos() * size, dy + spin.sin() * size),
            (
                dx + (spin + 1.57).cos() * size,
                dy + (spin + 1.57).sin() * size,
            ),
            (
                dx + (spin + 3.14).cos() * size,
                dy + (spin + 3.14).sin() * size,
            ),
            (
                dx + (spin + 4.71).cos() * size,
                dy + (spin + 4.71).sin() * size,
            ),
        ];

        let quad_color = Color::new(0.2, 0.5, 1.0, 0.7).to_array();
        let uv = [0.0, 0.0];
        let verts: Vec<CanvasVertex> = corners
            .iter()
            .map(|&(x, y)| {
                let p = screen_to_ndc(x, y, screen);
                CanvasVertex {
                    position: p,
                    color: quad_color,
                    uv,
                }
            })
            .collect();

        let shapes2 = frame.canvas(1);
        shapes2.shape(&[verts[0], verts[1], verts[2], verts[0], verts[2], verts[3]]);

        let label2 = frame.canvas(2);
        label2.text(
            dx - 60.0,
            dy - size - 20.0,
            "Spinning quad",
            14.0,
            Color::WHITE,
        );

        let stats = frame.canvas(3);
        let fps_text = format!("dt: {:.1}ms", engine.dt() * 1000.0);
        let time_text = format!("time: {:.1}s", self.time);
        stats.rect(
            -hw + 20.0,
            -hh + 10.0,
            160.0,
            50.0,
            Color::new(0.1, 0.1, 0.15, 0.85),
        );
        stats.text(-hw + 30.0, -hh + 46.0, &fps_text, 14.0, Color::YELLOW);
        stats.text(
            -hw + 30.0,
            -hh + 28.0,
            &time_text,
            14.0,
            Color::new(0.6, 0.8, 1.0, 1.0),
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<CanvasDemo>(EngineConfig {
        title: "Feature: Canvas Drawing".into(),
        width: 800,
        height: 600,
        ..Default::default()
    })
}
