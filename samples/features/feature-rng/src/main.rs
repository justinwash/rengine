use rengine::*;

struct RngDemo {
    seed: u64,
    results: Vec<String>,
    quit: bool,
}

impl Game for RngDemo {
    fn new(engine: &mut Engine) -> Self {
        let seed = 42;
        let mut results = Vec::new();

        let mut rng_a = Rng::new(seed);
        let mut rng_b = Rng::new(seed);
        let a_vals: Vec<u64> = (0..5).map(|_| rng_a.next_u64()).collect();
        let b_vals: Vec<u64> = (0..5).map(|_| rng_b.next_u64()).collect();
        assert_eq!(a_vals, b_vals, "Same seed must produce identical sequences");
        results.push(format!("Deterministic: seed={seed}, first 5 match: OK"));

        let mut rng = Rng::new(seed);
        let dice: Vec<i32> = (0..10).map(|_| rng.range(1, 6)).collect();
        results.push(format!("10 dice rolls (1-6): {:?}", dice));

        let floats: Vec<String> = (0..5).map(|_| format!("{:.3}", rng.f32())).collect();
        results.push(format!("5 f32 [0,1): [{}]", floats.join(", ")));

        let coins: Vec<&str> = (0..10).map(|_| if rng.chance(0.5) { "H" } else { "T" }).collect();
        results.push(format!("10 coin flips: {}", coins.join("")));

        let options = ["Common", "Uncommon", "Rare", "Legendary"];
        let weights = [60.0, 25.0, 12.0, 3.0];
        let mut counts = [0u32; 4];
        for _ in 0..1000 {
            counts[rng.weighted(&weights)] += 1;
        }
        results.push(format!(
            "1000 weighted draws: {}={}, {}={}, {}={}, {}={}",
            options[0], counts[0], options[1], counts[1], options[2], counts[2], options[3], counts[3]
        ));

        let mut deck: Vec<i32> = (1..=10).collect();
        rng.shuffle(&mut deck);
        results.push(format!("Shuffled 1-10: {:?}", deck));

        let names = ["Moss", "Clark", "Senna", "Prost", "Fangio"];
        let picked = rng.pick(&names);
        results.push(format!("Random pick from legends: {}", picked));

        let indices = rng.sample_indices(10, 3);
        results.push(format!("3 of 10 indices: {:?}", indices));

        let normals: Vec<String> = (0..8)
            .map(|_| format!("{:.1}", rng.normal(100.0, 15.0)))
            .collect();
        results.push(format!("Normal(100,15): [{}]", normals.join(", ")));

        let point = rng.in_circle(50.0);
        let dir = rng.direction();
        results.push(format!(
            "Random in circle(50): ({:.1}, {:.1}), direction: ({:.3}, {:.3})",
            point.x, point.y, dir.x, dir.y
        ));

        let mut child = rng.fork();
        let parent_val = rng.next_u64();
        let child_val = child.next_u64();
        results.push(format!(
            "Forked RNG: parent={parent_val}, child={child_val} (independent: {})",
            parent_val != child_val
        ));

        let engine_roll = engine.rng().range(1, 100);
        results.push(format!("engine.rng().range(1,100) = {engine_roll}"));

        Self { seed, results, quit: false }
    }

    fn update(&mut self, engine: &Engine) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);

        canvas.rect(
            -hw, -hh,
            sw as f32, sh as f32,
            Color::from_rgba8(20, 20, 30, 255),
            (sw, sh),
        );

        canvas.text(-hw + 20.0, hh - 20.0 - 28.0, "Rng Feature Demo", 28.0, Color::WHITE, (sw, sh), atlas);
        canvas.text(
            -hw + 20.0, hh - 52.0 - 18.0,
            &format!("Seed: {}", self.seed),
            18.0,
            Color::from_rgba8(180, 180, 180, 255),
            (sw, sh), atlas,
        );

        let mut y = hh - 90.0 - 16.0;
        for line in &self.results {
            canvas.text(-hw + 20.0, y, line, 16.0, Color::from_rgba8(200, 220, 255, 255), (sw, sh), atlas);
            y -= 22.0;
        }

        canvas.text(-hw + 20.0, y - 20.0 - 14.0, "ESC to quit", 14.0, Color::from_rgba8(120, 120, 120, 255), (sw, sh), atlas);
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: RNG".into(),
        width: 900,
        height: 520,
        show_fps: false,
        ..Default::default()
    };
    let _ = run::<RngDemo>(config);
}
