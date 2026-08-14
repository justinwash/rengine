use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use serde_json::Value;

const PROJECT_TREE_SCAN_MAX_NODES: usize = 4000;
const PROJECT_TREE_SCAN_MAX_DEPTH: usize = 8;

#[derive(Default)]
struct ProjectTreeScanBudget {
    remaining_nodes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct EditorProjectManifest {
    #[serde(alias = "name")]
    project_name: String,
    #[serde(alias = "root")]
    project_root: String,
    #[serde(default)]
    cached_files: Vec<String>,
    #[serde(default)]
    preferences: Option<Value>,
    /// Palette tokens the project's scenes author as `{name}`, as `"r,g,b"`.
    ///
    /// The editor cannot link the game, so without these every `{chalk}` in a
    /// scene resolves to nothing, `ui_color` falls back to white, and the
    /// preview turns into the wall of pale rectangles this field exists to
    /// fix. The game reads the same block out of the same file.
    #[serde(default)]
    theme: BTreeMap<String, String>,
    /// Typeface files by alias, so a scene's `{font_hud}` resolves to a font
    /// the editor actually loaded rather than to id 0.
    #[serde(default)]
    fonts: BTreeMap<String, String>,
}

pub(crate) struct StartupProjectSelection {
    pub(crate) workspace_root: PathBuf,
    pub(crate) project_name: String,
    pub(crate) project_file: Option<PathBuf>,
    pub(crate) project_issue: Option<String>,
    pub(crate) startup_logs: Vec<String>,
    /// Palette tokens and font aliases from the manifest, ready to be handed
    /// to the preview as ambient bindings. Empty for a workspace opened
    /// without a manifest, which just means the preview degrades to the
    /// literal-`{key}` behaviour it had before.
    pub(crate) theme: BTreeMap<String, String>,
    pub(crate) fonts: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectTreeEntry {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_dir: bool,
    pub(crate) children: Vec<ProjectTreeEntry>,
}

impl ProjectTreeEntry {
    pub(crate) fn scan(path: &Path) -> Self {
        let mut budget = ProjectTreeScanBudget {
            remaining_nodes: PROJECT_TREE_SCAN_MAX_NODES,
        };
        Self::scan_limited(path, 0, &mut budget)
    }

    fn scan_limited(path: &Path, depth: usize, budget: &mut ProjectTreeScanBudget) -> Self {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.display().to_string());

        let is_dir = is_project_tree_directory(path);
        let mut children = Vec::new();

        if is_dir && budget.remaining_nodes > 0 && depth < PROJECT_TREE_SCAN_MAX_DEPTH {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if budget.remaining_nodes == 0 {
                        break;
                    }

                    let child_path = entry.path();
                    if is_symlinked_directory(&child_path) {
                        continue;
                    }

                    let child_is_dir = is_project_tree_directory(&child_path);
                    if should_skip_entry(&child_path, child_is_dir) {
                        continue;
                    }

                    budget.remaining_nodes = budget.remaining_nodes.saturating_sub(1);
                    children.push(Self::scan_limited(&child_path, depth + 1, budget));
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

    pub(crate) fn contains_path(&self, path: &Path) -> bool {
        self.path == path || self.children.iter().any(|child| child.contains_path(path))
    }
}

pub(crate) fn resolve_startup_project_selection() -> StartupProjectSelection {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if let Some(cli_project_path) = cli_project_argument_path() {
        let path = absolute_path_from(&cwd, &cli_project_path);
        if let Ok(selection) = load_project_manifest_selection(&path) {
            return selection;
        }

        let mut fallback = StartupProjectSelection {
            workspace_root: cwd.clone(),
            project_name: "Workspace".to_string(),
            project_file: None,
            project_issue: Some(format!(
                "Failed to open project file from CLI: {}",
                path.display()
            )),
            startup_logs: vec![format!(
                "Failed to open project file from CLI: {}",
                path.display()
            )],
            theme: BTreeMap::new(),
            fonts: BTreeMap::new(),
        };
        fallback
            .startup_logs
            .push("Falling back to current working directory".to_string());
        return fallback;
    }

    if let Some(project_path) = find_project_manifest_in_directory(&cwd) {
        if let Ok(selection) = load_project_manifest_selection(&project_path) {
            return selection;
        }

        return StartupProjectSelection {
            workspace_root: cwd,
            project_name: "Workspace".to_string(),
            project_file: Some(project_path.clone()),
            project_issue: Some(format!(
                "Project manifest exists but could not be loaded: {}",
                project_path.display()
            )),
            startup_logs: vec![format!(
                "Project manifest exists but could not be loaded: {}",
                project_path.display()
            )],
            theme: BTreeMap::new(),
            fonts: BTreeMap::new(),
        };
    }

    if let Some(project_path) = FileDialog::new()
        .set_title("Open rengine project")
        .set_directory(&cwd)
        .add_filter("Rengine project", &["project", "json"])
        .pick_file()
    {
        if let Ok(selection) = load_project_manifest_selection(&project_path) {
            return selection;
        }

        return StartupProjectSelection {
            workspace_root: cwd.clone(),
            project_name: "Workspace".to_string(),
            project_file: None,
            project_issue: None,
            startup_logs: vec![format!(
                "Selected project file was invalid: {}",
                project_path.display()
            )],
            theme: BTreeMap::new(),
            fonts: BTreeMap::new(),
        };
    }

    StartupProjectSelection {
        workspace_root: cwd,
        project_name: "Workspace".to_string(),
        project_file: None,
        project_issue: Some("No project manifest found in the working directory".to_string()),
        startup_logs: vec!["No project file selected; using working directory".to_string()],
        theme: BTreeMap::new(),
        fonts: BTreeMap::new(),
    }
}

pub(crate) fn pick_project_manifest_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    FileDialog::new()
        .set_title("Open rengine project")
        .set_directory(&cwd)
        .add_filter("Rengine project", &["project", "json"])
        .pick_file()
}

pub(crate) fn pick_project_folder_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    FileDialog::new()
        .set_title("Choose project folder")
        .set_directory(&cwd)
        .pick_folder()
}

pub(crate) struct ProjectEntryLine<'a> {
    pub(crate) entry: &'a ProjectTreeEntry,
    pub(crate) depth: usize,
    pub(crate) is_collapsed: bool,
}

impl RengineNativeEditor {
    pub(crate) fn display_path(&self, path: &Path) -> String {
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

    pub(crate) fn default_scene_path(&self) -> PathBuf {
        self.workspace_root
            .join("editor")
            .join("scratch")
            .join("scene-prototype.scene.json")
    }

    pub(crate) fn autosave_directory(&self) -> PathBuf {
        self.workspace_root
            .join("editor")
            .join("scratch")
            .join("autosave")
    }

    pub(crate) fn dialog_directory(&self) -> PathBuf {
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

        self.project_browser_root.clone()
    }

    pub(crate) fn project_browser_root_label(&self) -> String {
        self.display_path(&self.project_browser_root)
    }

    pub(crate) fn project_cached_file_paths(&self) -> Vec<String> {
        let mut files = Vec::new();
        collect_project_tree_file_paths(&self.project_tree, &mut files);
        files
            .into_iter()
            .map(|path| self.stored_workspace_path(&path))
            .collect()
    }

    pub(crate) fn project_manifest_cached_files_json(&self) -> Option<Value> {
        Some(Value::Array(
            self.project_cached_file_paths()
                .into_iter()
                .map(Value::String)
                .collect(),
        ))
    }

    pub(crate) fn suggested_scene_file_name(&self) -> String {
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

    pub(crate) fn autosave_scene_path(&self, index: usize) -> PathBuf {
        let tab = &self.scene_tabs[index];
        let preferred_stem = tab
            .scene_path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|stem| stem.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| tab.display_name());
        let autosave_stem = sanitize_autosave_stem(preferred_stem.trim());

        self.autosave_directory().join(format!(
            "{:02}_{}.autosave.scene.json",
            index, autosave_stem
        ))
    }

    pub(crate) fn normalize_scene_save_path(&self, path: PathBuf) -> PathBuf {
        if path.extension().is_some() {
            path
        } else {
            path.with_extension("json")
        }
    }

    pub(crate) fn stored_workspace_path(&self, path: &Path) -> String {
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

    pub(crate) fn selected_sprite_source_path(&self) -> Option<PathBuf> {
        self.selected_project_path
            .as_ref()
            .filter(|path| path.is_file() && is_supported_sprite_path(path))
            .cloned()
    }

    pub(crate) fn pick_sprite_source_path(&self) -> Option<PathBuf> {
        if let Some(path) = self.selected_sprite_source_path() {
            return Some(path);
        }

        FileDialog::new()
            .set_directory(self.dialog_directory())
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
    }

    pub(crate) fn resolve_stored_path(&self, stored_path: &str) -> PathBuf {
        let path = PathBuf::from(stored_path);
        if path.is_absolute() {
            path
        } else {
            self.workspace_root.join(path)
        }
    }

    pub(crate) fn set_node_sprite_texture_path(&mut self, node_id: u64, path: &Path) -> bool {
        let stored_path = self.stored_workspace_path(path);
        let dimensions = image::image_dimensions(path)
            .ok()
            .map(|(width, height)| [width as f32, height as f32]);
        let sprite_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);
        let history_entry = SceneHistoryEntry::capture(self.active_scene_tab());

        let tab = self.active_scene_tab_mut();
        let Some(node) = tab.scene.node_mut(node_id) else {
            return false;
        };

        let mut changed = false;
        if node.asset_alias != stored_path {
            node.asset_alias = stored_path;
            changed = true;
        }

        if let Some(sprite_name) = sprite_name {
            if (node.name.trim().is_empty() || node.name.starts_with("Sprite "))
                && node.name != sprite_name
            {
                node.name = sprite_name;
                changed = true;
            }
        }

        if let Some(size) = dimensions {
            if node.size != size {
                node.size = size;
                changed = true;
            }
        }

        if changed {
            tab.mark_dirty();
        }

        if changed {
            tab.push_undo_entry(history_entry);
        }

        changed
    }

    pub(crate) fn seed_node_asset_alias_from_path(&mut self, node_id: u64, path: &Path) -> bool {
        let sprite_alias = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("sprite")
            .to_string();
        let history_entry = SceneHistoryEntry::capture(self.active_scene_tab());

        let tab = self.active_scene_tab_mut();
        let Some(node) = tab.scene.node_mut(node_id) else {
            return false;
        };

        if node.asset_alias.trim().is_empty() {
            node.asset_alias = sprite_alias;
            tab.mark_dirty();
            tab.push_undo_entry(history_entry);
            true
        } else {
            false
        }
    }

    pub(crate) fn clear_node_sprite_texture_path(&mut self, node_id: u64) -> bool {
        let history_entry = SceneHistoryEntry::capture(self.active_scene_tab());
        let tab = self.active_scene_tab_mut();
        let Some(node) = tab.scene.node_mut(node_id) else {
            return false;
        };

        if node.asset_alias.is_empty() {
            return false;
        }

        node.asset_alias.clear();
        tab.mark_dirty();
        tab.push_undo_entry(history_entry);
        true
    }

    pub(crate) fn choose_sprite_for_node(&mut self, node_id: u64) -> Option<PathBuf> {
        let path = self.pick_sprite_source_path()?;
        let texture_changed = self.set_node_sprite_texture_path(node_id, &path);
        let alias_changed = self.seed_node_asset_alias_from_path(node_id, &path);

        if texture_changed || alias_changed {
            self.push_log(format!(
                "Updated sprite source to {}",
                self.display_path(&path)
            ));
        }

        Some(path)
    }

    pub(crate) fn request_sprite_previews(&self, engine: &Engine) {
        for node in self
            .active_scene_tab()
            .scene
            .nodes
            .iter()
            .filter(|node| node.kind == SceneNodeKind::Sprite)
        {
            let stored_path = node.asset_alias.trim();
            if stored_path.is_empty() {
                continue;
            }

            let resolved_path = self.resolve_stored_path(stored_path);
            if resolved_path.is_file()
                && is_supported_sprite_path(&resolved_path)
                && engine.loaded_texture(&resolved_path).is_none()
            {
                engine.request_texture(&resolved_path);
            }
        }
    }

    pub(crate) fn sprite_preview_texture(
        &self,
        engine: &Engine,
        node: &SceneNode,
    ) -> Option<TextureId> {
        let stored_path = node.asset_alias.trim();
        if stored_path.is_empty() {
            return None;
        }

        let resolved_path = self.resolve_stored_path(stored_path);
        engine
            .loaded_texture(&resolved_path)
            .map(|texture| texture.texture())
    }

    pub(crate) fn refresh_project_tree(&mut self) {
        self.project_tree = ProjectTreeEntry::scan(&self.project_browser_root);
        self.recent_project_click = None;
        self.reload_script_manifest();
        self.push_log(format!(
            "Workspace browser refreshed at {}",
            self.display_path(&self.project_browser_root)
        ));
    }

    /// (Re)load the project's `scripts.manifest.json` from the browser root, if
    /// present. Drives the inspector's script picker, typed param widgets, and
    /// unknown-script validation. A missing file just clears the manifest (the
    /// inspector falls back to a free-text script path).
    pub(crate) fn reload_script_manifest(&mut self) {
        let path = self.project_browser_root.join("scripts.manifest.json");
        if !path.exists() {
            self.script_manifest = None;
            return;
        }
        match rengine::ScriptManifest::load_from_path(&path) {
            Ok(manifest) => {
                let count = manifest.scripts.len();
                self.script_manifest = Some(manifest);
                self.push_log(format!("Loaded scripts.manifest.json ({count} script(s))"));
            }
            Err(error) => {
                self.script_manifest = None;
                self.push_log(format!("Failed to load scripts.manifest.json: {error}"));
            }
        }
    }

    /// Build a validation registry from the loaded script manifest so the
    /// validator can flag `script_path`s with no matching manifest entry.
    pub(crate) fn script_validation_registry(&self) -> Option<rengine::SceneScriptRegistry2D> {
        self.script_manifest
            .as_ref()
            .map(|m| rengine::SceneScriptRegistry2D::from_known_paths(m.known_paths()))
    }

    pub(crate) fn toggle_project_entry(&mut self, path: &Path) {
        if !self.collapsed_project_paths.insert(path.to_path_buf()) {
            self.collapsed_project_paths.remove(path);
        }
    }

    pub(crate) fn set_project_browser_root(&mut self, path: PathBuf) {
        self.project_browser_root = path;
        self.collapsed_project_paths.clear();
        self.refresh_project_tree();
    }

    pub(crate) fn navigate_project_browser_up(&mut self) {
        let Some(parent) = self.project_browser_root.parent() else {
            return;
        };

        if is_project_browser_unsafe_root(parent) {
            self.push_log(format!(
                "Blocked navigation to {} to keep the project browser responsive",
                self.display_path(parent)
            ));
            return;
        }

        self.set_project_browser_root(parent.to_path_buf());
    }

    pub(crate) fn navigate_project_browser_workspace_root(&mut self) {
        self.set_project_browser_root(self.workspace_root.clone());
    }

    pub(crate) fn open_project_manifest_dialog(&mut self) {
        let Some(path) = pick_project_manifest_path() else {
            return;
        };

        self.open_project_manifest_path(path);
    }

    pub(crate) fn create_project_wizard(&mut self) {
        let Some(folder) = pick_project_folder_path() else {
            return;
        };

        let project_name = folder
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("rengine_project")
            .to_string();
        let manifest_path = folder.join(".project");
        if manifest_path.exists() {
            self.push_log(format!(
                "Project manifest already exists at {}; opening it",
                self.display_path(&manifest_path)
            ));
            self.open_project_manifest_path(manifest_path);
            return;
        }

        let manifest = EditorProjectManifest {
            project_name: project_name.clone(),
            project_root: ".".to_string(),
            cached_files: Vec::new(),
            preferences: None,
            theme: BTreeMap::new(),
            fonts: BTreeMap::new(),
        };

        let serialized = match serde_json::to_string_pretty(&manifest) {
            Ok(text) => text,
            Err(error) => {
                self.push_log(format!(
                    "Failed to serialize new project manifest: {}",
                    error
                ));
                return;
            }
        };

        match fs::write(&manifest_path, serialized) {
            Ok(()) => {
                self.push_log(format!(
                    "Created project {} at {}",
                    project_name,
                    self.display_path(&manifest_path)
                ));
                self.open_project_manifest_path(manifest_path);
            }
            Err(error) => {
                self.push_log(format!(
                    "Failed to create project manifest {}: {}",
                    self.display_path(&manifest_path),
                    error
                ));
            }
        }
    }

    pub(crate) fn open_project_manifest_path(&mut self, path: PathBuf) {
        let Ok(selection) = load_project_manifest_selection(&path) else {
            self.push_log(format!(
                "Failed to open project {}",
                self.display_path(&path)
            ));
            return;
        };

        self.workspace_root = selection.workspace_root;
        self.project_browser_root = self.workspace_root.clone();
        self.project_manifest_path = Some(path.clone());
        self.project_name = selection.project_name.clone();
        self.project_issue = selection.project_issue.clone();
        self.project_tree = ProjectTreeEntry::scan(&self.project_browser_root);
        self.branch_name = read_git_branch(&self.workspace_root);
        self.selected_project_path = None;
        self.collapsed_project_paths.clear();
        self.refresh_inspector_form();
        self.push_log(format!(
            "Loaded project {} from {}",
            selection.project_name,
            self.display_path(&path)
        ));
        for line in selection.startup_logs {
            self.push_log(line);
        }

        self.validate_project_to_log();
    }

    pub(crate) fn open_scene(&mut self) {
        let Some(path) = FileDialog::new()
            .set_directory(self.dialog_directory())
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };

        self.open_scene_path(path);
    }

    pub(crate) fn open_selected_scene(&mut self) {
        let Some(path) = self.selected_project_path.clone() else {
            return;
        };

        if path.is_file() && is_scene_path(&path) {
            self.recent_project_click = None;
            self.open_scene_path(path);
        }
    }

    pub(crate) fn open_scene_path(&mut self, path: PathBuf) {
        self.recent_project_click = None;
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

        let mut scene = match serde_json::from_str::<SceneDocument>(&text) {
            Ok(scene) => scene,
            Err(error) => {
                if is_json_path(&path) && !is_scene_path(&path) {
                    self.selected_project_path = Some(path.clone());
                    self.push_log(format!(
                        "Opened {} as generic JSON",
                        self.display_path(&path)
                    ));
                } else {
                    self.push_log(format!(
                        "Failed to parse {} as an editor scene: {}",
                        self.display_path(&path),
                        error
                    ));
                }
                return;
            }
        };

        scene.normalize_next_id();

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
        self.refresh_inspector_form();
        self.push_log(format!("Opened scene {}", self.display_path(&path)));
    }

    /// Open a node's `script_path` (resolved against the project root) in the
    /// OS's default editor. Scripts are compiled Rust, so this is a jump-to-
    /// source convenience — editing + recompiling happen outside the editor.
    pub(crate) fn open_script_in_os(&mut self, script_path: &str) {
        let path = self.project_browser_root.join(script_path);
        if !path.exists() {
            self.push_log(format!(
                "Script not found on disk: {}",
                self.display_path(&path)
            ));
            return;
        }
        let result = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "start", ""])
                .arg(&path)
                .spawn()
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg(&path).spawn()
        } else {
            Command::new("xdg-open").arg(&path).spawn()
        };
        match result {
            Ok(_) => self.push_log(format!("Opened script {}", self.display_path(&path))),
            Err(error) => self.push_log(format!(
                "Failed to open {}: {}",
                self.display_path(&path),
                error
            )),
        }
    }

    pub(crate) fn reveal_project_path(&mut self, path: &Path) {
        let result = if cfg!(target_os = "windows") {
            if path.is_dir() {
                Command::new("explorer").arg(path).spawn()
            } else {
                Command::new("explorer").arg("/select,").arg(path).spawn()
            }
        } else if cfg!(target_os = "macos") {
            if path.is_dir() {
                Command::new("open").arg(path).spawn()
            } else {
                Command::new("open").arg("-R").arg(path).spawn()
            }
        } else {
            let target = if path.is_dir() {
                path.to_path_buf()
            } else {
                path.parent().unwrap_or(path).to_path_buf()
            };
            Command::new("xdg-open").arg(target).spawn()
        };

        match result {
            Ok(_) => self.push_log(format!("Revealed {}", self.display_path(path))),
            Err(error) => self.push_log(format!(
                "Failed to reveal {}: {}",
                self.display_path(path),
                error
            )),
        }
    }

    pub(crate) fn save_scene(&mut self) {
        let path = self
            .active_scene_tab()
            .scene_path
            .clone()
            .unwrap_or_else(|| self.default_scene_path());
        self.save_scene_to_path(path);
    }

    pub(crate) fn save_scene_as(&mut self) {
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

    pub(crate) fn save_scene_to_path(&mut self, path: PathBuf) {
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

        let scene_json = self.active_scene_tab_mut().cached_scene_json().to_owned();
        let tree_has_path = self.project_tree.contains_path(&path);

        match fs::write(&path, &scene_json) {
            Ok(()) => {
                let tab = self.active_scene_tab_mut();
                tab.scene_path = Some(path.clone());
                tab.mark_saved(scene_json.clone());
                self.selected_project_path = Some(path.clone());
                if !tree_has_path {
                    self.refresh_project_tree();
                }
                self.push_log(format!("Saved scene to {}", self.display_path(&path)));
                self.validate_active_scene_to_log();
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

    /// Validate the active scene with the engine's scene validator and report
    /// any issues (broken/duplicate ids, missing scripts/assets, sourceless
    /// sprites, …) to the activity log — authoring safety on every save.
    pub(crate) fn validate_active_scene_to_log(&mut self) {
        let scene_json = self.active_scene_tab_mut().cached_scene_json().to_owned();
        let value: serde_json::Value = match serde_json::from_str(&scene_json) {
            Ok(value) => value,
            Err(error) => {
                self.push_log(format!("Scene validation skipped: invalid JSON ({error})"));
                return;
            }
        };

        let registry = self.script_validation_registry();
        let report = rengine::validate_editor_scene(&value, None, registry.as_ref());

        // Mirror the issues into structured state for the clickable Validation tab.
        self.validation_issues = report
            .issues()
            .iter()
            .map(|issue| {
                let is_error = issue.severity == rengine::SceneIssueSeverity::Error;
                let tag = if is_error { "error" } else { "warn" };
                let text = match issue.node_id {
                    Some(id) => format!("[{tag}] node {id}: {}", issue.message),
                    None => format!("[{tag}] {}", issue.message),
                };
                ValidationLine {
                    node_id: issue.node_id,
                    is_error,
                    text,
                }
            })
            .collect();

        if report.issues().is_empty() {
            self.push_log("Scene validation: no issues".to_string());
            return;
        }

        self.push_log(format!(
            "Scene validation: {} error(s), {} warning(s)",
            report.error_count(),
            report.warning_count()
        ));
        let lines: Vec<String> = self
            .validation_issues
            .iter()
            .map(|issue| format!("  {}", issue.text))
            .collect();
        for line in lines {
            self.push_log(line);
        }
    }

    /// Run the engine's project-wide scene validator over the project root and
    /// summarise the results in the activity log. Called on project open and
    /// available as an explicit action so authors catch cross-scene issues
    /// (duplicate node ids, broken references) before they reach runtime.
    pub(crate) fn validate_project_to_log(&mut self) {
        let root = self.project_browser_root.clone();
        let registry = self.script_validation_registry();
        let reports = rengine::validate_scene_dir(&root, None, registry.as_ref());

        if reports.is_empty() {
            self.push_log("Project validation: no scene files found".to_string());
            return;
        }

        let scene_count = reports.len();
        let total_errors: usize = reports.iter().map(|(_, r)| r.error_count()).sum();
        let total_warnings: usize = reports.iter().map(|(_, r)| r.warning_count()).sum();

        if total_errors == 0 && total_warnings == 0 {
            self.push_log(format!(
                "Project validation: {scene_count} scene(s), no issues"
            ));
            return;
        }

        self.push_log(format!(
            "Project validation: {scene_count} scene(s), {} error(s), {} warning(s)",
            total_errors, total_warnings
        ));
        for (path, report) in &reports {
            if report.issues().is_empty() {
                continue;
            }
            self.push_log(format!("  {}:", self.display_path(path)));
            for issue in report.issues() {
                let tag = match issue.severity {
                    rengine::SceneIssueSeverity::Error => "error",
                    rengine::SceneIssueSeverity::Warning => "warn",
                };
                match issue.node_id {
                    Some(id) => self.push_log(format!("    [{tag}] node {id}: {}", issue.message)),
                    None => self.push_log(format!("    [{tag}] {}", issue.message)),
                }
            }
        }
    }

    pub(crate) fn update_scene_autosave(&mut self, dt: f32) {
        let mut autosave_logs = Vec::new();

        for index in 0..self.scene_tabs.len() {
            let should_autosave = {
                let tab = &mut self.scene_tabs[index];
                if !tab.scene_dirty || tab.autosaved_revision == tab.edit_revision {
                    false
                } else {
                    tab.autosave_elapsed += dt;
                    tab.autosave_elapsed >= SCENE_AUTOSAVE_INTERVAL_SECONDS
                }
            };

            if !should_autosave {
                continue;
            }

            let autosave_path = self.autosave_scene_path(index);
            let scene_json = self.scene_tabs[index].cached_scene_json().to_owned();

            if let Some(parent) = autosave_path.parent() {
                if let Err(error) = fs::create_dir_all(parent) {
                    autosave_logs.push(format!(
                        "Failed to prepare autosave directory {}: {}",
                        self.display_path(parent),
                        error
                    ));
                    self.scene_tabs[index].autosave_elapsed = 0.0;
                    continue;
                }
            }

            match fs::write(&autosave_path, &scene_json) {
                Ok(()) => {
                    let tab = &mut self.scene_tabs[index];
                    tab.autosaved_revision = tab.edit_revision;
                    tab.autosave_elapsed = 0.0;
                    autosave_logs.push(format!(
                        "Autosaved scene to {}",
                        self.display_path(&autosave_path)
                    ));
                }
                Err(error) => {
                    self.scene_tabs[index].autosave_elapsed = 0.0;
                    autosave_logs.push(format!(
                        "Failed to autosave {}: {}",
                        self.display_path(&autosave_path),
                        error
                    ));
                }
            }
        }

        for log in autosave_logs {
            self.push_log(log);
        }
    }

    pub(crate) fn add_node_with_parent(
        &mut self,
        kind: SceneNodeKind,
        parent: Option<u64>,
        position: Option<[f32; 2]>,
    ) {
        let parent_label = parent
            .and_then(|id| {
                self.active_scene_tab()
                    .scene
                    .node_name(id)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| "scene root".to_string());

        let selected_sprite_path = self.selected_sprite_source_path();
        let history_entry = SceneHistoryEntry::capture(self.active_scene_tab());

        let node_id = {
            let tab = self.active_scene_tab_mut();
            let node_id = tab.scene.add_node(kind, parent);
            if let Some(position) = position {
                if let Some(node) = tab.scene.node_mut(node_id) {
                    node.position = position;
                }
            }
            tab.mark_dirty();
            tab.select_only_node(Some(node_id));
            tab.push_undo_entry(history_entry);
            node_id
        };

        if kind == SceneNodeKind::Sprite {
            if let Some(path) = &selected_sprite_path {
                self.set_node_sprite_texture_path(node_id, path);
                self.seed_node_asset_alias_from_path(node_id, path);
            }
        }

        let mut message = format!("Added {} under {}", kind.label(), parent_label);
        if kind == SceneNodeKind::Sprite {
            if let Some(path) = selected_sprite_path {
                message.push_str(&format!(" from {}", self.display_path(&path)));
            } else {
                message.push_str(" with placeholder preview");
            }
        }
        self.push_log(message);

        if let Some(node) = self.active_scene_tab().scene.node(node_id) {
            if node.kind == SceneNodeKind::Sprite && node.asset_alias.is_empty() {
                self.push_log("Sprite nodes need an asset alias before runtime export");
            }
        }

        self.refresh_inspector_form();
    }
}

pub(crate) fn flattened_project_tree<'a>(
    root: &'a ProjectTreeEntry,
    collapsed_paths: &HashSet<PathBuf>,
    filter: &str,
) -> Vec<ProjectEntryLine<'a>> {
    let mut lines = Vec::new();
    collect_project_tree_lines(root, 0, collapsed_paths, filter, &mut lines);
    lines
}

pub(crate) fn collect_project_tree_lines<'a>(
    entry: &'a ProjectTreeEntry,
    depth: usize,
    collapsed_paths: &HashSet<PathBuf>,
    filter: &str,
    lines: &mut Vec<ProjectEntryLine<'a>>,
) {
    let filter_active = !filter.is_empty();
    if depth > 0 && filter_active && !project_tree_matches_filter(entry, filter) {
        return;
    }

    let is_collapsed = !filter_active && collapsed_paths.contains(&entry.path);
    lines.push(ProjectEntryLine {
        entry,
        depth,
        is_collapsed,
    });

    if !is_collapsed {
        for child in &entry.children {
            collect_project_tree_lines(child, depth + 1, collapsed_paths, filter, lines);
        }
    }
}

pub(crate) fn project_tree_matches_filter(entry: &ProjectTreeEntry, filter: &str) -> bool {
    let entry_text = entry.path.to_string_lossy().to_ascii_lowercase();
    entry_text.contains(filter)
        || entry
            .children
            .iter()
            .any(|child| project_tree_matches_filter(child, filter))
}

pub(crate) fn is_project_tree_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| !metadata.file_type().is_symlink() && metadata.is_dir())
        .unwrap_or_else(|_| path.is_dir())
}

fn collect_project_tree_file_paths(entry: &ProjectTreeEntry, files: &mut Vec<PathBuf>) {
    if entry.is_dir {
        for child in &entry.children {
            collect_project_tree_file_paths(child, files);
        }
    } else {
        files.push(entry.path.clone());
    }
}

pub(crate) fn is_symlinked_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink() && path.is_dir())
        .unwrap_or(false)
}

pub(crate) fn should_skip_entry(path: &Path, is_dir: bool) -> bool {
    if !is_dir {
        return false;
    }

    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "target")
    )
}

fn is_project_browser_unsafe_root(path: &Path) -> bool {
    if path.parent().is_none() {
        return true;
    }

    false
}

pub(crate) fn read_git_branch(workspace_root: &Path) -> String {
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

pub(crate) fn is_scene_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".scene.json"))
}

pub(crate) fn is_json_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("json")
    )
}

pub(crate) fn is_supported_sprite_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp")
    )
}

fn sanitize_autosave_stem(stem: &str) -> String {
    let sanitized: String = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    let sanitized = sanitized.trim_matches('_');
    if sanitized.is_empty() {
        "untitled_scene".to_string()
    } else {
        sanitized.to_string()
    }
}

fn cli_project_argument_path() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--project=") {
            if !value.trim().is_empty() {
                return Some(PathBuf::from(value));
            }
            continue;
        }

        if arg == "--project" {
            if let Some(value) = args.next() {
                return Some(PathBuf::from(value));
            }
            continue;
        }

        let path = PathBuf::from(&arg);
        if is_project_manifest_path(&path) {
            return Some(path);
        }
    }

    None
}

pub(crate) fn is_project_manifest_path(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".project" || name.eq_ignore_ascii_case("rengine.project"))
    {
        return true;
    }

    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("project")
    )
}

fn find_project_manifest_in_directory(directory: &Path) -> Option<PathBuf> {
    let preferred = [
        directory.join(".project"),
        directory.join("rengine.project"),
        directory.join(".project.json"),
        directory.join("rengine.project.json"),
    ];
    for path in preferred {
        if path.is_file() {
            return Some(path);
        }
    }

    let entries = fs::read_dir(directory).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && is_project_manifest_path(&path) {
            return Some(path);
        }
    }

    None
}

fn load_project_manifest_selection(path: &Path) -> Result<StartupProjectSelection, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {}", path.display(), error))?;
    let manifest: EditorProjectManifest = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {}", path.display(), error))?;

    let root_value = manifest.project_root.trim();
    if root_value.is_empty() {
        return Err(format!("project_root was empty in {}", path.display()));
    }

    let root_path = PathBuf::from(root_value);
    let parent = path.parent().unwrap_or(Path::new("."));
    let resolved_root = absolute_path_from(parent, &root_path);
    if !resolved_root.is_dir() {
        return Err(format!(
            "project_root {} from {} is not a directory",
            resolved_root.display(),
            path.display()
        ));
    }

    let mut startup_logs = vec![format!(
        "Opened project {} ({})",
        manifest.project_name,
        path.display()
    )];
    if !manifest.cached_files.is_empty() {
        startup_logs.push(format!(
            "Project cache has {} file hints",
            manifest.cached_files.len()
        ));
    }
    if manifest.preferences.is_some() {
        startup_logs.push("Loaded project preferences".to_string());
    }
    if !manifest.theme.is_empty() {
        startup_logs.push(format!("Loaded {} palette tokens", manifest.theme.len()));
    }

    Ok(StartupProjectSelection {
        workspace_root: resolved_root,
        project_name: manifest.project_name,
        project_file: Some(path.to_path_buf()),
        project_issue: None,
        startup_logs,
        theme: manifest.theme,
        fonts: manifest.fonts,
    })
}

fn absolute_path_from(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    base.join(path)
}

/// The ambient bindings the `ui_preview` resolves against, from the project
/// manifest's `theme` and `fonts` blocks.
///
/// This is the whole of rule 3's editor half: the palette a scene authors as
/// `{chalk}` lives in one file, and the game and the editor both read it, so
/// the preview shows the game's real colours instead of white boxes.
///
/// Every step degrades rather than fails. A project with no manifest, no theme
/// block, or an unloadable font still opens — it just previews the way it did
/// before this existed, and says so in the activity log.
pub(crate) fn build_preview_bindings(
    engine: &mut Engine,
    workspace_root: &Path,
    theme: BTreeMap<String, String>,
    fonts: BTreeMap<String, String>,
    startup_logs: &mut Vec<String>,
) -> (Bindings, BTreeMap<String, FontId>) {
    let mut bindings: Bindings = theme.into_iter().collect();
    let mut loaded = BTreeMap::new();

    // `{font_hud}` resolves to a numeric id, so the face has to actually be
    // loaded here — an alias pointing at a font the editor never loaded would
    // silently draw everything in face 0 at the wrong metrics, which looks
    // like a layout bug rather than a missing asset.
    for (alias, relative) in fonts {
        let path = workspace_root.join(&relative);
        match engine.load_font(&path) {
            Ok(id) => {
                bindings.insert(format!("font_{alias}"), id.index().to_string());
                loaded.insert(format!("font_{alias}"), id);
            }
            Err(err) => startup_logs.push(format!(
                "Preview font `{alias}` ({}) failed to load: {err}",
                path.display()
            )),
        }
    }

    // Neighbouring projects' faces too: a scene opened from another project
    // adopts its palette lazily, but its fonts can only be loaded here.
    for manifest in neighbouring_project_manifests(workspace_root) {
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<EditorProjectManifest>(&text) else {
            continue;
        };
        let root = manifest.parent().unwrap_or(Path::new("."));
        for (alias, relative) in parsed.fonts {
            let key = format!("font_{alias}");
            if bindings.contains_key(&key) {
                continue;
            }
            if let Ok(id) = engine.load_font(root.join(&relative)) {
                bindings.insert(key.clone(), id.index().to_string());
                loaded.insert(key, id);
            }
        }
    }

    (bindings, loaded)
}

/// Bind the `{lh_*}` / `{row_h}` line-height tokens the scenes author as their
/// `ui_h`, by measuring the node's own font and size.
///
/// The game computes these the same way (see the game's `title_bindings`: a
/// text node is one line box tall, and a line box is a font metric that cannot
/// be authored as a literal because neither face is 1:1 with its size). The
/// editor has the fonts loaded now, so it can do the identical measurement
/// instead of leaving the token unresolved — which reads as `ui_h: 0` and
/// collapses the node to nothing.
///
/// Nodes that don't author their own `ui_text_size` are skipped: their height
/// comes from somewhere the editor genuinely cannot know, and a guessed value
/// would be a wrong preview rather than an honest empty one.
pub(crate) fn bind_authored_line_heights(
    world: &SceneWorld2D,
    canvas: &Canvas,
    fonts: &BTreeMap<String, FontId>,
    bindings: &mut Bindings,
) {
    for handle in world.handles() {
        let Some(node) = world.get(handle) else {
            continue;
        };
        // Only a bare `"{token}"` — anything with surrounding text or arithmetic
        // is not a plain line height and guessing at it would be wrong.
        let Some(key) = node
            .property("ui_h")
            .map(str::trim)
            .and_then(|h| h.strip_prefix('{')?.strip_suffix('}'))
            .filter(|key| !key.is_empty())
        else {
            continue;
        };
        if bindings.contains_key(key) {
            continue;
        }
        let Some(size) = node.property("ui_text_size").and_then(|s| s.trim().parse().ok()) else {
            continue;
        };
        // The node's own `ui_font` is a `{font_hud}` token, and the ids the
        // editor loaded are kept as real `FontId`s rather than reconstructed
        // from the numeric binding — `FontId` construction is crate-private on
        // purpose, since an id only means anything if the renderer loaded that
        // font.
        let font = node
            .property("ui_font")
            .map(str::trim)
            .and_then(|f| f.strip_prefix('{')?.strip_suffix('}'))
            .and_then(|alias| fonts.get(alias))
            .copied()
            .unwrap_or(FontId::DEFAULT);
        let height = canvas.line_height_in(font, size);
        bindings.insert(key.to_string(), height.to_string());
    }
}

#[cfg(test)]
mod preview_binding_tests {
    use super::*;

    /// A node whose `ui_h` is a bare `{token}` gets that token bound to its own
    /// measured line height. Without this the token stays literal, `ui_h`
    /// resolves to 0, and the node collapses — 50 authored nodes across the
    /// game's scenes are shaped exactly this way.
    #[test]
    fn a_bare_height_token_is_bound_from_the_nodes_own_text_size() {
        let mut world = SceneWorld2D::new();
        let handle = world.spawn(SceneNode2D::new("label"));
        let node = world.get_mut(handle).unwrap();
        node.set_property("ui", "text");
        node.set_property("ui_h", "{lh_name}");
        node.set_property("ui_text_size", "12");

        let canvas = Canvas::for_test((200, 200));
        let mut bindings = Bindings::new();
        bind_authored_line_heights(&world, &canvas, &BTreeMap::new(), &mut bindings);

        let bound: f32 = bindings
            .get("lh_name")
            .expect("`{lh_name}` should have been bound")
            .parse()
            .expect("a line height is a number");
        assert!(
            (bound - canvas.line_height_in(FontId::DEFAULT, 12.0)).abs() < 1e-3,
            "bound height must be the canvas's own measurement, not a guess"
        );
    }

    /// The honest-empty case: no `ui_text_size` means the height comes from
    /// somewhere the editor cannot see, so it must leave the token alone rather
    /// than invent a number and preview a layout the game will never draw.
    #[test]
    fn a_height_token_without_a_text_size_is_left_unbound() {
        let mut world = SceneWorld2D::new();
        let handle = world.spawn(SceneNode2D::new("panel"));
        let node = world.get_mut(handle).unwrap();
        node.set_property("ui", "rect");
        node.set_property("ui_h", "{menu_h}");

        let canvas = Canvas::for_test((200, 200));
        let mut bindings = Bindings::new();
        bind_authored_line_heights(&world, &canvas, &BTreeMap::new(), &mut bindings);

        assert!(!bindings.contains_key("menu_h"));
    }

    /// An existing binding wins. The manifest (and, later, any real per-screen
    /// value) is authoritative — this pass only fills gaps, so it must never
    /// overwrite a height someone actually supplied.
    #[test]
    fn an_already_bound_height_is_not_overwritten() {
        let mut world = SceneWorld2D::new();
        let handle = world.spawn(SceneNode2D::new("label"));
        let node = world.get_mut(handle).unwrap();
        node.set_property("ui", "text");
        node.set_property("ui_h", "{row_h}");
        node.set_property("ui_text_size", "12");

        let canvas = Canvas::for_test((200, 200));
        let mut bindings: Bindings = [("row_h".to_string(), "40".to_string())]
            .into_iter()
            .collect();
        bind_authored_line_heights(&world, &canvas, &BTreeMap::new(), &mut bindings);

        assert_eq!(bindings.get("row_h").map(String::as_str), Some("40"));
    }
}

/// Every project manifest the editor might preview a scene from: the one it
/// was launched with, plus any in a sibling directory of the workspace root.
///
/// Fonts can only be loaded in `new` (`load_font` needs `&mut Engine`, and the
/// update/render path only ever gets `&Engine`), so they cannot be resolved
/// lazily when a scene from another project opens. Loading the neighbours'
/// fonts up front is what lets `{font_hud}` resolve at all when the editor is
/// launched from the engine checkout and opens a game's scene — the exact
/// case that produced a screen of white boxes.
///
/// Sibling-only, not a full recursive scan: projects live next to each other
/// in a workspace, and walking an arbitrary tree at startup to find fonts is a
/// cost with no bound.
fn neighbouring_project_manifests(workspace_root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Some(parent) = workspace_root.parent() else {
        return found;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || path == workspace_root {
            continue;
        }
        if let Some(manifest) = find_project_manifest_in_directory(&path) {
            found.push(manifest);
        }
    }
    found.sort();
    found
}

/// The nearest project manifest at or above a scene's own directory.
///
/// A scene is opened by *path*, and that path is often outside the directory
/// the editor was launched from — the whole point of the file browser. Binding
/// the theme to the editor's cwd meant opening Formula R's race scene from the
/// rengine checkout resolved every `{chalk}` against rengine's own themeless
/// `.project`, and the preview came out as white boxes with the fix already in
/// place. The manifest has to follow the scene, not the shell.
fn project_manifest_for_scene(scene_path: &Path) -> Option<PathBuf> {
    let mut dir = scene_path.parent()?;
    loop {
        if let Some(found) = find_project_manifest_in_directory(dir) {
            return Some(found);
        }
        dir = dir.parent()?;
    }
}

impl RengineNativeEditor {
    /// Re-resolve the preview *palette* for a newly-opened scene.
    ///
    /// Only the theme: fonts need `&mut Engine`, which only exists in `new`,
    /// and font ids are stable for the process anyway — the startup load
    /// already covers them. A theme is just strings, so it can be swapped at
    /// any point.
    ///
    /// Cheap and idempotent: it no-ops unless the scene belongs to a different
    /// project than the one already loaded, so switching tabs within a project
    /// costs one path comparison.
    pub(crate) fn adopt_project_for_scene(&mut self, scene_path: &Path) {
        let Some(manifest) = project_manifest_for_scene(scene_path) else {
            return;
        };
        if self.preview_manifest_path.as_deref() == Some(manifest.as_path()) {
            return;
        }

        let Ok(text) = fs::read_to_string(&manifest) else {
            return;
        };
        let Ok(parsed) = serde_json::from_str::<EditorProjectManifest>(&text) else {
            return;
        };
        if parsed.theme.is_empty() {
            // Nothing to offer. Keep whatever is loaded rather than blanking a
            // good palette because a scene sits under a bare manifest.
            return;
        }

        let tokens = parsed.theme.len();
        // Theme first, then the font ids back on top: those were resolved at
        // startup against real loaded atlases and must not be shadowed by a
        // manifest's `fonts` block, which is only a path.
        self.preview_bindings = parsed.theme.into_iter().collect();
        for (alias, id) in &self.preview_fonts {
            self.preview_bindings
                .insert(alias.clone(), id.index().to_string());
        }
        self.preview_manifest_path = Some(manifest.clone());
        self.push_log(format!(
            "Preview theme: {tokens} tokens from {}",
            manifest.display()
        ));
    }
}

#[cfg(test)]
mod project_scope_tests {
    use super::*;

    /// The bug this exists for, in one assertion.
    ///
    /// The editor is launched from one project and opens a scene belonging to
    /// another — routine, since scenes are opened by path through the file
    /// browser. The palette must follow the *scene*. Binding it to the
    /// editor's working directory meant Formula R's race scene resolved every
    /// `{chalk}` against rengine's own themeless manifest, and previewed as a
    /// screen of white boxes with the binding fix already in place.
    #[test]
    fn a_scene_resolves_the_manifest_nearest_to_itself() {
        let temp = std::env::temp_dir().join("rengine_project_scope_test");
        let engine_project = temp.join("engine");
        let game_scenes = temp.join("game").join("scenes");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&engine_project).expect("temp dirs");
        fs::create_dir_all(&game_scenes).expect("temp dirs");

        fs::write(
            engine_project.join(".project"),
            r#"{"name":"engine","root":"."}"#,
        )
        .expect("write engine manifest");
        let game_manifest = temp.join("game").join(".project");
        fs::write(
            &game_manifest,
            r#"{"name":"game","root":".","theme":{"chalk":"232,228,217"}}"#,
        )
        .expect("write game manifest");

        let scene = game_scenes.join("race.scene.json");
        fs::write(&scene, "{}").expect("write scene");

        let found = project_manifest_for_scene(&scene).expect("a manifest above the scene");
        assert_eq!(
            found, game_manifest,
            "a scene must resolve its OWN project's manifest, not whichever \
             directory the editor happened to be launched from"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    /// A scene with no manifest anywhere above it must not adopt an unrelated
    /// one — better an unthemed preview than a confidently wrong palette.
    #[test]
    fn a_scene_outside_any_project_resolves_nothing() {
        let temp = std::env::temp_dir().join("rengine_project_scope_orphan");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).expect("temp dir");
        let scene = temp.join("loose.scene.json");
        fs::write(&scene, "{}").expect("write scene");

        // The temp dir itself has no manifest; anything found above it would
        // be an accident of the machine, so this only asserts the local miss.
        assert!(find_project_manifest_in_directory(&temp).is_none());

        let _ = fs::remove_dir_all(&temp);
    }
}
