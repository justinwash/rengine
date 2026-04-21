use crate::scene::{SceneDocument, SceneNode, SceneNodeKind};
use eframe::egui::{
    self, Align, Align2, CentralPanel, CollapsingHeader, Color32, ComboBox, Context, Layout, Pos2,
    RichText, ScrollArea, Sense, SidePanel, Stroke, TextEdit, TextureHandle, TopBottomPanel, Ui,
    Vec2,
};
use image::ImageReader;
use rfd::FileDialog;
use std::{
    cmp::Ordering,
    collections::{hash_map::Entry, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

const MAX_ACTIVITY_LOG_LINES: usize = 96;

#[derive(Clone, Debug)]
struct ProjectTreeEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    children: Vec<ProjectTreeEntry>,
}

impl ProjectTreeEntry {
    fn scan(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.display().to_string());

        let is_dir = path.is_dir();
        let mut children = Vec::new();

        if is_dir {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let child_path = entry.path();
                    let child_is_dir = child_path.is_dir();

                    if should_skip_entry(&child_path, child_is_dir) {
                        continue;
                    }

                    children.push(ProjectTreeEntry::scan(&child_path));
                }
            }

            children.sort_by(|left, right| match (left.is_dir, right.is_dir) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => left
                    .name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase()),
            });
        }

        Self {
            name,
            path: path.to_path_buf(),
            is_dir,
            children,
        }
    }

    fn matches_filter(&self, filter: &str) -> bool {
        if filter.is_empty() {
            return true;
        }

        self.name.to_ascii_lowercase().contains(filter)
            || self
                .children
                .iter()
                .any(|child| child.matches_filter(filter))
    }

    fn contains_path(&self, path: &Path) -> bool {
        self.path == path || self.children.iter().any(|child| child.contains_path(path))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BottomTab {
    Activity,
    SceneJson,
}

impl BottomTab {
    fn label(self) -> &'static str {
        match self {
            Self::Activity => "Activity",
            Self::SceneJson => "Scene JSON",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ViewportDrag {
    node_id: u64,
    pointer_offset: Vec2,
}

struct ProjectTreeSelection {
    path: PathBuf,
    log: bool,
}

#[derive(Clone, Copy, Debug)]
enum ProjectTreeScanReason {
    Refresh,
    SaveScene,
}

impl ProjectTreeScanReason {
    fn completed_message(self) -> &'static str {
        match self {
            Self::Refresh => "Workspace browser refreshed",
            Self::SaveScene => "Workspace browser updated",
        }
    }
}

struct PendingProjectTreeScan {
    receiver: Receiver<ProjectTreeEntry>,
    reason: ProjectTreeScanReason,
}

#[derive(Clone)]
struct SpritePreview {
    texture: TextureHandle,
    size: [usize; 2],
}

#[derive(Clone)]
enum SpritePreviewCacheEntry {
    Loaded(SpritePreview),
    Failed(String),
}

struct SceneTab {
    scene: SceneDocument,
    scene_dirty: bool,
    scene_path: Option<PathBuf>,
    selected_node: Option<u64>,
    viewport_drag: Option<ViewportDrag>,
    scene_json_cache: String,
    scene_json_dirty: bool,
}

impl SceneTab {
    fn new(scene: SceneDocument, scene_path: Option<PathBuf>) -> Self {
        let scene_json_cache = scene.pretty_json();

        Self {
            scene,
            scene_dirty: false,
            scene_path,
            selected_node: None,
            viewport_drag: None,
            scene_json_cache,
            scene_json_dirty: false,
        }
    }

    fn untitled() -> Self {
        Self::new(SceneDocument::new("untitled_scene"), None)
    }

    fn is_fresh_untitled(&self) -> bool {
        self.scene_path.is_none()
            && !self.scene_dirty
            && self.scene.nodes.is_empty()
            && self.scene.name == "untitled_scene"
    }

    fn display_name(&self) -> String {
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

    fn tab_label(&self) -> String {
        let mut label = self.display_name();
        if self.scene_dirty {
            label.push('*');
        }
        label
    }

    fn mark_dirty(&mut self) {
        self.scene_dirty = true;
        self.scene_json_dirty = true;
    }

    fn cached_scene_json(&mut self) -> &str {
        if self.scene_json_dirty {
            self.scene_json_cache = self.scene.pretty_json();
            self.scene_json_dirty = false;
        }

        &self.scene_json_cache
    }
}

pub struct RengineEditorApp {
    workspace_root: PathBuf,
    branch_name: String,
    project_tree: ProjectTreeEntry,
    scene_tabs: Vec<SceneTab>,
    active_scene_tab: usize,
    selected_project_path: Option<PathBuf>,
    file_filter: String,
    activity_log: Vec<String>,
    bottom_tab: BottomTab,
    viewport_menu_parent: Option<u64>,
    viewport_menu_position: Option<[f32; 2]>,
    sprite_preview_cache: HashMap<PathBuf, SpritePreviewCacheEntry>,
    pending_project_tree_scan: Option<PendingProjectTreeScan>,
    queued_project_tree_scan: Option<ProjectTreeScanReason>,
}

impl RengineEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_editor_theme(&cc.egui_ctx);

        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project_tree = ProjectTreeEntry::scan(&workspace_root);
        let branch_name = read_git_branch(&workspace_root);

        let mut app = Self {
            workspace_root,
            branch_name,
            project_tree,
            scene_tabs: vec![SceneTab::untitled()],
            active_scene_tab: 0,
            selected_project_path: None,
            file_filter: String::new(),
            activity_log: Vec::new(),
            bottom_tab: BottomTab::Activity,
            viewport_menu_parent: None,
            viewport_menu_position: None,
            sprite_preview_cache: HashMap::new(),
            pending_project_tree_scan: None,
            queued_project_tree_scan: None,
        };

        app.push_log("Booted editor shell prototype");
        app.push_log(format!(
            "Opened workspace {}",
            app.display_path(&app.workspace_root)
        ));
        app.push_log("Started new empty scene");
        app
    }

    fn active_scene_tab(&self) -> &SceneTab {
        &self.scene_tabs[self.active_scene_tab]
    }

    fn active_scene_tab_mut(&mut self) -> &mut SceneTab {
        &mut self.scene_tabs[self.active_scene_tab]
    }

    fn switch_to_scene_tab(&mut self, index: usize) {
        if index >= self.scene_tabs.len() || index == self.active_scene_tab {
            return;
        }

        self.active_scene_tab = index;
        let scene_path = self.active_scene_tab().scene_path.clone();
        let scene_label = self.active_scene_tab().display_name();

        if let Some(path) = scene_path {
            self.selected_project_path = Some(path.clone());
            self.push_log(format!("Switched to scene {}", self.display_path(&path)));
        } else {
            self.push_log(format!("Switched to scene {}", scene_label));
        }
    }

    fn scene_parent_label(&self, parent: Option<u64>) -> String {
        parent
            .and_then(|id| {
                self.active_scene_tab()
                    .scene
                    .node_name(id)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| "scene root".to_string())
    }

    fn push_log(&mut self, message: impl Into<String>) {
        self.activity_log.push(message.into());
        if self.activity_log.len() > MAX_ACTIVITY_LOG_LINES {
            let overflow = self.activity_log.len() - MAX_ACTIVITY_LOG_LINES;
            self.activity_log.drain(0..overflow);
        }
    }

    fn display_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.workspace_root)
            .map(|relative| {
                if relative.as_os_str().is_empty() {
                    ".".to_string()
                } else {
                    relative.display().to_string()
                }
            })
            .unwrap_or_else(|_| path.display().to_string())
    }

    fn default_scene_path(&self) -> PathBuf {
        self.workspace_root
            .join("editor")
            .join("scratch")
            .join("scene-prototype.scene.json")
    }

    fn dialog_directory(&self) -> PathBuf {
        if let Some(selected_path) = &self.selected_project_path {
            if selected_path.is_dir() {
                return selected_path.clone();
            }

            if let Some(parent) = selected_path.parent() {
                return parent.to_path_buf();
            }
        }

        if let Some(scene_path) = &self.active_scene_tab().scene_path {
            if let Some(parent) = scene_path.parent() {
                return parent.to_path_buf();
            }
        }

        self.workspace_root.clone()
    }

    fn suggested_scene_file_name(&self) -> String {
        let stem = if let Some(scene_path) = &self.active_scene_tab().scene_path {
            scene_path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
        } else {
            None
        };

        stem.unwrap_or_else(|| {
            let scene_name = self.active_scene_tab().scene.name.trim();
            if scene_name.is_empty() {
                "untitled_scene.scene.json".to_string()
            } else {
                format!("{}.scene.json", scene_name.replace(' ', "_"))
            }
        })
    }

    fn normalize_scene_save_path(&self, path: PathBuf) -> PathBuf {
        if path.extension().is_some() {
            path
        } else {
            path.with_extension("json")
        }
    }

    fn stored_workspace_path(&self, path: &Path) -> String {
        let stored_path = path
            .strip_prefix(&self.workspace_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        if stored_path.is_empty() {
            path.to_string_lossy().replace('\\', "/")
        } else {
            stored_path
        }
    }

    fn resolve_stored_path(&self, stored_path: &str) -> PathBuf {
        let path = PathBuf::from(stored_path);
        if path.is_absolute() {
            path
        } else {
            self.workspace_root.join(path)
        }
    }

    fn selected_sprite_source_path(&self) -> Option<PathBuf> {
        self.selected_project_path
            .as_ref()
            .filter(|path| path.is_file() && is_supported_sprite_path(path))
            .cloned()
    }

    fn pick_sprite_source_path(&self) -> Option<PathBuf> {
        if let Some(path) = self.selected_sprite_source_path() {
            return Some(path);
        }

        FileDialog::new()
            .set_directory(self.dialog_directory())
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
    }

    fn sprite_preview_entry(
        &mut self,
        ctx: &Context,
        stored_path: &str,
    ) -> Option<SpritePreviewCacheEntry> {
        let trimmed_path = stored_path.trim();
        if trimmed_path.is_empty() {
            return None;
        }

        let resolved_path = self.resolve_stored_path(trimmed_path);
        let entry = match self.sprite_preview_cache.entry(resolved_path.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let cached_entry = match load_sprite_preview_from_disk(ctx, &resolved_path) {
                    Ok(preview) => SpritePreviewCacheEntry::Loaded(preview),
                    Err(error) => SpritePreviewCacheEntry::Failed(error),
                };

                entry.insert(cached_entry.clone());
                cached_entry
            }
        };

        Some(entry)
    }

    fn set_node_sprite_texture_path(&mut self, node_id: u64, path: &Path) {
        let stored_path = self.stored_workspace_path(path);
        let dimensions = image::image_dimensions(path)
            .ok()
            .map(|(width, height)| [width as f32, height as f32]);
        let sprite_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);

        let tab = self.active_scene_tab_mut();
        let Some(node) = tab.scene.node_mut(node_id) else {
            return;
        };

        node.sprite.texture_path = stored_path;

        if let Some(sprite_name) = sprite_name {
            if node.name.trim().is_empty() || node.name.starts_with("Sprite ") {
                node.name = sprite_name;
            }
        }

        if let Some(size) = dimensions {
            node.size = size;
        }

        tab.mark_dirty();
    }

    fn clear_node_sprite_texture_path(&mut self, node_id: u64) {
        let tab = self.active_scene_tab_mut();
        let Some(node) = tab.scene.node_mut(node_id) else {
            return;
        };

        node.sprite.texture_path.clear();
        tab.mark_dirty();
    }

    fn choose_sprite_for_node(&mut self, node_id: u64) {
        let Some(path) = self.pick_sprite_source_path() else {
            return;
        };

        self.set_node_sprite_texture_path(node_id, &path);
        self.push_log(format!(
            "Updated sprite source to {}",
            self.display_path(&path)
        ));
    }

    fn project_tree_scan_pending(&self) -> bool {
        self.pending_project_tree_scan.is_some()
    }

    fn spawn_project_tree_scan(&mut self, reason: ProjectTreeScanReason) {
        let workspace_root = self.workspace_root.clone();
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let _ = sender.send(ProjectTreeEntry::scan(&workspace_root));
        });

        self.pending_project_tree_scan = Some(PendingProjectTreeScan { receiver, reason });
    }

    fn request_project_tree_scan(&mut self, reason: ProjectTreeScanReason) {
        if self.project_tree_scan_pending() {
            self.queued_project_tree_scan = Some(reason);
            return;
        }

        self.spawn_project_tree_scan(reason);
    }

    fn poll_project_tree_scan(&mut self) {
        let Some(pending_scan) = self.pending_project_tree_scan.take() else {
            return;
        };

        match pending_scan.receiver.try_recv() {
            Ok(project_tree) => {
                self.project_tree = project_tree;
                self.push_log(pending_scan.reason.completed_message());

                if let Some(queued_reason) = self.queued_project_tree_scan.take() {
                    self.spawn_project_tree_scan(queued_reason);
                }
            }
            Err(TryRecvError::Empty) => {
                self.pending_project_tree_scan = Some(pending_scan);
            }
            Err(TryRecvError::Disconnected) => {
                self.push_log("Workspace browser refresh failed");

                if let Some(queued_reason) = self.queued_project_tree_scan.take() {
                    self.spawn_project_tree_scan(queued_reason);
                }
            }
        }
    }

    fn refresh_project_tree(&mut self) {
        self.request_project_tree_scan(ProjectTreeScanReason::Refresh);
    }

    fn mark_scene_dirty(&mut self) {
        self.active_scene_tab_mut().mark_dirty();
    }

    fn cached_scene_json(&mut self) -> &str {
        self.active_scene_tab_mut().cached_scene_json()
    }

    fn new_scene(&mut self) {
        self.scene_tabs.push(SceneTab::untitled());
        self.active_scene_tab = self.scene_tabs.len() - 1;
        self.push_log("Started new empty scene");
    }

    fn open_scene(&mut self) {
        let Some(path) = FileDialog::new()
            .set_directory(self.dialog_directory())
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };

        self.open_scene_path(path);
    }

    fn open_scene_path(&mut self, path: PathBuf) {
        if let Some(index) = self
            .scene_tabs
            .iter()
            .position(|tab| tab.scene_path.as_deref() == Some(path.as_path()))
        {
            let was_active = index == self.active_scene_tab;
            self.selected_project_path = Some(path.clone());
            self.switch_to_scene_tab(index);
            if was_active {
                self.push_log(format!("Focused open scene {}", self.display_path(&path)));
            }
            return;
        }

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                self.push_log(format!(
                    "Failed to open {}: {}",
                    self.display_path(&path),
                    error
                ));
                return;
            }
        };

        let scene = match serde_json::from_str::<SceneDocument>(&text) {
            Ok(scene) => scene,
            Err(error) => {
                self.push_log(format!(
                    "Failed to parse {} as an editor scene: {}",
                    self.display_path(&path),
                    error
                ));
                return;
            }
        };

        let replace_active_tab =
            self.scene_tabs.len() == 1 && self.active_scene_tab().is_fresh_untitled();

        if replace_active_tab {
            self.scene_tabs[self.active_scene_tab] = SceneTab::new(scene, Some(path.clone()));
        } else {
            self.scene_tabs
                .push(SceneTab::new(scene, Some(path.clone())));
            self.active_scene_tab = self.scene_tabs.len() - 1;
        }

        self.selected_project_path = Some(path.clone());
        self.push_log(format!("Opened scene {}", self.display_path(&path)));
    }

    fn add_node_with_parent(
        &mut self,
        kind: SceneNodeKind,
        parent: Option<u64>,
        position: Option<[f32; 2]>,
    ) {
        let parent_label = self.scene_parent_label(parent);

        {
            let tab = self.active_scene_tab_mut();
            let node_id = tab.scene.add_node(kind, parent);

            if let Some(position) = position {
                if let Some(node) = tab.scene.node_mut(node_id) {
                    node.position = position;
                }
            }

            tab.mark_dirty();
            tab.selected_node = Some(node_id);
            node_id
        };

        self.push_log(format!("Added {} under {}", kind.label(), parent_label));
    }

    fn add_sprite_node_with_parent(&mut self, parent: Option<u64>, position: Option<[f32; 2]>) {
        let parent_label = self.scene_parent_label(parent);
        let sprite_path = self.pick_sprite_source_path();

        let node_id = {
            let tab = self.active_scene_tab_mut();
            let node_id = tab.scene.add_node(SceneNodeKind::Sprite, parent);

            if let Some(position) = position {
                if let Some(node) = tab.scene.node_mut(node_id) {
                    node.position = position;
                }
            }

            tab.mark_dirty();
            tab.selected_node = Some(node_id);
            node_id
        };

        if let Some(path) = sprite_path {
            self.set_node_sprite_texture_path(node_id, &path);
            self.push_log(format!(
                "Added Sprite under {} from {}",
                parent_label,
                self.display_path(&path)
            ));
        } else {
            self.push_log(format!(
                "Added Sprite under {} with placeholder preview",
                parent_label
            ));
        }
    }

    fn save_scene(&mut self) {
        let path = self
            .active_scene_tab()
            .scene_path
            .clone()
            .unwrap_or_else(|| self.default_scene_path());
        self.save_scene_to_path(path);
    }

    fn save_scene_as(&mut self) {
        let Some(path) = FileDialog::new()
            .set_directory(self.dialog_directory())
            .set_file_name(&self.suggested_scene_file_name())
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };

        self.save_scene_to_path(self.normalize_scene_save_path(path));
    }

    fn save_scene_to_path(&mut self, path: PathBuf) {
        let path = self.normalize_scene_save_path(path);

        if let Some(parent) = path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                self.push_log(format!(
                    "Failed to create {}: {}",
                    self.display_path(parent),
                    error
                ));
                return;
            }
        }

        let scene_json = self.cached_scene_json().to_owned();
        let tree_has_path = self.project_tree.contains_path(&path);

        match fs::write(&path, scene_json) {
            Ok(()) => {
                let tab = self.active_scene_tab_mut();
                tab.scene_path = Some(path.clone());
                tab.scene_dirty = false;
                self.selected_project_path = Some(path.clone());
                if !tree_has_path {
                    self.request_project_tree_scan(ProjectTreeScanReason::SaveScene);
                }
                self.push_log(format!("Saved scene to {}", self.display_path(&path)));
            }
            Err(error) => {
                self.push_log(format!(
                    "Failed to save {}: {}",
                    self.display_path(&path),
                    error
                ));
            }
        }
    }

    fn show_title_bar(&mut self, ui: &mut Ui) {
        let wide = ui.available_width() > 1200.0;
        let workspace_name = self
            .workspace_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
            .to_string();
        let active_scene_dirty = self.active_scene_tab().scene_dirty;
        let active_scene_path = self.active_scene_tab().scene_path.clone();

        ui.horizontal_wrapped(|ui| {
            ui.heading("Rengine Editor");
            ui.separator();
            ui.label(RichText::new(workspace_name).strong());

            if wide {
                ui.separator();
                ui.label(
                    RichText::new(self.display_path(&self.workspace_root))
                        .color(Color32::from_gray(170)),
                );
            }

            ui.separator();
            ui.label(
                RichText::new(format!("branch {}", self.branch_name))
                    .color(Color32::from_rgb(128, 196, 172)),
            );

            if active_scene_dirty {
                ui.separator();
                ui.label(RichText::new("unsaved scene").color(Color32::from_rgb(238, 187, 85)));
            }

            if wide {
                if let Some(scene_path) = &active_scene_path {
                    ui.separator();
                    ui.label(
                        RichText::new(self.display_path(scene_path)).color(Color32::from_gray(170)),
                    );
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Refresh Files").clicked() {
                    self.refresh_project_tree();
                }
                if ui.button("Save As").clicked() {
                    self.save_scene_as();
                }
                if ui.button("Save Scene").clicked() {
                    self.save_scene();
                }
                if ui.button("Open Scene").clicked() {
                    self.open_scene();
                }
                if ui.button("New Scene").clicked() {
                    self.new_scene();
                }
            });
        });
    }

    fn show_scene_tabs(&mut self, ui: &mut Ui) {
        let mut switch_to = None;
        let mut create_new_scene = false;

        ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for index in 0..self.scene_tabs.len() {
                        let label = self.scene_tabs[index].tab_label();
                        let tooltip = self.scene_tabs[index]
                            .scene_path
                            .as_ref()
                            .map(|path| self.display_path(path));
                        let mut response =
                            ui.selectable_label(self.active_scene_tab == index, label);

                        if let Some(path) = tooltip {
                            response = response.on_hover_text(path);
                        }

                        if response.clicked() {
                            switch_to = Some(index);
                        }
                    }

                    ui.separator();

                    if ui.small_button("+ New Scene").clicked() {
                        create_new_scene = true;
                    }
                });
            });

        if let Some(index) = switch_to {
            self.switch_to_scene_tab(index);
        }

        if create_new_scene {
            self.new_scene();
        }
    }

    fn show_add_node_menu(&mut self, ui: &mut Ui, parent: Option<u64>, position: Option<[f32; 2]>) {
        ui.label(
            RichText::new(format!("Add under {}", self.scene_parent_label(parent)))
                .color(Color32::from_gray(180)),
        );
        ui.separator();

        for kind in [
            SceneNodeKind::Group,
            SceneNodeKind::Empty,
            SceneNodeKind::Camera2d,
            SceneNodeKind::Sprite,
            SceneNodeKind::Trigger,
            SceneNodeKind::UiRoot,
        ] {
            let label = if kind == SceneNodeKind::Sprite {
                "Sprite..."
            } else {
                kind.label()
            };

            if ui.button(label).clicked() {
                if kind == SceneNodeKind::Sprite {
                    self.add_sprite_node_with_parent(parent, position);
                } else {
                    self.add_node_with_parent(kind, parent, position);
                }
                ui.close();
            }
        }
    }

    fn show_project_browser(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Files");
            if ui.small_button("Refresh").clicked() {
                self.refresh_project_tree();
            }
        });

        ui.label(
            RichText::new(self.display_path(&self.workspace_root)).color(Color32::from_gray(170)),
        );
        ui.add_space(4.0);
        ui.add(TextEdit::singleline(&mut self.file_filter).hint_text("Filter files"));
        ui.add_space(8.0);

        if self.project_tree_scan_pending() {
            ui.label(
                RichText::new("Refreshing workspace in background...")
                    .color(Color32::from_rgb(128, 196, 172)),
            );
            ui.add_space(6.0);
        }

        if let Some(selected_path) = &self.selected_project_path {
            ui.label(
                RichText::new(self.display_path(selected_path))
                    .color(Color32::from_rgb(120, 186, 255)),
            );
            ui.add_space(4.0);

            if selected_path.is_file() && ui.small_button("Open Selected Scene").clicked() {
                self.open_scene_path(selected_path.clone());
            }
        }

        let filter = self.file_filter.trim().to_ascii_lowercase();
        let selected_project_path = self.selected_project_path.as_deref();
        let mut pending_selection = None;
        ScrollArea::vertical().show(ui, |ui| {
            show_project_tree_entry(
                ui,
                &self.project_tree,
                &filter,
                0,
                selected_project_path,
                &mut pending_selection,
            );
        });

        if let Some(selection) = pending_selection {
            let selected_path = selection.path;
            self.selected_project_path = Some(selected_path.clone());
            if selection.log {
                self.push_log(format!("Selected {}", self.display_path(&selected_path)));
            }
        }
    }

    fn show_scene_hierarchy(&mut self, ui: &mut Ui) {
        ui.heading("Scene");
        ui.add_space(4.0);

        let scene_name_changed = {
            let tab = self.active_scene_tab_mut();
            ui.add(TextEdit::singleline(&mut tab.scene.name).hint_text("Scene name"))
                .changed()
        };

        if scene_name_changed {
            self.mark_scene_dirty();
        }

        ui.label(
            RichText::new("Right-click scene nodes or the viewport to add children.")
                .color(Color32::from_gray(165)),
        );
        ui.label(
            RichText::new(format!(
                "{} node(s)",
                self.active_scene_tab().scene.nodes.len()
            ))
            .color(Color32::from_gray(170)),
        );
        ui.add_space(8.0);

        ScrollArea::vertical().show(ui, |ui| {
            let root_ids = self.active_scene_tab().scene.root_ids();
            if root_ids.is_empty() {
                ui.label(
                    RichText::new(
                        "Scene is empty. Right-click in the viewport to add the first node.",
                    )
                    .color(Color32::from_gray(165)),
                );
                return;
            }

            for node_id in root_ids {
                self.show_scene_node_entry(ui, node_id);
            }
        });
    }

    fn show_scene_node_entry(&mut self, ui: &mut Ui, node_id: u64) {
        let (label, children) = if let Some(node) = self.active_scene_tab().scene.node(node_id) {
            (
                format!("{} {}", node.kind.short_label(), node.name),
                self.active_scene_tab().scene.child_ids(node_id),
            )
        } else {
            return;
        };

        if children.is_empty() {
            let response = ui.selectable_label(
                self.active_scene_tab().selected_node == Some(node_id),
                label,
            );

            if response.clicked() || response.secondary_clicked() {
                self.active_scene_tab_mut().selected_node = Some(node_id);
            }

            response.context_menu(|ui| {
                self.show_add_node_menu(ui, Some(node_id), None);
            });
            return;
        }

        let header = CollapsingHeader::new(label)
            .id_salt(node_id)
            .default_open(true)
            .show(ui, |ui| {
                for child_id in children {
                    self.show_scene_node_entry(ui, child_id);
                }
            });

        let header_response = header.header_response;
        if header_response.clicked() || header_response.secondary_clicked() {
            self.active_scene_tab_mut().selected_node = Some(node_id);
        }

        header_response.context_menu(|ui| {
            self.show_add_node_menu(ui, Some(node_id), None);
        });
    }

    fn show_inspector(&mut self, ui: &mut Ui) {
        ui.heading("Properties");
        ui.add_space(6.0);

        self.show_scene_properties(ui);

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        if let Some(node_id) = self.active_scene_tab().selected_node {
            self.show_selected_node_properties(ui, node_id);
        } else {
            ui.label(
                RichText::new("Select a node to edit its node-specific properties.")
                    .color(Color32::from_gray(170)),
            );
            ui.add_space(8.0);
            ui.label("Scene settings above control preview window size and camera framing.");
        }
    }

    fn show_scene_properties(&mut self, ui: &mut Ui) {
        let mut changed = false;

        CollapsingHeader::new("Scene View")
            .id_salt("scene_view_properties")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Window Size");
                    let tab = self.active_scene_tab_mut();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut tab.scene.view.window_size[0])
                                .range(64.0..=4096.0)
                                .speed(1.0),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut tab.scene.view.window_size[1])
                                .range(64.0..=4096.0)
                                .speed(1.0),
                        )
                        .changed();
                });

                ui.label(
                    RichText::new(
                        "Used as the default Camera2D screen preview size unless a camera overrides it.",
                    )
                    .color(Color32::from_gray(165)),
                );
            });

        if changed {
            self.mark_scene_dirty();
        }
    }

    fn show_selected_node_properties(&mut self, ui: &mut Ui, node_id: u64) {
        let Some(selected_node) = self.active_scene_tab().scene.node(node_id) else {
            ui.label("Selected node is no longer available.");
            return;
        };

        ui.label(
            RichText::new(format!("Node {}", selected_node.id)).color(Color32::from_gray(170)),
        );

        let mut changed = false;

        CollapsingHeader::new("Node")
            .id_salt(("node_section", node_id))
            .default_open(true)
            .show(ui, |ui| {
                let tab = self.active_scene_tab_mut();
                let Some(node) = tab.scene.node_mut(node_id) else {
                    return;
                };

                changed |= ui
                    .add(TextEdit::singleline(&mut node.name).hint_text("Node name"))
                    .changed();

                ComboBox::from_label("Kind")
                    .selected_text(node.kind.label())
                    .show_ui(ui, |ui| {
                        for kind in [
                            SceneNodeKind::Group,
                            SceneNodeKind::Empty,
                            SceneNodeKind::Camera2d,
                            SceneNodeKind::Sprite,
                            SceneNodeKind::Trigger,
                            SceneNodeKind::UiRoot,
                        ] {
                            changed |= ui
                                .selectable_value(&mut node.kind, kind, kind.label())
                                .changed();
                        }
                    });

                changed |= ui.checkbox(&mut node.visible, "Visible").changed();
            });

        CollapsingHeader::new("Transform")
            .id_salt(("transform_section", node_id))
            .default_open(true)
            .show(ui, |ui| {
                let tab = self.active_scene_tab_mut();
                let Some(node) = tab.scene.node_mut(node_id) else {
                    return;
                };

                ui.horizontal(|ui| {
                    ui.label("Position");
                    changed |= ui
                        .add(egui::DragValue::new(&mut node.position[0]).speed(1.0))
                        .changed();
                    changed |= ui
                        .add(egui::DragValue::new(&mut node.position[1]).speed(1.0))
                        .changed();
                });

                ui.horizontal(|ui| {
                    ui.label("Size");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut node.size[0])
                                .range(16.0..=4096.0)
                                .speed(1.0),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut node.size[1])
                                .range(16.0..=4096.0)
                                .speed(1.0),
                        )
                        .changed();
                });
            });

        CollapsingHeader::new("Script")
            .id_salt(("script_section", node_id))
            .default_open(false)
            .show(ui, |ui| {
                let tab = self.active_scene_tab_mut();
                let Some(node) = tab.scene.node_mut(node_id) else {
                    return;
                };

                changed |= ui
                    .add(
                        TextEdit::singleline(&mut node.script_path).hint_text("scripts/example.rs"),
                    )
                    .changed();
            });

        CollapsingHeader::new("Runtime Bridge")
            .id_salt(("runtime_bridge_section", node_id))
            .default_open(true)
            .show(ui, |ui| {
                let tab = self.active_scene_tab_mut();
                let Some(node) = tab.scene.node_mut(node_id) else {
                    return;
                };

                changed |= ui
                    .add(
                        TextEdit::singleline(&mut node.runtime_prefab)
                            .hint_text("Prefab name override (falls back to node name)"),
                    )
                    .changed();
                changed |= ui
                    .add(TextEdit::singleline(&mut node.asset_alias).hint_text("Sprite asset alias"))
                    .changed();

                if node.kind == SceneNodeKind::Sprite {
                    ui.label(
                        RichText::new(
                            "Texture preview and runtime asset alias are separate. Runtime export still uses the asset alias field.",
                        )
                        .color(Color32::from_gray(165)),
                    );
                } else if node.kind == SceneNodeKind::Group {
                    ui.label(
                        RichText::new(
                            "Group nodes export as composite prefabs. Descendant sprites are folded into the group's prefab using offsets from the group root.",
                        )
                        .color(Color32::from_gray(165)),
                    );
                } else {
                    ui.label(
                        RichText::new(
                            "Non-sprite nodes export as marker prefabs. Script path, size, and editor metadata are preserved in instance properties.",
                        )
                        .color(Color32::from_gray(165)),
                    );
                }
            });

        if changed {
            self.mark_scene_dirty();
        }

        let active_kind = self
            .active_scene_tab()
            .scene
            .node(node_id)
            .map(|node| node.kind);

        match active_kind {
            Some(SceneNodeKind::Sprite) => self.show_sprite_node_properties(ui, node_id),
            Some(SceneNodeKind::Camera2d) => self.show_camera_node_properties(ui, node_id),
            _ => {}
        }
    }

    fn show_sprite_node_properties(&mut self, ui: &mut Ui, node_id: u64) {
        let stored_path = self
            .active_scene_tab()
            .scene
            .node(node_id)
            .map(|node| node.sprite.texture_path.clone())
            .unwrap_or_default();
        let preview_entry = self.sprite_preview_entry(ui.ctx(), &stored_path);
        let mut browse_for_texture = false;
        let mut clear_texture = false;

        CollapsingHeader::new("Sprite")
            .id_salt(("sprite_section", node_id))
            .default_open(true)
            .show(ui, |ui| {
                if stored_path.trim().is_empty() {
                    ui.label(
                        RichText::new(
                            "No texture selected. This sprite uses the editor placeholder.",
                        )
                        .color(Color32::from_gray(170)),
                    );
                } else {
                    ui.label(RichText::new(stored_path.as_str()).color(Color32::from_gray(170)));
                }

                ui.horizontal(|ui| {
                    if ui.button("Browse PNG...").clicked() {
                        browse_for_texture = true;
                    }

                    if !stored_path.trim().is_empty() && ui.button("Use Placeholder").clicked() {
                        clear_texture = true;
                    }
                });

                ui.add_space(4.0);

                match preview_entry.as_ref() {
                    Some(SpritePreviewCacheEntry::Loaded(preview)) => {
                        let preview_size = fit_preview_size(preview.size, 180.0);
                        ui.image((preview.texture.id(), preview_size));
                        ui.label(
                            RichText::new(format!(
                                "{} x {} preview",
                                preview.size[0], preview.size[1]
                            ))
                            .color(Color32::from_gray(165)),
                        );
                    }
                    Some(SpritePreviewCacheEntry::Failed(error)) => {
                        show_placeholder_thumbnail(ui, Vec2::new(180.0, 120.0));
                        ui.label(
                            RichText::new(error.as_str()).color(Color32::from_rgb(214, 133, 133)),
                        );
                    }
                    None => {
                        show_placeholder_thumbnail(ui, Vec2::new(180.0, 120.0));
                    }
                }
            });

        if browse_for_texture {
            self.choose_sprite_for_node(node_id);
        }

        if clear_texture {
            self.clear_node_sprite_texture_path(node_id);
            self.push_log("Sprite reverted to placeholder preview");
        }
    }

    fn show_camera_node_properties(&mut self, ui: &mut Ui, node_id: u64) {
        let scene_window_size = self.active_scene_tab().scene.view.window_size;
        let mut changed = false;

        CollapsingHeader::new("Camera2D")
            .id_salt(("camera_section", node_id))
            .default_open(true)
            .show(ui, |ui| {
                let tab = self.active_scene_tab_mut();
                let Some(node) = tab.scene.node_mut(node_id) else {
                    return;
                };

                changed |= ui
                    .checkbox(&mut node.camera2d.show_bounds, "Show bounds in viewport")
                    .changed();
                changed |= ui
                    .checkbox(&mut node.camera2d.use_scene_view_size, "Use scene window size")
                    .changed();

                ui.horizontal(|ui| {
                    ui.label("Zoom");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut node.camera2d.zoom)
                                .range(0.1..=8.0)
                                .speed(0.05),
                        )
                        .changed();
                });

                let preview_size = if node.camera2d.use_scene_view_size {
                    scene_window_size
                } else {
                    node.camera2d.view_size
                };

                ui.label(
                    RichText::new(format!(
                        "Current preview: {:.0} x {:.0}",
                        preview_size[0], preview_size[1]
                    ))
                    .color(Color32::from_gray(165)),
                );

                if !node.camera2d.use_scene_view_size {
                    ui.horizontal(|ui| {
                        ui.label("View Size");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut node.camera2d.view_size[0])
                                    .range(64.0..=4096.0)
                                    .speed(1.0),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut node.camera2d.view_size[1])
                                    .range(64.0..=4096.0)
                                    .speed(1.0),
                            )
                            .changed();
                    });
                }

                ui.label(
                    RichText::new(
                        "The viewport draws the camera's screen rectangle from this view size and zoom.",
                    )
                    .color(Color32::from_gray(165)),
                );
            });

        if changed {
            self.mark_scene_dirty();
        }
    }

    fn show_bottom_panel(&mut self, ui: &mut Ui, freeze_expensive_views: bool) {
        ui.horizontal(|ui| {
            for tab in [BottomTab::Activity, BottomTab::SceneJson] {
                ui.selectable_value(&mut self.bottom_tab, tab, tab.label());
            }
        });
        ui.separator();

        match self.bottom_tab {
            BottomTab::Activity => {
                ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for line in &self.activity_log {
                        ui.label(line);
                    }
                });
            }
            BottomTab::SceneJson => {
                let preview_paused =
                    freeze_expensive_views && self.active_scene_tab().scene_json_dirty;

                ui.horizontal(|ui| {
                    if ui.button("Copy JSON").clicked() {
                        ui.ctx().copy_text(self.cached_scene_json().to_owned());
                    }

                    if preview_paused {
                        ui.label(
                            RichText::new("Preview paused during live interaction")
                                .color(Color32::from_rgb(238, 187, 85)),
                        );
                    }
                });

                let scene_json = if preview_paused {
                    self.active_scene_tab().scene_json_cache.clone()
                } else {
                    self.cached_scene_json().to_owned()
                };

                if preview_paused {
                    ui.add_space(2.0);
                }

                ui.add_space(6.0);
                ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new(scene_json).monospace());
                });
            }
        }
    }

    fn show_viewport(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Scene View");
            ui.label(
                RichText::new(
                    "Double-click to add an empty node. Right-click to add nodes, pick sprite PNGs, and preview Camera2D bounds.",
                )
                .color(Color32::from_gray(170)),
            );
        });
        ui.add_space(8.0);

        let available_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(available_size, Sense::click_and_drag());
        let rect = response.rect;

        painter.rect_filled(rect, 0.0, Color32::from_rgb(17, 22, 28));
        draw_grid(&painter, rect);

        let (scene_is_empty, selected_node, scene_window_size, visible_nodes) = {
            let tab = self.active_scene_tab();
            let visible_nodes = tab
                .scene
                .nodes
                .iter()
                .filter(|node| node.visible)
                .cloned()
                .collect::<Vec<_>>();

            (
                tab.scene.nodes.is_empty(),
                tab.selected_node,
                tab.scene.view.window_size,
                visible_nodes,
            )
        };

        for node in &visible_nodes {
            if node.kind != SceneNodeKind::Camera2d || !node.camera2d.show_bounds {
                continue;
            }

            let preview_size = camera_preview_size(node, scene_window_size);
            let zoom = node.camera2d.zoom.max(0.1);
            let bounds_rect = node_view_rect(
                node.position,
                [preview_size[0] / zoom, preview_size[1] / zoom],
                rect,
            );
            let bounds_fill = if selected_node == Some(node.id) {
                Color32::from_rgba_premultiplied(53, 125, 132, 24)
            } else {
                Color32::from_rgba_premultiplied(53, 125, 132, 14)
            };

            painter.rect_filled(bounds_rect, 0.0, bounds_fill);
            draw_rect_outline(
                &painter,
                bounds_rect,
                if selected_node == Some(node.id) {
                    Stroke::new(2.0, Color32::from_rgb(107, 210, 214))
                } else {
                    Stroke::new(1.0, Color32::from_rgb(72, 163, 166))
                },
            );
            painter.text(
                bounds_rect.left_top() + Vec2::new(8.0, 8.0),
                Align2::LEFT_TOP,
                format!(
                    "CAM VIEW\n{:.0} x {:.0} @ {:.2}x",
                    preview_size[0], preview_size[1], zoom
                ),
                egui::FontId::proportional(12.0),
                Color32::from_rgb(214, 236, 238),
            );
        }

        let mut draw_rects = Vec::with_capacity(visible_nodes.len());
        for node in &visible_nodes {
            let node_rect = node_view_rect(node.position, node.size, rect);

            if node.kind == SceneNodeKind::Sprite {
                match self.sprite_preview_entry(ui.ctx(), &node.sprite.texture_path) {
                    Some(SpritePreviewCacheEntry::Loaded(preview)) => {
                        painter.image(
                            preview.texture.id(),
                            node_rect,
                            egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    Some(SpritePreviewCacheEntry::Failed(_)) => {
                        draw_placeholder_sprite(&painter, node_rect, "Missing");
                    }
                    None => {
                        draw_placeholder_sprite(&painter, node_rect, "Placeholder");
                    }
                }
            } else {
                let fill = node_fill_color(node.kind);
                painter.rect_filled(node_rect, 0.0, fill);
            }

            draw_rect_outline(
                &painter,
                node_rect,
                if selected_node == Some(node.id) {
                    Stroke::new(2.0, Color32::from_rgb(247, 214, 93))
                } else {
                    Stroke::new(1.0, Color32::from_gray(36))
                },
            );
            painter.text(
                node_rect.center(),
                Align2::CENTER_CENTER,
                format!("{}\n{}", node.kind.short_label(), node.name),
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
            draw_rects.push((node.id, node_rect));
        }

        if scene_is_empty {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Empty scene\nDouble-click or right-click here to add the first node.",
                egui::FontId::proportional(18.0),
                Color32::from_gray(190),
            );
        }

        if response.double_clicked() {
            let parent = self.active_scene_tab().selected_node;
            let position = response
                .interact_pointer_pos()
                .map(|pointer| screen_to_scene(pointer, rect));
            self.add_node_with_parent(SceneNodeKind::Empty, parent, position);
        }

        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                self.active_scene_tab_mut().selected_node = hit_test(&draw_rects, pointer);
            }
        }

        if response.secondary_clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let target = hit_test(&draw_rects, pointer);
                let position = screen_to_scene(pointer, rect);

                self.viewport_menu_parent = target;
                self.viewport_menu_position = Some(position);
                self.active_scene_tab_mut().selected_node = target;
            } else {
                self.viewport_menu_parent = None;
                self.viewport_menu_position = None;
            }
        }

        response.context_menu(|ui| {
            self.show_add_node_menu(ui, self.viewport_menu_parent, self.viewport_menu_position);
        });

        if response.drag_started() {
            if let Some(pointer) = response.interact_pointer_pos() {
                if let Some(node_id) = hit_test(&draw_rects, pointer) {
                    let center = if let Some(node) = self.active_scene_tab().scene.node(node_id) {
                        scene_to_screen(node.position, rect)
                    } else {
                        return;
                    };

                    let tab = self.active_scene_tab_mut();
                    tab.selected_node = Some(node_id);
                    tab.viewport_drag = Some(ViewportDrag {
                        node_id,
                        pointer_offset: pointer - center,
                    });
                }
            }
        }

        if let Some(drag) = self.active_scene_tab().viewport_drag {
            if let Some(pointer) = response.interact_pointer_pos() {
                let delta = {
                    let tab = self.active_scene_tab();
                    let Some(node) = tab.scene.node(drag.node_id) else {
                        return;
                    };

                    let centered_pointer = pointer - drag.pointer_offset;
                    let next_position = screen_to_scene(centered_pointer, rect);
                    [
                        next_position[0] - node.position[0],
                        next_position[1] - node.position[1],
                    ]
                };

                if delta != [0.0, 0.0] {
                    let tab = self.active_scene_tab_mut();
                    tab.scene.translate_subtree(drag.node_id, delta);
                    tab.mark_dirty();
                }
            }
        }

        if !ui.ctx().input(|input| input.pointer.primary_down()) {
            self.active_scene_tab_mut().viewport_drag = None;
        }
    }
}

impl eframe::App for RengineEditorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_project_tree_scan();

        let live_interaction = self.active_scene_tab().viewport_drag.is_some();

        TopBottomPanel::top("editor_title_bar").show(ctx, |ui| {
            self.show_title_bar(ui);
        });

        TopBottomPanel::bottom("editor_bottom_panel")
            .resizable(true)
            .default_height(180.0)
            .show(ctx, |ui| {
                self.show_bottom_panel(ui, live_interaction);
            });

        SidePanel::left("editor_files")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                self.show_project_browser(ui);
            });

        SidePanel::left("editor_scene_tree")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                self.show_scene_hierarchy(ui);
            });

        SidePanel::right("editor_inspector")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                self.show_inspector(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
            self.show_scene_tabs(ui);
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(8.0);
            self.show_viewport(ui);
        });

        if live_interaction {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        if self.project_tree_scan_pending() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }
}

fn show_project_tree_entry(
    ui: &mut Ui,
    entry: &ProjectTreeEntry,
    filter: &str,
    depth: usize,
    selected_project_path: Option<&Path>,
    pending_selection: &mut Option<ProjectTreeSelection>,
) {
    if !entry.matches_filter(filter) {
        return;
    }

    if entry.is_dir {
        let header = CollapsingHeader::new(format!("{}/", entry.name))
            .id_salt(entry.path.to_string_lossy())
            .default_open(depth < 2)
            .show(ui, |ui| {
                for child in &entry.children {
                    show_project_tree_entry(
                        ui,
                        child,
                        filter,
                        depth + 1,
                        selected_project_path,
                        pending_selection,
                    );
                }
            });

        if header.header_response.clicked() {
            *pending_selection = Some(ProjectTreeSelection {
                path: entry.path.clone(),
                log: false,
            });
        }
    } else {
        let selected = selected_project_path.is_some_and(|path| path == entry.path.as_path());
        if ui.selectable_label(selected, &entry.name).clicked() {
            *pending_selection = Some(ProjectTreeSelection {
                path: entry.path.clone(),
                log: true,
            });
        }
    }
}

fn apply_editor_theme(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(232, 236, 239));
    visuals.panel_fill = Color32::from_rgb(21, 26, 33);
    visuals.window_fill = Color32::from_rgb(17, 22, 28);
    visuals.faint_bg_color = Color32::from_rgb(28, 34, 42);
    visuals.extreme_bg_color = Color32::from_rgb(12, 16, 21);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 29, 36);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 35, 43);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 56, 68);
    visuals.widgets.active.bg_fill = Color32::from_rgb(58, 76, 90);
    visuals.selection.bg_fill = Color32::from_rgb(66, 116, 132);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    ctx.set_style(style);
}

fn should_skip_entry(path: &Path, is_dir: bool) -> bool {
    if !is_dir {
        return false;
    }

    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "target")
    )
}

fn read_git_branch(workspace_root: &Path) -> String {
    let head_path = workspace_root.join(".git").join("HEAD");
    let Ok(head_contents) = fs::read_to_string(head_path) else {
        return "detached".to_string();
    };

    let head_contents = head_contents.trim();
    head_contents
        .strip_prefix("ref: refs/heads/")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "detached".to_string())
}

fn node_fill_color(kind: SceneNodeKind) -> Color32 {
    match kind {
        SceneNodeKind::Group => Color32::from_rgb(67, 79, 89),
        SceneNodeKind::Empty => Color32::from_rgb(92, 103, 112),
        SceneNodeKind::Camera2d => Color32::from_rgb(53, 125, 132),
        SceneNodeKind::Sprite => Color32::from_rgb(64, 114, 176),
        SceneNodeKind::Trigger => Color32::from_rgb(176, 125, 58),
        SceneNodeKind::UiRoot => Color32::from_rgb(70, 142, 104),
    }
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect) {
    let grid_step = 32.0;
    let center = rect.center();
    let minor = Stroke::new(1.0, Color32::from_rgb(28, 34, 42));
    let major = Stroke::new(1.0, Color32::from_rgb(53, 68, 79));

    let mut x = center.x;
    while x <= rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            minor,
        );
        x += grid_step;
    }

    let mut x = center.x - grid_step;
    while x >= rect.left() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            minor,
        );
        x -= grid_step;
    }

    let mut y = center.y;
    while y <= rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            minor,
        );
        y += grid_step;
    }

    let mut y = center.y - grid_step;
    while y >= rect.top() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            minor,
        );
        y -= grid_step;
    }

    painter.line_segment(
        [
            Pos2::new(center.x, rect.top()),
            Pos2::new(center.x, rect.bottom()),
        ],
        major,
    );
    painter.line_segment(
        [
            Pos2::new(rect.left(), center.y),
            Pos2::new(rect.right(), center.y),
        ],
        major,
    );
}

fn draw_rect_outline(painter: &egui::Painter, rect: egui::Rect, stroke: Stroke) {
    painter.line_segment([rect.left_top(), rect.right_top()], stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
    painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
}

fn draw_checkerboard(
    painter: &egui::Painter,
    rect: egui::Rect,
    cell_size: f32,
    light: Color32,
    dark: Color32,
) {
    let mut y = rect.top();
    let mut row = 0;

    while y < rect.bottom() {
        let mut x = rect.left();
        let mut col = row;

        while x < rect.right() {
            let cell_rect = egui::Rect::from_min_max(
                Pos2::new(x, y),
                Pos2::new(
                    (x + cell_size).min(rect.right()),
                    (y + cell_size).min(rect.bottom()),
                ),
            );
            painter.rect_filled(cell_rect, 0.0, if col % 2 == 0 { light } else { dark });
            x += cell_size;
            col += 1;
        }

        y += cell_size;
        row += 1;
    }
}

fn draw_placeholder_sprite(painter: &egui::Painter, rect: egui::Rect, label: &str) {
    draw_checkerboard(
        painter,
        rect,
        12.0,
        Color32::from_rgb(74, 92, 122),
        Color32::from_rgb(44, 56, 77),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("SPR\n{}", label),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(226, 232, 240),
    );
}

fn show_placeholder_thumbnail(ui: &mut Ui, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    draw_placeholder_sprite(ui.painter(), rect, "Placeholder");
    draw_rect_outline(ui.painter(), rect, Stroke::new(1.0, Color32::from_gray(36)));
}

fn fit_preview_size(size: [usize; 2], max_side: f32) -> Vec2 {
    let width = size[0].max(1) as f32;
    let height = size[1].max(1) as f32;
    let scale = (max_side / width).min(max_side / height).min(1.0);
    Vec2::new(width * scale, height * scale)
}

fn load_sprite_preview_from_disk(ctx: &Context, path: &Path) -> Result<SpritePreview, String> {
    let reader = ImageReader::open(path)
        .map_err(|error| format!("Failed to open image {}: {}", path.display(), error))?;
    let image = reader
        .decode()
        .map_err(|error| format!("Failed to decode image {}: {}", path.display(), error))?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    let texture = ctx.load_texture(
        path.to_string_lossy(),
        color_image,
        egui::TextureOptions::LINEAR,
    );

    Ok(SpritePreview { texture, size })
}

fn is_supported_sprite_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp")
    )
}

fn camera_preview_size(node: &SceneNode, scene_window_size: [f32; 2]) -> [f32; 2] {
    if node.camera2d.use_scene_view_size {
        scene_window_size
    } else {
        node.camera2d.view_size
    }
}

fn node_view_rect(position: [f32; 2], size: [f32; 2], rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(scene_to_screen(position, rect), Vec2::new(size[0], size[1]))
}

fn scene_to_screen(position: [f32; 2], rect: egui::Rect) -> Pos2 {
    Pos2::new(rect.center().x + position[0], rect.center().y + position[1])
}

fn screen_to_scene(position: Pos2, rect: egui::Rect) -> [f32; 2] {
    [position.x - rect.center().x, position.y - rect.center().y]
}

fn hit_test(nodes: &[(u64, egui::Rect)], pointer: Pos2) -> Option<u64> {
    nodes
        .iter()
        .rev()
        .find(|(_, rect)| rect.contains(pointer))
        .map(|(node_id, _)| *node_id)
}
