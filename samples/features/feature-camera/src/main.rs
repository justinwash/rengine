use rengine::*;

const WORLD_SIZE: f32 = 800.0;
const PLAYER_SPEED: f32 = 200.0;
const PLAYER_SIZE: f32 = 24.0;

struct CameraDemo {
    white: TextureId,
    checker: TextureId,
    player_pos: Vec2,
    rotation_mode: bool,
    pending_shake: bool,
}

impl Game for CameraDemo {
    fn new(engine: &mut Engine) -> Self {
        let white = engine.create_color_texture(1, 1, Color::WHITE);

        let grid_size: u32 = 16;
        let mut pixels = Vec::with_capacity((grid_size * grid_size * 4) as usize);
        for row in 0..grid_size {
            for col in 0..grid_size {
                let on_edge = row == 0 || col == 0;
                if on_edge {
                    pixels.extend_from_slice(&[100, 100, 110, 255]);
                } else {
                    pixels.extend_from_slice(&[50, 50, 58, 255]);
                }
            }
        }
        let checker = engine.create_texture(grid_size, grid_size, &pixels);

        Self {
            white,
            checker,
            player_pos: Vec2::ZERO,
            rotation_mode: false,
            pending_shake: false,
        }
    }

    fn update(&mut self, engine: &Engine) {
        let input = engine.input();
        let dt = engine.dt();

        let mut dir = Vec2::ZERO;
        if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if input.is_key_down(KeyCode::KeyS) || input.is_key_down(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if dir != Vec2::ZERO {
            self.player_pos += dir.normalize() * PLAYER_SPEED * dt;
        }

        let half = WORLD_SIZE / 2.0;
        self.player_pos = self
            .player_pos
            .clamp(Vec2::new(-half, -half), Vec2::new(half, half));

        if input.is_key_pressed(KeyCode::KeyR) {
            self.rotation_mode = !self.rotation_mode;
        }

        if input.is_key_pressed(KeyCode::Space) {
            self.pending_shake = true;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.08, 0.08, 0.12, 1.0);

        let cam = &mut frame.camera;
        cam.follow(self.player_pos, 2.0);
        cam.set_dead_zone(Vec2::new(80.0, 50.0));
        cam.bounds = Some(CameraBounds {
            min: Vec2::new(-WORLD_SIZE / 2.0, -WORLD_SIZE / 2.0),
            max: Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0),
        });

        if self.pending_shake {
            self.pending_shake = false;
            cam.shake(25.0, 0.5);
        }

        if self.rotation_mode {
            cam.rotation += engine.dt() * 1.5;
        }

        cam.update(engine.dt());

        let white = self.white;
        let checker = self.checker;

        let half = WORLD_SIZE / 2.0;
        let tile = 64.0;
        let mut y = -half;
        while y < half {
            let mut x = -half;
            while x < half {
                frame.draw_sprite(DrawParams::new(
                    checker,
                    Vec2::new(x, y),
                    Vec2::new(tile, tile),
                ));
                x += tile;
            }
            y += tile;
        }

        let boundary_color = Color::new(0.6, 0.2, 0.2, 1.0);
        let thick = 4.0;
        frame.draw_sprite(
            DrawParams::new(white, Vec2::new(-half, -half), Vec2::new(WORLD_SIZE, thick))
                .with_color(boundary_color),
        );
        frame.draw_sprite(
            DrawParams::new(
                white,
                Vec2::new(-half, half - thick),
                Vec2::new(WORLD_SIZE, thick),
            )
            .with_color(boundary_color),
        );
        frame.draw_sprite(
            DrawParams::new(white, Vec2::new(-half, -half), Vec2::new(thick, WORLD_SIZE))
                .with_color(boundary_color),
        );
        frame.draw_sprite(
            DrawParams::new(
                white,
                Vec2::new(half - thick, -half),
                Vec2::new(thick, WORLD_SIZE),
            )
            .with_color(boundary_color),
        );

        let hs = PLAYER_SIZE / 2.0;
        frame.draw_sprite(
            DrawParams::new(
                white,
                Vec2::new(self.player_pos.x - hs, self.player_pos.y - hs),
                Vec2::new(PLAYER_SIZE, PLAYER_SIZE),
            )
            .with_color(Color::new(0.2, 0.8, 1.0, 1.0))
            .with_z_order(10),
        );

        let (sw, sh) = engine.window_size();
        let canvas = frame.canvas(0);
        canvas.text(
            8.0,
            8.0,
            "WASD:Move  Space:Shake  R:Rotation",
            11.0,
            Color::WHITE,
            (sw, sh),
            engine.font_atlas(),
        );
    }
}

fn main() {
    rengine::run::<CameraDemo>(EngineConfig {
        title: "Feature: Camera".into(),
        width: 800,
        height: 600,
        show_fps: false,
        ..Default::default()
    })
    .unwrap();
}
