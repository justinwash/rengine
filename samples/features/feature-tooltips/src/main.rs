use rengine::*;

struct TooltipDemo {
    ui: Ui,
    badge: TextureId,
    morale: f32,
    tire_wear: f32,
    aggressive_calls: bool,
    fuel_saving: f32,
    note: &'static str,
}

impl TooltipDemo {
    fn create_badge(engine: &mut Engine) -> TextureId {
        let mut icon = pixelart::PixelCanvas::new(36, 36);
        icon.fill(Color::new(0.08, 0.09, 0.14, 0.0));
        let shell = Color::from_rgba8(232, 86, 74, 255);
        let visor = Color::from_rgba8(155, 228, 255, 255);
        let trim = Color::from_rgba8(251, 199, 92, 255);

        for y in 6..30 {
            for x in 6..30 {
                let dx = x as f32 - 17.5;
                let dy = y as f32 - 17.5;
                if dx * dx + dy * dy <= 130.0 {
                    icon.set(x, y, shell);
                }
            }
        }
        for y in 14..21 {
            for x in 12..27 {
                icon.set(x, y, visor);
            }
        }
        for x in 8..29 {
            icon.set(x, 29, trim);
        }

        engine.create_texture(36, 36, &icon.into_bytes())
    }

    fn build_ui(&mut self, engine: &Engine) {
        self.ui.begin(engine, -210.0, 40.0, 420.0);
        self.ui.label_centered("Tooltip Demo", 28.0, Color::WHITE);
        self.ui.tooltip("Text-only widgets can now explain themselves without becoming buttons or sliders.");
        self.ui.separator(10.0);

        self.ui.panel(7);
        self.ui.image(self.badge, Vec2::new(84.0, 84.0));
        self.ui.tooltip("Hovering portraits, cards, or facility icons can now reveal a tooltip even when the widget is not interactive.");
        self.ui.label_centered("Race Weekend Briefing", 18.0, Color::from_rgba8(220, 220, 240, 255));
        self.ui.tooltip("This is the kind of heading that usually needs a little contextual explanation in a management UI.");
        self.ui.progress_bar("Driver Morale", self.morale);
        self.ui.tooltip("Morale is a compact stand-in for any stat bar that benefits from hover-only explanation.");
        self.ui.progress_bar_colored("Tire Wear", self.tire_wear, Color::from_rgba8(236, 174, 72, 255));
        self.ui.tooltip("High tire wear would normally increase pit pressure and reduce late-stint pace.");
        self.ui.checkbox(1, "Aggressive Pit Calls", self.aggressive_calls);
        self.ui.tooltip("Checkbox tooltips also appear for keyboard focus, so gamepad and keyboard flows still surface explanations.");
        self.ui.slider(2, "Fuel Saving", self.fuel_saving, 0.0, 100.0);
        self.ui.tooltip_sized("This slider demonstrates the focused-control fallback: tabbing through the menu with keys still reveals the explanation even when the mouse is idle.", 260.0);
        self.ui.button(3, "Cycle Briefing");
        self.ui.tooltip("Rotate the sample values and footer note so the tooltip demo stays a little more alive.");
    }
}

impl Game for TooltipDemo {
    fn new(engine: &mut Engine) -> Self {
        Self {
            ui: Ui::default(),
            badge: Self::create_badge(engine),
            morale: 0.78,
            tire_wear: 0.34,
            aggressive_calls: false,
            fuel_saving: 42.0,
            note: "Hover widgets with the mouse or move focus with arrow keys.",
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.build_ui(engine);
        let response = self.ui.update(engine);

        if response.was_toggled(1) {
            self.aggressive_calls = !self.aggressive_calls;
        }
        if let Some(value) = response.value_for(2) {
            self.fuel_saving = value;
        }
        if response.was_activated(3) {
            self.morale = if self.morale > 0.7 { 0.56 } else { 0.84 };
            self.tire_wear = if self.tire_wear > 0.5 { 0.27 } else { 0.68 };
            self.note = if self.aggressive_calls {
                "Keyboard focus keeps tooltip text available even without mouse hover."
            } else {
                "Non-interactive widgets like labels and images can now carry hover text."
            };
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(17, 21, 31, 255);
        let (_, hh) = engine.half_size();

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);
        canvas.text_aligned(
            0.0,
            -hh + 34.0,
            self.note,
            12.0,
            Color::from_rgba8(184, 192, 208, 255),
            TextAlign::Center,
        );
        canvas.text_aligned(
            0.0,
            -hh + 16.0,
            "Mouse hover shows tooltip placement; arrow keys show focus-driven tooltips.",
            10.0,
            Color::from_rgba8(132, 142, 160, 255),
            TextAlign::Center,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<TooltipDemo>(EngineConfig {
        title: "Feature: Tooltips".into(),
        width: 960,
        height: 640,
        ..Default::default()
    })
}