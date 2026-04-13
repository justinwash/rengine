use rengine::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct PlayerData {
    high_score: u32,
    coins: u32,
    times_played: u32,
}

struct SaveLoadDemo {
    saves: SaveSystem,
    data: PlayerData,
    status: String,
    quit: bool,
}

const SLOT: &str = "demo";

impl Game for SaveLoadDemo {
    fn new(_engine: &mut Engine) -> Self {
        let saves =
            SaveSystem::new("rengine-feature-saveload").expect("failed to init save system");
        let (data, status) = match saves.load::<PlayerData>(SLOT) {
            Ok(d) => (
                d,
                format!("Loaded existing save from {}", saves.save_dir().display()),
            ),
            Err(_) if saves.exists(SLOT) => (
                PlayerData::default(),
                "Save exists but failed to load — starting fresh".into(),
            ),
            Err(_) => (
                PlayerData::default(),
                "No save found — starting fresh".into(),
            ),
        };
        Self {
            saves,
            data,
            status,
            quit: false,
        }
    }

    fn update(&mut self, engine: &Engine) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }

        if engine.input().is_key_pressed(KeyCode::Space) {
            self.data.coins += 10;
            self.data.high_score = self.data.high_score.max(self.data.coins);
            self.status = format!("+10 coins! Total: {}", self.data.coins);
        }

        if engine.input().is_key_pressed(KeyCode::KeyS) {
            self.data.times_played += 1;
            match self.saves.save(SLOT, &self.data) {
                Ok(()) => self.status = format!("Saved! (play #{})", self.data.times_played),
                Err(e) => self.status = format!("Save failed: {e}"),
            }
        }

        if engine.input().is_key_pressed(KeyCode::KeyL) {
            match self.saves.load::<PlayerData>(SLOT) {
                Ok(d) => {
                    self.data = d;
                    self.status = "Loaded!".into();
                }
                Err(e) => self.status = format!("Load failed: {e}"),
            }
        }

        if engine.input().is_key_pressed(KeyCode::KeyD) {
            match self.saves.delete(SLOT) {
                Ok(()) => {
                    self.data = PlayerData::default();
                    self.status = "Save deleted, data reset".into();
                }
                Err(e) => self.status = format!("Delete failed: {e}"),
            }
        }

        if engine.input().is_key_pressed(KeyCode::KeyR) {
            self.data = PlayerData::default();
            self.status = "Data reset (not saved yet)".into();
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);

        canvas.rect(
            -hw,
            -hh,
            sw as f32,
            sh as f32,
            Color::from_rgba8(20, 20, 30, 255),
            (sw, sh),
        );

        canvas.text(
            -hw + 20.0,
            hh - 30.0,
            "Save / Load Demo",
            24.0,
            Color::WHITE,
            (sw, sh),
            atlas,
        );

        let x = -hw + 30.0;
        let mut y = hh - 80.0;
        let label_col = Color::from_rgba8(180, 180, 180, 255);
        let val_col = Color::from_rgba8(100, 200, 255, 255);

        let line =
            |canvas: &mut Canvas, y: &mut f32, label: &str, value: &str, atlas: &FontAtlas| {
                canvas.text(x, *y, label, 16.0, label_col, (sw, sh), atlas);
                canvas.text(x + 160.0, *y, value, 16.0, val_col, (sw, sh), atlas);
                *y -= 28.0;
            };

        line(
            canvas,
            &mut y,
            "High Score:",
            &self.data.high_score.to_string(),
            atlas,
        );
        line(
            canvas,
            &mut y,
            "Coins:",
            &self.data.coins.to_string(),
            atlas,
        );
        line(
            canvas,
            &mut y,
            "Times Played:",
            &self.data.times_played.to_string(),
            atlas,
        );

        y -= 20.0;
        canvas.text(
            x,
            y,
            &self.status,
            14.0,
            Color::from_rgba8(255, 220, 80, 255),
            (sw, sh),
            atlas,
        );

        y -= 40.0;
        let slots = self.saves.list_slots();
        let slot_text = if slots.is_empty() {
            "none".into()
        } else {
            slots.join(", ")
        };
        line(canvas, &mut y, "Save Slots:", &slot_text, atlas);

        y -= 20.0;
        let hint_col = Color::from_rgba8(100, 100, 100, 255);
        canvas.text(
            x,
            y,
            "SPACE = earn coins | S = save | L = load | D = delete | R = reset | ESC = quit",
            12.0,
            hint_col,
            (sw, sh),
            atlas,
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: Save / Load".into(),
        width: 600,
        height: 400,
        show_fps: false,
        ..Default::default()
    };
    run::<SaveLoadDemo>(config).unwrap();
}
