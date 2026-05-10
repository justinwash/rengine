use super::*;
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub(crate) struct FileBrowserFormState {
    pub(crate) filter: String,
}

#[derive(Default)]
pub(crate) struct InspectorFormState {
    pub(crate) context_tab: usize,
    pub(crate) context_node: Option<u64>,
    pub(crate) scene_name: String,
    pub(crate) scene_window_width: String,
    pub(crate) scene_window_height: String,
    pub(crate) selected_node_kind: Option<SceneNodeKind>,
    pub(crate) node_name: String,
    pub(crate) node_visible: bool,
    pub(crate) node_position_x: String,
    pub(crate) node_position_y: String,
    pub(crate) node_size_width: String,
    pub(crate) node_size_height: String,
    pub(crate) script_path: String,
    pub(crate) runtime_prefab: String,
    pub(crate) asset_alias: String,
    pub(crate) sprite_texture_path: String,
    pub(crate) camera_zoom: f32,
    pub(crate) camera_show_bounds: bool,
    pub(crate) camera_use_scene_view_size: bool,
    pub(crate) camera_view_width: String,
    pub(crate) camera_view_height: String,
}

impl InspectorFormState {
    pub(crate) fn sync_from_editor(&mut self, editor: &RengineNativeEditor) {
        let tab = editor.active_scene_tab();
        self.context_tab = editor.active_scene_tab;
        self.context_node = tab.selected_node;
        self.scene_name = tab.scene.name.clone();
        self.scene_window_width = format!("{:.0}", tab.scene.view.window_size[0]);
        self.scene_window_height = format!("{:.0}", tab.scene.view.window_size[1]);

        if let Some(node_id) = tab.selected_node {
            if let Some(node) = tab.scene.node(node_id) {
                self.selected_node_kind = Some(node.kind);
                self.node_name = node.name.clone();
                self.node_visible = node.visible;
                self.node_position_x = format!("{:.0}", node.position[0]);
                self.node_position_y = format!("{:.0}", node.position[1]);
                self.node_size_width = format!("{:.0}", node.size[0]);
                self.node_size_height = format!("{:.0}", node.size[1]);
                self.script_path = node.script_path.clone();
                self.runtime_prefab = node.runtime_prefab.clone();
                self.asset_alias = node.asset_alias.clone();
                self.sprite_texture_path = node.sprite.texture_path.clone();
                self.camera_zoom = node.camera2d.zoom;
                self.camera_show_bounds = node.camera2d.show_bounds;
                self.camera_use_scene_view_size = node.camera2d.use_scene_view_size;
                self.camera_view_width = format!("{:.0}", node.camera2d.view_size[0]);
                self.camera_view_height = format!("{:.0}", node.camera2d.view_size[1]);
                return;
            }
        }

        self.selected_node_kind = None;
        self.node_name.clear();
        self.node_visible = true;
        self.node_position_x.clear();
        self.node_position_y.clear();
        self.node_size_width.clear();
        self.node_size_height.clear();
        self.script_path.clear();
        self.runtime_prefab.clear();
        self.asset_alias.clear();
        self.sprite_texture_path.clear();
        self.camera_zoom = 1.0;
        self.camera_show_bounds = true;
        self.camera_use_scene_view_size = true;
        self.camera_view_width = "960".to_string();
        self.camera_view_height = "720".to_string();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GenericJsonFieldKind {
    String,
    Number,
    Bool,
    Null,
}

#[derive(Clone, Copy, Default)]
struct JsonFieldHint {
    min: Option<f32>,
    max: Option<f32>,
    step: Option<f32>,
    slider: Option<bool>,
}

#[derive(Clone)]
struct GenericJsonField {
    pointer: String,
    label: String,
    section: String,
    kind: GenericJsonFieldKind,
    text: String,
    parse_error: Option<String>,
    slider_range: Option<(f32, f32)>,
    slider_step: Option<f32>,
    text_input_id: usize,
    slider_id: usize,
    checkbox_id: usize,
}

#[derive(Default)]
pub(crate) struct GenericJsonFormState {
    loaded_path: Option<PathBuf>,
    root: Value,
    fields: Vec<GenericJsonField>,
    hints: HashMap<String, JsonFieldHint>,
    load_error: Option<String>,
    dirty: bool,
    scroll: f32,
}

impl GenericJsonFormState {
    fn clear(&mut self) {
        *self = Self::default();
    }
}

impl RengineNativeEditor {
    pub(crate) fn update_file_browser_ui(&mut self, engine: &Engine, layout: &ShellLayout) {
        if !layout.files_open {
            self.file_browser_ui_focused = false;
            if self.active_text_input_owner == Some(TextInputOwner::FileBrowser) {
                self.active_text_input_owner = None;
            }
            return;
        }

        let inner = layout.files.inset(PANEL_PADDING);
        let ui_x = inner.x;
        let ui_y = inner.top() - 58.0;
        let ui_width = inner.w;

        let mut file_browser_ui = std::mem::take(&mut self.file_browser_ui);
        let mut file_browser_form = std::mem::take(&mut self.file_browser_form);
        file_browser_ui
            .set_text_input_enabled(self.text_input_enabled_for(TextInputOwner::FileBrowser));
        let mouse_pressed = engine.input().is_mouse_pressed(0);
        file_browser_ui.sync_at_with(
            engine,
            ui_x,
            ui_y,
            ui_width,
            &mut file_browser_form,
            Self::build_file_browser_ui,
            |response, state| {
                self.file_browser_ui_focused = response.focused_id.is_some();
                self.capture_text_input_owner(
                    TextInputOwner::FileBrowser,
                    response.focused_id,
                    response.hovered,
                    mouse_pressed,
                );
                if let Some(text) = response.text_for(FILE_FILTER_INPUT_ID) {
                    state.filter = text.to_string();
                }
            },
        );
        self.file_browser_form = file_browser_form;
        self.file_browser_ui = file_browser_ui;
    }

    fn build_file_browser_ui(ui: &mut Ui, state: &FileBrowserFormState) {
        ui.text_input(FILE_FILTER_INPUT_ID, &state.filter, "Filter files");
    }

    pub(crate) fn update_inspector_ui(&mut self, engine: &Engine, layout: &ShellLayout) {
        if !layout.inspector_open {
            self.inspector_ui_focused = false;
            if self.active_text_input_owner == Some(TextInputOwner::Inspector) {
                self.active_text_input_owner = None;
            }
            return;
        }

        if let Some(path) = self.selected_generic_json_path() {
            self.ensure_generic_json_form_loaded(&path);
            self.update_generic_json_inspector_ui(engine, layout);
            return;
        }

        if self.generic_json_form.loaded_path.is_some() {
            self.generic_json_form.clear();
        }

        self.sync_inspector_form_context();

        let ui_x = layout.inspector.x + PANEL_PADDING;
        let ui_y = inspector_form_top(layout.inspector);
        let ui_width = (layout.inspector.w - PANEL_PADDING * 2.0).max(0.0);
        let scroll_height = inspector_form_height(layout.inspector);
        let inspector_scroll = self.inspector_scroll;
        let selected_sprite_label = self
            .selected_sprite_source_path()
            .map(|path| self.display_path(&path));

        let mut inspector_ui = std::mem::take(&mut self.inspector_ui);
        let mut inspector_form = std::mem::take(&mut self.inspector_form);
        let mut kind_menu_request = None;
        let mut child_menu_request = None;
        inspector_ui.set_text_input_enabled(self.text_input_enabled_for(TextInputOwner::Inspector));
        let mouse_pressed = engine.input().is_mouse_pressed(0);
        inspector_ui.sync_at_with(
            engine,
            ui_x,
            ui_y,
            ui_width,
            &mut inspector_form,
            |ui, state| {
                Self::build_inspector_form_ui(
                    ui,
                    state,
                    selected_sprite_label.as_deref(),
                    inspector_scroll,
                    scroll_height,
                )
            },
            |response, state| {
                let request_kind_menu = response.was_activated(INSPECTOR_NODE_KIND_BUTTON_ID);
                let request_child_menu = response.was_activated(INSPECTOR_CREATE_CHILD_BUTTON_ID);
                self.inspector_ui_focused = response.focused_id.is_some();
                self.capture_text_input_owner(
                    TextInputOwner::Inspector,
                    response.focused_id,
                    response.hovered,
                    mouse_pressed,
                );
                if let Some(scroll) = response.scroll_for(INSPECTOR_SCROLL_REGION_ID) {
                    self.inspector_scroll = scroll;
                }
                self.apply_inspector_form_response(response, state);
                if request_kind_menu {
                    kind_menu_request = state.context_node;
                }
                if request_child_menu {
                    child_menu_request = state.context_node;
                }
            },
        );
        self.inspector_form = inspector_form;
        self.inspector_ui = inspector_ui;

        if let Some(node_id) = kind_menu_request {
            let inner = layout.inspector.inset(PANEL_PADDING);
            self.open_kind_menu(
                Vec2::new(
                    inner.right()
                        - popup_menu_width(NODE_KIND_OPTIONS.iter().map(|kind| kind.label())),
                    inner.top() - 264.0,
                ),
                node_id,
            );
        }

        if let Some(node_id) = child_menu_request {
            let inner = layout.inspector.inset(PANEL_PADDING);
            let labels: Vec<String> = NODE_KIND_OPTIONS
                .iter()
                .map(|kind| format!("Add Child {}", kind.label()))
                .collect();
            let popup_width = popup_menu_width(labels.iter().map(String::as_str));
            self.open_add_node_menu(
                Vec2::new(inner.right() - popup_width, inner.top() - 294.0),
                Some(node_id),
                None,
            );
        }
    }

    fn selected_generic_json_path(&self) -> Option<PathBuf> {
        self.selected_project_path.as_ref().and_then(|path| {
            if path.is_file() && is_json_path(path) && !is_scene_path(path) {
                Some(path.clone())
            } else {
                None
            }
        })
    }

    fn ensure_generic_json_form_loaded(&mut self, path: &Path) {
        if self
            .generic_json_form
            .loaded_path
            .as_deref()
            .is_some_and(|loaded| loaded == path)
        {
            return;
        }

        let mut form = GenericJsonFormState::default();
        form.loaded_path = Some(path.to_path_buf());

        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(root) => {
                    form.hints = extract_editor_hints(&root);
                    form.root = root;
                    form.fields = build_generic_json_fields(&form.root, &form.hints);
                    self.push_log(format!("Opened JSON {}", self.display_path(path)));
                }
                Err(error) => {
                    form.load_error = Some(format!("Invalid JSON: {}", error));
                    self.push_log(format!(
                        "Failed to parse {} as JSON: {}",
                        self.display_path(path),
                        error
                    ));
                }
            },
            Err(error) => {
                form.load_error = Some(format!("Read failed: {}", error));
                self.push_log(format!(
                    "Failed to read {}: {}",
                    self.display_path(path),
                    error
                ));
            }
        }

        self.generic_json_form = form;
    }

    fn update_generic_json_inspector_ui(&mut self, engine: &Engine, layout: &ShellLayout) {
        let ui_x = layout.inspector.x + PANEL_PADDING;
        let ui_y = inspector_form_top(layout.inspector);
        let ui_width = (layout.inspector.w - PANEL_PADDING * 2.0).max(0.0);
        let scroll_height = inspector_form_height(layout.inspector);

        let mut inspector_ui = std::mem::take(&mut self.inspector_ui);
        let mut generic_json_form = std::mem::take(&mut self.generic_json_form);
        inspector_ui.set_text_input_enabled(self.text_input_enabled_for(TextInputOwner::Inspector));
        let mouse_pressed = engine.input().is_mouse_pressed(0);
        inspector_ui.sync_at_with(
            engine,
            ui_x,
            ui_y,
            ui_width,
            &mut generic_json_form,
            |ui, state| Self::build_generic_json_form_ui(ui, state, scroll_height),
            |response, state| {
                self.inspector_ui_focused = response.focused_id.is_some();
                self.capture_text_input_owner(
                    TextInputOwner::Inspector,
                    response.focused_id,
                    response.hovered,
                    mouse_pressed,
                );
                if let Some(scroll) = response.scroll_for(INSPECTOR_JSON_SCROLL_REGION_ID) {
                    state.scroll = scroll;
                }
                self.apply_generic_json_form_response(response, state);
            },
        );

        self.generic_json_form = generic_json_form;
        self.inspector_ui = inspector_ui;
    }

    fn build_generic_json_form_ui(ui: &mut Ui, state: &GenericJsonFormState, scroll_height: f32) {
        if scroll_height <= 0.0 {
            return;
        }

        ui.scroll(
            INSPECTOR_JSON_SCROLL_REGION_ID,
            scroll_height,
            state.scroll,
            generic_json_widget_count(state),
        );

        if let Some(path) = &state.loaded_path {
            ui.label(
                &format!("Editing JSON: {}", path.display()),
                11.0,
                Color::from_rgba8(176, 186, 202, 255),
            );
        }

        if state.dirty {
            ui.label(
                "Unsaved changes",
                11.0,
                Color::from_rgba8(248, 196, 120, 255),
            );
        }

        ui.row(2);
        ui.button(INSPECTOR_JSON_SAVE_ID, "Save JSON");
        ui.button(INSPECTOR_JSON_RELOAD_ID, "Reload JSON");
        ui.separator(8.0);

        if let Some(error) = &state.load_error {
            ui.label(error, 11.0, Color::from_rgba8(236, 140, 140, 255));
            return;
        }

        let mut last_section: Option<&str> = None;
        for field in &state.fields {
            if last_section != Some(field.section.as_str()) {
                ui.separator(8.0);
                ui.label(&field.section, 12.0, Color::from_rgba8(208, 220, 236, 255));
                last_section = Some(field.section.as_str());
            }
            ui.label(&field.label, 11.0, Color::from_rgba8(148, 162, 180, 255));

            if field.kind == GenericJsonFieldKind::Bool {
                let checked = field.text.eq_ignore_ascii_case("true");
                ui.checkbox(field.checkbox_id, "Toggle", checked);
            }

            if let Some((min, max)) = field.slider_range {
                let slider_value = field
                    .text
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .unwrap_or(min)
                    .clamp(min, max);
                ui.slider(field.slider_id, "Value", slider_value, min, max);
            }

            let placeholder = match field.kind {
                GenericJsonFieldKind::String => "text",
                GenericJsonFieldKind::Number => "number",
                GenericJsonFieldKind::Bool => "true / false",
                GenericJsonFieldKind::Null => "null",
            };
            ui.text_input(field.text_input_id, &field.text, placeholder);

            if let Some(error) = &field.parse_error {
                ui.label(error, 10.0, Color::from_rgba8(236, 140, 140, 255));
            }

            ui.separator(6.0);
        }
    }

    fn apply_generic_json_form_response(
        &mut self,
        response: UiResponse,
        state: &mut GenericJsonFormState,
    ) {
        let mut changed = false;

        if response.was_activated(INSPECTOR_JSON_RELOAD_ID) {
            if let Some(path) = state.loaded_path.clone() {
                self.ensure_generic_json_form_loaded(&path);
                *state = std::mem::take(&mut self.generic_json_form);
            }
            return;
        }

        for field in &mut state.fields {
            if response.was_toggled(field.checkbox_id) && field.kind == GenericJsonFieldKind::Bool {
                let toggled = !field.text.eq_ignore_ascii_case("true");
                field.text = if toggled {
                    "true".to_string()
                } else {
                    "false".to_string()
                };
                if set_json_pointer_scalar(&mut state.root, &field.pointer, Value::Bool(toggled)) {
                    field.parse_error = None;
                    changed = true;
                }
            }

            if let Some(value) = response.value_for(field.slider_id) {
                let value = if let Some(step) = field.slider_step {
                    if step > 0.0 {
                        (value / step).round() * step
                    } else {
                        value
                    }
                } else {
                    value
                };
                field.text = format_float_for_editor(value);
                if set_json_pointer_scalar(
                    &mut state.root,
                    &field.pointer,
                    Value::from(value as f64),
                ) {
                    field.parse_error = None;
                    changed = true;
                }
            }

            if let Some(text) = response.text_for(field.text_input_id) {
                field.text = text.to_string();
                match parse_scalar_text(&field.text, field.kind) {
                    Ok(parsed) => {
                        if set_json_pointer_scalar(&mut state.root, &field.pointer, parsed) {
                            field.parse_error = None;
                            changed = true;
                        }
                    }
                    Err(error) => {
                        field.parse_error = Some(error);
                    }
                }
            }
        }

        if changed {
            state.dirty = true;
        }

        if response.was_activated(INSPECTOR_JSON_SAVE_ID) {
            let Some(path) = state.loaded_path.clone() else {
                return;
            };
            match serde_json::to_string_pretty(&state.root) {
                Ok(pretty) => match fs::write(&path, pretty) {
                    Ok(()) => {
                        state.dirty = false;
                        self.push_log(format!("Saved JSON {}", self.display_path(&path)));
                    }
                    Err(error) => {
                        self.push_log(format!(
                            "Failed to save {}: {}",
                            self.display_path(&path),
                            error
                        ));
                    }
                },
                Err(error) => {
                    self.push_log(format!("Failed to serialize JSON: {}", error));
                }
            }
        }
    }

    fn build_inspector_form_ui(
        ui: &mut Ui,
        state: &InspectorFormState,
        selected_sprite_label: Option<&str>,
        scroll_offset: f32,
        scroll_height: f32,
    ) {
        if scroll_height <= 0.0 {
            return;
        }

        ui.scroll(
            INSPECTOR_SCROLL_REGION_ID,
            scroll_height,
            scroll_offset,
            inspector_form_widget_count(state, selected_sprite_label),
        );
        ui.label("Scene Name", 11.0, Color::from_rgba8(148, 162, 180, 255));
        ui.text_input(INSPECTOR_SCENE_NAME_ID, &state.scene_name, "Scene name");
        ui.label(
            "Scene View Width",
            11.0,
            Color::from_rgba8(148, 162, 180, 255),
        );
        ui.text_input(INSPECTOR_SCENE_WIDTH_ID, &state.scene_window_width, "960");
        ui.label(
            "Scene View Height",
            11.0,
            Color::from_rgba8(148, 162, 180, 255),
        );
        ui.text_input(INSPECTOR_SCENE_HEIGHT_ID, &state.scene_window_height, "720");
        ui.separator(10.0);

        if let Some(kind) = state.selected_node_kind {
            ui.label(
                &format!("Selected Node: {}", kind.label()),
                13.0,
                Color::from_rgba8(232, 236, 242, 255),
            );
            let kind_label = format!("Kind: {}", kind.label());
            ui.button(INSPECTOR_NODE_KIND_BUTTON_ID, &kind_label);
            ui.tooltip_with(&kind_label, TooltipOptions::new().with_delay(0.35));
            ui.button(INSPECTOR_CREATE_CHILD_BUTTON_ID, "Create Child Node...");
            ui.tooltip_with(
                "Create Child Node...",
                TooltipOptions::new().with_delay(0.35),
            );
            ui.label("Node Name", 11.0, Color::from_rgba8(148, 162, 180, 255));
            ui.text_input(INSPECTOR_NODE_NAME_ID, &state.node_name, "Node name");
            ui.checkbox(INSPECTOR_NODE_VISIBLE_ID, "Visible", state.node_visible);
            ui.label("Position", 11.0, Color::from_rgba8(148, 162, 180, 255));
            ui.row(2);
            ui.text_input(INSPECTOR_NODE_POSITION_X_ID, &state.node_position_x, "x");
            ui.text_input(INSPECTOR_NODE_POSITION_Y_ID, &state.node_position_y, "y");
            ui.label("Size", 11.0, Color::from_rgba8(148, 162, 180, 255));
            ui.row(2);
            ui.text_input(
                INSPECTOR_NODE_SIZE_WIDTH_ID,
                &state.node_size_width,
                "width",
            );
            ui.text_input(
                INSPECTOR_NODE_SIZE_HEIGHT_ID,
                &state.node_size_height,
                "height",
            );
            ui.label("Script Path", 11.0, Color::from_rgba8(148, 162, 180, 255));
            ui.text_input(
                INSPECTOR_NODE_SCRIPT_ID,
                &state.script_path,
                "scripts/example.rs",
            );
            ui.label(
                "Runtime Prefab",
                11.0,
                Color::from_rgba8(148, 162, 180, 255),
            );
            ui.text_input(
                INSPECTOR_NODE_PREFAB_ID,
                &state.runtime_prefab,
                "runtime prefab id",
            );

            if kind == SceneNodeKind::Sprite {
                ui.separator(8.0);
                ui.label(
                    "Sprite Asset Alias",
                    11.0,
                    Color::from_rgba8(148, 162, 180, 255),
                );
                ui.text_input(INSPECTOR_SPRITE_ALIAS_ID, &state.asset_alias, "player_idle");
                ui.label(
                    "Sprite Texture Path",
                    11.0,
                    Color::from_rgba8(148, 162, 180, 255),
                );
                ui.text_input(
                    INSPECTOR_SPRITE_TEXTURE_ID,
                    &state.sprite_texture_path,
                    "assets/sprites/player.png",
                );
                ui.button(INSPECTOR_SPRITE_BROWSE_IMAGE_ID, "Browse Image...");
                ui.tooltip_with("Browse Image...", TooltipOptions::new().with_delay(0.35));
                if let Some(label) = selected_sprite_label {
                    ui.label(
                        &format!("Selected file: {}", label),
                        10.0,
                        Color::from_rgba8(120, 186, 255, 255),
                    );
                    ui.button(INSPECTOR_SPRITE_ASSIGN_SELECTED_ID, "Use Selected File");
                    ui.tooltip_with("Use Selected File", TooltipOptions::new().with_delay(0.35));
                }
                if !state.sprite_texture_path.trim().is_empty() {
                    ui.button(INSPECTOR_SPRITE_CLEAR_TEXTURE_ID, "Use Placeholder");
                    ui.tooltip_with("Use Placeholder", TooltipOptions::new().with_delay(0.35));
                }
            }

            if kind == SceneNodeKind::Camera2d {
                ui.separator(8.0);
                ui.slider(
                    INSPECTOR_CAMERA_ZOOM_ID,
                    "Zoom",
                    state.camera_zoom,
                    0.1,
                    8.0,
                );
                ui.checkbox(
                    INSPECTOR_CAMERA_SHOW_BOUNDS_ID,
                    "Show Bounds",
                    state.camera_show_bounds,
                );
                ui.checkbox(
                    INSPECTOR_CAMERA_USE_SCENE_SIZE_ID,
                    "Use Scene Window Size",
                    state.camera_use_scene_view_size,
                );
                if !state.camera_use_scene_view_size {
                    ui.label(
                        "Camera View Width",
                        11.0,
                        Color::from_rgba8(148, 162, 180, 255),
                    );
                    ui.text_input(
                        INSPECTOR_CAMERA_VIEW_WIDTH_ID,
                        &state.camera_view_width,
                        "960",
                    );
                    ui.label(
                        "Camera View Height",
                        11.0,
                        Color::from_rgba8(148, 162, 180, 255),
                    );
                    ui.text_input(
                        INSPECTOR_CAMERA_VIEW_HEIGHT_ID,
                        &state.camera_view_height,
                        "720",
                    );
                }
                let preview_width = if state.camera_use_scene_view_size {
                    state.scene_window_width.as_str()
                } else {
                    state.camera_view_width.as_str()
                };
                let preview_height = if state.camera_use_scene_view_size {
                    state.scene_window_height.as_str()
                } else {
                    state.camera_view_height.as_str()
                };
                ui.label(
                    &format!("Current preview: {} x {}", preview_width, preview_height),
                    10.0,
                    Color::from_rgba8(148, 162, 180, 255),
                );
            }
        }
    }

    fn apply_inspector_form_response(
        &mut self,
        response: UiResponse,
        state: &mut InspectorFormState,
    ) {
        let selected_sprite_path = self.selected_sprite_source_path();
        let mut changed = false;
        let mut history_entry = None;
        let mut log_messages = Vec::new();
        let mut manual_sprite_texture_for_node = None;
        let mut use_selected_sprite_for_node = None;
        let mut browse_sprite_for_node = None;
        let mut clear_sprite_for_node = None;
        let mut resync_form = false;

        {
            let tab = self.active_scene_tab_mut();
            let previous_state = SceneHistoryEntry::capture(tab);

            if let Some(text) = response.text_for(INSPECTOR_SCENE_NAME_ID) {
                state.scene_name = text.to_string();
                if tab.scene.name != state.scene_name {
                    tab.scene.name = state.scene_name.clone();
                    changed = true;
                }
            }

            if let Some(text) = response.text_for(INSPECTOR_SCENE_WIDTH_ID) {
                state.scene_window_width = text.to_string();
                if let Some(width) = parse_editor_number(&state.scene_window_width, 64.0, 4096.0) {
                    if (tab.scene.view.window_size[0] - width).abs() > f32::EPSILON {
                        tab.scene.view.window_size[0] = width;
                        changed = true;
                    }
                }
            }

            if let Some(text) = response.text_for(INSPECTOR_SCENE_HEIGHT_ID) {
                state.scene_window_height = text.to_string();
                if let Some(height) = parse_editor_number(&state.scene_window_height, 64.0, 4096.0)
                {
                    if (tab.scene.view.window_size[1] - height).abs() > f32::EPSILON {
                        tab.scene.view.window_size[1] = height;
                        changed = true;
                    }
                }
            }

            if let Some(node_id) = tab.selected_node {
                if let Some(node) = tab.scene.node_mut(node_id) {
                    if let Some(text) = response.text_for(INSPECTOR_NODE_NAME_ID) {
                        state.node_name = text.to_string();
                        if node.name != state.node_name {
                            node.name = state.node_name.clone();
                            changed = true;
                        }
                    }

                    if response.was_toggled(INSPECTOR_NODE_VISIBLE_ID) {
                        state.node_visible = !state.node_visible;
                        if node.visible != state.node_visible {
                            node.visible = state.node_visible;
                            changed = true;
                        }
                    }

                    if let Some(text) = response.text_for(INSPECTOR_NODE_POSITION_X_ID) {
                        state.node_position_x = text.to_string();
                        if let Some(x) = parse_editor_float(&state.node_position_x) {
                            if (node.position[0] - x).abs() > f32::EPSILON {
                                node.position[0] = x;
                                changed = true;
                            }
                        }
                    }

                    if let Some(text) = response.text_for(INSPECTOR_NODE_POSITION_Y_ID) {
                        state.node_position_y = text.to_string();
                        if let Some(y) = parse_editor_float(&state.node_position_y) {
                            if (node.position[1] - y).abs() > f32::EPSILON {
                                node.position[1] = y;
                                changed = true;
                            }
                        }
                    }

                    if let Some(text) = response.text_for(INSPECTOR_NODE_SIZE_WIDTH_ID) {
                        state.node_size_width = text.to_string();
                        if let Some(width) =
                            parse_editor_number(&state.node_size_width, 16.0, 4096.0)
                        {
                            if (node.size[0] - width).abs() > f32::EPSILON {
                                node.size[0] = width;
                                changed = true;
                            }
                        }
                    }

                    if let Some(text) = response.text_for(INSPECTOR_NODE_SIZE_HEIGHT_ID) {
                        state.node_size_height = text.to_string();
                        if let Some(height) =
                            parse_editor_number(&state.node_size_height, 16.0, 4096.0)
                        {
                            if (node.size[1] - height).abs() > f32::EPSILON {
                                node.size[1] = height;
                                changed = true;
                            }
                        }
                    }

                    if let Some(text) = response.text_for(INSPECTOR_NODE_SCRIPT_ID) {
                        state.script_path = text.to_string();
                        if node.script_path != state.script_path {
                            node.script_path = state.script_path.clone();
                            changed = true;
                        }
                    }

                    if let Some(text) = response.text_for(INSPECTOR_NODE_PREFAB_ID) {
                        state.runtime_prefab = text.to_string();
                        if node.runtime_prefab != state.runtime_prefab {
                            node.runtime_prefab = state.runtime_prefab.clone();
                            changed = true;
                        }
                    }

                    if node.kind == SceneNodeKind::Sprite {
                        if let Some(text) = response.text_for(INSPECTOR_SPRITE_ALIAS_ID) {
                            state.asset_alias = text.to_string();
                            if node.asset_alias != state.asset_alias {
                                node.asset_alias = state.asset_alias.clone();
                                changed = true;
                            }
                        }

                        if let Some(text) = response.text_for(INSPECTOR_SPRITE_TEXTURE_ID) {
                            state.sprite_texture_path = text.to_string();
                            if node.sprite.texture_path != state.sprite_texture_path {
                                node.sprite.texture_path = state.sprite_texture_path.clone();
                                changed = true;
                                if !state.sprite_texture_path.trim().is_empty() {
                                    manual_sprite_texture_for_node =
                                        Some((node_id, state.sprite_texture_path.clone()));
                                }
                            }
                        }

                        if response.was_activated(INSPECTOR_SPRITE_ASSIGN_SELECTED_ID) {
                            use_selected_sprite_for_node = Some(node_id);
                        }

                        if response.was_activated(INSPECTOR_SPRITE_BROWSE_IMAGE_ID) {
                            browse_sprite_for_node = Some(node_id);
                        }

                        if response.was_activated(INSPECTOR_SPRITE_CLEAR_TEXTURE_ID)
                            && !node.sprite.texture_path.is_empty()
                        {
                            clear_sprite_for_node = Some(node_id);
                        }
                    }

                    if node.kind == SceneNodeKind::Camera2d {
                        if let Some(zoom) = response.value_for(INSPECTOR_CAMERA_ZOOM_ID) {
                            let zoom = zoom.max(0.1);
                            state.camera_zoom = zoom;
                            if (node.camera2d.zoom - zoom).abs() > f32::EPSILON {
                                node.camera2d.zoom = zoom;
                                changed = true;
                            }
                        }

                        if response.was_toggled(INSPECTOR_CAMERA_SHOW_BOUNDS_ID) {
                            state.camera_show_bounds = !state.camera_show_bounds;
                            if node.camera2d.show_bounds != state.camera_show_bounds {
                                node.camera2d.show_bounds = state.camera_show_bounds;
                                changed = true;
                            }
                        }

                        if response.was_toggled(INSPECTOR_CAMERA_USE_SCENE_SIZE_ID) {
                            state.camera_use_scene_view_size = !state.camera_use_scene_view_size;
                            if node.camera2d.use_scene_view_size != state.camera_use_scene_view_size
                            {
                                node.camera2d.use_scene_view_size =
                                    state.camera_use_scene_view_size;
                                changed = true;
                            }
                        }

                        if let Some(text) = response.text_for(INSPECTOR_CAMERA_VIEW_WIDTH_ID) {
                            state.camera_view_width = text.to_string();
                            if let Some(width) =
                                parse_editor_number(&state.camera_view_width, 64.0, 4096.0)
                            {
                                if (node.camera2d.view_size[0] - width).abs() > f32::EPSILON {
                                    node.camera2d.view_size[0] = width;
                                    changed = true;
                                }
                            }
                        }

                        if let Some(text) = response.text_for(INSPECTOR_CAMERA_VIEW_HEIGHT_ID) {
                            state.camera_view_height = text.to_string();
                            if let Some(height) =
                                parse_editor_number(&state.camera_view_height, 64.0, 4096.0)
                            {
                                if (node.camera2d.view_size[1] - height).abs() > f32::EPSILON {
                                    node.camera2d.view_size[1] = height;
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }

            if changed {
                history_entry = Some(previous_state);
                tab.mark_dirty();
            }
        }

        if let Some(history_entry) = history_entry {
            self.active_scene_tab_mut().push_undo_entry(history_entry);
        }

        if let Some(node_id) = use_selected_sprite_for_node {
            if let Some(path) = &selected_sprite_path {
                let texture_changed = self.set_node_sprite_texture_path(node_id, path);
                let alias_changed = self.seed_node_asset_alias_from_path(node_id, path);

                if texture_changed || alias_changed {
                    log_messages.push(format!(
                        "Assigned sprite texture from {}",
                        self.display_path(path)
                    ));
                }

                resync_form = true;
            }
        }

        if let Some((node_id, stored_path)) = manual_sprite_texture_for_node {
            let resolved_path = self.resolve_stored_path(&stored_path);
            if resolved_path.is_file() && is_supported_sprite_path(&resolved_path) {
                if self.set_node_sprite_texture_path(node_id, &resolved_path) {
                    resync_form = true;
                }
            }
        }

        if let Some(node_id) = browse_sprite_for_node {
            if self.choose_sprite_for_node(node_id).is_some() {
                resync_form = true;
            }
        }

        if let Some(node_id) = clear_sprite_for_node {
            if self.clear_node_sprite_texture_path(node_id) {
                log_messages.push("Sprite reverted to placeholder preview".to_string());
                resync_form = true;
            }
        }

        if resync_form {
            state.sync_from_editor(self);
        }

        for message in log_messages {
            self.push_log(message);
        }
    }
}

fn extract_editor_hints(root: &Value) -> HashMap<String, JsonFieldHint> {
    let mut hints = HashMap::new();
    let Some(meta) = root.get("__editor") else {
        return hints;
    };
    let Some(fields) = meta.get("fields") else {
        return hints;
    };
    let Some(map) = fields.as_object() else {
        return hints;
    };

    for (pointer, spec) in map {
        let Some(spec_obj) = spec.as_object() else {
            continue;
        };
        let min = spec_obj
            .get("min")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);
        let max = spec_obj
            .get("max")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);
        let step = spec_obj
            .get("step")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);
        let slider = spec_obj
            .get("widget")
            .and_then(|v| v.as_str())
            .map(|widget| widget.eq_ignore_ascii_case("slider"));
        hints.insert(
            pointer.to_string(),
            JsonFieldHint {
                min,
                max,
                step,
                slider,
            },
        );
    }

    hints
}

fn build_generic_json_fields(
    root: &Value,
    hints: &HashMap<String, JsonFieldHint>,
) -> Vec<GenericJsonField> {
    let mut fields = Vec::new();
    collect_json_fields(root, "", "", hints, &mut fields);
    fields
}

fn collect_json_fields(
    value: &Value,
    pointer: &str,
    display_path: &str,
    hints: &HashMap<String, JsonFieldHint>,
    out: &mut Vec<GenericJsonField>,
) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable();
            for key in keys {
                if pointer.is_empty() && key == "__editor" {
                    continue;
                }
                let Some(child) = map.get(key) else {
                    continue;
                };
                let child_pointer = format!("{}/{}", pointer, escape_json_pointer_token(key));
                let child_display = if display_path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", display_path, key)
                };
                collect_json_fields(child, &child_pointer, &child_display, hints, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let child_pointer = format!("{}/{}", pointer, index);
                let child_display = format!("{}[{}]", display_path, index);
                collect_json_fields(child, &child_pointer, &child_display, hints, out);
            }
        }
        Value::String(text) => {
            out.push(make_generic_json_field(
                pointer,
                display_path,
                GenericJsonFieldKind::String,
                text.clone(),
                hints.get(pointer).copied().unwrap_or_default(),
            ));
        }
        Value::Number(number) => {
            out.push(make_generic_json_field(
                pointer,
                display_path,
                GenericJsonFieldKind::Number,
                number.to_string(),
                hints.get(pointer).copied().unwrap_or_default(),
            ));
        }
        Value::Bool(flag) => {
            out.push(make_generic_json_field(
                pointer,
                display_path,
                GenericJsonFieldKind::Bool,
                if *flag { "true" } else { "false" }.to_string(),
                hints.get(pointer).copied().unwrap_or_default(),
            ));
        }
        Value::Null => {
            out.push(make_generic_json_field(
                pointer,
                display_path,
                GenericJsonFieldKind::Null,
                "null".to_string(),
                hints.get(pointer).copied().unwrap_or_default(),
            ));
        }
    }
}

fn make_generic_json_field(
    pointer: &str,
    display_path: &str,
    kind: GenericJsonFieldKind,
    text: String,
    hint: JsonFieldHint,
) -> GenericJsonField {
    let label = if display_path.is_empty() {
        "<root>".to_string()
    } else {
        display_path.to_string()
    };
    let section = display_path
        .rfind('.')
        .map(|idx| display_path[..idx].to_string())
        .unwrap_or_else(|| "Root".to_string());
    let hash = hash_string(pointer);
    let text_input_id = INSPECTOR_JSON_TEXT_INPUT_BASE_ID + (hash % 8_000);
    let slider_id = INSPECTOR_JSON_SLIDER_BASE_ID + (hash % 8_000);
    let checkbox_id = INSPECTOR_JSON_CHECKBOX_BASE_ID + (hash % 8_000);
    let parsed_value = text.trim().parse::<f32>().ok();

    let slider_range = if kind == GenericJsonFieldKind::Number {
        let use_slider = hint.slider.unwrap_or(true);
        if use_slider {
            if let (Some(min), Some(max)) = (hint.min, hint.max) {
                Some((min.min(max), max.max(min)))
            } else if let Some(v) = parsed_value {
                Some(guess_slider_range(v))
            } else {
                Some((-1.0, 1.0))
            }
        } else {
            None
        }
    } else {
        None
    };

    GenericJsonField {
        pointer: pointer.to_string(),
        label,
        section,
        kind,
        text,
        parse_error: None,
        slider_range,
        slider_step: hint.step,
        text_input_id,
        slider_id,
        checkbox_id,
    }
}

fn generic_json_widget_count(state: &GenericJsonFormState) -> usize {
    let mut count = 6;
    if state.load_error.is_some() {
        return count + 1;
    }
    let mut last_section: Option<&str> = None;
    for field in &state.fields {
        if last_section != Some(field.section.as_str()) {
            count += 2;
            last_section = Some(field.section.as_str());
        }
        count += 3;
        if field.kind == GenericJsonFieldKind::Bool {
            count += 1;
        }
        if field.slider_range.is_some() {
            count += 1;
        }
        if field.parse_error.is_some() {
            count += 1;
        }
    }
    count
}

fn parse_scalar_text(text: &str, kind: GenericJsonFieldKind) -> Result<Value, String> {
    match kind {
        GenericJsonFieldKind::String => Ok(Value::String(text.to_string())),
        GenericJsonFieldKind::Number => {
            let value = text
                .trim()
                .parse::<f64>()
                .map_err(|_| "Expected a number".to_string())?;
            Ok(Value::from(value))
        }
        GenericJsonFieldKind::Bool => match text.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err("Expected true or false".to_string()),
        },
        GenericJsonFieldKind::Null => {
            if text.trim().eq_ignore_ascii_case("null") {
                Ok(Value::Null)
            } else {
                Err("Expected null".to_string())
            }
        }
    }
}

fn set_json_pointer_scalar(root: &mut Value, pointer: &str, value: Value) -> bool {
    if pointer.is_empty() {
        *root = value;
        return true;
    }
    if let Some(slot) = root.pointer_mut(pointer) {
        *slot = value;
        true
    } else {
        false
    }
}

fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn hash_string(value: &str) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish() as usize
}

fn guess_slider_range(value: f32) -> (f32, f32) {
    if value.abs() <= 1.0 {
        (-1.0, 1.0)
    } else if value.abs() <= 10.0 {
        if value >= 0.0 {
            (0.0, 20.0)
        } else {
            (-20.0, 0.0)
        }
    } else {
        let span = (value.abs() * 2.0).max(10.0);
        if value >= 0.0 {
            (0.0, span)
        } else {
            (-span, 0.0)
        }
    }
}

fn format_float_for_editor(value: f32) -> String {
    if (value.round() - value).abs() < 0.000_1 {
        format!("{:.0}", value)
    } else {
        format!("{:.4}", value)
    }
}

fn inspector_form_top(panel: PanelRect) -> f32 {
    let inner = panel.inset(PANEL_PADDING);
    inner.top() - 134.0
}

fn inspector_form_height(panel: PanelRect) -> f32 {
    let inner = panel.inset(PANEL_PADDING);
    (inspector_form_top(panel) - inner.y).max(0.0)
}

fn inspector_form_widget_count(
    state: &InspectorFormState,
    selected_sprite_label: Option<&str>,
) -> usize {
    let mut count = 7;

    if let Some(kind) = state.selected_node_kind {
        count += 18;

        if kind == SceneNodeKind::Sprite {
            count += 6;
            if selected_sprite_label.is_some() {
                count += 2;
            }
            if !state.sprite_texture_path.trim().is_empty() {
                count += 1;
            }
        }

        if kind == SceneNodeKind::Camera2d {
            count += 5;
            if !state.camera_use_scene_view_size {
                count += 4;
            }
        }
    }

    count
}

pub(crate) fn make_file_browser_ui() -> Ui {
    make_inspector_ui()
}

pub(crate) fn is_file_browser_text_input(id: usize) -> bool {
    matches!(id, FILE_FILTER_INPUT_ID)
}

pub(crate) fn is_inspector_text_input(id: usize) -> bool {
    matches!(
        id,
        INSPECTOR_SCENE_NAME_ID
            | INSPECTOR_SCENE_WIDTH_ID
            | INSPECTOR_SCENE_HEIGHT_ID
            | INSPECTOR_NODE_NAME_ID
            | INSPECTOR_NODE_SCRIPT_ID
            | INSPECTOR_NODE_PREFAB_ID
            | INSPECTOR_NODE_POSITION_X_ID
            | INSPECTOR_NODE_POSITION_Y_ID
            | INSPECTOR_NODE_SIZE_WIDTH_ID
            | INSPECTOR_NODE_SIZE_HEIGHT_ID
            | INSPECTOR_SPRITE_ALIAS_ID
            | INSPECTOR_SPRITE_TEXTURE_ID
            | INSPECTOR_CAMERA_VIEW_WIDTH_ID
            | INSPECTOR_CAMERA_VIEW_HEIGHT_ID
    ) || (INSPECTOR_JSON_TEXT_INPUT_BASE_ID..INSPECTOR_JSON_SLIDER_BASE_ID).contains(&id)
}

pub(crate) fn make_inspector_ui() -> Ui {
    let mut ui = Ui::default();
    let style = ui.style_mut();
    style.text_size = 12.0;
    style.spacing = 6.0;
    style.text_input_bg = Color::from_rgba8(29, 36, 44, 240);
    style.text_input_focused_bg = Color::from_rgba8(54, 84, 124, 255);
    style.text_input_text_color = Color::from_rgba8(234, 238, 242, 255);
    style.text_input_placeholder_color = Color::from_rgba8(118, 130, 146, 255);
    style.button_bg = Color::from_rgba8(38, 46, 58, 240);
    style.button_focused_bg = Color::from_rgba8(66, 116, 132, 255);
    style.button_pressed_bg = Color::from_rgba8(88, 146, 162, 255);
    style.button_text_color = Color::from_rgba8(228, 234, 240, 255);
    style.checkbox_bg = Color::from_rgba8(36, 42, 52, 255);
    style.checkbox_checked_bg = Color::from_rgba8(82, 156, 170, 255);
    style.slider_track_color = Color::from_rgba8(36, 42, 52, 255);
    style.slider_fill_color = Color::from_rgba8(88, 146, 162, 255);
    style.slider_thumb_color = Color::from_rgba8(236, 241, 246, 255);
    style.panel_bg = Color::new(0.0, 0.0, 0.0, 0.0);
    ui
}

fn parse_editor_float(text: &str) -> Option<f32> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    text.parse::<f32>().ok()
}

fn parse_editor_number(text: &str, min: f32, max: f32) -> Option<f32> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    text.parse::<f32>().ok().map(|value| value.clamp(min, max))
}
