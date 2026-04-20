use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once, OnceLock};
use std::time::Instant;

use crate::assets::Color;
use crate::canvas::Canvas;
use winit::event::{ElementState, Ime, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

const DEFAULT_LOG_CAPACITY: usize = 256;
const DEFAULT_OVERLAY_LOG_LIMIT: usize = 10;
const DEFAULT_SCROLL_STEP: usize = 3;
const MAX_COMMAND_HISTORY: usize = 64;
const OVERLAY_TITLE_SIZE: f32 = 13.0;
const OVERLAY_BODY_SIZE: f32 = 11.0;
const OVERLAY_HINT_SIZE: f32 = 10.0;
const OVERLAY_BUTTON_TEXT_SIZE: f32 = 10.0;
const OVERLAY_BUTTON_HEIGHT: f32 = 20.0;
const OVERLAY_BUTTON_GAP: f32 = 6.0;
const OVERLAY_BUTTON_PADDING_X: f32 = 8.0;
const OVERLAY_BUTTON_MIN_WIDTH: f32 = 54.0;
const CONSOLE_PANEL_HEIGHT: f32 = 84.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl DebugLogLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Trace => Color::from_rgba8(150, 160, 180, 255),
            Self::Debug => Color::from_rgba8(120, 180, 255, 255),
            Self::Info => Color::from_rgba8(235, 238, 245, 255),
            Self::Warn => Color::from_rgba8(255, 210, 96, 255),
            Self::Error => Color::from_rgba8(255, 120, 120, 255),
        }
    }
}

impl From<log::Level> for DebugLogLevel {
    fn from(value: log::Level) -> Self {
        match value {
            log::Level::Trace => Self::Trace,
            log::Level::Debug => Self::Debug,
            log::Level::Info => Self::Info,
            log::Level::Warn => Self::Warn,
            log::Level::Error => Self::Error,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugSeverityFilter {
    All,
    Debug,
    Info,
    Warn,
    Error,
}

impl DebugSeverityFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Debug => "debug+",
            Self::Info => "info+",
            Self::Warn => "warn+",
            Self::Error => "error",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Debug,
            Self::Debug => Self::Info,
            Self::Info => Self::Warn,
            Self::Warn => Self::Error,
            Self::Error => Self::All,
        }
    }

    pub fn allows(self, level: DebugLogLevel) -> bool {
        match self {
            Self::All => true,
            Self::Debug => !matches!(level, DebugLogLevel::Trace),
            Self::Info => matches!(
                level,
                DebugLogLevel::Info | DebugLogLevel::Warn | DebugLogLevel::Error
            ),
            Self::Warn => matches!(level, DebugLogLevel::Warn | DebugLogLevel::Error),
            Self::Error => level == DebugLogLevel::Error,
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "all" | "trace" => Some(Self::All),
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" | "warning" => Some(Self::Warn),
            "error" | "errors" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugToggle {
    On,
    Off,
    Toggle,
}

impl DebugToggle {
    pub fn apply(self, current: bool) -> bool {
        match self {
            Self::On => true,
            Self::Off => false,
            Self::Toggle => !current,
        }
    }

    pub fn parse(text: Option<&str>) -> Result<Self, String> {
        match text.map(|value| value.trim().to_ascii_lowercase()) {
            None => Ok(Self::Toggle),
            Some(value) if value.is_empty() => Ok(Self::Toggle),
            Some(value) if value == "toggle" => Ok(Self::Toggle),
            Some(value) if matches!(value.as_str(), "on" | "true" | "1") => Ok(Self::On),
            Some(value) if matches!(value.as_str(), "off" | "false" | "0") => Ok(Self::Off),
            Some(value) => Err(format!("unknown toggle value '{value}'")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugCommand {
    Help,
    State,
    Clear,
    Overlay(DebugToggle),
    Console(DebugToggle),
    Follow(DebugToggle),
    Level(DebugSeverityFilter),
    Target(Option<String>),
    HotReload(DebugToggle),
    Echo(DebugLogLevel, String),
}

#[derive(Clone, Debug)]
pub struct DebugLogEntry {
    pub index: u64,
    pub seconds: f32,
    pub level: DebugLogLevel,
    pub target: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug)]
pub struct DebugOverlayInfo<'a> {
    pub mode: &'a str,
    pub fps: f32,
    pub dt: f32,
    pub frame_count: u64,
    pub total_time: f32,
    pub window_size: (u32, u32),
    pub game_size: (u32, u32),
    pub hot_reload_enabled: bool,
    pub gamepads_connected: Option<usize>,
    pub mouse_captured: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DebugTextInputMode {
    None,
    Console,
    TargetFilter,
}

pub struct DebugUiState {
    overlay_visible: bool,
    console_open: bool,
    follow_logs: bool,
    severity_filter: DebugSeverityFilter,
    target_filter: String,
    target_filter_buffer: String,
    console_buffer: String,
    preedit: Option<(String, Option<(usize, usize)>)>,
    scroll_offset: usize,
    command_history: VecDeque<String>,
    history_index: Option<usize>,
    pending_commands: VecDeque<String>,
    input_mode: DebugTextInputMode,
    captured_mouse_buttons: [bool; 3],
}

impl DebugUiState {
    pub fn new(overlay_visible: bool) -> Self {
        Self {
            overlay_visible,
            console_open: false,
            follow_logs: true,
            severity_filter: DebugSeverityFilter::All,
            target_filter: String::new(),
            target_filter_buffer: String::new(),
            console_buffer: String::new(),
            preedit: None,
            scroll_offset: 0,
            command_history: VecDeque::new(),
            history_index: None,
            pending_commands: VecDeque::new(),
            input_mode: DebugTextInputMode::None,
            captured_mouse_buttons: [false; 3],
        }
    }

    pub fn overlay_visible(&self) -> bool {
        self.overlay_visible
    }

    pub fn set_overlay_visible(&mut self, visible: bool) {
        self.overlay_visible = visible;
        if !visible {
            self.console_open = false;
            self.input_mode = DebugTextInputMode::None;
            self.preedit = None;
            self.history_index = None;
            self.captured_mouse_buttons = [false; 3];
        }
    }

    pub fn toggle_overlay(&mut self) {
        self.set_overlay_visible(!self.overlay_visible);
    }

    pub fn console_open(&self) -> bool {
        self.console_open
    }

    pub fn set_console_open(&mut self, open: bool) {
        if open {
            self.overlay_visible = true;
            self.console_open = true;
            self.input_mode = DebugTextInputMode::Console;
        } else {
            self.console_open = false;
            if self.input_mode == DebugTextInputMode::Console {
                self.input_mode = DebugTextInputMode::None;
            }
        }
        self.preedit = None;
        self.history_index = None;
    }

    pub fn toggle_console(&mut self) {
        self.set_console_open(!self.console_open);
    }

    pub fn follow_logs(&self) -> bool {
        self.follow_logs
    }

    pub fn set_follow_logs(&mut self, follow: bool) {
        self.follow_logs = follow;
        if follow {
            self.scroll_offset = 0;
        }
    }

    pub fn toggle_follow_logs(&mut self) {
        self.set_follow_logs(!self.follow_logs);
    }

    pub fn severity_filter(&self) -> DebugSeverityFilter {
        self.severity_filter
    }

    pub fn set_severity_filter(&mut self, filter: DebugSeverityFilter) {
        self.severity_filter = filter;
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }

    pub fn cycle_severity_filter(&mut self) {
        self.set_severity_filter(self.severity_filter.next());
    }

    pub fn target_filter(&self) -> &str {
        &self.target_filter
    }

    pub fn set_target_filter(&mut self, filter: impl Into<String>) {
        self.target_filter = filter.into().trim().to_string();
        self.target_filter_buffer = self.target_filter.clone();
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }

    pub fn clear_target_filter(&mut self) {
        self.set_target_filter(String::new());
    }

    pub fn begin_target_filter_edit(&mut self) {
        self.overlay_visible = true;
        self.console_open = false;
        self.input_mode = DebugTextInputMode::TargetFilter;
        self.target_filter_buffer = self.target_filter.clone();
        self.preedit = None;
        self.history_index = None;
    }

    pub fn is_text_input_active(&self) -> bool {
        self.input_mode != DebugTextInputMode::None
    }

    pub fn handle_key_event(&mut self, event: &KeyEvent) -> bool {
        let key = match event.physical_key {
            PhysicalKey::Code(code) => code,
            _ => return self.is_text_input_active(),
        };

        if event.state == ElementState::Pressed {
            if key == KeyCode::F3 {
                self.toggle_overlay();
                return true;
            }
            if key == KeyCode::F4 || key == KeyCode::Backquote {
                self.toggle_console();
                return true;
            }
        }

        if self.is_text_input_active() {
            if event.state == ElementState::Released {
                return true;
            }

            return match key {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    self.submit_text_input();
                    true
                }
                KeyCode::Escape => {
                    self.cancel_text_input();
                    true
                }
                KeyCode::Backspace => {
                    self.active_buffer_backspace();
                    true
                }
                KeyCode::ArrowUp if self.input_mode == DebugTextInputMode::Console => {
                    self.console_history_up();
                    true
                }
                KeyCode::ArrowDown if self.input_mode == DebugTextInputMode::Console => {
                    self.console_history_down();
                    true
                }
                _ => true,
            };
        }

        if event.state != ElementState::Pressed || !self.overlay_visible {
            return false;
        }

        match key {
            KeyCode::F5 => {
                clear_logs();
                self.scroll_to_latest();
                true
            }
            KeyCode::F6 => {
                self.toggle_follow_logs();
                true
            }
            KeyCode::F7 => {
                self.cycle_severity_filter();
                true
            }
            KeyCode::F8 => {
                self.begin_target_filter_edit();
                true
            }
            KeyCode::PageUp => {
                self.scroll_up(DEFAULT_SCROLL_STEP);
                true
            }
            KeyCode::PageDown => {
                self.scroll_down(DEFAULT_SCROLL_STEP);
                true
            }
            KeyCode::Home => {
                self.scroll_to_oldest();
                true
            }
            KeyCode::End => {
                self.set_follow_logs(true);
                self.scroll_to_latest();
                true
            }
            _ => false,
        }
    }

    pub fn handle_committed_text(&mut self, text: &str) -> bool {
        if !self.is_text_input_active() {
            return false;
        }

        for ch in text.chars() {
            if ch.is_control() {
                continue;
            }
            self.active_buffer_mut().push(ch);
        }
        true
    }

    pub fn handle_ime_event(&mut self, ime: &Ime) -> bool {
        if !self.is_text_input_active() {
            return false;
        }

        match ime {
            Ime::Enabled => {}
            Ime::Preedit(text, cursor) => {
                if text.is_empty() {
                    self.preedit = None;
                } else {
                    self.preedit = Some((text.clone(), *cursor));
                }
            }
            Ime::Commit(text) => {
                self.preedit = None;
                self.handle_committed_text(text);
            }
            Ime::Disabled => {
                self.preedit = None;
            }
        }

        true
    }

    pub fn handle_scroll(
        &mut self,
        screen_size: (u32, u32),
        pointer: (f32, f32),
        dy: f32,
    ) -> bool {
        if !self.overlay_visible || dy.abs() < f32::EPSILON {
            return false;
        }

        let pointer_over_debug = self.pointer_hits_overlay(screen_size, pointer)
            || self.pointer_hits_console(screen_size, pointer);
        if !pointer_over_debug {
            return self.is_text_input_active();
        }
        if self.is_text_input_active() {
            return true;
        }

        let amount = dy.abs().ceil() as usize;
        if dy > 0.0 {
            self.scroll_up(amount.max(1));
        } else {
            self.scroll_down(amount.max(1));
        }
        true
    }

    pub fn handle_mouse_button(
        &mut self,
        screen_size: (u32, u32),
        pointer: (f32, f32),
        button: usize,
        state: ElementState,
    ) -> bool {
        if button >= self.captured_mouse_buttons.len() {
            return false;
        }

        match state {
            ElementState::Pressed => {
                let consume = self.is_text_input_active()
                    || self.pointer_hits_overlay(screen_size, pointer)
                    || self.pointer_hits_console(screen_size, pointer);

                if consume {
                    self.captured_mouse_buttons[button] = true;
                    if button == 0 {
                        self.handle_left_mouse_press(screen_size, pointer);
                    }
                }

                consume
            }
            ElementState::Released => {
                let consume = self.captured_mouse_buttons[button];
                self.captured_mouse_buttons[button] = false;
                consume
            }
        }
    }

    pub fn drain_pending_commands(&mut self) -> Vec<String> {
        self.pending_commands.drain(..).collect()
    }

    pub fn filtered_logs(&self, limit: usize) -> (Vec<DebugLogEntry>, usize) {
        let filtered = self.filtered_snapshot();
        let total = filtered.len();
        if total == 0 {
            return (Vec::new(), 0);
        }

        let offset = if self.follow_logs {
            0
        } else {
            self.scroll_offset.min(total.saturating_sub(1))
        };
        let end = total.saturating_sub(offset);
        let start = end.saturating_sub(limit.max(1));
        (filtered[start..end].to_vec(), total)
    }

    pub fn filtered_log_count(&self) -> usize {
        self.filtered_snapshot().len()
    }

    pub fn visible_log_limit(&self) -> usize {
        if self.console_open {
            DEFAULT_OVERLAY_LOG_LIMIT.saturating_sub(2).max(4)
        } else {
            DEFAULT_OVERLAY_LOG_LIMIT
        }
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn scroll_to_latest(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_oldest(&mut self) {
        self.follow_logs = false;
        self.scroll_offset = self.max_scroll_offset();
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.follow_logs = false;
        self.scroll_offset = (self.scroll_offset + amount).min(self.max_scroll_offset());
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.scroll_offset == 0 {
            self.follow_logs = true;
        }
    }

    pub fn console_buffer(&self) -> &str {
        &self.console_buffer
    }

    pub fn target_filter_buffer(&self) -> &str {
        &self.target_filter_buffer
    }

    pub fn preedit(&self) -> Option<(&str, Option<(usize, usize)>)> {
        self.preedit
            .as_ref()
            .map(|(text, cursor)| (text.as_str(), *cursor))
    }

    fn active_buffer_mut(&mut self) -> &mut String {
        match self.input_mode {
            DebugTextInputMode::Console => &mut self.console_buffer,
            DebugTextInputMode::TargetFilter => &mut self.target_filter_buffer,
            DebugTextInputMode::None => unreachable!(),
        }
    }

    fn active_buffer_backspace(&mut self) {
        self.active_buffer_mut().pop();
    }

    fn cancel_text_input(&mut self) {
        self.preedit = None;
        self.history_index = None;
        match self.input_mode {
            DebugTextInputMode::Console => self.set_console_open(false),
            DebugTextInputMode::TargetFilter => {
                self.target_filter_buffer = self.target_filter.clone();
                self.input_mode = DebugTextInputMode::None;
            }
            DebugTextInputMode::None => {}
        }
    }

    fn submit_text_input(&mut self) {
        self.preedit = None;
        self.history_index = None;
        match self.input_mode {
            DebugTextInputMode::Console => {
                let command = self.console_buffer.trim().to_string();
                if !command.is_empty() {
                    let should_push = match self.command_history.back() {
                        Some(last) => last != &command,
                        None => true,
                    };
                    if should_push {
                        self.command_history.push_back(command.clone());
                        if self.command_history.len() > MAX_COMMAND_HISTORY {
                            self.command_history.pop_front();
                        }
                    }
                    self.pending_commands.push_back(command);
                }
                self.console_buffer.clear();
            }
            DebugTextInputMode::TargetFilter => {
                self.target_filter = self.target_filter_buffer.trim().to_string();
                self.input_mode = DebugTextInputMode::None;
            }
            DebugTextInputMode::None => {}
        }
    }

    fn console_history_up(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(index) if index > 0 => index - 1,
            Some(index) => index,
            None => self.command_history.len() - 1,
        };
        self.history_index = Some(next_index);
        self.console_buffer = self.command_history[next_index].clone();
    }

    fn console_history_down(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        match self.history_index {
            Some(index) if index + 1 < self.command_history.len() => {
                let next_index = index + 1;
                self.history_index = Some(next_index);
                self.console_buffer = self.command_history[next_index].clone();
            }
            Some(_) => {
                self.history_index = None;
                self.console_buffer.clear();
            }
            None => {}
        }
    }

    fn filtered_snapshot(&self) -> Vec<DebugLogEntry> {
        let filter = self.target_filter.trim();
        snapshot_logs()
            .into_iter()
            .filter(|entry| self.severity_filter.allows(entry.level))
            .filter(|entry| {
                if filter.is_empty() {
                    return true;
                }

                contains_ascii_case_insensitive(&entry.target, filter)
                    || contains_ascii_case_insensitive(&entry.message, filter)
            })
            .collect()
    }

    fn max_scroll_offset(&self) -> usize {
        self.filtered_log_count()
            .saturating_sub(self.visible_log_limit())
    }

    fn handle_left_mouse_press(&mut self, screen_size: (u32, u32), pointer: (f32, f32)) {
        let layout = self.overlay_layout(screen_size);
        for button in layout.buttons {
            if button.rect.contains(pointer) {
                match button.action {
                    OverlayButtonAction::Clear => {
                        clear_logs();
                        self.scroll_to_latest();
                    }
                    OverlayButtonAction::ToggleConsole => self.toggle_console(),
                    OverlayButtonAction::ToggleFollow => self.toggle_follow_logs(),
                    OverlayButtonAction::CycleSeverity => self.cycle_severity_filter(),
                    OverlayButtonAction::EditTargetFilter => {
                        if self.input_mode == DebugTextInputMode::TargetFilter {
                            self.submit_text_input();
                        } else {
                            self.begin_target_filter_edit();
                        }
                    }
                }
                return;
            }
        }
    }

    fn pointer_hits_overlay(&self, screen_size: (u32, u32), pointer: (f32, f32)) -> bool {
        self.overlay_visible && self.overlay_layout(screen_size).panel.contains(pointer)
    }

    fn pointer_hits_console(&self, screen_size: (u32, u32), pointer: (f32, f32)) -> bool {
        self.console_open && console_panel_rect(screen_size).contains(pointer)
    }

    fn overlay_layout(&self, screen_size: (u32, u32)) -> OverlayLayout {
        let hw = screen_size.0 as f32 / 2.0;
        let hh = screen_size.1 as f32 / 2.0;
        let padding = 10.0;
        let panel_x = -hw + 8.0;
        let panel_top = hh - 8.0;
        let panel_width = (screen_size.0 as f32 - 16.0).max(220.0).min(620.0);

        let title_line_height = debug_line_height(OVERLAY_TITLE_SIZE);
        let body_line_height = debug_line_height(OVERLAY_BODY_SIZE);
        let hint_line_height = debug_line_height(OVERLAY_HINT_SIZE);
        let visible_log_lines = self.filtered_logs(self.visible_log_limit()).0.len().max(1);

        let buttons = build_overlay_buttons(
            self,
            panel_x + padding,
            panel_x + panel_width - padding,
            panel_top - padding - title_line_height - 6.0,
        );
        let button_rows = buttons
            .iter()
            .map(|button| button.row + 1)
            .max()
            .unwrap_or(0);
        let button_block_height = if button_rows == 0 {
            0.0
        } else {
            button_rows as f32 * OVERLAY_BUTTON_HEIGHT
                + button_rows.saturating_sub(1) as f32 * OVERLAY_BUTTON_GAP
                + 8.0
        };
        let input_line_height = if self.input_mode == DebugTextInputMode::TargetFilter {
            body_line_height + 6.0
        } else {
            0.0
        };
        let panel_height = padding * 2.0
            + title_line_height
            + 6.0
            + button_block_height
            + 4.0 * body_line_height
            + 8.0
            + (visible_log_lines as f32 + 1.0) * body_line_height
            + 8.0
            + hint_line_height * 2.0
            + input_line_height;

        OverlayLayout {
            panel: DebugRect {
                x: panel_x,
                y: panel_top - panel_height,
                w: panel_width,
                h: panel_height,
            },
            buttons,
            stats_y: panel_top - padding - title_line_height - 6.0 - button_block_height,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DebugRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl DebugRect {
    fn contains(self, point: (f32, f32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.w
            && point.1 >= self.y
            && point.1 <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverlayButtonAction {
    Clear,
    ToggleConsole,
    ToggleFollow,
    CycleSeverity,
    EditTargetFilter,
}

#[derive(Clone, Debug)]
struct OverlayButtonLayout {
    rect: DebugRect,
    action: OverlayButtonAction,
    label: String,
    active: bool,
    row: usize,
}

#[derive(Clone, Debug)]
struct OverlayLayout {
    panel: DebugRect,
    buttons: Vec<OverlayButtonLayout>,
    stats_y: f32,
}

#[derive(Debug)]
struct DebugLogBuffer {
    entries: VecDeque<DebugLogEntry>,
    capacity: usize,
    next_index: u64,
}

impl DebugLogBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity: capacity.max(1),
            next_index: 0,
        }
    }

    fn push_entry(&mut self, mut entry: DebugLogEntry) {
        entry.index = self.next_index;
        self.next_index += 1;

        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    fn push_record(&mut self, elapsed_seconds: f32, record: &log::Record<'_>) {
        self.push_entry(DebugLogEntry {
            index: 0,
            seconds: elapsed_seconds,
            level: record.level().into(),
            target: record.target().to_string(),
            message: record.args().to_string(),
        });
    }

    fn recent(&self, limit: usize) -> Vec<DebugLogEntry> {
        let start = self.entries.len().saturating_sub(limit);
        self.entries.iter().skip(start).cloned().collect()
    }

    fn snapshot(&self) -> Vec<DebugLogEntry> {
        self.entries.iter().cloned().collect()
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }
}

struct CombinedLogger {
    inner: env_logger::Logger,
}

impl log::Log for CombinedLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        log::Log::enabled(&self.inner, metadata)
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        log::Log::log(&self.inner, record);

        let mut buffer = log_buffer()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        buffer.push_record(log_start().elapsed().as_secs_f32(), record);
    }

    fn flush(&self) {
        log::Log::flush(&self.inner);
    }
}

static LOGGER_INIT: Once = Once::new();
static LOGGER_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static LOG_BUFFER: OnceLock<Mutex<DebugLogBuffer>> = OnceLock::new();
static LOG_START: OnceLock<Instant> = OnceLock::new();

fn log_buffer() -> &'static Mutex<DebugLogBuffer> {
    LOG_BUFFER.get_or_init(|| Mutex::new(DebugLogBuffer::new(DEFAULT_LOG_CAPACITY)))
}

fn log_start() -> &'static Instant {
    LOG_START.get_or_init(Instant::now)
}

pub fn init_logging() {
    log_buffer();
    log_start();

    LOGGER_INIT.call_once(|| {
        let mut builder = env_logger::Builder::from_default_env();
        builder.format_timestamp_millis();

        let inner = builder.build();
        let max_level = inner.filter();
        let logger = CombinedLogger { inner };

        match log::set_boxed_logger(Box::new(logger)) {
            Ok(()) => {
                LOGGER_CAPTURE_ACTIVE.store(true, Ordering::Relaxed);
                log::set_max_level(max_level);
            }
            Err(err) => {
                eprintln!("failed to initialize debug logger: {err}");
            }
        }
    });
}

pub fn recent_logs(limit: usize) -> Vec<DebugLogEntry> {
    let buffer = log_buffer()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.recent(limit)
}

pub fn snapshot_logs() -> Vec<DebugLogEntry> {
    let buffer = log_buffer()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.snapshot()
}

pub fn clear_logs() {
    let mut buffer = log_buffer()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.clear();
}

pub fn log_count() -> usize {
    let buffer = log_buffer()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.len()
}

pub fn log_capacity() -> usize {
    let buffer = log_buffer()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.capacity()
}

pub fn set_log_capacity(capacity: usize) {
    let mut buffer = log_buffer()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.set_capacity(capacity);
}

pub fn log_message(level: DebugLogLevel, target: &str, message: &str) {
    init_logging();

    if !LOGGER_CAPTURE_ACTIVE.load(Ordering::Relaxed) {
        let mut buffer = log_buffer()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        buffer.push_entry(DebugLogEntry {
            index: 0,
            seconds: log_start().elapsed().as_secs_f32(),
            level,
            target: target.to_string(),
            message: message.to_string(),
        });
    }

    match level {
        DebugLogLevel::Trace => log::trace!(target: target, "{message}"),
        DebugLogLevel::Debug => log::debug!(target: target, "{message}"),
        DebugLogLevel::Info => log::info!(target: target, "{message}"),
        DebugLogLevel::Warn => log::warn!(target: target, "{message}"),
        DebugLogLevel::Error => log::error!(target: target, "{message}"),
    }
}

pub fn parse_command(text: &str) -> Result<DebugCommand, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty command".into());
    }

    let split_at = trimmed.find(char::is_whitespace);
    let (command, rest) = match split_at {
        Some(index) => (&trimmed[..index], trimmed[index..].trim()),
        None => (trimmed, ""),
    };
    let command = command.to_ascii_lowercase();

    match command.as_str() {
        "help" | "?" => Ok(DebugCommand::Help),
        "state" => Ok(DebugCommand::State),
        "clear" => Ok(DebugCommand::Clear),
        "overlay" => Ok(DebugCommand::Overlay(DebugToggle::parse(non_empty(rest))?)),
        "console" => Ok(DebugCommand::Console(DebugToggle::parse(non_empty(rest))?)),
        "follow" => Ok(DebugCommand::Follow(DebugToggle::parse(non_empty(rest))?)),
        "level" | "severity" => {
            let value = match non_empty(rest) {
                Some(value) => value,
                None => {
                    return Err(
                        "level command expects one of: all, debug, info, warn, error".into(),
                    )
                }
            };
            let filter = match DebugSeverityFilter::parse(value) {
                Some(filter) => filter,
                None => return Err(format!("unknown severity filter '{value}'")),
            };
            Ok(DebugCommand::Level(filter))
        }
        "target" => {
            let value = rest.trim();
            if value.is_empty() || value.eq_ignore_ascii_case("clear") {
                Ok(DebugCommand::Target(None))
            } else {
                Ok(DebugCommand::Target(Some(value.to_string())))
            }
        }
        "hot_reload" | "hotreload" => {
            Ok(DebugCommand::HotReload(DebugToggle::parse(non_empty(rest))?))
        }
        "echo" => parse_echo_command(rest, DebugLogLevel::Info),
        "debug" => parse_echo_command(rest, DebugLogLevel::Debug),
        "info" => parse_echo_command(rest, DebugLogLevel::Info),
        "warn" | "warning" => parse_echo_command(rest, DebugLogLevel::Warn),
        "error" => parse_echo_command(rest, DebugLogLevel::Error),
        _ => Err(format!("unknown debug command '{command}'")),
    }
}

pub fn command_help_lines() -> &'static [&'static str] {
    &[
        "help | state | clear",
        "overlay [on|off|toggle] | console [on|off|toggle] | follow [on|off|toggle]",
        "level <all|debug|info|warn|error> | target <text> | target clear",
        "hot_reload [on|off|toggle]",
        "echo <msg> | debug <msg> | info <msg> | warn <msg> | error <msg>",
    ]
}

pub fn draw_overlay(
    canvas: &mut Canvas,
    info: &DebugOverlayInfo<'_>,
    state: &DebugUiState,
    mouse_position: Option<(f32, f32)>,
) {
    let visible_limit = state.visible_log_limit();
    let (logs, filtered_count) = state.filtered_logs(visible_limit);
    let screen_size = canvas.screen_size();
    let padding = 10.0;
    let body_line_height = debug_line_height(OVERLAY_BODY_SIZE);
    let hint_line_height = debug_line_height(OVERLAY_HINT_SIZE);
    let layout = state.overlay_layout(screen_size);
    let panel_x = layout.panel.x;
    let panel_y = layout.panel.y;
    let panel_width = layout.panel.w;
    let panel_top = panel_y + layout.panel.h;
    let stats = overlay_stats(info, state, filtered_count);

    canvas.rect(
        panel_x,
        panel_y,
        panel_width,
        layout.panel.h,
        Color::from_rgba8(8, 12, 18, 220),
    );
    canvas.rect(
        panel_x,
        panel_top - 30.0,
        panel_width,
        30.0,
        Color::from_rgba8(18, 28, 42, 240),
    );

    let title_y = panel_top - padding;
    canvas.text(
        panel_x + padding,
        title_y,
        "DEBUG OVERLAY",
        OVERLAY_TITLE_SIZE,
        Color::from_rgba8(122, 214, 255, 255),
    );
    canvas.text(
        panel_x + panel_width - 210.0,
        title_y,
        "click chips | F4 console | F5 clear",
        OVERLAY_HINT_SIZE,
        Color::from_rgba8(146, 163, 186, 255),
    );

    for button in &layout.buttons {
        let hovered = mouse_position.is_some_and(|pointer| button.rect.contains(pointer));
        let fill = overlay_button_fill(button, hovered);
        let text_color = overlay_button_text_color(button, hovered);
        canvas.rect(button.rect.x, button.rect.y, button.rect.w, button.rect.h, fill);
        canvas.text(
            button.rect.x + OVERLAY_BUTTON_PADDING_X,
            button.rect.y + OVERLAY_BUTTON_HEIGHT - 5.0,
            &button.label,
            OVERLAY_BUTTON_TEXT_SIZE,
            text_color,
        );
    }

    let mut y = layout.stats_y;
    for stat in stats {
        canvas.text(
            panel_x + padding,
            y,
            &stat,
            OVERLAY_BODY_SIZE,
            Color::from_rgba8(218, 224, 236, 255),
        );
        y -= body_line_height;
    }

    if state.input_mode == DebugTextInputMode::TargetFilter {
        y -= 4.0;
        canvas.text(
            panel_x + padding,
            y,
            &format!(
                "Target filter> {}_{}",
                state.target_filter_buffer(),
                format_preedit_suffix(state.preedit())
            ),
            OVERLAY_BODY_SIZE,
            Color::from_rgba8(188, 222, 255, 255),
        );
        y -= body_line_height;
    }

    y -= 4.0;
    canvas.text(
        panel_x + padding,
        y,
        &format!(
            "Recent logs ({filtered_count} filtered / {} total / {} capacity)",
            log_count(),
            log_capacity()
        ),
        OVERLAY_BODY_SIZE,
        Color::from_rgba8(146, 163, 186, 255),
    );

    y -= body_line_height;
    if logs.is_empty() {
        canvas.text(
            panel_x + padding,
            y,
            "No log entries captured for the current filters.",
            OVERLAY_BODY_SIZE,
            Color::from_rgba8(146, 163, 186, 255),
        );
        y -= body_line_height + 6.0;
    } else {
        let max_chars = (((panel_width - padding * 2.0) / 6.1).floor() as usize).max(24);
        for entry in logs.iter().rev() {
            let line = truncate_chars(&format_overlay_entry(entry), max_chars);
            canvas.text(
                panel_x + padding,
                y,
                &line,
                OVERLAY_BODY_SIZE,
                entry.level.color(),
            );
            y -= body_line_height;
        }
    }

    y -= 2.0;
    canvas.text(
        panel_x + padding,
        y,
        "Mouse wheel scrolls logs | F6 follow | F7 severity | F8 target",
        OVERLAY_HINT_SIZE,
        Color::from_rgba8(146, 163, 186, 255),
    );
    y -= hint_line_height;
    canvas.text(
        panel_x + padding,
        y,
        "PgUp/PgDn/Home/End still work. Console commands feed back into this log buffer.",
        OVERLAY_HINT_SIZE,
        Color::from_rgba8(122, 132, 148, 255),
    );
}

pub fn draw_console(canvas: &mut Canvas, state: &DebugUiState) {
    if !state.console_open() {
        return;
    }

    let panel = console_panel_rect(canvas.screen_size());
    let padding = 10.0;
    let body_size = 12.0;
    let hint_size = 10.0;
    let line_height = debug_line_height(body_size);

    canvas.rect(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba8(10, 14, 22, 238),
    );
    canvas.rect(
        panel.x,
        panel.y + panel.h - 26.0,
        panel.w,
        26.0,
        Color::from_rgba8(20, 30, 46, 244),
    );

    canvas.text(
        panel.x + padding,
        panel.y + 18.0,
        "Developer Console",
        13.0,
        Color::from_rgba8(122, 214, 255, 255),
    );
    canvas.text(
        panel.x + padding,
        panel.y + 32.0,
        "Examples: state | level warn | target renderer | hot_reload toggle | clear",
        hint_size,
        Color::from_rgba8(146, 163, 186, 255),
    );

    let prompt = format!(
        "> {}_{}",
        state.console_buffer(),
        format_preedit_suffix(state.preedit())
    );
    canvas.text(
        panel.x + padding,
        panel.y + panel.h - 8.0,
        &prompt,
        body_size,
        Color::from_rgba8(230, 236, 245, 255),
    );
    canvas.text(
        panel.x + padding,
        panel.y + panel.h - 8.0 - line_height - 4.0,
        "Enter run | Up/Down history | Esc close | click overlay chips for mouse controls",
        hint_size,
        Color::from_rgba8(156, 168, 188, 255),
    );
}

fn build_overlay_buttons(
    state: &DebugUiState,
    left: f32,
    right: f32,
    top: f32,
) -> Vec<OverlayButtonLayout> {
    let specs = overlay_button_specs(state);
    let mut buttons = Vec::with_capacity(specs.len());
    let mut cursor_x = left;
    let mut row = 0usize;

    for (action, label, active) in specs {
        let width = overlay_button_width(&label);
        if cursor_x > left && cursor_x + width > right {
            row += 1;
            cursor_x = left;
        }

        let rect = DebugRect {
            x: cursor_x,
            y: top - OVERLAY_BUTTON_HEIGHT - row as f32 * (OVERLAY_BUTTON_HEIGHT + OVERLAY_BUTTON_GAP),
            w: width,
            h: OVERLAY_BUTTON_HEIGHT,
        };
        buttons.push(OverlayButtonLayout {
            rect,
            action,
            label,
            active,
            row,
        });
        cursor_x += width + OVERLAY_BUTTON_GAP;
    }

    buttons
}

fn overlay_button_specs(state: &DebugUiState) -> Vec<(OverlayButtonAction, String, bool)> {
    let target_label = if state.input_mode == DebugTextInputMode::TargetFilter {
        "target: apply".to_string()
    } else if state.target_filter().is_empty() {
        "target: *".to_string()
    } else {
        format!("target: {}", truncate_chars(state.target_filter(), 12))
    };

    vec![
        (OverlayButtonAction::Clear, "clear logs".to_string(), false),
        (
            OverlayButtonAction::ToggleConsole,
            format!("console: {}", if state.console_open() { "open" } else { "closed" }),
            state.console_open(),
        ),
        (
            OverlayButtonAction::ToggleFollow,
            format!("follow: {}", if state.follow_logs() { "live" } else { "paused" }),
            state.follow_logs(),
        ),
        (
            OverlayButtonAction::CycleSeverity,
            format!("level: {}", state.severity_filter().label()),
            state.severity_filter() != DebugSeverityFilter::All,
        ),
        (
            OverlayButtonAction::EditTargetFilter,
            target_label,
            state.input_mode == DebugTextInputMode::TargetFilter || !state.target_filter().is_empty(),
        ),
    ]
}

fn overlay_button_width(label: &str) -> f32 {
    (approx_text_width(label, OVERLAY_BUTTON_TEXT_SIZE) + OVERLAY_BUTTON_PADDING_X * 2.0)
        .max(OVERLAY_BUTTON_MIN_WIDTH)
}

fn overlay_button_fill(button: &OverlayButtonLayout, hovered: bool) -> Color {
    match button.action {
        OverlayButtonAction::Clear => {
            if hovered {
                Color::from_rgba8(98, 36, 42, 244)
            } else {
                Color::from_rgba8(74, 28, 34, 232)
            }
        }
        _ if button.active && hovered => Color::from_rgba8(44, 84, 122, 244),
        _ if button.active => Color::from_rgba8(34, 66, 98, 236),
        _ if hovered => Color::from_rgba8(34, 42, 58, 236),
        _ => Color::from_rgba8(22, 28, 38, 228),
    }
}

fn overlay_button_text_color(button: &OverlayButtonLayout, hovered: bool) -> Color {
    match button.action {
        OverlayButtonAction::Clear => {
            if hovered {
                Color::from_rgba8(255, 220, 220, 255)
            } else {
                Color::from_rgba8(248, 196, 196, 255)
            }
        }
        _ if button.active => Color::from_rgba8(236, 242, 250, 255),
        _ if hovered => Color::from_rgba8(220, 228, 240, 255),
        _ => Color::from_rgba8(176, 188, 206, 255),
    }
}

fn console_panel_rect(screen_size: (u32, u32)) -> DebugRect {
    let hw = screen_size.0 as f32 / 2.0;
    let hh = screen_size.1 as f32 / 2.0;
    DebugRect {
        x: -hw + 8.0,
        y: -hh + 8.0,
        w: screen_size.0 as f32 - 16.0,
        h: CONSOLE_PANEL_HEIGHT,
    }
}

fn debug_line_height(size: f32) -> f32 {
    size * 1.35
}

fn approx_text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.58
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    let needle_bytes = needle.as_bytes();
    if needle_bytes.is_empty() {
        return true;
    }

    haystack
        .as_bytes()
        .windows(needle_bytes.len())
        .any(|window| window.eq_ignore_ascii_case(needle_bytes))
}

fn overlay_stats(
    info: &DebugOverlayInfo<'_>,
    state: &DebugUiState,
    filtered_count: usize,
) -> Vec<String> {
    let mut lines = Vec::with_capacity(5);
    lines.push(format!(
        "{} | {:.0} FPS | {:.2} ms | frame {} | {:.1}s",
        info.mode,
        info.fps,
        info.dt * 1000.0,
        info.frame_count,
        info.total_time,
    ));
    lines.push(format!(
        "window {}x{} | game {}x{} | hot reload {}",
        info.window_size.0,
        info.window_size.1,
        info.game_size.0,
        info.game_size.1,
        if info.hot_reload_enabled { "on" } else { "off" },
    ));
    lines.push(format!(
        "level {} | target {} | follow {} | scroll {} | visible {}",
        state.severity_filter().label(),
        if state.target_filter().is_empty() {
            "*"
        } else {
            state.target_filter()
        },
        if state.follow_logs() { "live" } else { "paused" },
        state.scroll_offset(),
        filtered_count,
    ));

    if let Some(gamepads) = info.gamepads_connected {
        lines.push(format!("gamepads connected {} | F3 toggles overlay", gamepads));
    } else if let Some(mouse_captured) = info.mouse_captured {
        lines.push(format!(
            "mouse captured {} | F3 toggles overlay",
            if mouse_captured { "yes" } else { "no" }
        ));
    } else {
        lines.push("F3 toggles overlay".to_string());
    }

    lines
}

fn format_overlay_entry(entry: &DebugLogEntry) -> String {
    format!(
        "{:>5.1}s {:<5} {:<18} {}",
        entry.seconds,
        entry.level.label(),
        compact_target(&entry.target),
        entry.message,
    )
}

fn compact_target(target: &str) -> String {
    let mut parts = target.rsplit("::");
    match (parts.next(), parts.next()) {
        (Some(last), Some(prev)) => format!("{}::{}", prev, last),
        (Some(last), None) => last.to_string(),
        _ => "log".to_string(),
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut out: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

fn non_empty(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_echo_command(rest: &str, level: DebugLogLevel) -> Result<DebugCommand, String> {
    let message = rest.trim();
    if message.is_empty() {
        Err("message cannot be empty".into())
    } else {
        Ok(DebugCommand::Echo(level, message.to_string()))
    }
}

fn format_preedit_suffix(preedit: Option<(&str, Option<(usize, usize)>)>) -> String {
    match preedit {
        Some((text, _)) => format!(" [{text}]"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_discards_oldest_entries() {
        let mut buffer = DebugLogBuffer::new(2);
        buffer.push_entry(DebugLogEntry {
            index: 0,
            seconds: 0.1,
            level: DebugLogLevel::Info,
            target: "alpha".into(),
            message: "one".into(),
        });
        buffer.push_entry(DebugLogEntry {
            index: 0,
            seconds: 0.2,
            level: DebugLogLevel::Warn,
            target: "beta".into(),
            message: "two".into(),
        });
        buffer.push_entry(DebugLogEntry {
            index: 0,
            seconds: 0.3,
            level: DebugLogLevel::Error,
            target: "gamma".into(),
            message: "three".into(),
        });

        let recent = buffer.recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message, "two");
        assert_eq!(recent[1].message, "three");
        assert_eq!(recent[0].index, 1);
        assert_eq!(recent[1].index, 2);
    }

    #[test]
    fn compact_target_keeps_last_segments() {
        assert_eq!(compact_target("rengine::renderer::mod"), "renderer::mod");
        assert_eq!(compact_target("gameplay"), "gameplay");
    }

    #[test]
    fn truncate_chars_adds_ellipsis() {
        assert_eq!(truncate_chars("abcdef", 6), "abcdef");
        assert_eq!(truncate_chars("abcdefgh", 6), "abc...");
    }

    #[test]
    fn parse_debug_commands() {
        assert_eq!(parse_command("help").unwrap(), DebugCommand::Help);
        assert_eq!(
            parse_command("level warn").unwrap(),
            DebugCommand::Level(DebugSeverityFilter::Warn)
        );
        assert_eq!(
            parse_command("target renderer").unwrap(),
            DebugCommand::Target(Some("renderer".into()))
        );
        assert_eq!(parse_command("target clear").unwrap(), DebugCommand::Target(None));
    }

    #[test]
    fn severity_filter_blocks_lower_levels() {
        assert!(DebugSeverityFilter::Warn.allows(DebugLogLevel::Warn));
        assert!(DebugSeverityFilter::Warn.allows(DebugLogLevel::Error));
        assert!(!DebugSeverityFilter::Warn.allows(DebugLogLevel::Info));
    }

    #[test]
    fn console_submit_queues_command() {
        let mut state = DebugUiState::new(true);
        state.set_console_open(true);
        state.handle_committed_text("state");
        state.submit_text_input();

        let commands = state.drain_pending_commands();
        assert_eq!(commands, vec!["state"]);
    }

    #[test]
    fn mouse_click_clear_button_clears_logs() {
        {
            let mut buffer = log_buffer()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            buffer.clear();
            buffer.push_entry(DebugLogEntry {
                index: 0,
                seconds: 0.1,
                level: DebugLogLevel::Info,
                target: "debug".into(),
                message: "entry".into(),
            });
        }

        let mut state = DebugUiState::new(true);
        let clear_button = state
            .overlay_layout((800, 600))
            .buttons
            .into_iter()
            .find(|button| button.action == OverlayButtonAction::Clear)
            .unwrap();
        let pointer = (
            clear_button.rect.x + clear_button.rect.w * 0.5,
            clear_button.rect.y + clear_button.rect.h * 0.5,
        );

        assert!(state.handle_mouse_button((800, 600), pointer, 0, ElementState::Pressed));
        assert!(state.handle_mouse_button((800, 600), pointer, 0, ElementState::Released));
        assert_eq!(log_count(), 0);
    }

    #[test]
    fn mouse_click_level_button_cycles_severity() {
        let mut state = DebugUiState::new(true);
        let level_button = state
            .overlay_layout((800, 600))
            .buttons
            .into_iter()
            .find(|button| button.action == OverlayButtonAction::CycleSeverity)
            .unwrap();
        let pointer = (
            level_button.rect.x + level_button.rect.w * 0.5,
            level_button.rect.y + level_button.rect.h * 0.5,
        );

        assert_eq!(state.severity_filter(), DebugSeverityFilter::All);
        assert!(state.handle_mouse_button((800, 600), pointer, 0, ElementState::Pressed));
        assert!(state.handle_mouse_button((800, 600), pointer, 0, ElementState::Released));
        assert_eq!(state.severity_filter(), DebugSeverityFilter::Debug);
    }

    #[test]
    fn target_filter_matches_case_insensitively_without_lowercasing_entries() {
        {
            let mut buffer = log_buffer()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            buffer.clear();
            buffer.push_entry(DebugLogEntry {
                index: 0,
                seconds: 0.1,
                level: DebugLogLevel::Info,
                target: "Renderer::Mesh".into(),
                message: "Frame Presented".into(),
            });
        }

        let mut state = DebugUiState::new(true);
        state.set_target_filter("renderER");

        let (logs, filtered_count) = state.filtered_logs(10);
        assert_eq!(filtered_count, 1);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].target, "Renderer::Mesh");
    }

    #[test]
    fn scroll_only_consumes_when_pointer_is_over_debug_ui_or_text_input_is_active() {
        let mut state = DebugUiState::new(true);
        let layout = state.overlay_layout((800, 600));
        let inside_overlay = (layout.panel.x + 12.0, layout.panel.y + 12.0);
        let outside_overlay = (380.0, -280.0);

        assert!(!state.handle_scroll((800, 600), outside_overlay, 1.0));
        assert!(state.handle_scroll((800, 600), inside_overlay, 1.0));

        state.begin_target_filter_edit();
        assert!(state.handle_scroll((800, 600), outside_overlay, 1.0));
    }
}
