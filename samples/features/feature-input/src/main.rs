use rengine::*;

const SPEED: f32 = 200.0;
const TILE: f32 = 32.0;

struct InputDemo {
    white: TextureId,
    player_x: f32,
    player_y: f32,
    player_color: Color,
    jump_timer: f32,
    shoot_flash: f32,
}

impl Game for InputDemo {
    fn new(engine: &mut Engine) -> Self {
        let actions = engine.actions_mut();

        actions.bind("jump", Binding::Key(KeyCode::Space));
        actions.bind("jump", Binding::GamepadButton(GamepadButton::South));

        actions.bind("shoot", Binding::MouseButton(0));
        actions.bind("shoot", Binding::GamepadButton(GamepadButton::West));

        actions.bind_axis(
            "move_x",
            AxisMapping {
                positive: vec![
                    Binding::Key(KeyCode::KeyD),
                    Binding::Key(KeyCode::ArrowRight),
                ],
                negative: vec![
                    Binding::Key(KeyCode::KeyA),
                    Binding::Key(KeyCode::ArrowLeft),
                ],
                gamepad_axis: Some(GamepadAxis::LeftStickX),
            },
        );

        actions.bind_axis(
            "move_y",
            AxisMapping {
                positive: vec![Binding::Key(KeyCode::KeyW), Binding::Key(KeyCode::ArrowUp)],
                negative: vec![
                    Binding::Key(KeyCode::KeyS),
                    Binding::Key(KeyCode::ArrowDown),
                ],
                gamepad_axis: Some(GamepadAxis::LeftStickY),
            },
        );

        let white = engine.create_color_texture(1, 1, Color::WHITE);

        Self {
            white,
            player_x: 400.0,
            player_y: 300.0,
            player_color: Color::new(0.31, 0.71, 1.0, 1.0),
            jump_timer: 0.0,
            shoot_flash: 0.0,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        let dt = engine.dt();

        let mx = engine.axis("move_x");
        let my = engine.axis("move_y");
        self.player_x += mx * SPEED * dt;
        self.player_y -= my * SPEED * dt;

        let (w, h) = engine.window_size();
        let max_x = (w as f32 - TILE).max(0.0);
        let max_y = (h as f32 - TILE).max(0.0);
        self.player_x = self.player_x.clamp(0.0, max_x);
        self.player_y = self.player_y.clamp(0.0, max_y);

        if engine.action_pressed("jump") {
            self.jump_timer = 0.3;
        }
        if self.jump_timer > 0.0 {
            self.jump_timer -= dt;
        }

        if engine.action_pressed("shoot") {
            self.shoot_flash = 0.15;
        }
        if self.shoot_flash > 0.0 {
            self.shoot_flash -= dt;
        }

        self.player_color = if self.jump_timer > 0.0 {
            Color::new(1.0, 0.86, 0.2, 1.0)
        } else if engine.action_down("shoot") {
            Color::new(1.0, 0.31, 0.31, 1.0)
        } else {
            Color::new(0.31, 0.71, 1.0, 1.0)
        };
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.12, 0.12, 0.15, 1.0);

        let (w, h) = engine.window_size();
        let grid_color = Color::new(0.2, 0.2, 0.24, 1.0);
        let mut x = 0.0;
        while x < w as f32 {
            frame.draw_sprite(
                DrawParams::new(self.white, Vec2::new(x, 0.0), Vec2::new(1.0, h as f32))
                    .with_color(grid_color),
            );
            x += 64.0;
        }
        let mut y = 0.0;
        while y < h as f32 {
            frame.draw_sprite(
                DrawParams::new(self.white, Vec2::new(0.0, y), Vec2::new(w as f32, 1.0))
                    .with_color(grid_color),
            );
            y += 64.0;
        }

        let scale = if self.jump_timer > 0.0 {
            1.0 + (self.jump_timer / 0.3) * 0.4
        } else {
            1.0
        };
        let size = TILE * scale;
        let offset = (size - TILE) * 0.5;
        frame.draw_sprite(
            DrawParams::new(
                self.white,
                Vec2::new(self.player_x - offset, self.player_y - offset),
                Vec2::new(size, size),
            )
            .with_color(self.player_color),
        );

        if self.shoot_flash > 0.0 {
            let flash_a = self.shoot_flash / 0.15 * 0.47;
            frame.draw_sprite(
                DrawParams::new(self.white, Vec2::ZERO, Vec2::new(w as f32, h as f32))
                    .with_color(Color::new(1.0, 0.4, 0.4, flash_a)),
            );
        }

        let mx = engine.axis("move_x");
        let my = engine.axis("move_y");
        let hud_y = h as f32 - 20.0;
        frame.draw_sprite(
            DrawParams::new(self.white, Vec2::new(9.0, hud_y - 1.0), Vec2::new(6.0, 6.0))
                .with_color(Color::new(0.31, 0.31, 0.35, 0.4)),
        );
        if mx.abs() > 0.0 || my.abs() > 0.0 {
            frame.draw_sprite(
                DrawParams::new(
                    self.white,
                    Vec2::new(10.0 + mx * 8.0, hud_y + my * -8.0),
                    Vec2::new(4.0, 4.0),
                )
                .with_color(Color::new(0.47, 1.0, 0.47, 0.78)),
            );
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = EngineConfig {
        title: "Feature: Input Action Mapping".into(),
        width: 800,
        height: 600,
        ..Default::default()
    };
    rengine::run::<InputDemo>(config)
}
