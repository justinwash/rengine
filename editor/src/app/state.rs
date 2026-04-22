use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ViewportDrag {
    pub(crate) node_id: u64,
    pub(crate) pointer_offset: Vec2,
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
    pub(crate) scene_json_dirty: bool,
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
            scene_json_cache,
            scene_json_dirty: false,
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

impl RengineNativeEditor {
    pub(crate) fn active_scene_tab(&self) -> &SceneTab {
        &self.scene_tabs[self.active_scene_tab]
    }

    pub(crate) fn active_scene_tab_mut(&mut self) -> &mut SceneTab {
        &mut self.scene_tabs[self.active_scene_tab]
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
}