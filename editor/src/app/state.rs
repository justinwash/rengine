use super::*;

const MAX_SCENE_HISTORY_STEPS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewportDragConstraint {
    Free,
    AxisX,
    AxisY,
}

#[derive(Clone, Debug)]
pub(crate) struct ViewportDrag {
    pub(crate) node_ids: Vec<u64>,
    pub(crate) transform_origin: [f32; 2],
    pub(crate) pointer_scene_origin: [f32; 2],
    pub(crate) applied_delta: [f32; 2],
    pub(crate) constraint: ViewportDragConstraint,
    pub(crate) history_captured: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ViewportBoxSelection {
    pub(crate) pointer_origin: Vec2,
    pub(crate) pointer_current: Vec2,
    pub(crate) additive: bool,
    pub(crate) initial_selected_node: Option<u64>,
    pub(crate) initial_selected_nodes: Vec<u64>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorTheme {
    Slate,
    Graphite,
    Ember,
}

impl BottomTab {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Activity => "Activity",
            Self::SceneJson => "Scene JSON",
        }
    }
}

impl EditorTheme {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Slate => "Slate",
            Self::Graphite => "Graphite",
            Self::Ember => "Ember",
        }
    }

    pub(crate) fn all() -> [EditorTheme; 3] {
        [Self::Slate, Self::Graphite, Self::Ember]
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SceneHistoryEntry {
    pub(crate) scene: SceneDocument,
    pub(crate) selected_node: Option<u64>,
    pub(crate) selected_nodes: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SceneSelectionEntry {
    pub(crate) selected_node: Option<u64>,
    pub(crate) selected_nodes: Vec<u64>,
}

impl SceneHistoryEntry {
    pub(crate) fn capture(tab: &SceneTab) -> Self {
        let selection = SceneSelectionEntry::capture(tab);
        Self {
            scene: tab.scene.clone(),
            selected_node: selection.selected_node,
            selected_nodes: selection.selected_nodes,
        }
    }
}

impl SceneSelectionEntry {
    pub(crate) fn capture(tab: &SceneTab) -> Self {
        let selected_nodes =
            sanitize_scene_selection_nodes(&tab.scene, &tab.selected_nodes, tab.selected_node);
        let selected_node =
            sanitize_scene_primary_selection(&tab.scene, tab.selected_node, &selected_nodes);

        Self {
            selected_node,
            selected_nodes,
        }
    }
}

pub(crate) struct SceneTab {
    pub(crate) scene: SceneDocument,
    pub(crate) scene_dirty: bool,
    pub(crate) scene_path: Option<PathBuf>,
    pub(crate) selected_node: Option<u64>,
    pub(crate) selected_nodes: Vec<u64>,
    pub(crate) viewport_drag: Option<ViewportDrag>,
    pub(crate) viewport_box_selection: Option<ViewportBoxSelection>,
    pub(crate) viewport_pan: Vec2,
    pub(crate) viewport_pan_drag: Option<ViewportPanDrag>,
    pub(crate) collapsed_nodes: HashSet<u64>,
    pub(crate) scene_json_cache: String,
    pub(crate) saved_scene_json: String,
    pub(crate) scene_json_dirty: bool,
    pub(crate) undo_history: Vec<SceneHistoryEntry>,
    pub(crate) redo_history: Vec<SceneHistoryEntry>,
    pub(crate) selection_back_history: Vec<SceneSelectionEntry>,
    pub(crate) selection_forward_history: Vec<SceneSelectionEntry>,
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
            selected_nodes: Vec::new(),
            viewport_drag: None,
            viewport_box_selection: None,
            viewport_pan: Vec2::ZERO,
            viewport_pan_drag: None,
            collapsed_nodes: HashSet::new(),
            saved_scene_json: scene_json_cache.clone(),
            scene_json_cache,
            scene_json_dirty: false,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            selection_back_history: Vec::new(),
            selection_forward_history: Vec::new(),
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
        Self::push_history_entry(&mut self.undo_history, entry);
        self.redo_history.clear();
    }

    pub(crate) fn selection_count(&self) -> usize {
        self.selected_nodes.len()
    }

    pub(crate) fn has_selection(&self) -> bool {
        !self.selected_nodes.is_empty()
    }

    pub(crate) fn is_node_selected(&self, node_id: u64) -> bool {
        self.selected_nodes.contains(&node_id)
    }

    pub(crate) fn selected_root_ids(&self) -> Vec<u64> {
        self.scene.selected_root_ids(&self.selected_nodes)
    }

    pub(crate) fn select_only_node(&mut self, node_id: Option<u64>) -> bool {
        self.set_selection(node_id, node_id.into_iter().collect())
    }

    pub(crate) fn set_selection(
        &mut self,
        selected_node: Option<u64>,
        selected_nodes: Vec<u64>,
    ) -> bool {
        self.restore_selection_entry(SceneSelectionEntry {
            selected_node,
            selected_nodes,
        })
    }

    pub(crate) fn focus_selected_node(&mut self, node_id: u64) -> bool {
        if !self.is_node_selected(node_id) {
            return self.select_only_node(Some(node_id));
        }

        self.set_selection(Some(node_id), self.selected_nodes.clone())
    }

    pub(crate) fn toggle_node_selection(&mut self, node_id: u64) -> bool {
        if self.is_node_selected(node_id) {
            let mut selected_nodes = self.selected_nodes.clone();
            selected_nodes.retain(|selected_id| *selected_id != node_id);
            let selected_node = if self.selected_node == Some(node_id) {
                selected_nodes.last().copied()
            } else {
                self.selected_node
            };
            return self.set_selection(selected_node, selected_nodes);
        }

        let mut selected_nodes = self.selected_nodes.clone();
        selected_nodes.retain(|selected_id| !self.scene.is_descendant_of(*selected_id, node_id));
        if let Some(ancestor_index) = selected_nodes
            .iter()
            .position(|selected_id| self.scene.is_descendant_of(node_id, *selected_id))
        {
            selected_nodes.remove(ancestor_index);
        }
        selected_nodes.push(node_id);
        self.set_selection(Some(node_id), selected_nodes)
    }

    pub(crate) fn push_selection_history_entry(&mut self, entry: SceneSelectionEntry) {
        Self::push_selection_entry(&mut self.selection_back_history, entry);
        self.selection_forward_history.clear();
    }

    pub(crate) fn selection_back(&mut self) -> bool {
        let Some(entry) = self.selection_back_history.pop() else {
            return false;
        };

        let current = SceneSelectionEntry::capture(self);
        Self::push_selection_entry(&mut self.selection_forward_history, current);
        self.restore_selection_entry(entry)
    }

    pub(crate) fn selection_forward(&mut self) -> bool {
        let Some(entry) = self.selection_forward_history.pop() else {
            return false;
        };

        let current = SceneSelectionEntry::capture(self);
        Self::push_selection_entry(&mut self.selection_back_history, current);
        self.restore_selection_entry(entry)
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
        self.set_selection(entry.selected_node, entry.selected_nodes);
        self.viewport_drag = None;
        self.viewport_box_selection = None;
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

        let current_state = SceneHistoryEntry::capture(self);
        Self::push_history_entry(&mut self.redo_history, current_state);
        self.restore_history_entry(entry);
        true
    }

    pub(crate) fn redo(&mut self) -> bool {
        let Some(entry) = self.redo_history.pop() else {
            return false;
        };

        let current_state = SceneHistoryEntry::capture(self);
        Self::push_history_entry(&mut self.undo_history, current_state);
        self.restore_history_entry(entry);
        true
    }

    fn push_history_entry(stack: &mut Vec<SceneHistoryEntry>, entry: SceneHistoryEntry) {
        if stack.last().is_some_and(|last| {
            last.scene == entry.scene
                && last.selected_node == entry.selected_node
                && last.selected_nodes == entry.selected_nodes
        }) {
            return;
        }

        stack.push(entry);
        if stack.len() > MAX_SCENE_HISTORY_STEPS {
            let overflow = stack.len() - MAX_SCENE_HISTORY_STEPS;
            stack.drain(0..overflow);
        }
    }

    pub(crate) fn cached_scene_json(&mut self) -> &str {
        if self.scene_json_dirty {
            self.scene_json_cache = self.scene.pretty_json();
            self.scene_json_dirty = false;
        }

        &self.scene_json_cache
    }

    fn restore_selection_entry(&mut self, entry: SceneSelectionEntry) -> bool {
        let selected_nodes =
            sanitize_scene_selection_nodes(&self.scene, &entry.selected_nodes, entry.selected_node);
        let selected_node =
            sanitize_scene_primary_selection(&self.scene, entry.selected_node, &selected_nodes);

        if self.selected_node == selected_node && self.selected_nodes == selected_nodes {
            return false;
        }

        self.selected_node = selected_node;
        self.selected_nodes = selected_nodes;
        self.viewport_drag = None;
        self.viewport_box_selection = None;
        true
    }

    fn push_selection_entry(stack: &mut Vec<SceneSelectionEntry>, entry: SceneSelectionEntry) {
        if stack.last().is_some_and(|last| last == &entry) {
            return;
        }

        stack.push(entry);
        if stack.len() > MAX_SCENE_HISTORY_STEPS {
            let overflow = stack.len() - MAX_SCENE_HISTORY_STEPS;
            stack.drain(0..overflow);
        }
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

fn sanitize_scene_primary_selection(
    scene: &SceneDocument,
    selected_node: Option<u64>,
    selected_nodes: &[u64],
) -> Option<u64> {
    selected_node
        .filter(|node_id| scene.node(*node_id).is_some())
        .filter(|node_id| selected_nodes.contains(node_id))
        .or_else(|| selected_nodes.last().copied())
}

fn sanitize_scene_selection_nodes(
    scene: &SceneDocument,
    selected_nodes: &[u64],
    selected_node: Option<u64>,
) -> Vec<u64> {
    let mut selected_ids: HashSet<u64> = selected_nodes
        .iter()
        .copied()
        .filter(|node_id| scene.node(*node_id).is_some())
        .collect();
    if let Some(node_id) = selected_node.filter(|node_id| scene.node(*node_id).is_some()) {
        selected_ids.insert(node_id);
    }

    scene.selected_root_ids(&selected_ids.into_iter().collect::<Vec<_>>())
}

pub(crate) fn history_modifier_down(engine: &Engine) -> bool {
    let input = engine.input();
    input.is_key_down(KeyCode::ControlLeft) || input.is_key_down(KeyCode::ControlRight)
}

fn history_shift_down(engine: &Engine) -> bool {
    let input = engine.input();
    input.is_key_down(KeyCode::ShiftLeft) || input.is_key_down(KeyCode::ShiftRight)
}

fn selection_alt_down(engine: &Engine) -> bool {
    let input = engine.input();
    input.is_key_down(KeyCode::AltLeft) || input.is_key_down(KeyCode::AltRight)
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

    pub(crate) fn update_scene_selection<F>(&mut self, mutate: F) -> bool
    where
        F: FnOnce(&mut SceneTab) -> bool,
    {
        let previous = SceneSelectionEntry::capture(self.active_scene_tab());
        let changed = {
            let tab = self.active_scene_tab_mut();
            mutate(tab)
        };

        if changed {
            self.active_scene_tab_mut()
                .push_selection_history_entry(previous);
            self.refresh_inspector_form();
        }

        changed
    }

    pub(crate) fn select_only_scene_node(&mut self, node_id: Option<u64>) -> bool {
        self.update_scene_selection(|tab| tab.select_only_node(node_id))
    }

    pub(crate) fn focus_scene_node(&mut self, node_id: u64) -> bool {
        self.update_scene_selection(|tab| tab.focus_selected_node(node_id))
    }

    pub(crate) fn toggle_scene_node_selection(&mut self, node_id: u64) -> bool {
        self.update_scene_selection(|tab| tab.toggle_node_selection(node_id))
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
        if self.active_text_input_owner.is_some() {
            return;
        }

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

    pub(crate) fn handle_scene_selection_shortcuts(&mut self, engine: &Engine) {
        if self.active_text_input_owner.is_some() {
            return;
        }

        if history_modifier_down(engine) && engine.input().is_key_pressed(KeyCode::KeyD) {
            if self.active_scene_tab().has_selection() {
                self.duplicate_selected_nodes();
            }
            return;
        }

        if selection_alt_down(engine) && engine.input().is_key_pressed(KeyCode::ArrowLeft) {
            self.navigate_selection_history(false);
            return;
        }

        if selection_alt_down(engine) && engine.input().is_key_pressed(KeyCode::ArrowRight) {
            self.navigate_selection_history(true);
            return;
        }

        if !self.active_scene_tab().has_selection() {
            return;
        }

        if selection_alt_down(engine) && engine.input().is_key_pressed(KeyCode::ArrowUp) {
            self.reorder_selected_nodes(SceneNodeReorderDirection::Up);
            return;
        }

        if selection_alt_down(engine) && engine.input().is_key_pressed(KeyCode::ArrowDown) {
            self.reorder_selected_nodes(SceneNodeReorderDirection::Down);
            return;
        }

        if engine.input().is_key_pressed(KeyCode::Tab) {
            if history_shift_down(engine) {
                self.outdent_selected_nodes();
            } else {
                self.indent_selected_nodes();
            }
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

    pub(crate) fn duplicate_selected_nodes(&mut self) {
        let selected_root_ids = self.active_scene_tab().selected_root_ids();
        if selected_root_ids.is_empty() {
            return;
        }

        let history_entry = SceneHistoryEntry::capture(self.active_scene_tab());
        let duplicated_root_ids = {
            let tab = self.active_scene_tab_mut();
            let duplicated_root_ids = tab.scene.duplicate_nodes(&selected_root_ids, [28.0, 28.0]);
            if duplicated_root_ids.is_empty() {
                return;
            }
            tab.mark_dirty();
            tab.set_selection(
                duplicated_root_ids.last().copied(),
                duplicated_root_ids.clone(),
            );
            tab.push_undo_entry(history_entry);
            duplicated_root_ids
        };

        self.refresh_inspector_form();
        self.push_log(format!("Duplicated {} node(s)", duplicated_root_ids.len()));
    }

    pub(crate) fn reorder_selected_nodes(&mut self, direction: SceneNodeReorderDirection) {
        let selected_root_ids = self.active_scene_tab().selected_root_ids();
        if selected_root_ids.is_empty() {
            return;
        }

        let history_entry = SceneHistoryEntry::capture(self.active_scene_tab());
        let changed = {
            let tab = self.active_scene_tab_mut();
            let changed = tab.scene.reorder_nodes(&selected_root_ids, direction);
            if changed {
                tab.mark_dirty();
                tab.push_undo_entry(history_entry);
            }
            changed
        };

        if changed {
            self.push_log(match direction {
                SceneNodeReorderDirection::Up => "Moved selection earlier in sibling order",
                SceneNodeReorderDirection::Down => "Moved selection later in sibling order",
            });
        }
    }

    pub(crate) fn indent_selected_nodes(&mut self) {
        let Some(primary_node_id) = self.active_scene_tab().selected_node else {
            return;
        };
        let scene = &self.active_scene_tab().scene;
        let parent = scene.node(primary_node_id).and_then(|node| node.parent);
        let sibling_ids = if let Some(parent_id) = parent {
            scene.child_ids(parent_id)
        } else {
            scene.root_ids()
        };
        let Some(index) = sibling_ids
            .iter()
            .position(|sibling_id| *sibling_id == primary_node_id)
        else {
            return;
        };
        if index == 0 {
            return;
        }

        self.reparent_selected_nodes(Some(sibling_ids[index - 1]), "Indented selection");
    }

    pub(crate) fn outdent_selected_nodes(&mut self) {
        let Some(primary_node_id) = self.active_scene_tab().selected_node else {
            return;
        };
        let Some(parent_id) = self
            .active_scene_tab()
            .scene
            .node(primary_node_id)
            .and_then(|node| node.parent)
        else {
            return;
        };
        let new_parent = self
            .active_scene_tab()
            .scene
            .node(parent_id)
            .and_then(|node| node.parent);

        self.reparent_selected_nodes(new_parent, "Outdented selection");
    }

    pub(crate) fn reparent_selected_nodes(&mut self, new_parent: Option<u64>, log_message: &str) {
        let selected_root_ids = self.active_scene_tab().selected_root_ids();
        if selected_root_ids.is_empty() {
            return;
        }

        let history_entry = SceneHistoryEntry::capture(self.active_scene_tab());
        let changed = {
            let tab = self.active_scene_tab_mut();
            let changed = tab.scene.reparent_nodes(&selected_root_ids, new_parent);
            if changed {
                tab.mark_dirty();
                tab.push_undo_entry(history_entry);
            }
            changed
        };

        if changed {
            self.push_log(log_message.to_string());
        }
    }

    pub(crate) fn navigate_selection_history(&mut self, forward: bool) {
        let changed = if forward {
            self.active_scene_tab_mut().selection_forward()
        } else {
            self.active_scene_tab_mut().selection_back()
        };

        if changed {
            self.refresh_inspector_form();
            self.push_log(if forward {
                "Advanced selection history".to_string()
            } else {
                "Rewound selection history".to_string()
            });
        }
    }
}
