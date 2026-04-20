use rengine::*;

const TEAM_NAME_ID: usize = 0;
const RISK_MODE_ID: usize = 1;
const AERO_BALANCE_ID: usize = 2;
const REVIEW_ID: usize = 3;

struct PitWallState {
    team_name: String,
    aggressive_calls: bool,
    aero_balance: f32,
    show_review: bool,
    note: String,
}

struct UiFlowDemo {
    ui: Ui,
    state: PitWallState,
}

impl UiFlowDemo {
    fn build_ui(ui: &mut Ui, state: &PitWallState) {
        ui.style_mut().tooltip_delay = 0.0;
        ui.label_centered("Single-Build UI Flow", 28.0, Color::WHITE);
        ui.separator(8.0);
        ui.label_centered(
            "Ui::sync_with rebuilds the widget tree after input",
            11.0,
            Color::from_rgba8(176, 184, 202, 255),
        );
        ui.label_centered(
            "so toggles, labels, and panels stay in sync on the same frame.",
            11.0,
            Color::from_rgba8(176, 184, 202, 255),
        );
        ui.separator(12.0);

        ui.panel(if state.show_review { 9 } else { 4 });
        ui.text_input(TEAM_NAME_ID, &state.team_name, "ENTER PROFILE NAME");
        ui.tooltip("This field now rebuilds through Ui::sync_with, so the review panel sees the edited value immediately.");
        ui.checkbox(RISK_MODE_ID, "High-risk mode", state.aggressive_calls);
        ui.tooltip("Toggle risk appetite and the review button label updates on the same frame.");
        ui.slider(
            AERO_BALANCE_ID,
            "Primary Balance",
            state.aero_balance,
            38.0,
            62.0,
        );
        ui.tooltip("Slider changes update the review panel immediately instead of waiting until the next frame.");
        ui.button(
            REVIEW_ID,
            if state.show_review {
                "Hide Review Panel"
            } else {
                "Show Review Panel"
            },
        );
        ui.tooltip("This button demonstrates same-frame container visibility changes through the sync helper.");

        if state.show_review {
            ui.panel(4);
            ui.label_centered("Live Review", 18.0, Color::from_rgba8(228, 232, 240, 255));
            ui.progress_bar(
                "Confidence",
                ((state.aero_balance - 38.0) / 24.0).clamp(0.0, 1.0),
            );
            ui.label(
                &format!("Profile: {}", state.team_name),
                14.0,
                Color::from_rgba8(212, 218, 230, 255),
            );
            ui.label(
                if state.aggressive_calls {
                    "Mode: high risk, faster response."
                } else {
                    "Mode: steady, safer response."
                },
                13.0,
                Color::from_rgba8(170, 178, 194, 255),
            );
        }
    }

    fn handle_response(response: UiResponse, state: &mut PitWallState) {
        if let Some(text) = response.text_for(TEAM_NAME_ID) {
            state.team_name = text.to_string();
            state.note = format!(
                "Updated the profile instantly. The review card now reads '{}'.",
                state.team_name
            );
        }
        if response.was_toggled(RISK_MODE_ID) {
            state.aggressive_calls = !state.aggressive_calls;
            state.note = if state.aggressive_calls {
                "High-risk mode switched on and the review copy rebuilt immediately."
            } else {
                "High-risk mode switched off and the review copy rebuilt immediately."
            }
            .into();
        }
        if let Some(value) = response.value_for(AERO_BALANCE_ID) {
            state.aero_balance = value;
            state.note = format!(
                "Primary balance moved to {:.1}. The confidence bar rebuilt on the same frame.",
                state.aero_balance
            );
        }
        if response.was_activated(REVIEW_ID) {
            state.show_review = !state.show_review;
            state.note = if state.show_review {
                "Review panel returned without duplicating the widget tree in game code."
            } else {
                "Review panel hid cleanly while the button label stayed in sync."
            }
            .into();
        }
    }
}

impl Game for UiFlowDemo {
    fn new(_engine: &mut Engine) -> Self {
        Self {
            ui: Ui::default(),
            state: PitWallState {
                team_name: "Studio North".into(),
                aggressive_calls: false,
                aero_balance: 49.5,
                show_review: true,
                note: "Type, toggle, or drag the slider. Ui::sync_with rebuilds the widgets after each response so the panel stays current immediately.".into(),
            },
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.ui.sync_at_with(
            engine,
            -230.0,
            312.0,
            460.0,
            &mut self.state,
            Self::build_ui,
            |response, state| Self::handle_response(response, state),
        );
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(15, 18, 28, 255);
        let (_, hh) = engine.half_size();
        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);
        canvas.text_block(
            0.0,
            -hh + 52.0,
            &self.state.note,
            12.0,
            Color::from_rgba8(190, 198, 214, 255),
            760.0,
            TextAlign::Center,
        );
        canvas.text_block(
            0.0,
            -hh + 24.0,
            "Ui::run handles simple menus. Ui::sync_with is for stateful flows that need the rebuilt tree before render.",
            10.0,
            Color::from_rgba8(132, 142, 160, 255),
            760.0,
            TextAlign::Center,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<UiFlowDemo>(EngineConfig {
        title: "Feature: UI Flow".into(),
        width: 1000,
        height: 720,
        ..Default::default()
    })
}
