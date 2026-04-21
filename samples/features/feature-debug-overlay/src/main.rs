use rengine::*;

const AUTO_LOG_INTERVAL: f32 = 1.2;
const MAX_DEBUG_LOGS: usize = 256;

const SAMPLE_EVENTS: [(DebugLogLevel, &str, &str); 5] = [
    (
        DebugLogLevel::Trace,
        "sample::render",
        "Frame pacing pulse from the overlay sample.",
    ),
    (
        DebugLogLevel::Debug,
        "sample::physics",
        "Collision broadphase refreshed its active pairs.",
    ),
    (
        DebugLogLevel::Info,
        "sample::ui",
        "UI sync completed and kept the current frame coherent.",
    ),
    (
        DebugLogLevel::Warn,
        "sample::assets",
        "Hot-reload watcher skipped an optional development-only file.",
    ),
    (
        DebugLogLevel::Error,
        "sample::netcode",
        "Synthetic desync marker emitted for console filter testing.",
    ),
];

fn has_flag(flag: &str) -> bool {
    std::env::args().any(|argument| argument == flag)
}

fn arg_value(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|argument| argument == name)
        .and_then(|index| args.get(index + 1).cloned())
}

fn frame_limit() -> Option<u64> {
    arg_value("--frames")
        .and_then(|value| value.parse().ok())
        .or_else(|| has_flag("--headless").then_some(120))
}

fn emit_debug_log(engine: &Engine, level: DebugLogLevel, target: &str, message: &str) {
    match level {
        DebugLogLevel::Trace => engine.log_trace(target, message),
        DebugLogLevel::Debug => engine.log_debug(target, message),
        DebugLogLevel::Info => engine.log_info(target, message),
        DebugLogLevel::Warn => engine.log_warn(target, message),
        DebugLogLevel::Error => engine.log_error(target, message),
    }
}

fn level_index(level: DebugLogLevel) -> usize {
    match level {
        DebugLogLevel::Trace => 0,
        DebugLogLevel::Debug => 1,
        DebugLogLevel::Info => 2,
        DebugLogLevel::Warn => 3,
        DebugLogLevel::Error => 4,
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut output: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    output.push_str("...");
    output
}

struct DebugOverlayDemo {
    quit: bool,
    elapsed: f32,
    auto_log_timer: f32,
    auto_log_index: usize,
    frame: u64,
    max_frames: Option<u64>,
    activity: [f32; 5],
    clear_flash: f32,
    burst_flash: f32,
}

impl DebugOverlayDemo {
    fn emit_event(&mut self, engine: &Engine, level: DebugLogLevel, target: &str, message: &str) {
        emit_debug_log(engine, level, target, message);
        self.activity[level_index(level)] = 1.0;
    }

    fn emit_burst(&mut self, engine: &Engine) {
        for (level, target, message) in SAMPLE_EVENTS {
            self.emit_event(engine, level, target, &format!("Manual burst: {message}"));
        }
        self.burst_flash = 1.0;
    }
}

impl Game for DebugOverlayDemo {
    fn new(engine: &mut Engine) -> Self {
        if has_flag("--headless") {
            println!(
                "==============================================\n  RENGINE DEBUG OVERLAY SAMPLE - HEADLESS\n=============================================="
            );
        }

        engine.clear_debug_logs();
        engine.log_info(
            "sample::debug_overlay",
            "Debug overlay sample booted. The overlay starts open so you can inspect the built-in surface immediately.",
        );
        engine.log_debug(
            "sample::console",
            "Press F4 or ` to open the console and type `help`, `state`, `level warn`, or `target sample::assets`.",
        );
        engine.log_warn(
            "sample::filters",
            "Use F7 to cycle severity filters and F8 to edit the target filter in-place.",
        );

        Self {
            quit: false,
            elapsed: 0.0,
            auto_log_timer: 0.0,
            auto_log_index: 0,
            frame: 0,
            max_frames: frame_limit(),
            activity: [0.0; 5],
            clear_flash: 0.0,
            burst_flash: 0.0,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        let dt = engine.dt();
        self.elapsed += dt;
        self.auto_log_timer += dt;
        self.clear_flash = (self.clear_flash - dt).max(0.0);
        self.burst_flash = (self.burst_flash - dt).max(0.0);

        for value in &mut self.activity {
            *value = (*value - dt * 0.8).max(0.0);
        }

        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }

        if engine.input().is_key_pressed(KeyCode::KeyQ) {
            self.emit_event(
                engine,
                DebugLogLevel::Trace,
                "sample::render",
                "Manual trace ping from the render channel.",
            );
        }
        if engine.input().is_key_pressed(KeyCode::KeyW) {
            self.emit_event(
                engine,
                DebugLogLevel::Debug,
                "sample::physics",
                "Manual debug ping from the physics channel.",
            );
        }
        if engine.input().is_key_pressed(KeyCode::KeyE) {
            self.emit_event(
                engine,
                DebugLogLevel::Info,
                "sample::ui",
                "Manual info ping from the UI channel.",
            );
        }
        if engine.input().is_key_pressed(KeyCode::KeyR) {
            self.emit_event(
                engine,
                DebugLogLevel::Warn,
                "sample::assets",
                "Manual warning ping from the asset channel.",
            );
        }
        if engine.input().is_key_pressed(KeyCode::KeyT) {
            self.emit_event(
                engine,
                DebugLogLevel::Error,
                "sample::netcode",
                "Manual error ping from the networking channel.",
            );
        }
        if engine.input().is_key_pressed(KeyCode::Space) {
            self.emit_burst(engine);
        }
        if engine.input().is_key_pressed(KeyCode::KeyC) {
            engine.clear_debug_logs();
            self.clear_flash = 1.0;
        }

        while self.auto_log_timer >= AUTO_LOG_INTERVAL {
            self.auto_log_timer -= AUTO_LOG_INTERVAL;
            let (level, target, message) = SAMPLE_EVENTS[self.auto_log_index % SAMPLE_EVENTS.len()];
            self.emit_event(engine, level, target, message);
            self.auto_log_index += 1;
        }

        self.frame += 1;
        if let Some(limit) = self.max_frames {
            if self.frame >= limit {
                println!("OK {}", self.frame);
                self.quit = true;
            }
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;
        frame.clear_color = Color::from_rgba8(12, 16, 24, 255);
        let canvas = frame.canvas(0);
        let logs = engine.debug_logs(MAX_DEBUG_LOGS);

        canvas.rect(
            -hw,
            -hh,
            sw as f32,
            sh as f32,
            Color::from_rgba8(12, 16, 24, 255),
        );

        canvas.text(
            -hw + 24.0,
            hh - 28.0,
            "Debug Overlay + Console",
            30.0,
            Color::WHITE,
        );
        canvas.text_block(
            -hw + 24.0,
            hh - 62.0,
            "This sample starts the built-in debug surface open so you can inspect log capture, filters, console commands, and the engine-facing logging helpers without wiring anything yourself.",
            13.0,
            Color::from_rgba8(184, 194, 212, 255),
            sw as f32 - 48.0,
            TextAlign::Left,
        );

        let left_x = -hw + 24.0;
        let left_w = 430.0;
        let right_x = -hw + 478.0;
        let right_w = sw as f32 - (right_x + hw) - 24.0;
        let panel_top = hh - 130.0;
        let panel_h = 214.0;

        canvas.rect(
            left_x,
            panel_top - panel_h,
            left_w,
            panel_h,
            Color::from_rgba8(20, 26, 38, 235),
        );
        canvas.rect(
            right_x,
            panel_top - panel_h,
            right_w,
            panel_h,
            Color::from_rgba8(20, 26, 38, 235),
        );

        canvas.text(
            left_x + 16.0,
            panel_top - 18.0,
            "Engine State",
            18.0,
            Color::WHITE,
        );
        canvas.text(
            left_x + 16.0,
            panel_top - 50.0,
            &format!(
                "overlay: {}   console: {}   hot reload: {}",
                if engine.debug_overlay_visible() {
                    "visible"
                } else {
                    "hidden"
                },
                if engine.debug_console_open() {
                    "open"
                } else {
                    "closed"
                },
                if engine.hot_reload_enabled() {
                    "on"
                } else {
                    "off"
                },
            ),
            13.0,
            Color::from_rgba8(214, 222, 236, 255),
        );

        let mut counts = [0usize; 5];
        for entry in &logs {
            counts[level_index(entry.level)] += 1;
        }

        canvas.text(
            left_x + 16.0,
            panel_top - 76.0,
            &format!(
                "captured logs: {}   auto emit: every {:.1}s   frame: {}",
                logs.len(),
                AUTO_LOG_INTERVAL,
                self.frame,
            ),
            13.0,
            Color::from_rgba8(160, 224, 255, 255),
        );
        canvas.text(
            left_x + 16.0,
            panel_top - 102.0,
            &format!(
                "trace {}   debug {}   info {}   warn {}   error {}",
                counts[0], counts[1], counts[2], counts[3], counts[4]
            ),
            13.0,
            Color::from_rgba8(194, 204, 222, 255),
        );

        let latest_summary = logs.last().map_or_else(
            || "latest: buffer is empty".to_string(),
            |entry| {
                format!(
                    "latest: [{}] {} — {}",
                    entry.level.label(),
                    entry.target,
                    truncate_text(&entry.message, 70),
                )
            },
        );
        canvas.text_block(
            left_x + 16.0,
            panel_top - 134.0,
            &latest_summary,
            12.0,
            Color::from_rgba8(176, 188, 208, 255),
            left_w - 32.0,
            TextAlign::Left,
        );

        let clear_text = if self.clear_flash > 0.0 {
            "buffer cleared from sample input"
        } else if self.burst_flash > 0.0 {
            "manual burst emitted across all five levels"
        } else {
            "F5 or the console `clear` command also empties the buffer"
        };
        let clear_color = if self.clear_flash > 0.0 {
            Color::from_rgba8(255, 214, 120, 255)
        } else if self.burst_flash > 0.0 {
            Color::from_rgba8(140, 228, 255, 255)
        } else {
            Color::from_rgba8(148, 160, 180, 255)
        };
        canvas.text_block(
            left_x + 16.0,
            panel_top - 182.0,
            clear_text,
            12.0,
            clear_color,
            left_w - 32.0,
            TextAlign::Left,
        );

        canvas.text(
            right_x + 16.0,
            panel_top - 18.0,
            "Controls",
            18.0,
            Color::WHITE,
        );
        canvas.text_block(
            right_x + 16.0,
            panel_top - 50.0,
            "F3 overlay   F4 or ` console\nF5 clear   F6 follow   F7 severity   F8 target filter\nQ trace   W debug   E info   R warn   T error\nSpace emits a five-line burst   C clears logs   Esc quits",
            13.0,
            Color::from_rgba8(214, 222, 236, 255),
            right_w - 32.0,
            TextAlign::Left,
        );
        canvas.text_block(
            right_x + 16.0,
            panel_top - 156.0,
            "Console prompts are IME-aware, target filtering is case-insensitive, and scrolling only captures the wheel when the pointer is actually over the overlay or console.",
            12.0,
            Color::from_rgba8(158, 170, 188, 255),
            right_w - 32.0,
            TextAlign::Left,
        );

        let bottom_x = -hw + 24.0;
        let bottom_w = sw as f32 - 48.0;
        let bottom_h = 240.0;
        let bottom_top = -20.0;
        canvas.rect(
            bottom_x,
            bottom_top - bottom_h,
            bottom_w,
            bottom_h,
            Color::from_rgba8(20, 26, 38, 235),
        );
        canvas.text(
            bottom_x + 16.0,
            bottom_top - 18.0,
            "Live Activity + Recent Mirror",
            18.0,
            Color::WHITE,
        );

        let activity_rows = [
            (DebugLogLevel::Trace, "Q / trace"),
            (DebugLogLevel::Debug, "W / debug"),
            (DebugLogLevel::Info, "E / info"),
            (DebugLogLevel::Warn, "R / warn"),
            (DebugLogLevel::Error, "T / error"),
        ];

        for (row_index, (level, label)) in activity_rows.iter().enumerate() {
            let row_y = bottom_top - 54.0 - row_index as f32 * 28.0;
            let meter_fill = 220.0 * self.activity[level_index(*level)].clamp(0.0, 1.0);
            let active_color = if self.activity[level_index(*level)] > 0.05 {
                level.color()
            } else {
                Color::from_rgba8(66, 76, 94, 255)
            };

            canvas.text(
                bottom_x + 16.0,
                row_y,
                label,
                12.0,
                Color::from_rgba8(210, 218, 232, 255),
            );
            canvas.rect(
                bottom_x + 116.0,
                row_y - 14.0,
                220.0,
                16.0,
                Color::from_rgba8(34, 40, 52, 255),
            );
            canvas.rect(
                bottom_x + 116.0,
                row_y - 14.0,
                meter_fill.max(6.0),
                16.0,
                active_color,
            );
        }

        let recent_x = bottom_x + 374.0;
        canvas.text(
            recent_x,
            bottom_top - 42.0,
            "Recent mirror (via engine.debug_logs)",
            13.0,
            Color::from_rgba8(160, 224, 255, 255),
        );
        for (line_index, entry) in logs.iter().rev().take(5).enumerate() {
            let line_y = bottom_top - 72.0 - line_index as f32 * 28.0;
            canvas.text(
                recent_x,
                line_y,
                &format!(
                    "[{:<5}] {}",
                    entry.level.label(),
                    truncate_text(&format!("{} — {}", entry.target, entry.message), 64)
                ),
                12.0,
                entry.level.color(),
            );
        }
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let headless = has_flag("--headless");

    let config = EngineConfig {
        title: "Feature: Debug Overlay + Console".into(),
        width: 1020,
        height: 720,
        headless,
        hot_reload: !headless,
        show_fps: false,
        show_debug_overlay: true,
        ..Default::default()
    };

    let _ = run::<DebugOverlayDemo>(config);
}
