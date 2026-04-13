use rengine::*;

struct MenuScene {
    focus: usize,
    message: String,
}

impl Scene for MenuScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let hh = sh as f32 / 2.0;

        let mut ui = Ui::new(-120.0, hh - 80.0, 240.0, (sw, sh), atlas)
            .with_focus(self.focus);

        ui.label_centered("Main Menu", 28.0, Color::WHITE);
        ui.separator(10.0);
        ui.button(0, "Start Game");
        ui.button(1, "Options");
        ui.button(2, "Credits");
        ui.button(3, "Quit");

        let response = ui.update(engine.input());
        if let Some(f) = response.focused {
            self.focus = f;
        }

        if let Some(id) = response.activated {
            match id {
                0 => return SceneOp::Switch(Box::new(GameScene::new())),
                1 => {
                    self.message = "Options not implemented".into();
                    return SceneOp::Push(Box::new(OptionsScene { focus: 0 }));
                }
                2 => self.message = "Credits: rengine UI demo".into(),
                3 => return SceneOp::Quit,
                _ => {}
            }
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(25, 25, 40, 255);

        let canvas = frame.canvas(0);

        let mut ui = Ui::new(-120.0, hh - 80.0, 240.0, (sw, sh), atlas)
            .with_focus(self.focus);

        ui.label_centered("Main Menu", 28.0, Color::WHITE);
        ui.separator(10.0);
        ui.button(0, "Start Game");
        ui.button(1, "Options");
        ui.button(2, "Credits");
        ui.button(3, "Quit");

        ui.render(canvas);

        if !self.message.is_empty() {
            canvas.text_aligned(
                0.0,
                -hh + 30.0,
                &self.message,
                12.0,
                Color::YELLOW,
                TextAlign::Center,
                (sw, sh),
                atlas,
            );
        }

        canvas.text_aligned(
            0.0,
            -hh + 12.0,
            "Arrow keys: navigate | Enter: select",
            10.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
            (sw, sh),
            atlas,
        );
    }
}

struct OptionsScene {
    focus: usize,
}

impl Scene for OptionsScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let hh = sh as f32 / 2.0;

        let style = UiStyle {
            button_bg: Color::from_rgba8(50, 70, 50, 200),
            button_focused_bg: Color::from_rgba8(70, 120, 70, 240),
            button_pressed_bg: Color::from_rgba8(100, 160, 100, 255),
            ..UiStyle::default()
        };

        let mut ui = Ui::new(-100.0, hh - 60.0, 200.0, (sw, sh), atlas)
            .with_style(style)
            .with_focus(self.focus);

        ui.label_centered("Options", 24.0, Color::from_rgba8(150, 220, 150, 255));
        ui.separator(8.0);
        ui.button(0, "Audio");
        ui.button(1, "Video");
        ui.button(2, "Back");

        let response = ui.update(engine.input());
        if let Some(f) = response.focused {
            self.focus = f;
        }

        if let Some(id) = response.activated {
            if id == 2 {
                return SceneOp::Pop;
            }
        }

        if engine.input().is_key_pressed(KeyCode::Escape) {
            return SceneOp::Pop;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();

        let canvas = frame.canvas(1);
        let fw = sw as f32;
        let fh = sh as f32;
        canvas.rect(-fw / 2.0, -hh, fw, fh, Color::new(0.0, 0.0, 0.0, 0.6), (sw, sh));

        let style = UiStyle {
            button_bg: Color::from_rgba8(50, 70, 50, 200),
            button_focused_bg: Color::from_rgba8(70, 120, 70, 240),
            button_pressed_bg: Color::from_rgba8(100, 160, 100, 255),
            ..UiStyle::default()
        };

        let mut ui = Ui::new(-100.0, hh - 60.0, 200.0, (sw, sh), atlas)
            .with_style(style)
            .with_focus(self.focus);

        ui.label_centered("Options", 24.0, Color::from_rgba8(150, 220, 150, 255));
        ui.separator(8.0);
        ui.button(0, "Audio");
        ui.button(1, "Video");
        ui.button(2, "Back");

        ui.render(canvas);

        canvas.text_aligned(
            0.0,
            -hh + 12.0,
            "ESC: back",
            10.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
            (sw, sh),
            atlas,
        );
    }
}

struct GameScene {
    time: f32,
}

impl GameScene {
    fn new() -> Self {
        Self { time: 0.0 }
    }
}

impl Scene for GameScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        self.time += engine.dt();
        if engine.input().is_key_pressed(KeyCode::Escape) {
            return SceneOp::Switch(Box::new(MenuScene {
                focus: 0,
                message: String::new(),
            }));
        }
        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(20, 30, 50, 255);

        let canvas = frame.canvas(0);
        canvas.text_aligned(
            0.0,
            50.0,
            "Game Running",
            24.0,
            Color::WHITE,
            TextAlign::Center,
            (sw, sh),
            atlas,
        );
        canvas.text_aligned(
            0.0,
            20.0,
            &format!("Time: {:.1}s", self.time),
            16.0,
            Color::from_rgba8(180, 180, 180, 255),
            TextAlign::Center,
            (sw, sh),
            atlas,
        );
        canvas.text_aligned(
            0.0,
            -20.0,
            "ESC: back to menu",
            12.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
            (sw, sh),
            atlas,
        );
    }
}

fn main() {
    let config = EngineConfig {
        title: "UI Widget Demo".into(),
        width: 500,
        height: 400,
        ..Default::default()
    };
    let _ = rengine::run_with_scenes(config, |_engine, _globals| -> Box<dyn Scene> {
        Box::new(MenuScene {
            focus: 0,
            message: String::new(),
        })
    });
}
