use rengine::*;

struct MenuScene {
    ui: Ui,
    message: String,
}

impl MenuScene {
    fn build_menu(ui: &mut Ui) {
        ui.label_centered("Main Menu", 28.0, Color::WHITE);
        ui.separator(10.0);
        ui.button(0, "Start Game");
        ui.button(1, "Options");
        ui.button(2, "Widget Demo");
        ui.button(3, "Layout Demo");
        ui.button(5, "Scroll Demo");
        ui.button(4, "Quit");
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        self.ui.begin(engine, -120.0, 80.0, 240.0);
        Self::build_menu(&mut self.ui);
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.ui.begin(engine, -120.0, 80.0, 240.0);
        Self::build_menu(&mut self.ui);
        let response = self.ui.update(engine);

        if let Some(id) = response.activated {
            match id {
                0 => return SceneOp::Switch(Box::new(GameScene::new())),
                1 => {
                    self.message = "Options not implemented".into();
                    return SceneOp::Push(Box::new(OptionsScene { ui: Ui::default() }));
                }
                2 => return SceneOp::Push(Box::new(DemoScene::new())),
                3 => return SceneOp::Push(Box::new(LayoutScene::new())),
                5 => return SceneOp::Push(Box::new(ScrollScene::new())),
                4 => return SceneOp::Quit,
                _ => {}
            }
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (_, hh) = engine.half_size();
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(25, 25, 40, 255);

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);

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
    ui: Ui,
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
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        self.ui = Ui::default().with_style(Self::options_style());
        self.ui.begin(engine, -100.0, 60.0, 200.0);
        Self::build_options(&mut self.ui);
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.ui.begin(engine, -100.0, 60.0, 200.0);
        Self::build_options(&mut self.ui);
        let response = self.ui.update(engine);

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
        let (hw, hh) = engine.half_size();
        let atlas = engine.font_atlas();

        let canvas = frame.canvas(1);
        canvas.rect(-hw, -hh, hw * 2.0, hh * 2.0, Color::new(0.0, 0.0, 0.0, 0.6));

        self.ui.render(canvas, engine);

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
    ui: Ui,
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
            ui: Ui::default(),
            speed: 1.5,
            volume: 0.75,
            fullscreen: false,
            vsync: true,
            health: 0.82,
            fuel: 0.45,
        }
    }

    fn build_widgets(
        ui: &mut Ui,
        health: f32,
        fuel: f32,
        fullscreen: bool,
        vsync: bool,
        speed: f32,
        volume: f32,
    ) {
        ui.label_centered("Widget Demo", 24.0, Color::WHITE);
        ui.separator(8.0);

        ui.panel(8);
        ui.label("Stats", 18.0, Color::from_rgba8(180, 200, 255, 255));
        ui.separator(4.0);
        ui.progress_bar("Health", health);
        ui.progress_bar_colored("Fuel", fuel, Color::from_rgba8(220, 160, 40, 255));
        ui.separator(4.0);
        ui.checkbox(10, "Fullscreen", fullscreen);
        ui.checkbox(11, "VSync", vsync);
        ui.slider(20, "Game Speed", speed, 0.5, 3.0);
        ui.slider(21, "Volume", volume, 0.0, 1.0);

        ui.separator(8.0);
        ui.button(99, "Back");
    }
}

impl Scene for DemoScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        self.ui.begin(engine, -180.0, 40.0, 360.0);
        Self::build_widgets(
            &mut self.ui,
            self.health,
            self.fuel,
            self.fullscreen,
            self.vsync,
            self.speed,
            self.volume,
        );
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.ui.begin(engine, -180.0, 40.0, 360.0);
        Self::build_widgets(
            &mut self.ui,
            self.health,
            self.fuel,
            self.fullscreen,
            self.vsync,
            self.speed,
            self.volume,
        );
        let response = self.ui.update(engine);

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
        let (_, hh) = engine.half_size();
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(20, 20, 35, 255);

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);

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

struct LayoutScene {
    ui: Ui,
}

impl LayoutScene {
    fn new() -> Self {
        Self { ui: Ui::default() }
    }

    fn build_widgets(ui: &mut Ui) {
        ui.label_centered("Layout Demo", 24.0, Color::WHITE);
        ui.separator(8.0);

        ui.label(
            "Row (2 buttons):",
            14.0,
            Color::from_rgba8(180, 200, 255, 255),
        );
        ui.row(2);
        ui.button(0, "Left");
        ui.button(1, "Right");

        ui.separator(6.0);

        ui.label(
            "Row spaced (3 buttons):",
            14.0,
            Color::from_rgba8(180, 200, 255, 255),
        );
        ui.row_spaced(8.0, 3);
        ui.button(2, "A");
        ui.button(3, "B");
        ui.button(4, "C");

        ui.separator(6.0);

        ui.label("Grid 2x2:", 14.0, Color::from_rgba8(180, 200, 255, 255));
        ui.grid_spaced(2, 4.0, 4);
        ui.button(5, "One");
        ui.button(6, "Two");
        ui.button(7, "Three");
        ui.button(8, "Four");

        ui.separator(6.0);

        ui.label(
            "Grid 3-col (5 items):",
            14.0,
            Color::from_rgba8(180, 200, 255, 255),
        );
        ui.grid_spaced(3, 4.0, 5);
        ui.button(9, "1");
        ui.button(10, "2");
        ui.button(11, "3");
        ui.button(12, "4");
        ui.button(13, "5");

        ui.separator(10.0);
        ui.button(99, "Back");
    }
}

impl Scene for LayoutScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        self.ui.begin(engine, -200.0, 30.0, 400.0);
        Self::build_widgets(&mut self.ui);
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.ui.begin(engine, -200.0, 30.0, 400.0);
        Self::build_widgets(&mut self.ui);
        let response = self.ui.update(engine);

        if response.was_activated(99) || engine.input().is_key_pressed(KeyCode::Escape) {
            return SceneOp::Pop;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (_, hh) = engine.half_size();
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(25, 20, 35, 255);

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);

        canvas.text_aligned(
            0.0,
            -hh + 12.0,
            "ESC: back | Arrows: navigate | Enter: select",
            10.0,
            Color::from_rgba8(140, 140, 140, 255),
            TextAlign::Center,
            atlas,
        );
    }
}

struct ScrollScene {
    ui: Ui,
    scroll_offset: f32,
}

impl ScrollScene {
    fn new() -> Self {
        Self {
            ui: Ui::default(),
            scroll_offset: 0.0,
        }
    }

    fn build_widgets(ui: &mut Ui, scroll_offset: f32) {
        ui.label_centered("Scroll Demo", 24.0, Color::WHITE);
        ui.separator(8.0);

        ui.label(
            "Scrollable list:",
            14.0,
            Color::from_rgba8(180, 200, 255, 255),
        );
        ui.scroll(100, 150.0, scroll_offset, 10);
        ui.button(10, "Item 1");
        ui.button(11, "Item 2");
        ui.button(12, "Item 3");
        ui.button(13, "Item 4");
        ui.button(14, "Item 5");
        ui.button(15, "Item 6");
        ui.button(16, "Item 7");
        ui.button(17, "Item 8");
        ui.button(18, "Item 9");
        ui.button(19, "Item 10");

        ui.separator(10.0);
        ui.button(99, "Back");
    }
}

impl Scene for ScrollScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        self.ui.begin(engine, -200.0, 30.0, 400.0);
        Self::build_widgets(&mut self.ui, self.scroll_offset);
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.ui.begin(engine, -200.0, 30.0, 400.0);
        Self::build_widgets(&mut self.ui, self.scroll_offset);
        let response = self.ui.update(engine);

        if let Some(offset) = response.scroll_for(100) {
            self.scroll_offset = offset;
        }

        if response.was_activated(99) || engine.input().is_key_pressed(KeyCode::Escape) {
            return SceneOp::Pop;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (_, hh) = engine.half_size();
        let atlas = engine.font_atlas();
        frame.clear_color = Color::from_rgba8(25, 20, 35, 255);

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);

        canvas.text_aligned(
            0.0,
            -hh + 12.0,
            "ESC: back | Mouse wheel: scroll | Arrows: navigate",
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
                ui: Ui::default(),
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
            ui: Ui::default(),
            message: String::new(),
        })
    });
}
