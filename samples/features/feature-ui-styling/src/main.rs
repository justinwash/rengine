use rengine::*;

const ACADEMY_ID: usize = 0;
const VETERAN_ID: usize = 1;
const AERO_ID: usize = 2;
const TEAM_NAME_ID: usize = 10;
const UNDERCUT_ID: usize = 11;
const PIT_WINDOW_ID: usize = 12;

#[derive(Clone, Copy)]
struct OfferCard {
    id: usize,
    title: &'static str,
    summary: &'static str,
    action: &'static str,
    tooltip: &'static str,
    sponsor_fit: f32,
    panel_bg: Color,
    panel_text: Color,
    summary_text: Color,
    panel_tooltip_bg: Color,
    button_bg: Color,
    button_focus: Color,
    button_pressed: Color,
    button_text: Color,
    progress_bg: Color,
    progress_fill: Color,
}

struct UiStylingScene {
    ui: Ui,
    team_name: String,
    aggressive_undercut: bool,
    pit_window: f32,
    note: String,
}

impl UiStylingScene {
    fn new() -> Self {
        Self {
            ui: Ui::default().with_style(Self::base_style()),
            team_name: "Aurora GP".into(),
            aggressive_undercut: true,
            pit_window: 2.5,
            note: "Hover the offer cards, then mix and match the styled widgets below.".into(),
        }
    }

    fn base_style() -> UiStyle {
        UiStyle {
            text_color: Color::from_rgba8(225, 229, 235, 255),
            panel_bg: Color::from_rgba8(23, 28, 36, 230),
            panel_padding: 12.0,
            button_bg: Color::from_rgba8(38, 58, 76, 235),
            button_focused_bg: Color::from_rgba8(60, 98, 124, 255),
            button_pressed_bg: Color::from_rgba8(27, 44, 58, 255),
            button_text_color: Color::from_rgba8(235, 241, 246, 255),
            button_focused_text_color: Color::WHITE,
            text_input_bg: Color::from_rgba8(18, 23, 30, 255),
            text_input_focused_bg: Color::from_rgba8(28, 39, 51, 255),
            text_input_text_color: Color::from_rgba8(240, 244, 248, 255),
            text_input_placeholder_color: Color::from_rgba8(120, 133, 148, 255),
            text_input_caret_color: Color::from_rgba8(255, 214, 102, 255),
            checkbox_bg: Color::from_rgba8(32, 42, 53, 255),
            checkbox_checked_bg: Color::from_rgba8(74, 143, 114, 255),
            progress_bg: Color::from_rgba8(16, 20, 27, 255),
            progress_fill: Color::from_rgba8(82, 162, 212, 255),
            slider_track_color: Color::from_rgba8(25, 36, 47, 255),
            slider_fill_color: Color::from_rgba8(245, 180, 63, 255),
            slider_thumb_color: Color::from_rgba8(255, 224, 147, 255),
            tooltip_bg: Color::from_rgba8(13, 16, 22, 244),
            tooltip_text_color: Color::from_rgba8(234, 239, 244, 255),
            spacing: 10.0,
            ..UiStyle::default()
        }
    }

    fn cards() -> [OfferCard; 3] {
        [
            OfferCard {
                id: ACADEMY_ID,
                title: "Academy Prospect",
                summary: "Cheap upside, patient sponsors.",
                action: "Sign Prospect",
                tooltip: "A low-cost long play. This card styles the panel, tooltip, progress bar, and button independently.",
                sponsor_fit: 0.58,
                panel_bg: Color::from_rgba8(35, 60, 86, 240),
                panel_text: Color::from_rgba8(223, 240, 255, 255),
                summary_text: Color::from_rgba8(182, 210, 232, 255),
                panel_tooltip_bg: Color::from_rgba8(18, 35, 53, 245),
                button_bg: Color::from_rgba8(53, 100, 138, 255),
                button_focus: Color::from_rgba8(87, 142, 186, 255),
                button_pressed: Color::from_rgba8(31, 69, 101, 255),
                button_text: Color::from_rgba8(240, 248, 255, 255),
                progress_bg: Color::from_rgba8(22, 38, 53, 255),
                progress_fill: Color::from_rgba8(116, 196, 255, 255),
            },
            OfferCard {
                id: VETERAN_ID,
                title: "Veteran Closer",
                summary: "Late-race pace, expensive wages.",
                action: "Back Veteran",
                tooltip: "A more urgent visual variant with amber fills and a warmer tooltip treatment for high-pressure racecraft.",
                sponsor_fit: 0.77,
                panel_bg: Color::from_rgba8(74, 54, 24, 240),
                panel_text: Color::from_rgba8(255, 238, 205, 255),
                summary_text: Color::from_rgba8(235, 204, 151, 255),
                panel_tooltip_bg: Color::from_rgba8(57, 37, 14, 245),
                button_bg: Color::from_rgba8(138, 93, 33, 255),
                button_focus: Color::from_rgba8(182, 126, 56, 255),
                button_pressed: Color::from_rgba8(104, 68, 18, 255),
                button_text: Color::from_rgba8(255, 244, 225, 255),
                progress_bg: Color::from_rgba8(54, 39, 16, 255),
                progress_fill: Color::from_rgba8(247, 183, 79, 255),
            },
            OfferCard {
                id: AERO_ID,
                title: "Aero Windfall",
                summary: "Big ceiling, volatile spend curve.",
                action: "Fund Upgrade",
                tooltip: "A sharper red-toned variant for risky development bets. Focus, pressed, and fill colors all diverge from the global theme.",
                sponsor_fit: 0.91,
                panel_bg: Color::from_rgba8(82, 28, 40, 240),
                panel_text: Color::from_rgba8(255, 227, 232, 255),
                summary_text: Color::from_rgba8(244, 184, 196, 255),
                panel_tooltip_bg: Color::from_rgba8(54, 14, 25, 245),
                button_bg: Color::from_rgba8(156, 48, 66, 255),
                button_focus: Color::from_rgba8(208, 77, 101, 255),
                button_pressed: Color::from_rgba8(110, 23, 40, 255),
                button_text: Color::from_rgba8(255, 240, 243, 255),
                progress_bg: Color::from_rgba8(59, 16, 28, 255),
                progress_fill: Color::from_rgba8(255, 121, 145, 255),
            },
        ]
    }

    fn build_offer_card(ui: &mut Ui, card: OfferCard) {
        ui.panel(4);
        ui.style_with(
            UiWidgetStyle::new()
                .with_panel(card.panel_bg, 12.0)
                .with_tooltip_colors(card.panel_tooltip_bg, card.panel_text),
        );
        ui.tooltip_with(
            card.tooltip,
            TooltipOptions::new()
                .with_max_width(190.0)
                .with_delay(0.05)
                .with_animation(TooltipAnimation::FadeSlide {
                    duration: 0.12,
                    offset: Vec2::new(0.0, 8.0),
                }),
        );
        ui.label_centered(card.title, 18.0, card.panel_text);
        ui.label_centered(card.summary, 12.0, card.summary_text);
        ui.progress_bar("Sponsor fit", card.sponsor_fit);
        ui.style_with(
            UiWidgetStyle::new()
                .with_progress_colors(card.progress_bg, card.progress_fill)
                .with_text_color(card.panel_text),
        );
        ui.button(card.id, card.action);
        ui.style_with(
            UiWidgetStyle::new()
                .with_button_colors(card.button_bg, card.button_focus, card.button_pressed)
                .with_button_text_colors(card.button_text, Color::WHITE),
        );
    }

    fn build_ui(&mut self, engine: &Engine) {
        self.ui.begin(engine, -300.0, 84.0, 600.0);
        self.ui.label_centered(
            "Widget Styling Variants",
            30.0,
            Color::from_rgba8(247, 249, 251, 255),
        );
        self.ui.label_centered(
            "Mix panel, button, progress, text input, checkbox, slider, and tooltip variants without replacing the whole UI theme.",
            12.0,
            Color::from_rgba8(152, 166, 182, 255),
        );
        self.ui.separator(16.0);

        self.ui.row_spaced(12.0, 3);
        for card in Self::cards() {
            Self::build_offer_card(&mut self.ui, card);
        }

        self.ui.separator(14.0);
        self.ui.panel(4);
        self.ui
            .style_with(UiWidgetStyle::new().with_panel(Color::from_rgba8(17, 24, 31, 236), 14.0));
        self.ui.label(
            "Pit Wall Setup",
            18.0,
            Color::from_rgba8(235, 239, 242, 255),
        );

        self.ui
            .text_input(TEAM_NAME_ID, &self.team_name, "ENTER TEAM NAME");
        self.ui.style_with(
            UiWidgetStyle::new()
                .with_text_input_colors(
                    Color::from_rgba8(20, 33, 45, 255),
                    Color::from_rgba8(29, 52, 73, 255),
                    Color::from_rgba8(239, 247, 252, 255),
                    Color::from_rgba8(132, 151, 169, 255),
                    Color::from_rgba8(118, 213, 255, 255),
                )
                .with_tooltip_colors(
                    Color::from_rgba8(15, 31, 43, 245),
                    Color::from_rgba8(224, 242, 252, 255),
                ),
        );
        self.ui.tooltip(
            "This field only overrides the text-input and tooltip treatment. The rest of the screen still uses the shared base theme.",
        );

        self.ui.checkbox(
            UNDERCUT_ID,
            "Aggressive undercut calls",
            self.aggressive_undercut,
        );
        self.ui.style_with(
            UiWidgetStyle::new()
                .with_checkbox_colors(
                    Color::from_rgba8(52, 37, 17, 255),
                    Color::from_rgba8(209, 137, 42, 255),
                )
                .with_text_color(Color::from_rgba8(252, 231, 196, 255))
                .with_button_colors(
                    Color::from_rgba8(52, 37, 17, 255),
                    Color::from_rgba8(232, 172, 82, 255),
                    Color::from_rgba8(52, 37, 17, 255),
                ),
        );

        self.ui.slider(
            PIT_WINDOW_ID,
            "Preferred pit window",
            self.pit_window,
            1.0,
            5.0,
        );
        self.ui.style_with(
            UiWidgetStyle::new()
                .with_slider_colors(
                    Color::from_rgba8(20, 47, 44, 255),
                    Color::from_rgba8(62, 183, 150, 255),
                    Color::from_rgba8(176, 255, 233, 255),
                )
                .with_text_color(Color::from_rgba8(206, 255, 243, 255))
                .with_button_colors(
                    Color::from_rgba8(20, 47, 44, 255),
                    Color::from_rgba8(82, 220, 183, 255),
                    Color::from_rgba8(20, 47, 44, 255),
                ),
        );
    }

    fn apply_response(&mut self, response: &UiResponse) {
        if response.was_activated(ACADEMY_ID) {
            self.note =
                "Signed an academy prospect: long-term upside with a calm blue variant.".into();
        }
        if response.was_activated(VETERAN_ID) {
            self.note = "Backed the veteran closer: amber styling now reflects a high-pressure race weekend.".into();
        }
        if response.was_activated(AERO_ID) {
            self.note = "Funded the aero upgrade: the riskier red card demonstrates an entirely different widget treatment.".into();
        }
        if response.was_toggled(UNDERCUT_ID) {
            self.aggressive_undercut = !self.aggressive_undercut;
        }
        if let Some(value) = response.value_for(PIT_WINDOW_ID) {
            self.pit_window = value;
        }
        if let Some(text) = response.text_for(TEAM_NAME_ID) {
            self.team_name = text.to_string();
        }
    }
}

impl Scene for UiStylingScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        self.build_ui(engine);
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.build_ui(engine);
        let response = self.ui.update(engine);
        self.apply_response(&response);

        if engine.input().is_key_pressed(KeyCode::Escape) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        let (hw, hh) = engine.half_size();
        frame.clear_color = Color::from_rgba8(11, 15, 21, 255);

        let canvas = frame.canvas(0);
        canvas.rect(
            -hw,
            hh - 84.0,
            hw * 2.0,
            84.0,
            Color::from_rgba8(18, 24, 33, 255),
        );
        canvas.rect(-hw, -hh, hw * 2.0, 54.0, Color::from_rgba8(15, 20, 28, 255));
        self.ui.render(canvas, engine);

        canvas.text_aligned(
            0.0,
            -hh + 34.0,
            &self.note,
            12.0,
            Color::from_rgba8(231, 236, 241, 255),
            TextAlign::Center,
        );
        canvas.text_aligned(
            0.0,
            -hh + 16.0,
            &format!(
                "Team: {} | Pit window: lap {:.1} | Aggressive undercut: {} | ESC: quit",
                self.team_name,
                self.pit_window,
                if self.aggressive_undercut {
                    "on"
                } else {
                    "off"
                }
            ),
            10.0,
            Color::from_rgba8(148, 161, 176, 255),
            TextAlign::Center,
        );
    }
}

fn main() {
    let config = EngineConfig {
        title: "UI Styling Variants".into(),
        width: 720,
        height: 520,
        ..Default::default()
    };
    let _ = rengine::run_with_scenes(config, |_engine, _globals| -> Box<dyn Scene> {
        Box::new(UiStylingScene::new())
    });
}
