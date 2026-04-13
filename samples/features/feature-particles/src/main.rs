use rengine::*;

struct ParticleDemo {
    white_tex: TextureId,
    fire: ParticleEmitter,
    fountain: ParticleEmitter,
    sparkle: ParticleEmitter,
    demo_mode: bool,
    max_frames: Option<u32>,
    frame_count: u32,
}

impl Game for ParticleDemo {
    fn new(engine: &mut Engine) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let demo_mode = args.contains(&"--demo".to_string());
        let max_frames = args
            .iter()
            .position(|a| a == "--frames")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok());

        let white_tex = engine.white_texture();

        let up = -std::f32::consts::FRAC_PI_2;
        let spread = std::f32::consts::FRAC_PI_4;

        let fire = ParticleEmitter::new(
            EmitterConfig::default()
                .with_emit_rate(80.0)
                .with_lifetime((0.3, 0.8))
                .with_speed((30.0, 80.0))
                .with_angle((up - spread, up + spread))
                .with_size_start((4.0, 8.0))
                .with_size_end((1.0, 3.0))
                .with_color_start(Color::new(1.0, 0.6, 0.1, 1.0))
                .with_color_end(Color::new(1.0, 0.0, 0.0, 0.0))
                .with_gravity(Vec2::new(0.0, -30.0))
                .with_emit_shape(EmitShape::Rect(20.0, 2.0))
                .with_max_particles(256),
        );

        let fountain = ParticleEmitter::new(
            EmitterConfig::default()
                .with_emit_rate(50.0)
                .with_lifetime((0.8, 1.6))
                .with_speed((80.0, 140.0))
                .with_angle((up - spread * 0.5, up + spread * 0.5))
                .with_size_start((3.0, 5.0))
                .with_size_end((1.0, 2.0))
                .with_color_start(Color::new(0.3, 0.6, 1.0, 1.0))
                .with_color_end(Color::new(0.1, 0.3, 1.0, 0.0))
                .with_gravity(Vec2::new(0.0, 120.0))
                .with_max_particles(256),
        );

        let sparkle = ParticleEmitter::new(
            EmitterConfig::default()
                .with_emit_rate(0.0)
                .with_burst_count(40)
                .with_lifetime((0.4, 1.2))
                .with_speed((40.0, 160.0))
                .with_angle((0.0, std::f32::consts::TAU))
                .with_spin((-5.0, 5.0))
                .with_size_start((3.0, 7.0))
                .with_size_end((0.0, 1.0))
                .with_color_start(Color::YELLOW)
                .with_color_end(Color::new(1.0, 0.5, 0.0, 0.0))
                .with_damping(2.0)
                .with_looping(false)
                .with_max_particles(128),
        );

        Self {
            white_tex,
            fire,
            fountain,
            sparkle,
            demo_mode,
            max_frames,
            frame_count: 0,
        }
    }

    fn update(&mut self, engine: &Engine) {
        self.frame_count += 1;
        let dt = engine.dt();
        let mut rng = engine.rng();
        let (w, h) = engine.game_size();
        let wf = w as f32;
        let hf = h as f32;

        self.fire.set_position(Vec2::new(wf * 0.2, hf * 0.85));
        self.fountain.set_position(Vec2::new(wf * 0.5, hf * 0.85));

        self.fire.update(dt, &mut rng);
        self.fountain.update(dt, &mut rng);
        self.sparkle.update(dt, &mut rng);

        if self.demo_mode {
            if self.frame_count % 60 == 0 {
                self.sparkle = ParticleEmitter::new(self.sparkle.config().clone());
                self.sparkle.set_position(Vec2::new(wf * 0.8, hf * 0.5));
                self.sparkle.burst(&mut rng);
                self.sparkle.set_active(false);
            }
        } else if engine.input().is_key_pressed(KeyCode::Space) {
            self.sparkle = ParticleEmitter::new(self.sparkle.config().clone());
            self.sparkle.set_position(Vec2::new(wf * 0.8, hf * 0.5));
            self.sparkle.burst(&mut rng);
            self.sparkle.set_active(false);
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::new(0.05, 0.05, 0.1, 1.0);

        self.fire.draw(frame, self.white_tex);
        self.fountain.draw(frame, self.white_tex);
        self.sparkle.draw(frame, self.white_tex);

        let screen = engine.game_size();
        let font = engine.font_atlas();
        let canvas = frame.canvas(0);
        let sx = screen.0 as f32;

        canvas.text(
            sx * 0.2 - 40.0,
            14.0,
            "Fire",
            16.0,
            Color::new(1.0, 0.6, 0.1, 1.0),
            screen,
            &font,
        );
        canvas.text(
            sx * 0.2 - 40.0,
            34.0,
            &format!("alive: {}", self.fire.alive_count()),
            14.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
            screen,
            &font,
        );

        canvas.text(
            sx * 0.5 - 40.0,
            14.0,
            "Fountain",
            16.0,
            Color::new(0.3, 0.6, 1.0, 1.0),
            screen,
            &font,
        );
        canvas.text(
            sx * 0.5 - 40.0,
            34.0,
            &format!("alive: {}", self.fountain.alive_count()),
            14.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
            screen,
            &font,
        );

        canvas.text(
            sx * 0.8 - 40.0,
            14.0,
            "Sparkle (Space)",
            16.0,
            Color::YELLOW,
            screen,
            &font,
        );
        canvas.text(
            sx * 0.8 - 40.0,
            34.0,
            &format!("alive: {}", self.sparkle.alive_count()),
            14.0,
            Color::new(0.7, 0.7, 0.7, 1.0),
            screen,
            &font,
        );
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
    rengine::run::<ParticleDemo>(EngineConfig {
        title: "Particles".into(),
        width: 960,
        height: 720,
        render_width: Some(480),
        render_height: Some(360),
        ..Default::default()
    })
}
