use super::*;

#[derive(Clone)]
pub(crate) enum PopupMenuKind {
    AddNode {
        parent: Option<u64>,
        position: Option<[f32; 2]>,
    },
    ChangeNodeKind {
        node_id: u64,
    },
    ProjectEntry {
        path: PathBuf,
    },
}

#[derive(Clone)]
pub(crate) struct PopupMenuState {
    pub(crate) anchor: Vec2,
    pub(crate) kind: PopupMenuKind,
}

#[derive(Clone)]
pub(crate) enum PopupMenuAction {
    AddNode {
        kind: SceneNodeKind,
        parent: Option<u64>,
        position: Option<[f32; 2]>,
    },
    ChangeNodeKind {
        node_id: u64,
        kind: SceneNodeKind,
    },
    ProjectOpenScene {
        path: PathBuf,
    },
    ProjectRevealInExplorer {
        path: PathBuf,
    },
    ProjectRefreshBrowser,
}

impl RengineNativeEditor {
    pub(crate) fn open_popup_menu(&mut self, anchor: Vec2, kind: PopupMenuKind) {
        self.popup_menu = Some(PopupMenuState { anchor, kind });
    }

    pub(crate) fn open_add_node_menu(
        &mut self,
        anchor: Vec2,
        parent: Option<u64>,
        position: Option<[f32; 2]>,
    ) {
        self.open_popup_menu(anchor, PopupMenuKind::AddNode { parent, position });
    }

    pub(crate) fn open_kind_menu(&mut self, anchor: Vec2, node_id: u64) {
        self.open_popup_menu(anchor, PopupMenuKind::ChangeNodeKind { node_id });
    }

    pub(crate) fn open_project_entry_menu(&mut self, anchor: Vec2, path: PathBuf) {
        self.open_popup_menu(anchor, PopupMenuKind::ProjectEntry { path });
    }

    pub(crate) fn popup_menu_actions(&self, kind: &PopupMenuKind) -> Vec<PopupMenuAction> {
        match kind {
            PopupMenuKind::AddNode { parent, position } => NODE_KIND_OPTIONS
                .into_iter()
                .map(|kind| PopupMenuAction::AddNode {
                    kind,
                    parent: *parent,
                    position: *position,
                })
                .collect(),
            PopupMenuKind::ChangeNodeKind { node_id } => NODE_KIND_OPTIONS
                .into_iter()
                .map(|kind| PopupMenuAction::ChangeNodeKind {
                    node_id: *node_id,
                    kind,
                })
                .collect(),
            PopupMenuKind::ProjectEntry { path } => {
                let mut actions = Vec::new();
                if path.is_file() && is_scene_path(path) {
                    actions.push(PopupMenuAction::ProjectOpenScene { path: path.clone() });
                }
                actions.push(PopupMenuAction::ProjectRevealInExplorer { path: path.clone() });
                actions.push(PopupMenuAction::ProjectRefreshBrowser);
                actions
            }
        }
    }

    pub(crate) fn popup_action_label(&self, action: &PopupMenuAction) -> String {
        match action {
            PopupMenuAction::AddNode {
                kind,
                parent,
                position,
            } => {
                if parent.is_some() {
                    format!("Add Child {}", kind.label())
                } else if position.is_some() {
                    format!("Add {} Here", kind.label())
                } else {
                    format!("Add {}", kind.label())
                }
            }
            PopupMenuAction::ChangeNodeKind { kind, .. } => kind.label().to_string(),
            PopupMenuAction::ProjectOpenScene { .. } => "Open Scene".to_string(),
            PopupMenuAction::ProjectRevealInExplorer { .. } => "Show in Explorer".to_string(),
            PopupMenuAction::ProjectRefreshBrowser => "Refresh Browser".to_string(),
        }
    }

    pub(crate) fn popup_action_active(&self, action: &PopupMenuAction) -> bool {
        match action {
            PopupMenuAction::ChangeNodeKind { node_id, kind } => self
                .active_scene_tab()
                .scene
                .node(*node_id)
                .is_some_and(|node| node.kind == *kind),
            PopupMenuAction::AddNode { .. }
            | PopupMenuAction::ProjectOpenScene { .. }
            | PopupMenuAction::ProjectRevealInExplorer { .. }
            | PopupMenuAction::ProjectRefreshBrowser => false,
        }
    }

    pub(crate) fn apply_popup_action(&mut self, action: PopupMenuAction) {
        match action {
            PopupMenuAction::AddNode {
                kind,
                parent,
                position,
            } => self.add_node_with_parent(kind, parent, position),
            PopupMenuAction::ChangeNodeKind { node_id, kind } => {
                let mut changed = false;
                {
                    let tab = self.active_scene_tab_mut();
                    if let Some(node) = tab.scene.node_mut(node_id) {
                        if node.kind != kind {
                            node.kind = kind;
                            tab.mark_dirty();
                            changed = true;
                        }
                    }
                }

                if changed {
                    self.push_log(format!("Changed node {} kind to {}", node_id, kind.label()));
                    self.refresh_inspector_form();
                }
            }
            PopupMenuAction::ProjectOpenScene { path } => {
                self.selected_project_path = Some(path);
                self.open_selected_scene();
            }
            PopupMenuAction::ProjectRevealInExplorer { path } => {
                self.reveal_project_path(&path);
            }
            PopupMenuAction::ProjectRefreshBrowser => self.refresh_project_tree(),
        }
    }
}

pub(crate) fn popup_menu_width<'a>(labels: impl IntoIterator<Item = &'a str>) -> f32 {
    labels
        .into_iter()
        .map(|label| label.chars().count() as f32 * 12.0 + 72.0)
        .fold(POPUP_MENU_MIN_WIDTH, f32::max)
}

pub(crate) fn editor_window_rect(engine: &Engine) -> PanelRect {
    let (window_width, window_height) = engine.window_size();
    let window_width = window_width as f32;
    let window_height = window_height as f32;
    PanelRect::new(
        -window_width * 0.5,
        -window_height * 0.5,
        window_width,
        window_height,
    )
}

pub(crate) fn popup_menu_rect(
    menu: &PopupMenuState,
    width: f32,
    window_rect: PanelRect,
) -> PanelRect {
    let item_count = popup_menu_item_count(&menu.kind);
    let height = item_count as f32 * POPUP_MENU_ITEM_HEIGHT + 10.0;
    let pad = 8.0;
    let min_x = window_rect.x + pad;
    let max_x = (window_rect.right() - width - pad).max(min_x);
    let min_y = window_rect.y + pad;
    let max_y = (window_rect.top() - height - pad).max(min_y);
    let x = menu.anchor.x.clamp(min_x, max_x);
    let preferred_y = menu.anchor.y - height;
    let y = if preferred_y < min_y {
        (menu.anchor.y + pad).clamp(min_y, max_y)
    } else {
        preferred_y.clamp(min_y, max_y)
    };
    PanelRect::new(x, y, width, height)
}

pub(crate) fn popup_menu_item_count(kind: &PopupMenuKind) -> usize {
    match kind {
        PopupMenuKind::AddNode { .. } | PopupMenuKind::ChangeNodeKind { .. } => {
            NODE_KIND_OPTIONS.len()
        }
        PopupMenuKind::ProjectEntry { path } => {
            if path.is_file() && is_scene_path(path) {
                3
            } else {
                2
            }
        }
    }
}

pub(crate) fn popup_menu_item_rect(menu_rect: PanelRect, index: usize) -> PanelRect {
    let top = menu_rect.top() - 6.0 - index as f32 * POPUP_MENU_ITEM_HEIGHT;
    PanelRect::new(
        menu_rect.x + 6.0,
        top - POPUP_MENU_ITEM_HEIGHT + 2.0,
        menu_rect.w - 12.0,
        POPUP_MENU_ITEM_HEIGHT - 4.0,
    )
}