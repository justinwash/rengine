use rengine::*;

const AUTO_LOG_INTERVAL: f32 = 0.35;
const MAX_DEBUG_LOGS: usize = 1024;

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

fn draw_card(canvas: &mut Canvas, x: f32, top: f32, w: f32, h: f32, title: &str) {
    canvas.rect(x, top - h, w, h, Color::from_rgba8(20, 26, 38, 235));
    canvas.text(x + 16.0, top - 18.0, title, 18.0, Color::WHITE);
}

fn wrapped_lines(engine: &Engine, text: &str, size: f32, width: f32) -> Vec<String> {
    wrap_text(text, size, width, engine.font_atlas())
}

fn draw_wrapped_lines(
    canvas: &mut Canvas,
    x: f32,
    top: f32,
    lines: &[String],
    size: f32,
    color: Color,
) -> f32 {
    let line_height = canvas.line_height(size);
    for (index, line) in lines.iter().enumerate() {
        canvas.text(x, top - index as f32 * line_height, line, size, color);
    }
    line_height * lines.len().max(1) as f32
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

        engine.set_debug_overlay_visible(true);
        engine.set_debug_console_open(false);
        engine.clear_debug_logs();
        engine.log_info(
            "sample::debug_overlay",
            "Debug overlay sample booted. The overlay starts open and the console starts closed so you can inspect the built-in surface immediately.",
        );
        engine.log_debug(
            "sample::console",
            "Press F4 or ` to open the console and type `help`, `state`, `level warn`, `capacity 8192`, or `target sample::assets`.",
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
        let log_capacity = engine.debug_log_capacity();
        let log_count = engine.debug_log_count();
        let logs = engine.debug_logs(MAX_DEBUG_LOGS.min(log_capacity));
        let overlay_visible = engine.debug_overlay_visible();
        let console_open = engine.debug_console_open();
        let overlay_priority_layout = overlay_visible || console_open;
        let content_left = -hw + 24.0;
        let content_w = sw as f32 - 48.0;
        let body_size = 12.0;
        let small_size = 11.0;
        let body_color = Color::from_rgba8(214, 222, 236, 255);
        let muted_color = Color::from_rgba8(176, 188, 208, 255);
        let hint_color = Color::from_rgba8(158, 170, 188, 255);

        canvas.rect(
            -hw,
            -hh,
            sw as f32,
            sh as f32,
            Color::from_rgba8(12, 16, 24, 255),
        );

        let mut counts = [0usize; 5];
        for entry in &logs {
            counts[level_index(entry.level)] += 1;
        }

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

        let clear_color = if self.clear_flash > 0.0 {
            Color::from_rgba8(255, 214, 120, 255)
        } else if self.burst_flash > 0.0 {
            Color::from_rgba8(140, 228, 255, 255)
        } else {
            Color::from_rgba8(148, 160, 180, 255)
        };

        if !overlay_priority_layout {
            let hero_top = hh - 28.0;
            let hero_lines = wrapped_lines(
                engine,
                "The overlay starts open and the console starts closed. Hide the overlay with F3 or open the console with F4 to compare the engine UI against the quieter sample backdrop.",
                13.0,
                content_w,
            );
            canvas.text(
                content_left,
                hero_top,
                "Debug Overlay + Console",
                30.0,
                Color::WHITE,
            );
            draw_wrapped_lines(
                canvas,
                content_left,
                hero_top - 34.0,
                &hero_lines,
                13.0,
                Color::from_rgba8(184, 194, 212, 255),
            );

            let card_gap = 20.0;
            let card_w = (content_w - card_gap) * 0.5;
            let card_top = 132.0;
            let card_h = 176.0;
            let left_x = content_left;
            let right_x = left_x + card_w + card_gap;
            let latest_lines = wrapped_lines(engine, &latest_summary, small_size, card_w - 32.0);
            let state_line_1 = wrapped_lines(
                engine,
                &format!(
                    "overlay {}   console {}   hot reload {}",
                    if overlay_visible { "visible" } else { "hidden" },
                    if console_open { "open" } else { "closed" },
                    if engine.hot_reload_enabled() {
                        "on"
                    } else {
                        "off"
                    },
                ),
                body_size,
                card_w - 32.0,
            );
            let state_line_2 = wrapped_lines(
                engine,
                &format!(
                    "captured logs {} / {}   auto emit {:.2}s   frame {}",
                    log_count, log_capacity, AUTO_LOG_INTERVAL, self.frame,
                ),
                body_size,
                card_w - 32.0,
            );
            let state_line_3 = wrapped_lines(
                engine,
                &format!(
                    "recent trace {}   debug {}   info {}   warn {}   error {}",
                    counts[0], counts[1], counts[2], counts[3], counts[4]
                ),
                small_size,
                card_w - 32.0,
            );
            let use_case_lines = wrapped_lines(
                engine,
                "Combat: filter a `combat` target and watch cooldowns, damage spikes, and boss phase changes live.\nQuest + dialogue: verify triggers fire in the right order without alt-tabbing.\nNetcode + save/load: isolate rollback, persistence, or desync spam to one subsystem target.\nHot reload: leave the overlay open while assets and data are changing under the running build.",
                small_size,
                card_w - 32.0,
            );

            draw_card(canvas, left_x, card_top, card_w, card_h, "Current State");
            let mut current_y = card_top - 44.0;
            current_y -= draw_wrapped_lines(
                canvas,
                left_x + 16.0,
                current_y,
                &state_line_1,
                body_size,
                body_color,
            );
            current_y -= 8.0;
            current_y -= draw_wrapped_lines(
                canvas,
                left_x + 16.0,
                current_y,
                &state_line_2,
                body_size,
                Color::from_rgba8(160, 224, 255, 255),
            );
            current_y -= 8.0;
            current_y -= draw_wrapped_lines(
                canvas,
                left_x + 16.0,
                current_y,
                &state_line_3,
                small_size,
                Color::from_rgba8(194, 204, 222, 255),
            );
            current_y -= 10.0;
            draw_wrapped_lines(
                canvas,
                left_x + 16.0,
                current_y,
                &latest_lines,
                small_size,
                muted_color,
            );

            draw_card(
                canvas,
                right_x,
                card_top,
                card_w,
                card_h,
                "Useful In A Real Game",
            );
            draw_wrapped_lines(
                canvas,
                right_x + 16.0,
                card_top - 42.0,
                &use_case_lines,
                small_size,
                body_color,
            );
        }

        let bottom_x = content_left;
        let bottom_w = content_w;
        let bottom_h = if overlay_priority_layout {
            248.0
        } else {
            224.0
        };
        let bottom_top = if overlay_priority_layout {
            -56.0
        } else {
            -82.0
        };
        draw_card(
            canvas,
            bottom_x,
            bottom_top,
            bottom_w,
            bottom_h,
            "Live Activity + Recent Mirror",
        );
        let overlay_note_lines = wrapped_lines(
            engine,
            if overlay_priority_layout {
                "Keep this UI open while you play the actual game build: watch combat, quest, save/load, or netcode targets update live, then use the engine overlay to narrow the view without leaving the running scene."
            } else {
                "When you bring the overlay back with F3, it acts like an in-game diagnosis surface rather than a second app window. Use F6 for live follow, F7 for severity, and F8 to lock onto a subsystem target."
            },
            small_size,
            bottom_w - 32.0,
        );
        draw_wrapped_lines(
            canvas,
            bottom_x + 16.0,
            bottom_top - 40.0,
            &overlay_note_lines,
            small_size,
            hint_color,
        );

        let activity_rows = [
            (DebugLogLevel::Trace, "Q / trace"),
            (DebugLogLevel::Debug, "W / debug"),
            (DebugLogLevel::Info, "E / info"),
            (DebugLogLevel::Warn, "R / warn"),
            (DebugLogLevel::Error, "T / error"),
        ];
        let meter_x = bottom_x + 116.0;
        let meter_w = 220.0;
        let activity_top = bottom_top - 92.0;

        for (row_index, (level, label)) in activity_rows.iter().enumerate() {
            let row_y = activity_top - row_index as f32 * 28.0;
            let meter_fill = meter_w * self.activity[level_index(*level)].clamp(0.0, 1.0);
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
                meter_x,
                row_y - 14.0,
                meter_w,
                16.0,
                Color::from_rgba8(34, 40, 52, 255),
            );
            canvas.rect(
                meter_x,
                row_y - 14.0,
                meter_fill.max(6.0),
                16.0,
                active_color,
            );
        }

        let recent_x = meter_x + meter_w + 56.0;
        let recent_w = bottom_x + bottom_w - recent_x - 16.0;
        canvas.text(
            recent_x,
            activity_top + 12.0,
            "Recent mirror (via engine.debug_logs)",
            13.0,
            Color::from_rgba8(160, 224, 255, 255),
        );
        let recent_line_height = canvas.line_height(small_size);
        let mut recent_y = activity_top - 14.0;
        for entry in logs.iter().rev().take(4) {
            let wrapped = wrapped_lines(
                engine,
                &format!(
                    "[{}] {} — {}",
                    entry.level.label(),
                    entry.target,
                    entry.message
                ),
                small_size,
                recent_w,
            );
            for line in wrapped.iter().take(2) {
                canvas.text(recent_x, recent_y, line, small_size, entry.level.color());
                recent_y -= recent_line_height;
            }
            recent_y -= 8.0;
        }

        let footer_h = 72.0;
        let footer_top = -hh + footer_h;
        canvas.rect(
            -hw,
            -hh,
            sw as f32,
            footer_h,
            Color::from_rgba8(14, 18, 28, 255),
        );
        canvas.text_block(
            content_left,
            footer_top - 16.0,
            if overlay_priority_layout {
                "Console stays closed at startup so the overlay can do the first pass. Open it with F4 when you want command entry, and keep follow live enabled when you want the newest gameplay logs pinned in view."
            } else {
                "Controls: F3 overlay, F4 console, F6 follow, F7 severity, F8 target filter, Q/W/E/R/T emit logs, Space bursts, C clears, Esc quits."
            },
            11.0,
            clear_color,
            content_w,
            TextAlign::Left,
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let headless = has_flag("--headless");

    let config = EngineConfig {
        title: "Feature: Debug Overlay + Console".into(),
        width: 1360,
        height: 900,
        headless,
        hot_reload: !headless,
        show_fps: false,
        show_debug_overlay: true,
        debug_log_capacity: 16384,
        ..Default::default()
    };

    let _ = run::<DebugOverlayDemo>(config);
}
