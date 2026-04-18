use rengine::*;

const TEAM_NAME_ID: usize = 0;
const SPACE_ID: usize = 200;
const BACKSPACE_ID: usize = 201;
const CLEAR_ID: usize = 202;
const ACCEPT_ID: usize = 203;
const MAX_TEAM_NAME_CHARS: usize = 18;

const ROW_ONE: [(&str, usize); 10] = [
    ("Q", 100),
    ("W", 101),
    ("E", 102),
    ("R", 103),
    ("T", 104),
    ("Y", 105),
    ("U", 106),
    ("I", 107),
    ("O", 108),
    ("P", 109),
];
const ROW_TWO: [(&str, usize); 9] = [
    ("A", 110),
    ("S", 111),
    ("D", 112),
    ("F", 113),
    ("G", 114),
    ("H", 115),
    ("J", 116),
    ("K", 117),
    ("L", 118),
];
const ROW_THREE: [(&str, usize); 7] = [
    ("Z", 119),
    ("X", 120),
    ("C", 121),
    ("V", 122),
    ("B", 123),
    ("N", 124),
    ("M", 125),
];
const FOCUSABLE_COUNT: usize = 31;

struct TextInputDemo {
    ui: Ui,
    team_name: String,
    saved_name: String,
    focus_index: usize,
    quit: bool,
}

impl TextInputDemo {
    fn build_ui(&mut self, engine: &Engine) {
        self.ui.set_focus(self.focus_index.min(FOCUSABLE_COUNT - 1));
        self.ui.style_mut().tooltip_delay = 0.0;

        self.ui.begin(engine, -420.0, 32.0, 840.0);
        self.ui
            .label_centered("Text Input Widget", 28.0, Color::WHITE);
        self.ui.label_centered(
            "Keyboard and IME text are engine-level. The on-screen keyboard below is pure game/sample code built from Ui buttons.",
            12.0,
            Color::from_rgba8(176, 186, 210, 255),
        );
        self.ui.separator(12.0);

        self.ui
            .text_input(TEAM_NAME_ID, &self.team_name, "ENTER TEAM NAME");
        self.ui.tooltip(
            "This field consumes committed keyboard text and IME commits from the engine input layer.",
        );
        self.ui.separator(8.0);
        self.ui.label_centered(
            &format!(
                "Garage Banner: {}",
                if self.team_name.is_empty() {
                    "UNNAMED TEAM"
                } else {
                    &self.team_name
                }
            ),
            16.0,
            Color::from_rgba8(242, 197, 98, 255),
        );
        self.ui.separator(12.0);

        Self::build_key_row(&mut self.ui, &ROW_ONE);
        Self::build_key_row(&mut self.ui, &ROW_TWO);
        Self::build_key_row(&mut self.ui, &ROW_THREE);

        self.ui.row_spaced(8.0, 4);
        self.ui.button(SPACE_ID, "Space");
        self.ui.button(BACKSPACE_ID, "Bksp");
        self.ui.button(CLEAR_ID, "Clear");
        self.ui.button(ACCEPT_ID, "Accept");
        self.ui.tooltip(
            "Gamepad-friendly row: d-pad moves focus, South confirms a key, and East works as a quick backspace shortcut.",
        );
    }

    fn build_key_row(ui: &mut Ui, keys: &[(&str, usize)]) {
        ui.row_spaced(6.0, keys.len());
        for (label, id) in keys {
            ui.button(*id, label);
        }
    }

    fn clamp_name(text: &str) -> String {
        text.chars().take(MAX_TEAM_NAME_CHARS).collect()
    }

    fn cursor(&self) -> usize {
        self.ui
            .text_cursor(TEAM_NAME_ID)
            .unwrap_or(self.team_name.len())
            .min(self.team_name.len())
    }

    fn set_cursor(&self, cursor: usize) {
        self.ui.set_text_cursor(TEAM_NAME_ID, cursor);
    }

    fn prev_char_boundary(text: &str, index: usize) -> usize {
        if index == 0 {
            return 0;
        }

        text[..index]
            .char_indices()
            .next_back()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn insert_text(&mut self, text: &str) {
        let current_chars = self.team_name.chars().count();
        let available = MAX_TEAM_NAME_CHARS.saturating_sub(current_chars);
        if available == 0 {
            return;
        }

        let insertable: String = text.chars().take(available).collect();
        if insertable.is_empty() {
            return;
        }

        let cursor = self.cursor();
        self.team_name.insert_str(cursor, &insertable);
        self.set_cursor(cursor + insertable.len());
    }

    fn backspace(&mut self) {
        let cursor = self.cursor();
        if cursor == 0 {
            return;
        }

        let start = Self::prev_char_boundary(&self.team_name, cursor);
        self.team_name.replace_range(start..cursor, "");
        self.set_cursor(start);
    }

    fn clear_name(&mut self) {
        self.team_name.clear();
        self.set_cursor(0);
    }

    fn accept_name(&mut self) {
        self.saved_name = if self.team_name.is_empty() {
            "UNNAMED TEAM".to_string()
        } else {
            self.team_name.clone()
        };
    }

    fn focus_row_len(row: usize) -> usize {
        match row {
            0 => 1,
            1 => ROW_ONE.len(),
            2 => ROW_TWO.len(),
            3 => ROW_THREE.len(),
            4 => 4,
            _ => 1,
        }
    }

    fn focus_row_col(index: usize) -> (usize, usize) {
        match index {
            0 => (0, 0),
            1..=10 => (1, index - 1),
            11..=19 => (2, index - 11),
            20..=26 => (3, index - 20),
            _ => (4, index.saturating_sub(27).min(3)),
        }
    }

    fn focus_index(row: usize, col: usize) -> usize {
        match row {
            0 => 0,
            1 => 1 + col.min(ROW_ONE.len() - 1),
            2 => 11 + col.min(ROW_TWO.len() - 1),
            3 => 20 + col.min(ROW_THREE.len() - 1),
            _ => 27 + col.min(3),
        }
    }

    fn move_focus_horizontal(&mut self, step: isize) {
        let (row, col) = Self::focus_row_col(self.focus_index);
        let max_col = Self::focus_row_len(row).saturating_sub(1) as isize;
        let next_col = (col as isize + step).clamp(0, max_col) as usize;
        self.focus_index = Self::focus_index(row, next_col);
    }

    fn move_focus_vertical(&mut self, step: isize) {
        let (row, col) = Self::focus_row_col(self.focus_index);
        let next_row = (row as isize + step).clamp(0, 4) as usize;
        let next_col = col.min(Self::focus_row_len(next_row).saturating_sub(1));
        self.focus_index = Self::focus_index(next_row, next_col);
    }

    fn focused_widget_id(&self) -> usize {
        match self.focus_index {
            0 => TEAM_NAME_ID,
            1..=10 => ROW_ONE[self.focus_index - 1].1,
            11..=19 => ROW_TWO[self.focus_index - 11].1,
            20..=26 => ROW_THREE[self.focus_index - 20].1,
            27 => SPACE_ID,
            28 => BACKSPACE_ID,
            29 => CLEAR_ID,
            _ => ACCEPT_ID,
        }
    }

    fn label_for_id(id: usize) -> Option<&'static str> {
        for (label, key_id) in ROW_ONE {
            if key_id == id {
                return Some(label);
            }
        }
        for (label, key_id) in ROW_TWO {
            if key_id == id {
                return Some(label);
            }
        }
        for (label, key_id) in ROW_THREE {
            if key_id == id {
                return Some(label);
            }
        }
        None
    }

    fn activate(&mut self, id: usize) {
        match id {
            TEAM_NAME_ID => {}
            SPACE_ID => self.insert_text(" "),
            BACKSPACE_ID => self.backspace(),
            CLEAR_ID => self.clear_name(),
            ACCEPT_ID => self.accept_name(),
            _ => {
                if let Some(label) = Self::label_for_id(id) {
                    self.insert_text(label);
                }
            }
        }
    }

    fn apply_gamepad_navigation(&mut self, engine: &Engine) {
        let gamepad = engine.gamepad(0);
        if !gamepad.is_connected() {
            return;
        }

        if gamepad.is_button_pressed(GamepadButton::DPadLeft) {
            self.move_focus_horizontal(-1);
        }
        if gamepad.is_button_pressed(GamepadButton::DPadRight) {
            self.move_focus_horizontal(1);
        }
        if gamepad.is_button_pressed(GamepadButton::DPadUp) {
            self.move_focus_vertical(-1);
        }
        if gamepad.is_button_pressed(GamepadButton::DPadDown) {
            self.move_focus_vertical(1);
        }
    }
}

impl Game for TextInputDemo {
    fn new(_engine: &mut Engine) -> Self {
        Self {
            ui: Ui::default(),
            team_name: String::new(),
            saved_name: "UNNAMED TEAM".to_string(),
            focus_index: 0,
            quit: false,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
            return;
        }

        self.apply_gamepad_navigation(engine);
        self.build_ui(engine);

        let response = self.ui.update(engine);
        if let Some(text) = response.text_for(TEAM_NAME_ID) {
            self.team_name = Self::clamp_name(text);
            self.set_cursor(self.cursor().min(self.team_name.len()));
        }

        for id in [
            SPACE_ID,
            BACKSPACE_ID,
            CLEAR_ID,
            ACCEPT_ID,
            ROW_ONE[0].1,
            ROW_ONE[1].1,
            ROW_ONE[2].1,
            ROW_ONE[3].1,
            ROW_ONE[4].1,
            ROW_ONE[5].1,
            ROW_ONE[6].1,
            ROW_ONE[7].1,
            ROW_ONE[8].1,
            ROW_ONE[9].1,
            ROW_TWO[0].1,
            ROW_TWO[1].1,
            ROW_TWO[2].1,
            ROW_TWO[3].1,
            ROW_TWO[4].1,
            ROW_TWO[5].1,
            ROW_TWO[6].1,
            ROW_TWO[7].1,
            ROW_TWO[8].1,
            ROW_THREE[0].1,
            ROW_THREE[1].1,
            ROW_THREE[2].1,
            ROW_THREE[3].1,
            ROW_THREE[4].1,
            ROW_THREE[5].1,
            ROW_THREE[6].1,
        ] {
            if response.was_activated(id) {
                self.activate(id);
            }
        }

        if let Some(focused) = response.focused {
            self.focus_index = focused.min(FOCUSABLE_COUNT - 1);
        }

        let gamepad = engine.gamepad(0);
        if gamepad.is_button_pressed(GamepadButton::South) {
            self.activate(self.focused_widget_id());
        }
        if gamepad.is_button_pressed(GamepadButton::East) {
            self.backspace();
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(15, 18, 28, 255);
        let (hw, hh) = engine.half_size();
        let canvas = frame.canvas(0);

        canvas.rect(
            -hw,
            hh - 120.0,
            hw * 2.0,
            100.0,
            Color::from_rgba8(28, 34, 52, 255),
        );
        canvas.rect(
            -hw,
            hh - 126.0,
            hw * 2.0,
            6.0,
            Color::from_rgba8(242, 197, 98, 255),
        );

        self.ui.render(canvas, engine);

        canvas.text_aligned(
            0.0,
            -hh + 48.0,
            &format!("Accepted team name: {}", self.saved_name),
            16.0,
            Color::from_rgba8(242, 197, 98, 255),
            TextAlign::Center,
        );

        let input_hint = if engine.gamepad(0).is_connected() {
            "Keyboard: type directly | Gamepad: d-pad moves focus, South confirms, East deletes"
        } else {
            "Keyboard: type directly into the field, or click the on-screen keyboard buttons"
        };
        canvas.text_aligned(
            0.0,
            -hh + 28.0,
            input_hint,
            11.0,
            Color::from_rgba8(158, 168, 188, 255),
            TextAlign::Center,
        );

        if let Some((preedit, _)) = engine.input().ime_preedit() {
            if !preedit.is_empty() {
                canvas.text_aligned(
                    0.0,
                    -hh + 12.0,
                    &format!("IME preedit: {}", preedit),
                    11.0,
                    Color::from_rgba8(130, 208, 255, 255),
                    TextAlign::Center,
                );
            }
        }
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<TextInputDemo>(EngineConfig {
        title: "Feature: Text Input".into(),
        width: 1100,
        height: 760,
        gamepad_assign: GamepadAssignMode::OnConnect,
        ..Default::default()
    })
}
