use rengine::pixelart::{darken, lighten, PixelCanvas};
use rengine::*;

const CELL_W: u32 = 30;
const CELL_H: u32 = 18;
const COLS: u32 = 4;
const ROWS: u32 = 5;
const DISPLAY_W: f32 = 140.0;
const DISPLAY_H: f32 = 84.0;
const TRACK_WIDTH: f32 = 860.0;
const TRACK_HEIGHT: f32 = 120.0;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum CarState {
    Idle,
    Launch,
    Cruise,
    Brake,
    SpinOut,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum CarEvent {
    Accelerate,
    Brake,
    Crash,
    Recover,
}

struct AnimationStateMachineDemo {
    white: TextureId,
    sheet: SpriteSheet,
    car: AnimationStateMachine<CarState, CarEvent>,
    speed: f32,
    distance: f32,
    last_event: &'static str,
    quit: bool,
}

impl AnimationStateMachineDemo {
    fn build_state_machine() -> AnimationStateMachine<CarState, CarEvent> {
        let mut machine = AnimationStateMachine::new(
            CarState::Idle,
            Animation::new(vec![(0, 0), (1, 0), (0, 0), (2, 0)], 3.0),
        );
        machine.add_state_with(
            CarState::Launch,
            AnimationState::new(Animation::once(vec![(0, 1), (1, 1), (2, 1), (3, 1)], 11.0))
                .with_on_complete(CarState::Cruise),
        );
        machine.add_state(
            CarState::Cruise,
            Animation::new(vec![(0, 2), (1, 2), (2, 2), (3, 2)], 9.0),
        );
        machine.add_state_with(
            CarState::Brake,
            AnimationState::new(Animation::once(vec![(0, 3), (1, 3), (2, 3), (3, 3)], 10.0))
                .with_on_complete(CarState::Idle),
        );
        machine.add_state(
            CarState::SpinOut,
            Animation::new(vec![(0, 4), (1, 4), (2, 4), (3, 4)], 12.0)
                .with_loop_mode(LoopMode::PingPong),
        );
        machine.add_transition(CarState::Idle, CarEvent::Accelerate, CarState::Launch);
        machine.add_transition(CarState::Brake, CarEvent::Accelerate, CarState::Launch);
        machine.add_transition(CarState::Cruise, CarEvent::Brake, CarState::Brake);
        machine.add_transition(CarState::Launch, CarEvent::Brake, CarState::Brake);
        machine.add_transition(CarState::SpinOut, CarEvent::Recover, CarState::Idle);
        machine.add_global_transition(CarEvent::Crash, CarState::SpinOut);
        machine
    }

    fn make_sheet(engine: &mut Engine) -> SpriteSheet {
        let mut canvas = PixelCanvas::new(COLS * CELL_W, ROWS * CELL_H);
        canvas.fill(Color::new(0.0, 0.0, 0.0, 0.0));

        let body = Color::from_rgba8(72, 179, 255, 255);
        let launch_body = lighten(body, 1.08);
        let cruise_body = Color::from_rgba8(86, 209, 152, 255);
        let brake_body = Color::from_rgba8(255, 184, 74, 255);
        let spin_body = Color::from_rgba8(201, 126, 255, 255);

        for col in 0..COLS {
            Self::paint_frame(
                &mut canvas,
                col,
                0,
                body,
                0,
                col == 1 || col == 3,
                false,
                0,
                false,
            );
        }

        let launch_exhaust = [1, 3, 2, 1];
        let launch_shift = [0, 1, 1, 0];
        for col in 0..COLS {
            Self::paint_frame(
                &mut canvas,
                col,
                1,
                launch_body,
                launch_exhaust[col as usize],
                true,
                false,
                launch_shift[col as usize],
                false,
            );
        }

        let cruise_shift = [0, 1, 0, -1];
        for col in 0..COLS {
            Self::paint_frame(
                &mut canvas,
                col,
                2,
                cruise_body,
                0,
                true,
                false,
                cruise_shift[col as usize],
                true,
            );
        }

        let brake_shift = [1, 0, -1, 0];
        for col in 0..COLS {
            Self::paint_frame(
                &mut canvas,
                col,
                3,
                brake_body,
                0,
                false,
                true,
                brake_shift[col as usize],
                false,
            );
        }

        let spin_shift = [-2, -1, 1, 2];
        for col in 0..COLS {
            Self::paint_frame(
                &mut canvas,
                col,
                4,
                spin_body,
                0,
                false,
                false,
                spin_shift[col as usize],
                col % 2 == 0,
            );
        }

        let bytes = canvas.into_bytes();
        let texture = engine.create_texture(COLS * CELL_W, ROWS * CELL_H, &bytes);
        SpriteSheet::new(texture, COLS * CELL_W, ROWS * CELL_H, CELL_W, CELL_H)
    }

    fn paint_frame(
        canvas: &mut PixelCanvas,
        col: u32,
        row: u32,
        body: Color,
        exhaust: i32,
        headlights: bool,
        brake_lights: bool,
        lean: i32,
        speed_lines: bool,
    ) {
        let ox = (col * CELL_W) as i32;
        let oy = (row * CELL_H) as i32;
        let rear_shift = lean / 2;
        let front_shift = lean;
        let shell = body;
        let shell_mid = darken(body, 0.88);
        let shell_dark = darken(body, 0.62);
        let canopy = Color::from_rgba8(28, 34, 48, 255);
        let accent = lighten(body, 1.28);
        let accent_soft = lighten(body, 1.12);
        let stripe = Color::from_rgba8(242, 245, 250, 255);
        let tire = Color::from_rgba8(18, 22, 28, 255);
        let rim = Color::from_rgba8(92, 100, 116, 255);
        let glow = Color::from_rgba8(255, 246, 173, 255);
        let brake = Color::from_rgba8(255, 74, 74, 255);
        let flame = Color::from_rgba8(255, 156, 74, 255);
        let flame_core = Color::from_rgba8(255, 230, 126, 255);
        let shadow = Color::from_rgba8(0, 0, 0, 78);

        if speed_lines {
            canvas.fill_rect(ox, oy + 5, 4, 1, Color::from_rgba8(170, 214, 255, 115));
            canvas.fill_rect(ox + 1, oy + 8, 5, 1, Color::from_rgba8(170, 214, 255, 90));
            canvas.fill_rect(ox + 3, oy + 11, 4, 1, Color::from_rgba8(170, 214, 255, 70));
        }

        canvas.fill_rect(ox + 7 + rear_shift, oy + 4, 18, 10, shadow);
        canvas.fill_rect(ox + 11 + front_shift, oy + 3, 10, 12, shadow);

        if exhaust > 0 {
            canvas.fill_rect(ox + 1 + rear_shift - exhaust, oy + 7, exhaust + 1, 4, flame);
            canvas.fill_rect(
                ox + 2 + rear_shift - exhaust / 2,
                oy + 8,
                exhaust.max(1),
                2,
                flame_core,
            );
        }

        canvas.fill_rect(ox + 2 + rear_shift, oy + 5, 3, 8, shell_dark);
        canvas.fill_rect(ox + 4 + rear_shift, oy + 6, 2, 6, shell_mid);
        canvas.fill_rect(ox + 3 + rear_shift, oy + 7, 1, 4, accent_soft);

        canvas.fill_rect(ox + 5 + rear_shift, oy + 1, 4, 5, tire);
        canvas.fill_rect(ox + 5 + rear_shift, oy + 12, 4, 5, tire);
        canvas.fill_rect(ox + 6 + rear_shift, oy + 2, 2, 3, rim);
        canvas.fill_rect(ox + 6 + rear_shift, oy + 13, 2, 3, rim);

        canvas.fill_rect(ox + 8 + front_shift, oy + 4, 8, 10, shell);
        canvas.fill_rect(ox + 9 + front_shift, oy + 5, 10, 8, shell_mid);
        canvas.fill_rect(ox + 10 + front_shift, oy + 3, 6, 1, accent);
        canvas.fill_rect(ox + 10 + front_shift, oy + 14, 6, 1, accent_soft);
        canvas.fill_rect(ox + 10 + front_shift, oy + 5, 2, 1, shell_dark);
        canvas.fill_rect(ox + 10 + front_shift, oy + 12, 2, 1, shell_dark);

        canvas.fill_rect(ox + 11 + front_shift, oy + 5, 6, 8, accent_soft);
        canvas.fill_rect(ox + 12 + front_shift, oy + 6, 4, 6, canopy);
        canvas.fill_rect(ox + 13 + front_shift, oy + 7, 1, 4, accent);
        canvas.set(ox + 12 + front_shift, oy + 5, accent);
        canvas.set(ox + 16 + front_shift, oy + 5, accent);
        canvas.set(ox + 12 + front_shift, oy + 12, accent_soft);
        canvas.set(ox + 16 + front_shift, oy + 12, accent_soft);

        canvas.fill_rect(ox + 18 + front_shift, oy + 1, 4, 5, tire);
        canvas.fill_rect(ox + 18 + front_shift, oy + 12, 4, 5, tire);
        canvas.fill_rect(ox + 19 + front_shift, oy + 2, 2, 3, rim);
        canvas.fill_rect(ox + 19 + front_shift, oy + 13, 2, 3, rim);

        canvas.fill_rect(ox + 16 + front_shift, oy + 6, 9, 6, shell);
        canvas.fill_rect(ox + 18 + front_shift, oy + 7, 7, 4, accent_soft);
        canvas.fill_rect(ox + 19 + front_shift, oy + 8, 6, 2, stripe);
        canvas.fill_rect(ox + 24 + front_shift, oy + 7, 3, 4, accent);
        canvas.fill_rect(ox + 26 + front_shift, oy + 5, 2, 8, shell_dark);
        canvas.set(ox + 25 + front_shift, oy + 6, accent_soft);
        canvas.set(ox + 25 + front_shift, oy + 11, accent_soft);
        canvas.set(ox + 27 + front_shift, oy + 4, accent_soft);
        canvas.set(ox + 27 + front_shift, oy + 13, accent_soft);
        canvas.set(ox + 27 + front_shift, oy + 7, accent);
        canvas.set(ox + 27 + front_shift, oy + 10, accent);

        if headlights {
            canvas.fill_rect(ox + 26 + front_shift, oy + 6, 1, 1, glow);
            canvas.fill_rect(ox + 26 + front_shift, oy + 11, 1, 1, glow);
        }

        if brake_lights {
            canvas.fill_rect(ox + 2 + rear_shift, oy + 6, 1, 1, brake);
            canvas.fill_rect(ox + 2 + rear_shift, oy + 11, 1, 1, brake);
            canvas.fill_rect(ox + 4 + rear_shift, oy + 6, 1, 1, brake);
            canvas.fill_rect(ox + 4 + rear_shift, oy + 11, 1, 1, brake);
        }
    }

    fn fire(&mut self, event: CarEvent, label: &'static str) {
        self.last_event = if self.car.trigger(event) {
            label
        } else {
            "Input ignored by current state"
        };
    }

    fn state_name(state: CarState) -> &'static str {
        match state {
            CarState::Idle => "Idle",
            CarState::Launch => "Launch",
            CarState::Cruise => "Cruise",
            CarState::Brake => "Brake",
            CarState::SpinOut => "Spin Out",
        }
    }

    fn state_color(state: CarState) -> Color {
        match state {
            CarState::Idle => Color::from_rgba8(146, 171, 204, 255),
            CarState::Launch => Color::from_rgba8(110, 217, 255, 255),
            CarState::Cruise => Color::from_rgba8(106, 230, 172, 255),
            CarState::Brake => Color::from_rgba8(255, 190, 82, 255),
            CarState::SpinOut => Color::from_rgba8(210, 138, 255, 255),
        }
    }
}

impl Game for AnimationStateMachineDemo {
    fn new(engine: &mut Engine) -> Self {
        Self {
            white: engine.create_color_texture(1, 1, Color::WHITE),
            sheet: Self::make_sheet(engine),
            car: Self::build_state_machine(),
            speed: 0.0,
            distance: 0.0,
            last_event: "Up: launch | Down: brake | Space: crash | Enter: recover",
            quit: false,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        let input = engine.input();
        if input.is_key_pressed(KeyCode::Escape) {
            self.quit = true;
            return;
        }

        if input.is_key_pressed(KeyCode::ArrowUp) {
            self.fire(CarEvent::Accelerate, "Accelerate -> transition fired");
        }
        if input.is_key_pressed(KeyCode::ArrowDown) {
            self.fire(CarEvent::Brake, "Brake -> transition fired");
        }
        if input.is_key_pressed(KeyCode::Space) {
            self.fire(CarEvent::Crash, "Crash -> global transition fired");
        }
        if input.is_key_pressed(KeyCode::Enter) {
            self.fire(CarEvent::Recover, "Recover -> transition fired");
        }

        self.car.update(engine.dt());

        let (target_speed, response) = match *self.car.current_state() {
            CarState::Idle => (0.0, 220.0),
            CarState::Launch => (170.0, 340.0),
            CarState::Cruise => (220.0, 180.0),
            CarState::Brake => (0.0, 420.0),
            CarState::SpinOut => (48.0, 200.0),
        };

        let delta = target_speed - self.speed;
        let max_step = response * engine.dt();
        self.speed += delta.clamp(-max_step, max_step);
        self.distance += self.speed * engine.dt();

        let lap_len = TRACK_WIDTH - 180.0;
        if self.distance >= lap_len {
            self.distance -= lap_len;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(13, 16, 24, 255);
        let (hw, hh) = engine.half_size();
        let track_left = -TRACK_WIDTH * 0.5;
        let track_base_y = 72.0;
        let track_center_y = track_base_y + TRACK_HEIGHT * 0.5;
        let car_state = *self.car.current_state();
        let car_x = track_left + 90.0 + self.distance;
        let car_y = track_center_y;
        let car_rotation = match car_state {
            CarState::Launch => -0.05,
            CarState::Brake => 0.06,
            CarState::SpinOut => engine.time().total_time().sin() * 0.65,
            _ => 0.0,
        };

        frame.draw_sprite(
            DrawParams::new(
                self.white,
                Vec2::new(0.0, track_center_y),
                Vec2::new(TRACK_WIDTH, TRACK_HEIGHT),
            )
            .with_centered_origin()
            .with_color(Color::from_rgba8(42, 48, 58, 255))
            .with_z_order(0),
        );
        frame.draw_sprite(
            DrawParams::new(
                self.white,
                Vec2::new(0.0, track_center_y + TRACK_HEIGHT * 0.5 - 4.0),
                Vec2::new(TRACK_WIDTH, 8.0),
            )
            .with_centered_origin()
            .with_color(Color::from_rgba8(76, 82, 96, 255))
            .with_z_order(1),
        );
        frame.draw_sprite(
            DrawParams::new(
                self.white,
                Vec2::new(0.0, track_center_y - TRACK_HEIGHT * 0.5 + 4.0),
                Vec2::new(TRACK_WIDTH, 8.0),
            )
            .with_centered_origin()
            .with_color(Color::from_rgba8(76, 82, 96, 255))
            .with_z_order(1),
        );
        for i in 0..9 {
            let dash_x = track_left + 36.0 + i as f32 * 96.0 + 26.0;
            frame.draw_sprite(
                DrawParams::new(
                    self.white,
                    Vec2::new(dash_x, track_center_y + 2.0),
                    Vec2::new(52.0, 4.0),
                )
                .with_centered_origin()
                .with_color(Color::from_rgba8(230, 233, 238, 255))
                .with_z_order(1),
            );
        }

        frame.draw_sprite(
            DrawParams::new(
                self.sheet.texture,
                Vec2::new(car_x, car_y),
                Vec2::new(DISPLAY_W, DISPLAY_H),
            )
            .with_centered_origin()
            .with_rotation(car_rotation)
            .with_z_order(2)
            .with_uv_rect(self.car.current_uv_rect(&self.sheet)),
        );

        {
            let canvas = frame.canvas(0);
            let stats_x = -hw + 24.0;
            let stats_y = track_base_y + TRACK_HEIGHT;
            let stats_w = 318.0;
            let stats_h = 110.0;

            canvas.rect(
                stats_x,
                stats_y,
                stats_w,
                stats_h,
                Color::from_rgba8(22, 28, 40, 235),
            );
            canvas.text(
                -hw + 28.0,
                hh - 28.0,
                "Animation State Machines",
                26.0,
                Color::WHITE,
            );
            canvas.text(
                -hw + 28.0,
                hh - 56.0,
                "SpriteSheet clips now support Loop, Once, and PingPong playback; state machines layer trigger-driven transitions on top.",
                12.0,
                Color::from_rgba8(178, 188, 208, 255),
            );
            canvas.text(
                stats_x + 16.0,
                stats_y + stats_h - 20.0,
                &format!("State: {}", Self::state_name(car_state)),
                18.0,
                Self::state_color(car_state),
            );
            canvas.text(
                stats_x + 16.0,
                stats_y + stats_h - 44.0,
                &format!("Playback: {:?}", self.car.animation().loop_mode()),
                13.0,
                Color::from_rgba8(188, 199, 216, 255),
            );
            canvas.text(
                stats_x + 16.0,
                stats_y + stats_h - 64.0,
                &format!("Speed: {:>5.1} px/s", self.speed),
                13.0,
                Color::from_rgba8(188, 199, 216, 255),
            );
            let (col, row) = self.car.current_frame();
            canvas.text(
                stats_x + 16.0,
                stats_y + stats_h - 84.0,
                &format!("Frame: ({}, {})", col, row),
                13.0,
                Color::from_rgba8(188, 199, 216, 255),
            );
            canvas.text(
                stats_x + 16.0,
                stats_y + stats_h - 104.0,
                &format!("Last event: {}", self.last_event),
                13.0,
                Color::from_rgba8(140, 214, 255, 255),
            );

            let states = [
                CarState::Idle,
                CarState::Launch,
                CarState::Cruise,
                CarState::Brake,
                CarState::SpinOut,
            ];
            for (index, state) in states.iter().enumerate() {
                let box_x = 190.0 + index as f32 * 110.0;
                let active = *state == car_state;
                let bg = if active {
                    Self::state_color(*state)
                } else {
                    Color::from_rgba8(44, 52, 68, 255)
                };
                let fg = if active {
                    Color::from_rgba8(10, 14, 20, 255)
                } else {
                    Color::from_rgba8(188, 199, 216, 255)
                };
                canvas.rect(box_x, -hh + 74.0, 98.0, 28.0, bg);
                canvas.text(box_x + 10.0, -hh + 94.0, Self::state_name(*state), 12.0, fg);
            }

            canvas.text(
                -hw + 28.0,
                -hh + 54.0,
                "Launch auto-falls through to Cruise. Brake auto-falls through to Idle. Crash is a global transition into Spin Out, which uses PingPong playback until Recover fires.",
                12.0,
                Color::from_rgba8(176, 186, 206, 255),
            );
            canvas.text(
                -hw + 28.0,
                -hh + 28.0,
                "Up: Accelerate | Down: Brake | Space: Crash | Enter: Recover | Esc: Quit",
                12.0,
                Color::from_rgba8(214, 224, 242, 255),
            );
        }
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<AnimationStateMachineDemo>(EngineConfig {
        title: "Feature: Animation State Machines".into(),
        width: 1180,
        height: 720,
        ..Default::default()
    })
}
