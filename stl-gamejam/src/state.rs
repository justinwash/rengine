use rengine::*;

pub fn menu_footer_hint() -> &'static str {
    "Mouse, arrows, or W/S navigate | Enter or Space selects | Esc backs out"
}

pub fn options_footer_hint() -> &'static str {
    "Enter/Space toggles | Back: Esc/Backspace/B | Quit: Q/Select"
}

pub fn scene_footer_hint() -> &'static str {
    "Continue: Enter/Space/A | Back: Esc/Backspace/B | Quit: Q/Select"
}

pub fn back_quit_hint() -> &'static str {
    "Back: Esc/Backspace/B | Quit: Q/Select"
}

pub fn confirm_pressed(engine: &Engine) -> bool {
    engine.input().is_key_pressed(KeyCode::Enter)
        || engine.input().is_key_pressed(KeyCode::Space)
        || engine.gamepad(0).is_button_pressed(GamepadButton::South)
}

pub fn back_pressed(engine: &Engine) -> bool {
    engine.input().is_key_pressed(KeyCode::Escape)
        || engine.input().is_key_pressed(KeyCode::Backspace)
        || engine.gamepad(0).is_button_pressed(GamepadButton::East)
}

pub fn quit_pressed(engine: &Engine) -> bool {
    engine.input().is_key_pressed(KeyCode::KeyQ)
        || engine.gamepad(0).is_button_pressed(GamepadButton::Select)
}

pub fn sync_gamepad_pairing(engine: &mut Engine) {
    engine.set_gamepad_assign_mode(GamepadAssignMode::OnConnect);
}

#[derive(Debug, Clone, Copy)]
pub struct JamOptions {
    pub show_route_overlay: bool,
    pub show_floor_grid: bool,
    pub show_footer_hints: bool,
    pub show_title_atmosphere: bool,
}

impl Default for JamOptions {
    fn default() -> Self {
        Self {
            show_route_overlay: true,
            show_floor_grid: true,
            show_footer_hints: true,
            show_title_atmosphere: true,
        }
    }
}

#[derive(Debug, Default)]
pub struct SessionState {
    pub main_scene_visits: u32,
    pub completed_runs: u32,
    pub last_stop: String,
    pub last_conversion: f32,
    pub last_contractors_stopped: u32,
    pub options: JamOptions,
}
