use rengine::*;

struct TweenDemo {
    tweens: Vec<(Tween, &'static str)>,
    elapsed: f32,
    quit: bool,
}

const EASINGS: &[(Easing, &str)] = &[
    (Easing::Linear, "Linear"),
    (Easing::InQuad, "InQuad"),
    (Easing::OutQuad, "OutQuad"),
    (Easing::InOutQuad, "InOutQuad"),
    (Easing::InCubic, "InCubic"),
    (Easing::OutCubic, "OutCubic"),
    (Easing::InOutCubic, "InOutCubic"),
    (Easing::InQuart, "InQuart"),
    (Easing::OutQuart, "OutQuart"),
    (Easing::InOutQuart, "InOutQuart"),
    (Easing::InSine, "InSine"),
    (Easing::OutSine, "OutSine"),
    (Easing::InOutSine, "InOutSine"),
    (Easing::InExpo, "InExpo"),
    (Easing::OutExpo, "OutExpo"),
    (Easing::InOutExpo, "InOutExpo"),
    (Easing::InBack, "InBack"),
    (Easing::OutBack, "OutBack"),
    (Easing::InOutBack, "InOutBack"),
    (Easing::InElastic, "InElastic"),
    (Easing::OutElastic, "OutElastic"),
    (Easing::InOutElastic, "InOutElastic"),
    (Easing::InBounce, "InBounce"),
    (Easing::OutBounce, "OutBounce"),
    (Easing::InOutBounce, "InOutBounce"),
];

const DURATION: f32 = 2.0;
const BAR_WIDTH: f32 = 240.0;

impl Game for TweenDemo {
    fn new(_engine: &mut Engine) -> Self {
        let tweens = EASINGS
            .iter()
            .map(|&(easing, name)| {
                let tw = Tween::new(0.0, BAR_WIDTH, DURATION, easing).looping(LoopMode::PingPong);
                (tw, name)
            })
            .collect();
        Self {
            tweens,
            elapsed: 0.0,
            quit: false,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }
        let dt = engine.dt();
        self.elapsed += dt;
        for (tw, _) in &mut self.tweens {
            tw.update(dt);
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let canvas = frame.canvas(0);

        canvas.rect(
            -hw,
            -hh,
            sw as f32,
            sh as f32,
            Color::from_rgba8(20, 20, 30, 255),
        );
        canvas.text(
            -hw + 20.0,
            hh - 24.0,
            "Tween / Easing Demo",
            22.0,
            Color::WHITE,
        );

        let label_x = -hw + 20.0;
        let bar_x = -hw + 150.0;
        let row_h = 28.0;
        let bar_h = 16.0;
        let top = hh - 60.0;

        let track_color = Color::from_rgba8(40, 40, 55, 255);
        let fill_color = Color::from_rgba8(100, 180, 255, 255);
        let dot_color = Color::from_rgba8(255, 220, 80, 255);

        for (i, (tw, name)) in self.tweens.iter().enumerate() {
            let y = top - i as f32 * row_h;
            let val = tw.value();

            canvas.text(
                label_x,
                y,
                name,
                13.0,
                Color::from_rgba8(180, 180, 180, 255),
            );
            canvas.rect(bar_x, y - 2.0, BAR_WIDTH, bar_h, track_color);
            canvas.rect(bar_x, y - 2.0, val, bar_h, fill_color);
            canvas.rect(bar_x + val - 3.0, y - 4.0, 6.0, bar_h + 4.0, dot_color);
        }

        let bounce_panel_x = hw - 180.0;
        let bounce_panel_y = -120.0;
        let bounce_val = ease(
            0.0,
            118.0,
            (self.elapsed * 1.5 % 2.0) / 2.0,
            Easing::OutBounce,
        );
        canvas.rect(
            bounce_panel_x,
            bounce_panel_y,
            150.0,
            170.0,
            Color::from_rgba8(32, 36, 50, 255),
        );
        canvas.text(
            bounce_panel_x + 12.0,
            bounce_panel_y + 146.0,
            "OutBounce",
            14.0,
            Color::WHITE,
        );
        canvas.rect(
            bounce_panel_x + 18.0,
            bounce_panel_y + 18.0,
            114.0,
            2.0,
            Color::from_rgba8(120, 120, 140, 255),
        );
        canvas.rect(
            bounce_panel_x + 56.0,
            bounce_panel_y + 20.0 + bounce_val,
            20.0,
            20.0,
            Color::from_rgba8(255, 100, 100, 255),
        );
        canvas.text(
            bounce_panel_x + 12.0,
            bounce_panel_y + 126.0,
            "ease(0..118)",
            11.0,
            Color::from_rgba8(180, 180, 180, 255),
        );

        canvas.text(
            -hw + 20.0,
            -hh + 16.0,
            "ESC to quit | tweens ping-pong automatically",
            12.0,
            Color::from_rgba8(100, 100, 100, 255),
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: Tweening / Easing".into(),
        width: 820,
        height: 860,
        show_fps: false,
        ..Default::default()
    };
    run::<TweenDemo>(config).unwrap();
}
