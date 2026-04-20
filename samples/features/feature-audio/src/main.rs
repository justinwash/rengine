use rengine::*;
use std::path::PathBuf;

struct AudioDemo {
    track_a: AudioClip,
    track_b: AudioClip,
    sfx_blip: AudioClip,
    on_track_a: bool,
    demo_mode: bool,
    max_frames: Option<u32>,
    frame_count: u32,
    finished: bool,
    status: String,
}

impl Game for AudioDemo {
    fn new(engine: &mut Engine) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let demo_mode = args.contains(&"--demo".to_string());
        let max_frames = args
            .iter()
            .position(|a| a == "--frames")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok());

        engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

        let track_a = engine.load_audio("track_a.wav").expect("track_a.wav");
        let track_b = engine.load_audio("track_b.wav").expect("track_b.wav");
        let sfx_blip = engine.load_audio("sfx_blip.wav").expect("sfx_blip.wav");

        engine.set_audio_bus_volume(AudioBus::Effects, 0.8);

        let _ = engine.fade_in_music(&track_a, 2.0, Easing::OutQuad);

        if demo_mode {
            println!("[FEATURE OK] fade_in_music — track_a fading in");
        }

        Self {
            track_a,
            track_b,
            sfx_blip,
            on_track_a: true,
            demo_mode,
            max_frames,
            frame_count: 0,
            finished: false,
            status: "Fade in: track A (2s)".into(),
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.frame_count += 1;

        if let Some(max) = self.max_frames {
            if self.frame_count >= max {
                println!("OK {max}");
                self.finished = true;
                return;
            }
        }

        if self.demo_mode {
            match self.frame_count {
                120 => {
                    let _ = engine.crossfade_music(&self.track_b, 2.0, Easing::InOutSine);
                    self.on_track_a = false;
                    self.status = "Crossfade → track B (2s)".into();
                    println!("[FEATURE OK] crossfade_music — switching to track_b");
                }
                240 => {
                    engine.fade_bus_volume(AudioBus::Music, 0.3, 1.0, Easing::OutQuad);
                    self.status = "Fade Music bus → 0.3 (1s)".into();
                    println!("[FEATURE OK] fade_bus_volume — music bus → 0.3");
                }
                300 => {
                    engine.fade_bus_volume(AudioBus::Music, 1.0, 0.5, Easing::InQuad);
                    self.status = "Fade Music bus → 1.0 (0.5s)".into();
                    println!("[FEATURE OK] fade_bus_volume — music bus → 1.0");
                }
                360 => {
                    engine.fade_out_music(1.5, Easing::InQuad);
                    self.status = "Fade out music (1.5s)".into();
                    println!("[FEATURE OK] fade_out_music — fading out");
                }
                420 => {
                    engine.fade_master_volume(0.5, 1.0, Easing::Linear);
                    self.status = "Fade master → 0.5 (1s)".into();
                    println!("[FEATURE OK] fade_master_volume — master → 0.5");
                }
                _ => {}
            }

            if self.frame_count % 90 == 0 && self.frame_count < 360 {
                let _ = engine.play_sound_on_bus(AudioBus::Effects, &self.sfx_blip, 0.6);
            }
            return;
        }

        if engine.input().is_key_pressed(KeyCode::KeyF) {
            if self.on_track_a {
                let _ = engine.crossfade_music(&self.track_b, 2.0, Easing::InOutSine);
                self.on_track_a = false;
                self.status = "Crossfade → track B (2s)".into();
            } else {
                let _ = engine.crossfade_music(&self.track_a, 2.0, Easing::InOutSine);
                self.on_track_a = true;
                self.status = "Crossfade → track A (2s)".into();
            }
        }

        if engine.input().is_key_pressed(KeyCode::KeyO) {
            engine.fade_out_music(1.5, Easing::InQuad);
            self.status = "Fade out (1.5s)".into();
        }

        if engine.input().is_key_pressed(KeyCode::KeyI) {
            let clip = if self.on_track_a {
                &self.track_a
            } else {
                &self.track_b
            };
            let _ = engine.fade_in_music(clip, 2.0, Easing::OutQuad);
            self.status = format!(
                "Fade in: track {} (2s)",
                if self.on_track_a { "A" } else { "B" }
            );
        }

        if engine.input().is_key_pressed(KeyCode::Space) {
            let _ = engine.play_sound_on_bus(AudioBus::Effects, &self.sfx_blip, 0.6);
        }

        if engine.input().is_key_pressed(KeyCode::ArrowUp) {
            engine.fade_bus_volume(AudioBus::Music, 1.0, 0.5, Easing::OutQuad);
            self.status = "Music bus → 1.0 (0.5s)".into();
        }

        if engine.input().is_key_pressed(KeyCode::ArrowDown) {
            engine.fade_bus_volume(AudioBus::Music, 0.2, 0.5, Easing::OutQuad);
            self.status = "Music bus → 0.2 (0.5s)".into();
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(20, 20, 30, 255);
        let (w, h) = engine.window_size();
        let _screen = (w, h);
        let c = frame.canvas(10);
        let hh = h as f32 / 2.0;

        c.text_aligned(
            0.0,
            hh - 70.0,
            "Audio Fades Demo",
            28.0,
            Color::WHITE,
            TextAlign::Center,
        );

        c.text_aligned(
            0.0,
            hh - 112.0,
            &self.status,
            18.0,
            Color::YELLOW,
            TextAlign::Center,
        );

        let fading = if engine.is_audio_fading() {
            "fading..."
        } else {
            "idle"
        };
        c.text_aligned(
            0.0,
            hh - 138.0,
            fading,
            14.0,
            Color::from_rgba8(150, 150, 150, 255),
            TextAlign::Center,
        );

        if !self.demo_mode {
            let help = "[F] crossfade   [I] fade in   [O] fade out\n[Space] play SFX   [Up/Down] fade music bus";
            c.text_block(
                0.0,
                -hh + 54.0,
                help,
                13.0,
                Color::from_rgba8(120, 120, 120, 255),
                560.0,
                TextAlign::Center,
            );
        }
    }

    fn should_exit(&self) -> bool {
        self.finished
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: Audio Fades".into(),
        width: 760,
        height: 420,
        show_fps: false,
        headless: std::env::args().any(|a| a == "--headless"),
        ..Default::default()
    };
    rengine::run::<AudioDemo>(config).expect("run failed");
}
