use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ProjectTreeEntry {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_dir: bool,
    pub(crate) children: Vec<ProjectTreeEntry>,
}

impl ProjectTreeEntry {
    pub(crate) fn scan(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.display().to_string());

        let is_dir = is_project_tree_directory(path);
        let mut children = Vec::new();

        if is_dir {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let child_path = entry.path();
                    if is_symlinked_directory(&child_path) {
                        continue;
                    }

                    let child_is_dir = is_project_tree_directory(&child_path);
                    if should_skip_entry(&child_path, child_is_dir) {
                        continue;
                    }

                    children.push(Self::scan(&child_path));
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

        self.workspace_root.clone()
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
        if node.sprite.texture_path != stored_path {
            node.sprite.texture_path = stored_path;
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

        if node.sprite.texture_path.is_empty() {
            return false;
        }

        node.sprite.texture_path.clear();
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
            let stored_path = node.sprite.texture_path.trim();
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
        let stored_path = node.sprite.texture_path.trim();
        if stored_path.is_empty() {
            return None;
        }

        let resolved_path = self.resolve_stored_path(stored_path);
        engine
            .loaded_texture(&resolved_path)
            .map(|texture| texture.texture())
    }

    pub(crate) fn refresh_project_tree(&mut self) {
        self.project_tree = ProjectTreeEntry::scan(&self.workspace_root);
        self.recent_project_click = None;
        self.push_log("Workspace browser refreshed");
    }

    pub(crate) fn toggle_project_entry(&mut self, path: &Path) {
        if path == self.workspace_root {
            return;
        }

        if !self.collapsed_project_paths.insert(path.to_path_buf()) {
            self.collapsed_project_paths.remove(path);
        }
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
                if is_json_path(&path) {
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

        if scene.normalize_next_id() {
            self.push_log(format!(
                "Normalized next node id for {} to {}",
                self.display_path(&path),
                scene.next_id
            ));
        }

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
    workspace_root: &Path,
    filter: &str,
) -> Vec<ProjectEntryLine<'a>> {
    let mut lines = Vec::new();
    collect_project_tree_lines(root, 0, collapsed_paths, workspace_root, filter, &mut lines);
    lines
}

pub(crate) fn collect_project_tree_lines<'a>(
    entry: &'a ProjectTreeEntry,
    depth: usize,
    collapsed_paths: &HashSet<PathBuf>,
    workspace_root: &Path,
    filter: &str,
    lines: &mut Vec<ProjectEntryLine<'a>>,
) {
    let filter_active = !filter.is_empty();
    if depth > 0 && filter_active && !project_tree_matches_filter(entry, filter) {
        return;
    }

    let is_collapsed =
        !filter_active && entry.path != workspace_root && collapsed_paths.contains(&entry.path);
    lines.push(ProjectEntryLine {
        entry,
        depth,
        is_collapsed,
    });

    if !is_collapsed {
        for child in &entry.children {
            collect_project_tree_lines(
                child,
                depth + 1,
                collapsed_paths,
                workspace_root,
                filter,
                lines,
            );
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
