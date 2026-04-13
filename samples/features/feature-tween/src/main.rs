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
    (Easing::InSine, "InSine"),
    (Easing::OutSine, "OutSine"),
    (Easing::InOutSine, "InOutSine"),
    (Easing::InExpo, "InExpo"),
    (Easing::OutExpo, "OutExpo"),
    (Easing::InBack, "InBack"),
    (Easing::OutBack, "OutBack"),
    (Easing::InElastic, "InElastic"),
    (Easing::OutElastic, "OutElastic"),
    (Easing::InBounce, "InBounce"),
    (Easing::OutBounce, "OutBounce"),
];

const DURATION: f32 = 2.0;
const BAR_WIDTH: f32 = 300.0;

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

    fn update(&mut self, engine: &Engine) {
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
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);

        canvas.rect(-hw, -hh, sw as f32, sh as f32, Color::from_rgba8(20, 20, 30, 255), (sw, sh));
        canvas.text(-hw + 20.0, hh - 24.0, "Tween / Easing Demo", 24.0, Color::WHITE, (sw, sh), atlas);

        let label_x = -hw + 20.0;
        let bar_x = -hw + 160.0;
        let row_h = 28.0;
        let bar_h = 16.0;
        let top = hh - 60.0;

        let track_color = Color::from_rgba8(40, 40, 55, 255);
        let fill_color = Color::from_rgba8(100, 180, 255, 255);
        let dot_color = Color::from_rgba8(255, 220, 80, 255);

        for (i, (tw, name)) in self.tweens.iter().enumerate() {
            let y = top - i as f32 * row_h;
            let val = tw.value();

            canvas.text(label_x, y, name, 13.0, Color::from_rgba8(180, 180, 180, 255), (sw, sh), atlas);
            canvas.rect(bar_x, y - 2.0, BAR_WIDTH, bar_h, track_color, (sw, sh));
            canvas.rect(bar_x, y - 2.0, val, bar_h, fill_color, (sw, sh));
            canvas.rect(bar_x + val - 3.0, y - 4.0, 6.0, bar_h + 4.0, dot_color, (sw, sh));
        }

        let bounce_y = top - self.tweens.len() as f32 * row_h - 40.0;
        let bounce_val = ease(0.0, 200.0, (self.elapsed * 1.5 % 2.0) / 2.0, Easing::OutBounce);
        canvas.rect(-hw + 200.0, bounce_y + bounce_val - 10.0, 20.0, 20.0, Color::from_rgba8(255, 100, 100, 255), (sw, sh));
        canvas.text(-hw + 20.0, bounce_y - 10.0, "OutBounce ball:", 13.0, Color::from_rgba8(180, 180, 180, 255), (sw, sh), atlas);

        canvas.text(
            -hw + 20.0, -hh + 16.0,
            "ESC to quit | tweens ping-pong automatically",
            12.0, Color::from_rgba8(100, 100, 100, 255), (sw, sh), atlas,
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: Tweening / Easing".into(),
        width: 700,
        height: 620,
        show_fps: false,
        ..Default::default()
    };
    run::<TweenDemo>(config).unwrap();
}
