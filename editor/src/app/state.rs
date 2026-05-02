use super::*;

const MAX_SCENE_HISTORY_STEPS: usize = 128;

#[derive(Clone, Debug)]
pub(crate) struct ViewportDrag {
    pub(crate) node_id: u64,
    pub(crate) pointer_offset: Vec2,
    pub(crate) history_captured: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ViewportPanDrag {
    pub(crate) pointer_origin: Vec2,
    pub(crate) pan_origin: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BottomTab {
    Activity,
    SceneJson,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextInputOwner {
    FileBrowser,
    Inspector,
}

impl BottomTab {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Activity => "Activity",
            Self::SceneJson => "Scene JSON",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SceneHistoryEntry {
    pub(crate) scene: SceneDocument,
    pub(crate) selected_node: Option<u64>,
}

impl SceneHistoryEntry {
    pub(crate) fn capture(tab: &SceneTab) -> Self {
        Self {
            scene: tab.scene.clone(),
            selected_node: sanitize_scene_history_selection(&tab.scene, tab.selected_node),
        }
    }
}

pub(crate) struct SceneTab {
    pub(crate) scene: SceneDocument,
    pub(crate) scene_dirty: bool,
    pub(crate) scene_path: Option<PathBuf>,
    pub(crate) selected_node: Option<u64>,
    pub(crate) viewport_drag: Option<ViewportDrag>,
    pub(crate) viewport_pan: Vec2,
    pub(crate) viewport_pan_drag: Option<ViewportPanDrag>,
    pub(crate) collapsed_nodes: HashSet<u64>,
    pub(crate) scene_json_cache: String,
    pub(crate) saved_scene_json: String,
    pub(crate) scene_json_dirty: bool,
    pub(crate) undo_history: Vec<SceneHistoryEntry>,
    pub(crate) redo_history: Vec<SceneHistoryEntry>,
    pub(crate) edit_revision: u64,
    pub(crate) autosaved_revision: u64,
    pub(crate) autosave_elapsed: f32,
}

impl SceneTab {
    pub(crate) fn new(scene: SceneDocument, scene_path: Option<PathBuf>) -> Self {
        let scene_json_cache = scene.pretty_json();

        Self {
            scene,
            scene_dirty: false,
            scene_path,
            selected_node: None,
            viewport_drag: None,
            viewport_pan: Vec2::ZERO,
            viewport_pan_drag: None,
            collapsed_nodes: HashSet::new(),
            saved_scene_json: scene_json_cache.clone(),
            scene_json_cache,
            scene_json_dirty: false,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            edit_revision: 0,
            autosaved_revision: 0,
            autosave_elapsed: 0.0,
        }
    }

    pub(crate) fn untitled() -> Self {
        Self::new(SceneDocument::new("untitled_scene"), None)
    }

    pub(crate) fn is_fresh_untitled(&self) -> bool {
        self.scene_path.is_none()
            && !self.scene_dirty
            && self.scene.nodes.is_empty()
            && self.scene.name == "untitled_scene"
    }

    pub(crate) fn display_name(&self) -> String {
        if let Some(scene_path) = &self.scene_path {
            if let Some(name) = scene_path.file_name().and_then(|name| name.to_str()) {
                if let Some(name) = name.strip_suffix(".scene.json") {
                    return name.to_string();
                }

                if let Some(name) = name.strip_suffix(".json") {
                    return name.to_string();
                }

                return name.to_string();
            }
        }

        let scene_name = self.scene.name.trim();
        if scene_name.is_empty() {
            "untitled_scene".to_string()
        } else {
            scene_name.to_string()
        }
    }

    pub(crate) fn tab_label(&self) -> String {
        let mut label = self.display_name();
        if self.scene_dirty {
            label.push('*');
        }
        label
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.scene_dirty = true;
        self.scene_json_dirty = true;
        self.edit_revision = self.edit_revision.saturating_add(1);
        self.autosave_elapsed = 0.0;
    }

    pub(crate) fn push_undo_entry(&mut self, entry: SceneHistoryEntry) {
        if self.undo_history.last().is_some_and(|last| {
            last.scene == entry.scene && last.selected_node == entry.selected_node
        }) {
            self.redo_history.clear();
            return;
        }

        self.undo_history.push(entry);
        if self.undo_history.len() > MAX_SCENE_HISTORY_STEPS {
            let overflow = self.undo_history.len() - MAX_SCENE_HISTORY_STEPS;
            self.undo_history.drain(0..overflow);
        }
        self.redo_history.clear();
    }

    pub(crate) fn mark_saved(&mut self, scene_json: String) {
        self.saved_scene_json = scene_json.clone();
        self.scene_json_cache = scene_json;
        self.scene_json_dirty = false;
        self.scene_dirty = false;
        self.autosaved_revision = self.edit_revision;
        self.autosave_elapsed = 0.0;
    }

    fn restore_history_entry(&mut self, entry: SceneHistoryEntry) {
        self.scene = entry.scene;
        self.selected_node = sanitize_scene_history_selection(&self.scene, entry.selected_node);
        self.viewport_drag = None;
        self.viewport_pan_drag = None;
        self.collapsed_nodes
            .retain(|node_id| self.scene.node(*node_id).is_some());
        self.scene_json_cache = self.scene.pretty_json();
        self.scene_json_dirty = false;
        self.scene_dirty = self.scene_json_cache != self.saved_scene_json;
        self.edit_revision = self.edit_revision.saturating_add(1);
        self.autosave_elapsed = 0.0;
        if !self.scene_dirty {
            self.autosaved_revision = self.edit_revision;
        }
    }

    pub(crate) fn undo(&mut self) -> bool {
        let Some(entry) = self.undo_history.pop() else {
            return false;
        };

        self.redo_history.push(SceneHistoryEntry::capture(self));
        self.restore_history_entry(entry);
        true
    }

    pub(crate) fn redo(&mut self) -> bool {
        let Some(entry) = self.redo_history.pop() else {
            return false;
        };

        self.undo_history.push(SceneHistoryEntry::capture(self));
        self.restore_history_entry(entry);
        true
    }

    pub(crate) fn cached_scene_json(&mut self) -> &str {
        if self.scene_json_dirty {
            self.scene_json_cache = self.scene.pretty_json();
            self.scene_json_dirty = false;
        }

        &self.scene_json_cache
    }
}

pub(crate) struct CanvasTooltipTarget {
    pub(crate) key: String,
    pub(crate) rect: PanelRect,
    pub(crate) text: String,
}

impl CanvasTooltipTarget {
    pub(crate) fn new(rect: PanelRect, text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            key: format!(
                "{}:{:.1}:{:.1}:{:.1}:{:.1}",
                text, rect.x, rect.y, rect.w, rect.h
            ),
            rect,
            text,
        }
    }
}

pub(crate) struct CanvasTooltipHoverState {
    pub(crate) key: String,
    pub(crate) elapsed: f32,
}

pub(crate) struct ProjectEntryClickState {
    pub(crate) path: PathBuf,
    pub(crate) elapsed: f32,
}

fn sanitize_scene_history_selection(
    scene: &SceneDocument,
    selected_node: Option<u64>,
) -> Option<u64> {
    selected_node.filter(|node_id| scene.node(*node_id).is_some())
}

fn history_modifier_down(engine: &Engine) -> bool {
    let input = engine.input();
    input.is_key_down(KeyCode::ControlLeft) || input.is_key_down(KeyCode::ControlRight)
}

fn history_shift_down(engine: &Engine) -> bool {
    let input = engine.input();
    input.is_key_down(KeyCode::ShiftLeft) || input.is_key_down(KeyCode::ShiftRight)
}

impl RengineNativeEditor {
    pub(crate) fn active_scene_tab(&self) -> &SceneTab {
        &self.scene_tabs[self.active_scene_tab]
    }

    pub(crate) fn active_scene_tab_mut(&mut self) -> &mut SceneTab {
        &mut self.scene_tabs[self.active_scene_tab]
    }

    pub(crate) fn scene_json_preview_text(&mut self) -> &str {
        let defer_refresh = self.active_scene_tab().scene_json_dirty
            && self.active_scene_tab().viewport_drag.is_some();
        let tab = self.active_scene_tab_mut();
        if defer_refresh {
            &tab.scene_json_cache
        } else {
            tab.cached_scene_json()
        }
    }

    pub(crate) fn scene_json_preview_line_count(&mut self) -> usize {
        self.scene_json_preview_text().lines().count()
    }

    pub(crate) fn push_log(&mut self, message: impl Into<String>) {
        self.activity_log.push(message.into());
        if self.activity_log.len() > MAX_ACTIVITY_LOG_LINES {
            let overflow = self.activity_log.len() - MAX_ACTIVITY_LOG_LINES;
            self.activity_log.drain(0..overflow);
        }
        self.bottom_scroll = 0.0;
    }

    pub(crate) fn refresh_inspector_form(&mut self) {
        let mut form = std::mem::take(&mut self.inspector_form);
        form.sync_from_editor(self);
        self.inspector_form = form;
    }

    pub(crate) fn sync_inspector_form_context(&mut self) {
        if self.inspector_form.context_tab != self.active_scene_tab
            || self.inspector_form.context_node != self.active_scene_tab().selected_node
        {
            self.refresh_inspector_form();
        }
    }

    pub(crate) fn ui_has_focus(&self) -> bool {
        self.file_browser_ui_focused || self.inspector_ui_focused
    }

    pub(crate) fn text_input_enabled_for(&self, owner: TextInputOwner) -> bool {
        self.active_text_input_owner.is_none() || self.active_text_input_owner == Some(owner)
    }

    pub(crate) fn clear_text_input_owner(&mut self) {
        self.active_text_input_owner = None;
    }

    pub(crate) fn update_recent_project_click(&mut self, dt: f32) {
        if let Some(state) = &mut self.recent_project_click {
            state.elapsed += dt;
            if state.elapsed > PROJECT_DOUBLE_CLICK_DELAY {
                self.recent_project_click = None;
            }
        }
    }

    pub(crate) fn register_project_click(&mut self, path: &Path) -> bool {
        let activate = self.recent_project_click.as_ref().is_some_and(|state| {
            state.path.as_path() == path && state.elapsed <= PROJECT_DOUBLE_CLICK_DELAY
        });
        self.recent_project_click = Some(ProjectEntryClickState {
            path: path.to_path_buf(),
            elapsed: 0.0,
        });
        activate
    }

    pub(crate) fn toggle_scene_node(&mut self, node_id: u64) {
        let tab = self.active_scene_tab_mut();
        if !tab.collapsed_nodes.insert(node_id) {
            tab.collapsed_nodes.remove(&node_id);
        }
    }

    pub(crate) fn switch_to_scene_tab(&mut self, index: usize) {
        if index >= self.scene_tabs.len() || index == self.active_scene_tab {
            return;
        }

        self.active_scene_tab = index;
        self.refresh_inspector_form();
        let scene_path = self.active_scene_tab().scene_path.clone();
        let scene_label = self.active_scene_tab().display_name();

        if let Some(path) = scene_path {
            self.selected_project_path = Some(path.clone());
            self.push_log(format!("Switched to scene {}", self.display_path(&path)));
        } else {
            self.push_log(format!("Switched to scene {}", scene_label));
        }
    }

    pub(crate) fn new_scene(&mut self) {
        self.scene_tabs.push(SceneTab::untitled());
        self.active_scene_tab = self.scene_tabs.len() - 1;
        self.refresh_inspector_form();
        self.push_log("Started new empty scene");
    }

    pub(crate) fn handle_scene_history_shortcuts(&mut self, engine: &Engine) {
        if !history_modifier_down(engine) {
            return;
        }

        if engine.input().is_key_pressed(KeyCode::KeyY)
            || (history_shift_down(engine) && engine.input().is_key_pressed(KeyCode::KeyZ))
        {
            self.redo_active_scene();
            return;
        }

        if engine.input().is_key_pressed(KeyCode::KeyZ) {
            self.undo_active_scene();
        }
    }

    pub(crate) fn undo_active_scene(&mut self) {
        let scene_label = self.active_scene_tab().display_name();
        if self.active_scene_tab_mut().undo() {
            self.refresh_inspector_form();
            self.push_log(format!("Undid edit in {}", scene_label));
        }
    }

    pub(crate) fn redo_active_scene(&mut self) {
        let scene_label = self.active_scene_tab().display_name();
        if self.active_scene_tab_mut().redo() {
            self.refresh_inspector_form();
            self.push_log(format!("Redid edit in {}", scene_label));
        }
    }
}
