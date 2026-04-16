use rengine::*;

struct MenuScene {
    focus: usize,
    message: String,
}

impl MenuScene {
    fn build_menu(ui: &mut Ui) {
        ui.label_centered("Main Menu", 28.0, Color::WHITE);
        ui.separator(10.0);
        ui.button(0, "Start Game");
        ui.button(1, "Options");
        ui.button(2, "Widget Demo");
        ui.button(3, "Quit");
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let hh = sh as f32 / 2.0;

        let mut ui = Ui::new(-120.0, hh - 80.0, 240.0, (sw, sh)).with_focus(self.focus);
        Self::build_menu(&mut ui);

        let response = ui.update(engine.input(), atlas);
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
                2 => {
                    return SceneOp::Push(Box::new(DemoScene::new()));
                }
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

        let mut ui = Ui::new(-120.0, hh - 80.0, 240.0, (sw, sh)).with_focus(self.focus);
        Self::build_menu(&mut ui);

        ui.render(canvas, atlas);

        if !self.message.is_empty() {
            canvas.text_aligned(
                0.0,
                -hh + 30.0,
                &self.message,
                12.0,
                Color::YELLOW,
                TextAlign::Center,
                atlas,
            );
        }

        canvas.text_aligned(
            0.0,
            -hh + 12.0,
            "Mouse or arrow keys: navigate | Click or Enter: select",
            10.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
            atlas,
        );
    }
}

struct OptionsScene {
    focus: usize,
}

impl OptionsScene {
    fn options_style() -> UiStyle {
        UiStyle {
            button_bg: Color::from_rgba8(50, 70, 50, 200),
            button_focused_bg: Color::from_rgba8(70, 120, 70, 240),
            button_pressed_bg: Color::from_rgba8(100, 160, 100, 255),
            ..UiStyle::default()
        }
    }

    fn build_options(ui: &mut Ui) {
        ui.label_centered("Options", 24.0, Color::from_rgba8(150, 220, 150, 255));
        ui.separator(8.0);
        ui.button(0, "Audio");
        ui.button(1, "Video");
        ui.button(2, "Back");
    }
}

impl Scene for OptionsScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let hh = sh as f32 / 2.0;

        let mut ui = Ui::new(-100.0, hh - 60.0, 200.0, (sw, sh))
            .with_style(Self::options_style())
            .with_focus(self.focus);
        Self::build_options(&mut ui);

        let response = ui.update(engine.input(), atlas);
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
        canvas.rect(
            -fw / 2.0,
            -hh,
            fw,
            fh,
            Color::new(0.0, 0.0, 0.0, 0.6),
        );

        let mut ui = Ui::new(-100.0, hh - 60.0, 200.0, (sw, sh))
            .with_style(Self::options_style())
            .with_focus(self.focus);
        Self::build_options(&mut ui);

        ui.render(canvas, atlas);

        canvas.text_aligned(
            0.0,
            -hh + 12.0,
            "ESC: back",
            10.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
            atlas,
        );
    }
}

struct DemoScene {
    focus: usize,
    dragging: Option<usize>,
    speed: f32,
    volume: f32,
    fullscreen: bool,
    vsync: bool,
    health: f32,
    fuel: f32,
}

impl DemoScene {
    fn new() -> Self {
        Self {
            focus: 0,
            dragging: None,
            speed: 1.5,
            volume: 0.75,
            fullscreen: false,
            vsync: true,
            health: 0.82,
            fuel: 0.45,
        }
    }

    fn build_widgets(&self, ui: &mut Ui) {
        ui.label_centered("Widget Demo", 24.0, Color::WHITE);
        ui.separator(8.0);

        ui.panel(8);
        ui.label("Stats", 18.0, Color::from_rgba8(180, 200, 255, 255));
        ui.separator(4.0);
        ui.progress_bar("Health", self.health);
        ui.progress_bar_colored("Fuel", self.fuel, Color::from_rgba8(220, 160, 40, 255));
        ui.separator(4.0);
        ui.checkbox(10, "Fullscreen", self.fullscreen);
        ui.checkbox(11, "VSync", self.vsync);
        ui.slider(20, "Game Speed", self.speed, 0.5, 3.0);
        ui.slider(21, "Volume", self.volume, 0.0, 1.0);

        ui.separator(8.0);
        ui.button(99, "Back");
    }
}

impl Scene for DemoScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let hh = sh as f32 / 2.0;

        let mut ui = Ui::new(-180.0, hh - 40.0, 360.0, (sw, sh))
            .with_focus(self.focus)
            .with_dragging(self.dragging);
        self.build_widgets(&mut ui);

        let response = ui.update(engine.input(), atlas);
        if let Some(f) = response.focused {
            self.focus = f;
        }
        self.dragging = response.dragging;

        if response.was_toggled(10) {
            self.fullscreen = !self.fullscreen;
        }
        if response.was_toggled(11) {
            self.vsync = !self.vsync;
        }
        if let Some(v) = response.value_for(20) {
            self.speed = v;
        }
        if let Some(v) = response.value_for(21) {
            self.volume = v;
        }

        if response.was_activated(99) || engine.input().is_key_pressed(KeyCode::Escape) {
            return SceneOp::Pop;
        }

        self.health = (self.health - engine.dt() * 0.02).max(0.0);
        self.fuel = (self.fuel - engine.dt() * 0.01).max(0.0);

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(20, 20, 35, 255);

        let canvas = frame.canvas(0);

        let mut ui = Ui::new(-180.0, hh - 40.0, 360.0, (sw, sh))
            .with_focus(self.focus)
            .with_dragging(self.dragging);
        self.build_widgets(&mut ui);

        ui.render(canvas, atlas);

        let mouse = engine.mouse_screen_pos();
        canvas.text_aligned(
            0.0,
            -hh + 30.0,
            &format!("Mouse: ({:.0}, {:.0})", mouse.x, mouse.y),
            10.0,
            Color::from_rgba8(180, 180, 180, 255),
            TextAlign::Center,
            atlas,
        );

        canvas.text_aligned(
            0.0,
            -hh + 12.0,
            "Mouse or arrows: navigate | Click/Enter: interact | Left/Right: adjust sliders",
            10.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
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

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
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
            atlas,
        );
        canvas.text_aligned(
            0.0,
            20.0,
            &format!("Time: {:.1}s", self.time),
            16.0,
            Color::from_rgba8(180, 180, 180, 255),
            TextAlign::Center,
            atlas,
        );
        canvas.text_aligned(
            0.0,
            -20.0,
            "ESC: back to menu",
            12.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
            atlas,
        );
    }
}

fn main() {
    let config = EngineConfig {
        title: "UI Widget Demo".into(),
        width: 500,
        height: 500,
        ..Default::default()
    };
    let _ = rengine::run_with_scenes(config, |_engine, _globals| -> Box<dyn Scene> {
        Box::new(MenuScene {
            focus: 0,
            message: String::new(),
        })
    });
}
