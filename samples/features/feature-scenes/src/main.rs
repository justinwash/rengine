use rengine::*;
use std::collections::HashMap;

struct SwitchCounter(u32);

struct SceneCounters(HashMap<&'static str, u32>);

fn draw_label(frame: &mut Frame, engine: &Engine, y: f32, size: f32, color: Color, text: &str) {
    let screen = engine.window_size();
    let hw = screen.0 as f32 / 2.0;
    let hh = screen.1 as f32 / 2.0;
    let canvas = frame.canvas(0);
    canvas.text(-hw + 20.0, hh - y, text, size, color, screen, engine.font_atlas());
}

struct ColorScene {
    name: &'static str,
    bg: Color,
}

impl ColorScene {
    fn new(name: &'static str, bg: Color) -> Self {
        Self { name, bg }
    }
}

impl Scene for ColorScene {
    fn on_enter(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        if let Some(counter) = globals.get_mut::<SwitchCounter>() {
            counter.0 += 1;
        }

        if let Some(counters) = globals.get_mut::<SceneCounters>() {
            *counters.0.entry(self.name).or_insert(0) += 1;
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp {
        let input = engine.input();

        if input.is_key_pressed(KeyCode::Digit1) {
            return SceneOp::Switch(Box::new(ColorScene::new(
                "Red",
                Color::new(0.8, 0.2, 0.2, 1.0),
            )));
        }
        if input.is_key_pressed(KeyCode::Digit2) {
            return SceneOp::Switch(Box::new(ColorScene::new(
                "Green",
                Color::new(0.2, 0.7, 0.2, 1.0),
            )));
        }
        if input.is_key_pressed(KeyCode::Digit3) {
            return SceneOp::Switch(Box::new(ColorScene::new(
                "Blue",
                Color::new(0.2, 0.3, 0.9, 1.0),
            )));
        }

        if input.is_key_pressed(KeyCode::KeyP) {
            return SceneOp::Push(Box::new(PauseOverlay));
        }

        if input.is_key_pressed(KeyCode::Space) {
            if let Some(counters) = globals.get_mut::<SceneCounters>() {
                *counters.0.entry(self.name).or_insert(0) += 1;
            }
        }

        if input.is_key_pressed(KeyCode::Escape) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        frame.clear_color = self.bg;

        let switches = globals.get::<SwitchCounter>().map_or(0, |c| c.0);
        let scene_count = globals
            .get::<SceneCounters>()
            .and_then(|c| c.0.get(self.name).copied())
            .unwrap_or(0);

        draw_label(
            frame,
            engine,
            10.0,
            14.0,
            Color::WHITE,
            &format!(
                "Scene: {}  |  Visits: {}  |  Total switches: {}",
                self.name, scene_count, switches
            ),
        );
        draw_label(
            frame,
            engine,
            28.0,
            12.0,
            Color::WHITE,
            "[1] Red  [2] Green  [3] Blue  [P] Pause  [Space] +1  [Esc] Quit",
        );
    }

    fn on_pause(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[{}] on_pause", self.name);
    }

    fn on_resume(&mut self, _engine: &mut Engine, _globals: &mut Globals) {
        println!("[{}] on_resume", self.name);
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[{}] on_exit", self.name);
    }
}

struct PauseOverlay;

impl Scene for PauseOverlay {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {
        println!("[Pause] on_enter");
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        if engine.input().is_key_pressed(KeyCode::Escape)
            || engine.input().is_key_pressed(KeyCode::KeyP)
        {
            return SceneOp::Pop;
        }
        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (w, h) = engine.window_size();
        let hw = w as f32 / 2.0;
        let hh = h as f32 / 2.0;
        let canvas = frame.canvas(1);
        canvas.rect(
            -hw,
            -hh,
            w as f32,
            h as f32,
            Color::new(0.0, 0.0, 0.0, 0.6),
            (w, h),
        );
        canvas.text(
            -hw + 20.0,
            hh - 200.0,
            "PAUSED  —  press P or Esc to resume",
            28.0,
            Color::WHITE,
            (w, h),
            engine.font_atlas(),
        );
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[Pause] on_exit");
    }
}

fn main() {
    rengine::run_with_scenes(
        EngineConfig {
            title: "Feature: Scene Switching".into(),
            width: 800,
            height: 600,
            show_fps: false,
            ..Default::default()
        },
        |_engine, globals| {
            globals.set(SwitchCounter(0));
            globals.set(SceneCounters(HashMap::new()));

            Box::new(ColorScene::new("Red", Color::new(0.8, 0.2, 0.2, 1.0)))
        },
    )
    .unwrap();
}
