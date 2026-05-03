use super::*;

const VIEWPORT_BOX_SELECTION_CLICK_THRESHOLD: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DockPanelKind {
    Files,
    Hierarchy,
    Inspector,
    Bottom,
}

#[derive(Clone, Copy)]
pub(crate) struct PanelRect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl PanelRect {
    pub(crate) fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub(crate) fn contains(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.w
            && point.y >= self.y
            && point.y <= self.y + self.h
    }

    pub(crate) fn top(self) -> f32 {
        self.y + self.h
    }

    pub(crate) fn right(self) -> f32 {
        self.x + self.w
    }

    pub(crate) fn inset(self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            w: (self.w - amount * 2.0).max(0.0),
            h: (self.h - amount * 2.0).max(0.0),
        }
    }

    pub(crate) fn center(self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DockPanelState {
    pub(crate) open: bool,
    pub(crate) size: f32,
}

impl DockPanelState {
    pub(crate) const fn new(size: f32) -> Self {
        Self { open: true, size }
    }

    pub(crate) fn displayed(self, collapsed_size: f32) -> f32 {
        if self.open {
            self.size
        } else {
            collapsed_size
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PanelLayoutState {
    pub(crate) files: DockPanelState,
    pub(crate) hierarchy: DockPanelState,
    pub(crate) inspector: DockPanelState,
    pub(crate) bottom: DockPanelState,
}

impl Default for PanelLayoutState {
    fn default() -> Self {
        Self {
            files: DockPanelState::new(FILES_PANEL_WIDTH),
            hierarchy: DockPanelState::new(HIERARCHY_PANEL_WIDTH),
            inspector: DockPanelState::new(INSPECTOR_PANEL_WIDTH),
            bottom: DockPanelState::new(BOTTOM_PANEL_HEIGHT),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PanelResizeDrag {
    pub(crate) panel: DockPanelKind,
    pub(crate) pointer_origin: Vec2,
    pub(crate) size_origin: f32,
}

pub(crate) struct ShellLayout {
    pub(crate) top_bar: PanelRect,
    pub(crate) files: PanelRect,
    pub(crate) files_open: bool,
    pub(crate) hierarchy: PanelRect,
    pub(crate) hierarchy_open: bool,
    pub(crate) inspector: PanelRect,
    pub(crate) inspector_open: bool,
    pub(crate) center: PanelRect,
    pub(crate) scene_tabs: PanelRect,
    pub(crate) viewport: PanelRect,
    pub(crate) bottom: PanelRect,
    pub(crate) bottom_open: bool,
    pub(crate) bottom_tabs: PanelRect,
    pub(crate) bottom_content: PanelRect,
    pub(crate) files_resize: Option<PanelRect>,
    pub(crate) hierarchy_resize: Option<PanelRect>,
    pub(crate) inspector_resize: Option<PanelRect>,
    pub(crate) bottom_resize: Option<PanelRect>,
}

impl ShellLayout {
    pub(crate) fn new(engine: &Engine, panels: &PanelLayoutState) -> Self {
        let (window_width, window_height) = engine.window_size();
        let hw = window_width as f32 * 0.5;
        let hh = window_height as f32 * 0.5;

        let top_bar = PanelRect::new(-hw, hh - TOP_BAR_HEIGHT, hw * 2.0, TOP_BAR_HEIGHT);
        let bottom = PanelRect::new(
            -hw,
            -hh,
            hw * 2.0,
            panels.bottom.displayed(PANEL_COLLAPSED_HEIGHT),
        );

        let content_bottom = bottom.top() + PANEL_GAP;
        let content_top = top_bar.y - PANEL_GAP;
        let content_height = (content_top - content_bottom).max(0.0);

        let mut left_cursor = -hw + PANEL_GAP;
        let mut left_collapsed_strip_x = None;
        let mut left_collapsed_count = 0usize;

        let files = if panels.files.open {
            let rect = PanelRect::new(
                left_cursor,
                content_bottom,
                panels.files.size,
                content_height,
            );
            left_cursor = rect.right() + PANEL_GAP;
            rect
        } else {
            let strip_x = *left_collapsed_strip_x.get_or_insert_with(|| {
                let x = left_cursor;
                left_cursor += PANEL_COLLAPSED_WIDTH + PANEL_GAP;
                x
            });
            let rect = collapsed_side_panel_rect(strip_x, content_top, left_collapsed_count);
            left_collapsed_count += 1;
            rect
        };

        let hierarchy = if panels.hierarchy.open {
            let rect = PanelRect::new(
                left_cursor,
                content_bottom,
                panels.hierarchy.size,
                content_height,
            );
            left_cursor = rect.right() + PANEL_GAP;
            rect
        } else {
            let strip_x = *left_collapsed_strip_x.get_or_insert_with(|| {
                let x = left_cursor;
                left_cursor += PANEL_COLLAPSED_WIDTH + PANEL_GAP;
                x
            });
            collapsed_side_panel_rect(strip_x, content_top, left_collapsed_count)
        };

        let mut right_cursor = hw - PANEL_GAP;
        let inspector = if panels.inspector.open {
            let rect = PanelRect::new(
                right_cursor - panels.inspector.size,
                content_bottom,
                panels.inspector.size,
                content_height,
            );
            right_cursor = rect.x - PANEL_GAP;
            rect
        } else {
            right_cursor -= PANEL_COLLAPSED_WIDTH;
            let rect = collapsed_side_panel_rect(right_cursor, content_top, 0);
            right_cursor -= PANEL_GAP;
            rect
        };

        let center = PanelRect::new(
            left_cursor,
            content_bottom,
            (right_cursor - left_cursor).max(0.0),
            content_height,
        );

        let scene_tabs = PanelRect::new(
            center.x + PANEL_PADDING,
            center.top() - PANEL_PADDING - TAB_HEIGHT,
            (center.w - PANEL_PADDING * 2.0).max(0.0),
            TAB_HEIGHT,
        );
        let viewport = PanelRect::new(
            center.x + PANEL_PADDING,
            center.y + PANEL_PADDING,
            (center.w - PANEL_PADDING * 2.0).max(0.0),
            (scene_tabs.y - center.y - PANEL_PADDING * 2.0).max(0.0),
        );

        let bottom_tabs_height = (bottom.h - PANEL_PADDING * 2.0).clamp(0.0, TAB_HEIGHT);

        let bottom_tabs = PanelRect::new(
            bottom.x + PANEL_PADDING,
            bottom.top() - PANEL_PADDING - bottom_tabs_height,
            (bottom.w - PANEL_PADDING * 3.0 - PANEL_TOGGLE_BUTTON_SIZE - BUTTON_GAP).max(0.0),
            bottom_tabs_height,
        );
        let bottom_content = PanelRect::new(
            bottom.x + PANEL_PADDING,
            bottom.y + PANEL_PADDING,
            (bottom.w - PANEL_PADDING * 2.0).max(0.0),
            (bottom_tabs.y - bottom.y - PANEL_PADDING * 2.0).max(0.0),
        );

        let files_resize = panels.files.open.then(|| {
            PanelRect::new(
                files.right() - PANEL_RESIZE_HANDLE_SIZE * 0.5,
                content_bottom,
                PANEL_RESIZE_HANDLE_SIZE,
                content_height,
            )
        });
        let hierarchy_resize = panels.hierarchy.open.then(|| {
            PanelRect::new(
                hierarchy.right() - PANEL_RESIZE_HANDLE_SIZE * 0.5,
                content_bottom,
                PANEL_RESIZE_HANDLE_SIZE,
                content_height,
            )
        });
        let inspector_resize = panels.inspector.open.then(|| {
            PanelRect::new(
                inspector.x - PANEL_RESIZE_HANDLE_SIZE * 0.5,
                content_bottom,
                PANEL_RESIZE_HANDLE_SIZE,
                content_height,
            )
        });
        let bottom_resize = panels.bottom.open.then(|| {
            PanelRect::new(
                -hw,
                bottom.top() - PANEL_RESIZE_HANDLE_SIZE * 0.5,
                hw * 2.0,
                PANEL_RESIZE_HANDLE_SIZE,
            )
        });

        Self {
            top_bar,
            files,
            files_open: panels.files.open,
            hierarchy,
            hierarchy_open: panels.hierarchy.open,
            inspector,
            inspector_open: panels.inspector.open,
            center,
            scene_tabs,
            viewport,
            bottom,
            bottom_open: panels.bottom.open,
            bottom_tabs,
            bottom_content,
            files_resize,
            hierarchy_resize,
            inspector_resize,
            bottom_resize,
        }
    }
}

pub(crate) struct SceneNodeLine {
    pub(crate) node_id: u64,
    pub(crate) depth: usize,
    pub(crate) label: String,
    pub(crate) has_children: bool,
    pub(crate) is_collapsed: bool,
}

impl RengineNativeEditor {
    pub(crate) fn panel_state(&self, kind: DockPanelKind) -> DockPanelState {
        match kind {
            DockPanelKind::Files => self.panel_layout.files,
            DockPanelKind::Hierarchy => self.panel_layout.hierarchy,
            DockPanelKind::Inspector => self.panel_layout.inspector,
            DockPanelKind::Bottom => self.panel_layout.bottom,
        }
    }

    pub(crate) fn panel_state_mut(&mut self, kind: DockPanelKind) -> &mut DockPanelState {
        match kind {
            DockPanelKind::Files => &mut self.panel_layout.files,
            DockPanelKind::Hierarchy => &mut self.panel_layout.hierarchy,
            DockPanelKind::Inspector => &mut self.panel_layout.inspector,
            DockPanelKind::Bottom => &mut self.panel_layout.bottom,
        }
    }

    pub(crate) fn toggle_panel(&mut self, kind: DockPanelKind) {
        let is_open = {
            let panel = self.panel_state_mut(kind);
            panel.open = !panel.open;
            panel.open
        };

        if !is_open {
            match kind {
                DockPanelKind::Files => {
                    self.file_browser_ui_focused = false;
                    if self.active_text_input_owner == Some(TextInputOwner::FileBrowser) {
                        self.active_text_input_owner = None;
                    }
                }
                DockPanelKind::Inspector => {
                    self.inspector_ui_focused = false;
                    if self.active_text_input_owner == Some(TextInputOwner::Inspector) {
                        self.active_text_input_owner = None;
                    }
                }
                DockPanelKind::Bottom | DockPanelKind::Hierarchy => {}
            }
        }

        if self
            .panel_resize_drag
            .is_some_and(|drag| drag.panel == kind)
        {
            self.panel_resize_drag = None;
        }
    }

    pub(crate) fn begin_panel_resize(&mut self, panel: DockPanelKind, pointer: Vec2) {
        self.panel_resize_drag = Some(PanelResizeDrag {
            panel,
            pointer_origin: pointer,
            size_origin: self.panel_state(panel).size,
        });
        self.clear_text_input_owner();
    }

    pub(crate) fn clamp_panel_layout(&mut self, engine: &Engine) {
        let (window_width, window_height) = engine.window_size();
        let window_width = window_width as f32;
        let window_height = window_height as f32;

        self.panel_layout.files.size = self.panel_layout.files.size.clamp(
            MIN_FILES_PANEL_WIDTH,
            window_width.max(MIN_FILES_PANEL_WIDTH),
        );
        self.panel_layout.hierarchy.size = self.panel_layout.hierarchy.size.clamp(
            MIN_HIERARCHY_PANEL_WIDTH,
            window_width.max(MIN_HIERARCHY_PANEL_WIDTH),
        );
        self.panel_layout.inspector.size = self.panel_layout.inspector.size.clamp(
            MIN_INSPECTOR_PANEL_WIDTH,
            window_width.max(MIN_INSPECTOR_PANEL_WIDTH),
        );

        let max_bottom_height =
            (window_height - TOP_BAR_HEIGHT - PANEL_GAP * 2.0 - MIN_CENTER_PANEL_HEIGHT)
                .max(MIN_BOTTOM_PANEL_HEIGHT);
        self.panel_layout.bottom.size = self
            .panel_layout
            .bottom
            .size
            .clamp(MIN_BOTTOM_PANEL_HEIGHT, max_bottom_height);

        let left_collapsed_visible =
            !self.panel_layout.files.open || !self.panel_layout.hierarchy.open;
        let left_visible_total = if self.panel_layout.files.open {
            self.panel_layout.files.size
        } else {
            0.0
        } + if self.panel_layout.hierarchy.open {
            self.panel_layout.hierarchy.size
        } else {
            0.0
        } + if left_collapsed_visible {
            PANEL_COLLAPSED_WIDTH
        } else {
            0.0
        };
        let right_visible_total = if self.panel_layout.inspector.open {
            self.panel_layout.inspector.size
        } else {
            PANEL_COLLAPSED_WIDTH
        };
        let visible_side_total = left_visible_total + right_visible_total;
        let visible_side_elements = if self.panel_layout.files.open { 1 } else { 0 }
            + if self.panel_layout.hierarchy.open {
                1
            } else {
                0
            }
            + if left_collapsed_visible { 1 } else { 0 }
            + 1;
        let gap_count = visible_side_elements + 2;
        let max_side_total = (window_width - MIN_CENTER_PANEL_WIDTH - PANEL_GAP * gap_count as f32)
            .max(PANEL_COLLAPSED_WIDTH * 2.0);
        let mut overflow = (visible_side_total - max_side_total).max(0.0);

        for (panel, min_size) in [
            (DockPanelKind::Inspector, MIN_INSPECTOR_PANEL_WIDTH),
            (DockPanelKind::Hierarchy, MIN_HIERARCHY_PANEL_WIDTH),
            (DockPanelKind::Files, MIN_FILES_PANEL_WIDTH),
        ] {
            if overflow <= 0.0 {
                break;
            }

            let state = self.panel_state_mut(panel);
            if !state.open {
                continue;
            }

            let reducible = (state.size - min_size).max(0.0);
            let reduction = overflow.min(reducible);
            state.size -= reduction;
            overflow -= reduction;
        }
    }

    pub(crate) fn handle_panel_toggle_click(&mut self, mouse: Vec2, layout: &ShellLayout) -> bool {
        for (panel, rect) in [
            (DockPanelKind::Files, layout.files),
            (DockPanelKind::Hierarchy, layout.hierarchy),
            (DockPanelKind::Inspector, layout.inspector),
            (DockPanelKind::Bottom, layout.bottom),
        ] {
            if panel_toggle_rect(rect).contains(mouse) {
                self.toggle_panel(panel);
                return true;
            }
        }

        false
    }

    pub(crate) fn handle_panel_resize_start(&mut self, mouse: Vec2, layout: &ShellLayout) -> bool {
        if let Some(rect) = layout.files_resize {
            if rect.contains(mouse) {
                self.begin_panel_resize(DockPanelKind::Files, mouse);
                return true;
            }
        }
        if let Some(rect) = layout.hierarchy_resize {
            if rect.contains(mouse) {
                self.begin_panel_resize(DockPanelKind::Hierarchy, mouse);
                return true;
            }
        }
        if let Some(rect) = layout.inspector_resize {
            if rect.contains(mouse) {
                self.begin_panel_resize(DockPanelKind::Inspector, mouse);
                return true;
            }
        }
        if let Some(rect) = layout.bottom_resize {
            if rect.contains(mouse) {
                self.begin_panel_resize(DockPanelKind::Bottom, mouse);
                return true;
            }
        }

        false
    }

    pub(crate) fn update_panel_resize(&mut self, engine: &Engine) {
        if !engine.input().is_mouse_down(0) {
            self.panel_resize_drag = None;
            return;
        }

        let Some(drag) = self.panel_resize_drag else {
            return;
        };

        let pointer = engine.mouse_screen_pos();
        let delta = pointer - drag.pointer_origin;
        let next_size = match drag.panel {
            DockPanelKind::Files | DockPanelKind::Hierarchy => drag.size_origin + delta.x,
            DockPanelKind::Inspector => drag.size_origin - delta.x,
            DockPanelKind::Bottom => drag.size_origin + delta.y,
        };

        self.panel_state_mut(drag.panel).size = next_size;
    }

    pub(crate) fn capture_text_input_owner(
        &mut self,
        owner: TextInputOwner,
        focused: Option<usize>,
        hovered: Option<usize>,
        mouse_pressed: bool,
    ) {
        let hovered_text_input = match (owner, hovered) {
            (TextInputOwner::FileBrowser, Some(id)) if is_file_browser_text_input(id) => Some(id),
            (TextInputOwner::Inspector, Some(id)) if is_inspector_text_input(id) => Some(id),
            _ => None,
        };
        let focused_text_input = match (owner, focused) {
            (TextInputOwner::FileBrowser, Some(id)) if is_file_browser_text_input(id) => Some(id),
            (TextInputOwner::Inspector, Some(id)) if is_inspector_text_input(id) => Some(id),
            _ => None,
        };

        if mouse_pressed {
            if hovered_text_input.is_some() {
                self.active_text_input_owner = Some(owner);
            }
            return;
        }

        if self.active_text_input_owner == Some(owner) && focused_text_input.is_some() {
            self.active_text_input_owner = Some(owner);
        }
    }

    pub(crate) fn update_scrolls(&mut self, engine: &Engine, layout: &ShellLayout) {
        let filter = self.file_browser_form.filter.trim().to_ascii_lowercase();
        let project_line_count = if layout.files_open {
            flattened_project_tree(
                &self.project_tree,
                &self.collapsed_project_paths,
                &self.workspace_root,
                &filter,
            )
            .len()
        } else {
            0
        };
        let project_list_rect = project_browser_list_rect(layout.files);
        let project_max = if layout.files_open {
            scroll_max_for_lines(project_line_count, project_list_rect)
        } else {
            0.0
        };

        let hierarchy_line_count = if layout.hierarchy_open {
            self.scene_node_lines().len()
        } else {
            0
        };
        let hierarchy_list_rect = scene_hierarchy_list_rect(layout.hierarchy);
        let hierarchy_max = if layout.hierarchy_open {
            scroll_max_for_lines(hierarchy_line_count, hierarchy_list_rect)
        } else {
            0.0
        };

        let bottom_line_count = if layout.bottom_open {
            match self.bottom_tab {
                BottomTab::Activity => self.activity_log.len(),
                BottomTab::SceneJson => self.scene_json_preview_line_count(),
            }
        } else {
            0
        };
        let bottom_max = if layout.bottom_open {
            scroll_max_for_lines(bottom_line_count, layout.bottom_content)
        } else {
            0.0
        };

        self.project_scroll = self.project_scroll.clamp(0.0, project_max);
        self.hierarchy_scroll = self.hierarchy_scroll.clamp(0.0, hierarchy_max);
        self.bottom_scroll = self.bottom_scroll.clamp(0.0, bottom_max);

        let (_, scroll_y) = engine.input().scroll_delta();
        if scroll_y == 0.0 {
            return;
        }

        let mouse = engine.mouse_screen_pos();
        let delta = scroll_y * 28.0;

        if layout.files_open && project_list_rect.contains(mouse) {
            self.project_scroll = (self.project_scroll - delta).clamp(0.0, project_max);
        } else if layout.hierarchy_open && hierarchy_list_rect.contains(mouse) {
            self.hierarchy_scroll = (self.hierarchy_scroll - delta).clamp(0.0, hierarchy_max);
        } else if layout.bottom_open && layout.bottom_content.contains(mouse) {
            self.bottom_scroll = (self.bottom_scroll - delta).clamp(0.0, bottom_max);
        }
    }

    pub(crate) fn handle_context_clicks(&mut self, engine: &Engine, layout: &ShellLayout) {
        let input = engine.input();
        let context_click =
            input.is_mouse_pressed(1) || (self.popup_menu.is_none() && input.is_mouse_released(1));
        if !context_click {
            return;
        }

        self.clear_text_input_owner();

        let mouse = engine.mouse_screen_pos();

        if self.handle_project_tree_context_click(mouse, layout) {
            return;
        }

        if self.handle_scene_tree_context_click(mouse, layout) {
            return;
        }

        if self.handle_viewport_context_click(mouse, layout) {
            return;
        }

        self.popup_menu = None;
    }

    pub(crate) fn handle_clicks(&mut self, engine: &Engine, layout: &ShellLayout) {
        if !engine.input().is_mouse_pressed(0) {
            return;
        }

        self.clear_text_input_owner();

        let mouse = engine.mouse_screen_pos();

        if self.handle_popup_click(engine, mouse) {
            return;
        }
        if self.handle_panel_resize_start(mouse, layout) {
            return;
        }
        if self.handle_panel_toggle_click(mouse, layout) {
            return;
        }

        if self.handle_top_bar_click(mouse, layout) {
            return;
        }
        if self.handle_scene_tab_click(mouse, layout) {
            return;
        }
        if self.handle_bottom_tab_click(mouse, layout) {
            return;
        }
        if self.handle_project_tree_click(mouse, layout) {
            return;
        }
        if self.handle_scene_tree_click(engine, mouse, layout) {
            return;
        }
        self.handle_viewport_press(engine, mouse, layout);
    }

    pub(crate) fn handle_top_bar_click(&mut self, mouse: Vec2, layout: &ShellLayout) -> bool {
        let buttons = self.top_bar_buttons(layout.top_bar);
        for (label, rect) in buttons {
            if rect.contains(mouse) {
                match label {
                    "New" => self.new_scene(),
                    "Open" => self.open_scene(),
                    "Save" => self.save_scene(),
                    "Save As" => self.save_scene_as(),
                    "Refresh" => self.refresh_project_tree(),
                    "Quit" => self.quit_requested = true,
                    _ => {}
                }
                return true;
            }
        }

        false
    }

    pub(crate) fn handle_project_tree_context_click(
        &mut self,
        mouse: Vec2,
        layout: &ShellLayout,
    ) -> bool {
        let Some((toggle_only, path)) = self.project_tree_hit(mouse, layout) else {
            return false;
        };
        if toggle_only {
            return false;
        }

        self.selected_project_path = Some(path.clone());
        self.recent_project_click = None;
        self.open_project_entry_menu(mouse, path);
        true
    }

    pub(crate) fn handle_scene_tree_context_click(
        &mut self,
        mouse: Vec2,
        layout: &ShellLayout,
    ) -> bool {
        if !layout.hierarchy_open || !layout.hierarchy.contains(mouse) {
            return false;
        }

        let header_rect = scene_hierarchy_header_rect(layout.hierarchy);
        let list_rect = scene_hierarchy_list_rect(layout.hierarchy);
        if !header_rect.contains(mouse) && !list_rect.contains(mouse) {
            return false;
        }

        let lines = self.scene_node_lines();
        let mut parent = None;
        if list_rect.contains(mouse) {
            for (index, line) in lines.iter().enumerate() {
                let rect = list_line_rect(list_rect, index, self.hierarchy_scroll);
                if rect.contains(mouse) {
                    parent = Some(line.node_id);
                    break;
                }
            }
        }

        self.select_only_scene_node(parent);
        self.open_add_node_menu(mouse, parent, None);
        true
    }

    pub(crate) fn handle_viewport_context_click(
        &mut self,
        mouse: Vec2,
        layout: &ShellLayout,
    ) -> bool {
        if !layout.viewport.contains(mouse) {
            return false;
        }

        let target = self
            .viewport_node_rects(layout.viewport)
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(mouse))
            .map(|(node_id, _)| *node_id);
        let position = Some(screen_to_scene(
            mouse,
            layout.viewport,
            self.active_scene_tab().viewport_pan,
        ));

        self.select_only_scene_node(target);
        self.open_add_node_menu(mouse, target, position);
        true
    }

    pub(crate) fn handle_popup_click(&mut self, engine: &Engine, mouse: Vec2) -> bool {
        let Some(menu) = self.popup_menu.as_ref() else {
            return false;
        };

        let actions = self.popup_menu_actions(&menu.kind);
        let labels: Vec<String> = actions
            .iter()
            .map(|action| self.popup_action_label(action))
            .collect();
        let window_rect = editor_window_rect(engine);
        let rect = popup_menu_rect(
            menu,
            popup_menu_width(labels.iter().map(String::as_str)),
            window_rect,
        );
        let mut selected_action = None;
        if rect.contains(mouse) {
            for (index, action) in actions.iter().enumerate() {
                let item_rect = popup_menu_item_rect(rect, index);
                if item_rect.contains(mouse) {
                    selected_action = Some(action.clone());
                    break;
                }
            }
        }

        self.popup_menu = None;
        if let Some(action) = selected_action {
            self.apply_popup_action(action);
        }
        true
    }

    pub(crate) fn handle_scene_tab_click(&mut self, mouse: Vec2, layout: &ShellLayout) -> bool {
        let tabs = self.scene_tab_buttons(layout.scene_tabs);
        for (index, rect) in tabs {
            if rect.contains(mouse) {
                self.switch_to_scene_tab(index);
                return true;
            }
        }

        false
    }

    pub(crate) fn handle_bottom_tab_click(&mut self, mouse: Vec2, layout: &ShellLayout) -> bool {
        if !layout.bottom_open {
            return false;
        }

        let tabs = self.bottom_tab_buttons(layout.bottom_tabs);
        for (tab, rect) in tabs {
            if rect.contains(mouse) {
                self.bottom_tab = tab;
                self.bottom_scroll = 0.0;
                return true;
            }
        }

        false
    }

    pub(crate) fn handle_project_tree_click(&mut self, mouse: Vec2, layout: &ShellLayout) -> bool {
        if let Some((toggle_only, path)) = self.project_tree_hit(mouse, layout) {
            if toggle_only {
                self.toggle_project_entry(&path);
            } else {
                self.selected_project_path = Some(path.clone());
                self.push_log(format!("Selected {}", self.display_path(&path)));
                if path.is_file() && is_scene_path(&path) && self.register_project_click(&path) {
                    self.open_selected_scene();
                }
            }
            return true;
        }

        false
    }

    pub(crate) fn project_tree_hit(
        &self,
        mouse: Vec2,
        layout: &ShellLayout,
    ) -> Option<(bool, PathBuf)> {
        if !layout.files_open || !layout.files.contains(mouse) {
            return None;
        }

        let list_rect = project_browser_list_rect(layout.files);
        if !list_rect.contains(mouse) {
            return None;
        }

        let filter = self.file_browser_form.filter.trim().to_ascii_lowercase();
        let lines = flattened_project_tree(
            &self.project_tree,
            &self.collapsed_project_paths,
            &self.workspace_root,
            &filter,
        );

        for (index, line) in lines.iter().enumerate() {
            let rect = list_line_rect(list_rect, index, self.project_scroll);
            if rect.contains(mouse) {
                if line.entry.is_dir && !line.entry.children.is_empty() {
                    let toggle_rect = tree_toggle_rect(rect, line.depth);
                    if toggle_rect.contains(mouse) {
                        return Some((true, line.entry.path.clone()));
                    }
                }

                return Some((false, line.entry.path.clone()));
            }
        }

        None
    }

    pub(crate) fn handle_scene_tree_click(
        &mut self,
        engine: &Engine,
        mouse: Vec2,
        layout: &ShellLayout,
    ) -> bool {
        if !layout.hierarchy_open || !layout.hierarchy.contains(mouse) {
            return false;
        }

        let additive = history_modifier_down(engine);

        let header_rect = scene_hierarchy_header_rect(layout.hierarchy);
        if header_rect.contains(mouse) {
            if !additive {
                self.select_only_scene_node(None);
            }
            return true;
        }

        let list_rect = scene_hierarchy_list_rect(layout.hierarchy);
        if !list_rect.contains(mouse) {
            return false;
        }

        let lines = self.scene_node_lines();
        for (index, line) in lines.iter().enumerate() {
            let rect = list_line_rect(list_rect, index, self.hierarchy_scroll);
            if rect.contains(mouse) {
                if line.has_children {
                    let toggle_rect = tree_toggle_rect(rect, line.depth);
                    if toggle_rect.contains(mouse) {
                        self.toggle_scene_node(line.node_id);
                        return true;
                    }
                }

                if additive {
                    self.toggle_scene_node_selection(line.node_id);
                } else if self.active_scene_tab().is_node_selected(line.node_id) {
                    self.focus_scene_node(line.node_id);
                } else {
                    self.select_only_scene_node(Some(line.node_id));
                }
                return true;
            }
        }

        if !additive {
            self.select_only_scene_node(None);
        }
        true
    }

    pub(crate) fn handle_viewport_press(
        &mut self,
        engine: &Engine,
        mouse: Vec2,
        layout: &ShellLayout,
    ) {
        if !layout.viewport.contains(mouse) {
            return;
        }

        let additive = history_modifier_down(engine);

        if !additive {
            let gizmo_consumed = {
                let tab = self.active_scene_tab();
                let bounds = scene_nodes_bounds(
                    tab.scene
                        .nodes
                        .iter()
                        .filter(|node| tab.is_node_selected(node.id)),
                );
                match (bounds, tab.gizmo_mode) {
                    (Some(bounds), GizmoMode::Translate) => {
                        if let Some(handle) =
                            selection_translate_gizmo(bounds, layout.viewport, tab.viewport_pan)
                                .hit_test(mouse)
                        {
                            let drag = ViewportDrag {
                                node_ids: tab.selected_root_ids(),
                                transform_origin: scene_bounds_center(bounds),
                                pointer_scene_origin: screen_to_scene(
                                    mouse,
                                    layout.viewport,
                                    tab.viewport_pan,
                                ),
                                applied_delta: [0.0, 0.0],
                                constraint: match handle {
                                    ViewportTranslateHandle::Plane => ViewportDragConstraint::Free,
                                    ViewportTranslateHandle::AxisX => ViewportDragConstraint::AxisX,
                                    ViewportTranslateHandle::AxisY => ViewportDragConstraint::AxisY,
                                },
                                history_captured: false,
                            };
                            Some(Box::new(move |tab_mut: &mut SceneTab| {
                                tab_mut.viewport_drag = Some(drag);
                            })
                                as Box<dyn FnOnce(&mut SceneTab)>)
                        } else {
                            None
                        }
                    }
                    (Some(bounds), GizmoMode::Rotate) => {
                        let rotate_gizmo =
                            selection_rotate_gizmo(bounds, layout.viewport, tab.viewport_pan);
                        if rotate_gizmo.hit_test(mouse) {
                            let node_ids = tab.selected_root_ids();
                            let pivot = scene_bounds_center(bounds);
                            let angle_start = rotate_gizmo.pointer_angle(mouse);
                            Some(Box::new(move |tab_mut: &mut SceneTab| {
                                tab_mut.viewport_rotate_drag = Some(ViewportRotateDrag {
                                    node_ids,
                                    pivot_scene: pivot,
                                    angle_start,
                                    applied_degrees: 0.0,
                                    history_captured: false,
                                });
                            })
                                as Box<dyn FnOnce(&mut SceneTab)>)
                        } else {
                            None
                        }
                    }
                    (Some(bounds), GizmoMode::Scale) => {
                        let scale_gizmo =
                            selection_scale_gizmo(bounds, layout.viewport, tab.viewport_pan);
                        if scale_gizmo.hit_test(mouse).is_some() {
                            let node_ids = tab.selected_root_ids();
                            let pivot = scene_bounds_center(bounds);
                            let pivot_screen =
                                scene_to_screen(pivot, layout.viewport, tab.viewport_pan);
                            let dx = mouse.x - pivot_screen.x;
                            let dy = mouse.y - pivot_screen.y;
                            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                            let original_offsets: Vec<[f32; 2]> = node_ids
                                .iter()
                                .filter_map(|id| tab.scene.node(*id))
                                .map(|n| [n.position[0] - pivot[0], n.position[1] - pivot[1]])
                                .collect();
                            let original_sizes: Vec<[f32; 2]> = node_ids
                                .iter()
                                .filter_map(|id| tab.scene.node(*id))
                                .map(|n| n.size)
                                .collect();
                            Some(Box::new(move |tab_mut: &mut SceneTab| {
                                tab_mut.viewport_scale_drag = Some(ViewportScaleDrag {
                                    node_ids,
                                    pivot_scene: pivot,
                                    pointer_start_dist: dist,
                                    original_offsets,
                                    original_sizes,
                                    applied_factor: 1.0,
                                    history_captured: false,
                                });
                            })
                                as Box<dyn FnOnce(&mut SceneTab)>)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            };
            if let Some(apply) = gizmo_consumed {
                let tab = self.active_scene_tab_mut();
                tab.viewport_box_selection = None;
                apply(tab);
                return;
            }
        }

        let node_rects = self.viewport_node_rects(layout.viewport);
        if let Some((node_id, _rect)) = node_rects
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(mouse))
        {
            self.active_scene_tab_mut().viewport_box_selection = None;

            if additive {
                self.toggle_scene_node_selection(*node_id);
                self.active_scene_tab_mut().viewport_drag = None;
                return;
            }

            if self.active_scene_tab().is_node_selected(*node_id) {
                self.focus_scene_node(*node_id);
            } else {
                self.select_only_scene_node(Some(*node_id));
            }

            let drag_node_ids = self.active_scene_tab().selected_root_ids();
            let drag_origin = self
                .active_scene_tab()
                .scene
                .node(*node_id)
                .map(|node| node.position)
                .unwrap_or([0.0, 0.0]);
            self.active_scene_tab_mut().viewport_drag = Some(ViewportDrag {
                node_ids: drag_node_ids,
                transform_origin: drag_origin,
                pointer_scene_origin: screen_to_scene(
                    mouse,
                    layout.viewport,
                    self.active_scene_tab().viewport_pan,
                ),
                applied_delta: [0.0, 0.0],
                constraint: ViewportDragConstraint::Free,
                history_captured: false,
            });
        } else {
            self.active_scene_tab_mut().viewport_drag = None;
            self.active_scene_tab_mut().viewport_box_selection = Some(ViewportBoxSelection {
                pointer_origin: mouse,
                pointer_current: mouse,
                additive,
                initial_selected_node: self.active_scene_tab().selected_node,
                initial_selected_nodes: self.active_scene_tab().selected_nodes.clone(),
            });
        }
    }

    pub(crate) fn handle_viewport_pan_press(&mut self, engine: &Engine, layout: &ShellLayout) {
        if !engine.input().is_mouse_pressed(2) {
            return;
        }

        let mouse = engine.mouse_screen_pos();
        if !layout.viewport.contains(mouse) {
            return;
        }

        let pan = self.active_scene_tab().viewport_pan;
        self.active_scene_tab_mut().viewport_pan_drag = Some(ViewportPanDrag {
            pointer_origin: mouse,
            pan_origin: pan,
        });
    }

    pub(crate) fn update_viewport_drag(&mut self, engine: &Engine, layout: &ShellLayout) {
        let left_mouse_down = engine.input().is_mouse_down(0);
        if !left_mouse_down {
            let box_selection = self.active_scene_tab().viewport_box_selection.clone();
            {
                let tab = self.active_scene_tab_mut();
                tab.viewport_drag = None;
                tab.viewport_rotate_drag = None;
                tab.viewport_scale_drag = None;
                tab.viewport_box_selection = None;
            }
            if let Some(box_selection) = box_selection {
                self.finish_viewport_box_selection(box_selection, layout.viewport);
            }
        }

        if !engine.input().is_mouse_down(2) {
            self.active_scene_tab_mut().viewport_pan_drag = None;
        } else if let Some(pan_drag) = self.active_scene_tab().viewport_pan_drag {
            let pointer = engine.mouse_screen_pos();
            self.active_scene_tab_mut().viewport_pan =
                pan_drag.pan_origin + (pointer - pan_drag.pointer_origin);
        }

        if !left_mouse_down {
            return;
        }

        let pointer = engine.mouse_screen_pos();
        if let Some(box_selection) = self.active_scene_tab_mut().viewport_box_selection.as_mut() {
            box_selection.pointer_current = pointer;
            return;
        }

        if self.active_scene_tab().viewport_rotate_drag.is_some() {
            self.update_rotate_drag(engine, layout, pointer);
            return;
        }

        if self.active_scene_tab().viewport_scale_drag.is_some() {
            self.update_scale_drag(engine, layout, pointer);
            return;
        }

        let Some(drag) = self.active_scene_tab().viewport_drag.clone() else {
            return;
        };

        let pointer_scene = screen_to_scene(
            pointer,
            layout.viewport,
            self.active_scene_tab().viewport_pan,
        );
        let target_position = viewport_drag_target(
            pointer_scene,
            &drag,
            viewport_snap_enabled(engine),
            VIEWPORT_GRID_STEP,
        );
        let desired_delta = [
            target_position[0] - drag.transform_origin[0],
            target_position[1] - drag.transform_origin[1],
        ];
        let delta = [
            desired_delta[0] - drag.applied_delta[0],
            desired_delta[1] - drag.applied_delta[1],
        ];

        if delta != [0.0, 0.0] {
            let history_entry = (!drag.history_captured)
                .then(|| SceneHistoryEntry::capture(self.active_scene_tab()));
            let tab = self.active_scene_tab_mut();
            if let Some(history_entry) = history_entry {
                tab.push_undo_entry(history_entry);
                if let Some(active_drag) = tab.viewport_drag.as_mut() {
                    active_drag.history_captured = true;
                }
            }
            for node_id in &drag.node_ids {
                tab.scene.translate_subtree(*node_id, delta);
            }
            if let Some(active_drag) = tab.viewport_drag.as_mut() {
                active_drag.applied_delta = desired_delta;
            }
            tab.mark_dirty();
        }
    }

    fn update_rotate_drag(&mut self, engine: &Engine, layout: &ShellLayout, pointer: Vec2) {
        let Some(drag) = self.active_scene_tab().viewport_rotate_drag.clone() else {
            return;
        };
        let pivot_screen = scene_to_screen(
            drag.pivot_scene,
            layout.viewport,
            self.active_scene_tab().viewport_pan,
        );
        let current_angle = {
            let dx = pointer.x - pivot_screen.x;
            let dy = pointer.y - pivot_screen.y;
            dy.atan2(dx)
        };
        let raw_degrees = (current_angle - drag.angle_start).to_degrees();
        let target_degrees = if viewport_snap_enabled(engine) {
            (raw_degrees / 15.0).round() * 15.0
        } else {
            raw_degrees
        };
        let delta_degrees = target_degrees - drag.applied_degrees;

        if delta_degrees.abs() > f32::EPSILON {
            let history_entry = (!drag.history_captured)
                .then(|| SceneHistoryEntry::capture(self.active_scene_tab()));
            let tab = self.active_scene_tab_mut();
            if let Some(history_entry) = history_entry {
                tab.push_undo_entry(history_entry);
                if let Some(d) = tab.viewport_rotate_drag.as_mut() {
                    d.history_captured = true;
                }
            }
            for node_id in &drag.node_ids {
                tab.scene.rotate_node(*node_id, delta_degrees);
            }
            if let Some(d) = tab.viewport_rotate_drag.as_mut() {
                d.applied_degrees = target_degrees;
            }
            tab.mark_dirty();
        }
    }

    fn update_scale_drag(&mut self, _engine: &Engine, layout: &ShellLayout, pointer: Vec2) {
        let Some(drag) = self.active_scene_tab().viewport_scale_drag.clone() else {
            return;
        };
        let pivot_screen = scene_to_screen(
            drag.pivot_scene,
            layout.viewport,
            self.active_scene_tab().viewport_pan,
        );
        let dx = pointer.x - pivot_screen.x;
        let dy = pointer.y - pivot_screen.y;
        let current_dist = (dx * dx + dy * dy).sqrt().max(1.0);
        let new_factor = (current_dist / drag.pointer_start_dist).max(0.05);
        let factor_delta = new_factor / drag.applied_factor;

        if (factor_delta - 1.0).abs() > 0.001 {
            let history_entry = (!drag.history_captured)
                .then(|| SceneHistoryEntry::capture(self.active_scene_tab()));
            let tab = self.active_scene_tab_mut();
            if let Some(history_entry) = history_entry {
                tab.push_undo_entry(history_entry);
                if let Some(d) = tab.viewport_scale_drag.as_mut() {
                    d.history_captured = true;
                }
            }
            for (i, node_id) in drag.node_ids.iter().enumerate() {
                let orig_offset = drag.original_offsets[i];
                let orig_size = drag.original_sizes[i];
                let new_position = [
                    drag.pivot_scene[0] + orig_offset[0] * new_factor,
                    drag.pivot_scene[1] + orig_offset[1] * new_factor,
                ];
                let new_size = [orig_size[0] * new_factor, orig_size[1] * new_factor];
                tab.scene
                    .set_node_position_and_size(*node_id, new_position, new_size);
            }
            if let Some(d) = tab.viewport_scale_drag.as_mut() {
                d.applied_factor = new_factor;
            }
            tab.mark_dirty();
        }
    }

    pub(crate) fn finish_viewport_box_selection(
        &mut self,
        box_selection: ViewportBoxSelection,
        viewport: PanelRect,
    ) {
        let selection_rect = viewport_box_selection_rect(&box_selection);
        let box_hit_ids = if selection_rect.w < VIEWPORT_BOX_SELECTION_CLICK_THRESHOLD
            && selection_rect.h < VIEWPORT_BOX_SELECTION_CLICK_THRESHOLD
        {
            Vec::new()
        } else {
            self.viewport_node_rects(viewport)
                .into_iter()
                .filter(|(_, node_rect)| panel_rects_overlap(selection_rect, *node_rect))
                .map(|(node_id, _)| node_id)
                .collect::<Vec<_>>()
        };
        let next_selected_node = box_hit_ids.last().copied().or(if box_selection.additive {
            box_selection.initial_selected_node
        } else {
            None
        });

        if box_selection.additive {
            let mut next_selected_nodes = box_selection.initial_selected_nodes;
            next_selected_nodes.extend(box_hit_ids);
            self.update_scene_selection(|tab| {
                tab.set_selection(next_selected_node, next_selected_nodes)
            });
            return;
        }

        self.update_scene_selection(|tab| tab.set_selection(next_selected_node, box_hit_ids));
    }

    pub(crate) fn top_bar_buttons(&self, top_bar: PanelRect) -> Vec<(&'static str, PanelRect)> {
        let labels = ["New", "Open", "Save", "Save As", "Refresh", "Quit"];
        let available_width =
            (top_bar.w - PANEL_PADDING * 2.0 - (top_bar.w * 0.32).clamp(320.0, 460.0)).max(0.0);
        if available_width <= 0.0 {
            return Vec::new();
        }

        let preferred_widths = labels.map(button_preferred_width);
        let widths = distribute_button_widths(
            &preferred_widths,
            available_width,
            BUTTON_GAP,
            TOP_BAR_BUTTON_MIN_WIDTH,
        );
        let mut buttons = Vec::with_capacity(labels.len());
        let mut x = top_bar.x + PANEL_PADDING;
        let y = top_bar.y + 14.0;
        for (index, label) in labels.iter().enumerate() {
            let width = widths[index];
            buttons.push((*label, PanelRect::new(x, y, width, BUTTON_HEIGHT)));
            x += width + BUTTON_GAP;
        }
        buttons
    }

    pub(crate) fn scene_tab_buttons(&self, rect: PanelRect) -> Vec<(usize, PanelRect)> {
        if rect.w <= 0.0 || self.scene_tabs.is_empty() {
            return Vec::new();
        }

        let labels: Vec<String> = self.scene_tabs.iter().map(|tab| tab.tab_label()).collect();
        let preferred_widths: Vec<f32> = labels
            .iter()
            .map(|label| button_preferred_width(label))
            .collect();
        let widths = distribute_button_widths(
            &preferred_widths,
            rect.w,
            BUTTON_GAP,
            SCENE_TAB_BUTTON_MIN_WIDTH,
        );

        let mut buttons = Vec::with_capacity(labels.len());
        let mut x = rect.x;
        for (index, width) in widths.iter().enumerate() {
            buttons.push((index, PanelRect::new(x, rect.y, *width, rect.h)));
            x += *width + BUTTON_GAP;
        }
        buttons
    }

    pub(crate) fn bottom_tab_buttons(&self, rect: PanelRect) -> Vec<(BottomTab, PanelRect)> {
        if rect.w <= 0.0 {
            return Vec::new();
        }

        let tabs = [BottomTab::Activity, BottomTab::SceneJson];
        let preferred_widths = tabs.map(|tab| button_preferred_width(tab.label()));
        let widths = distribute_button_widths(
            &preferred_widths,
            rect.w,
            BUTTON_GAP,
            BOTTOM_TAB_BUTTON_MIN_WIDTH,
        );

        let mut buttons = Vec::with_capacity(tabs.len());
        let mut x = rect.x;
        for (index, tab) in tabs.iter().enumerate() {
            let width = widths[index];
            buttons.push((*tab, PanelRect::new(x, rect.y, width, rect.h)));
            x += width + BUTTON_GAP;
        }
        buttons
    }

    pub(crate) fn scene_node_lines(&self) -> Vec<SceneNodeLine> {
        let mut lines = Vec::new();
        let roots = self.active_scene_tab().scene.root_ids();
        for root_id in roots {
            self.collect_scene_node_lines(root_id, 0, &mut lines);
        }
        lines
    }

    pub(crate) fn collect_scene_node_lines(
        &self,
        node_id: u64,
        depth: usize,
        lines: &mut Vec<SceneNodeLine>,
    ) {
        if let Some(node) = self.active_scene_tab().scene.node(node_id) {
            let child_ids = self.active_scene_tab().scene.child_ids(node_id);
            let is_collapsed = self.active_scene_tab().collapsed_nodes.contains(&node_id);
            lines.push(SceneNodeLine {
                node_id,
                depth,
                label: format!("{} {}", node.kind.short_label(), node.name),
                has_children: !child_ids.is_empty(),
                is_collapsed,
            });

            if !is_collapsed {
                for child_id in child_ids {
                    self.collect_scene_node_lines(child_id, depth + 1, lines);
                }
            }
        }
    }

    pub(crate) fn viewport_node_rects(&self, viewport: PanelRect) -> Vec<(u64, PanelRect)> {
        let pan = self.active_scene_tab().viewport_pan;
        self.active_scene_tab()
            .scene
            .nodes
            .iter()
            .filter(|node| node.visible)
            .map(|node| {
                let center = scene_to_screen(node.position, viewport, pan);
                (
                    node.id,
                    PanelRect::new(
                        center.x - node.size[0] * 0.5,
                        center.y - node.size[1] * 0.5,
                        node.size[0],
                        node.size[1],
                    ),
                )
            })
            .collect()
    }

    pub(crate) fn frame_active_scene_view(&mut self) {
        let (target_center, frame_label) = {
            let tab = self.active_scene_tab();
            if tab.has_selection() {
                (
                    scene_nodes_frame_center(
                        tab.scene
                            .nodes
                            .iter()
                            .filter(|node| tab.is_node_selected(node.id)),
                    ),
                    format!("Framed {} selected node(s)", tab.selection_count()),
                )
            } else {
                (
                    scene_nodes_frame_center(tab.scene.nodes.iter()),
                    if tab.scene.nodes.is_empty() {
                        "Centered viewport on scene origin".to_string()
                    } else {
                        format!("Framed scene ({}) node(s)", tab.scene.nodes.len())
                    },
                )
            }
        };

        let next_pan = target_center
            .map(|center| Vec2::new(-center[0], -center[1]))
            .unwrap_or(Vec2::ZERO);
        let tab = self.active_scene_tab_mut();
        if tab.viewport_pan == next_pan {
            return;
        }

        tab.viewport_pan = next_pan;
        tab.viewport_pan_drag = None;
        tab.viewport_drag = None;
        tab.viewport_box_selection = None;
        self.push_log(frame_label);
    }
}

pub(crate) fn list_line_rect(list_rect: PanelRect, index: usize, scroll: f32) -> PanelRect {
    let top = list_rect.top() + scroll - index as f32 * LINE_HEIGHT;
    PanelRect::new(list_rect.x, top - LINE_HEIGHT, list_rect.w, LINE_HEIGHT)
}

fn viewport_box_selection_rect(box_selection: &ViewportBoxSelection) -> PanelRect {
    PanelRect::new(
        box_selection
            .pointer_origin
            .x
            .min(box_selection.pointer_current.x),
        box_selection
            .pointer_origin
            .y
            .min(box_selection.pointer_current.y),
        (box_selection.pointer_current.x - box_selection.pointer_origin.x).abs(),
        (box_selection.pointer_current.y - box_selection.pointer_origin.y).abs(),
    )
}

fn panel_rects_overlap(left: PanelRect, right: PanelRect) -> bool {
    left.x <= right.right()
        && left.right() >= right.x
        && left.y <= right.top()
        && left.top() >= right.y
}

fn scene_nodes_frame_center<'a>(
    nodes: impl IntoIterator<Item = &'a SceneNode>,
) -> Option<[f32; 2]> {
    scene_nodes_bounds(nodes).map(scene_bounds_center)
}

fn snap_scene_position(position: [f32; 2], step: f32) -> [f32; 2] {
    if step <= f32::EPSILON {
        return position;
    }

    [
        (position[0] / step).round() * step,
        (position[1] / step).round() * step,
    ]
}

fn constrain_drag_delta(delta: [f32; 2], constraint: ViewportDragConstraint) -> [f32; 2] {
    match constraint {
        ViewportDragConstraint::Free => delta,
        ViewportDragConstraint::AxisX => [delta[0], 0.0],
        ViewportDragConstraint::AxisY => [0.0, delta[1]],
    }
}

fn snap_drag_target(
    target: [f32; 2],
    origin: [f32; 2],
    constraint: ViewportDragConstraint,
    step: f32,
) -> [f32; 2] {
    if step <= f32::EPSILON {
        return target;
    }

    match constraint {
        ViewportDragConstraint::Free => snap_scene_position(target, step),
        ViewportDragConstraint::AxisX => [snap_scene_position(target, step)[0], origin[1]],
        ViewportDragConstraint::AxisY => [origin[0], snap_scene_position(target, step)[1]],
    }
}

fn viewport_drag_target(
    pointer_scene: [f32; 2],
    drag: &ViewportDrag,
    snap_enabled: bool,
    step: f32,
) -> [f32; 2] {
    let raw_delta = [
        pointer_scene[0] - drag.pointer_scene_origin[0],
        pointer_scene[1] - drag.pointer_scene_origin[1],
    ];
    let constrained_delta = constrain_drag_delta(raw_delta, drag.constraint);
    let target = [
        drag.transform_origin[0] + constrained_delta[0],
        drag.transform_origin[1] + constrained_delta[1],
    ];
    if snap_enabled {
        snap_drag_target(target, drag.transform_origin, drag.constraint, step)
    } else {
        target
    }
}

pub(crate) fn button_preferred_width(label: &str) -> f32 {
    label.chars().count() as f32 * 8.6 + 30.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Camera2dNodeSettings, SpriteNodeSettings};

    fn test_node(id: u64, position: [f32; 2], size: [f32; 2]) -> SceneNode {
        SceneNode {
            id,
            parent: None,
            name: format!("Node {id}"),
            kind: SceneNodeKind::Empty,
            position,
            size,
            rotation: 0.0,
            visible: true,
            script_path: String::new(),
            runtime_prefab: String::new(),
            asset_alias: String::new(),
            sprite: SpriteNodeSettings::default(),
            camera2d: Camera2dNodeSettings::default(),
            properties: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn scene_nodes_frame_center_uses_node_bounds() {
        let nodes = vec![
            test_node(1, [10.0, 20.0], [20.0, 10.0]),
            test_node(2, [70.0, 40.0], [10.0, 30.0]),
        ];

        assert_eq!(scene_nodes_frame_center(nodes.iter()), Some([37.5, 35.0]));
    }

    #[test]
    fn scene_nodes_frame_center_returns_none_for_empty_input() {
        assert_eq!(scene_nodes_frame_center(std::iter::empty()), None);
    }

    #[test]
    fn snap_scene_position_rounds_to_grid() {
        assert_eq!(
            snap_scene_position([17.0, -47.0], VIEWPORT_GRID_STEP),
            [32.0, -32.0]
        );
    }

    #[test]
    fn snap_scene_position_preserves_position_for_zero_step() {
        assert_eq!(snap_scene_position([19.0, 11.0], 0.0), [19.0, 11.0]);
    }

    #[test]
    fn constrain_drag_delta_locks_axis_motion() {
        assert_eq!(
            constrain_drag_delta([12.0, -7.0], ViewportDragConstraint::AxisX),
            [12.0, 0.0]
        );
        assert_eq!(
            constrain_drag_delta([12.0, -7.0], ViewportDragConstraint::AxisY),
            [0.0, -7.0]
        );
    }

    #[test]
    fn viewport_drag_target_snaps_only_the_active_axis() {
        let drag = ViewportDrag {
            node_ids: vec![1],
            transform_origin: [10.0, 18.0],
            pointer_scene_origin: [10.0, 18.0],
            applied_delta: [0.0, 0.0],
            constraint: ViewportDragConstraint::AxisX,
            history_captured: false,
        };

        assert_eq!(
            viewport_drag_target([41.0, 65.0], &drag, true, VIEWPORT_GRID_STEP),
            [32.0, 18.0]
        );
    }
}

pub(crate) fn distribute_button_widths(
    preferred_widths: &[f32],
    total_width: f32,
    gap: f32,
    min_width: f32,
) -> Vec<f32> {
    if preferred_widths.is_empty() {
        return Vec::new();
    }

    let count = preferred_widths.len();
    let total_gap = gap * count.saturating_sub(1) as f32;
    let available_width = (total_width - total_gap).max(0.0);
    if available_width <= 0.0 {
        return vec![0.0; count];
    }

    let preferred_total: f32 = preferred_widths.iter().sum();
    let mut widths = if preferred_total <= available_width {
        let extra_per_button = (available_width - preferred_total) / count as f32;
        preferred_widths
            .iter()
            .map(|width| width + extra_per_button)
            .collect::<Vec<_>>()
    } else {
        let hard_min_total = min_width * count as f32;
        if available_width <= hard_min_total {
            vec![available_width / count as f32; count]
        } else {
            let shrinkable_total: f32 = preferred_widths
                .iter()
                .map(|width| (width - min_width).max(0.0))
                .sum();
            if shrinkable_total <= f32::EPSILON {
                vec![available_width / count as f32; count]
            } else {
                let overflow = preferred_total - available_width;
                preferred_widths
                    .iter()
                    .map(|width| {
                        let shrinkable = (width - min_width).max(0.0);
                        let reduction = overflow * (shrinkable / shrinkable_total);
                        (width - reduction).max(min_width)
                    })
                    .collect::<Vec<_>>()
            }
        }
    };

    let used_width = widths.iter().sum::<f32>() + total_gap;
    if let Some(last_width) = widths.last_mut() {
        *last_width = (*last_width + (total_width - used_width)).max(0.0);
    }

    widths
}

pub(crate) fn project_browser_list_rect(panel: PanelRect) -> PanelRect {
    let inner = panel.inset(PANEL_PADDING);
    PanelRect::new(
        inner.x,
        inner.y,
        inner.w,
        (inner.h - PROJECT_BROWSER_CONTROLS_HEIGHT).max(0.0),
    )
}

pub(crate) fn scene_hierarchy_list_rect(panel: PanelRect) -> PanelRect {
    let inner = panel.inset(PANEL_PADDING);
    PanelRect::new(inner.x, inner.y, inner.w, (inner.h - 46.0).max(0.0))
}

pub(crate) fn scene_hierarchy_header_rect(panel: PanelRect) -> PanelRect {
    let inner = panel.inset(PANEL_PADDING);
    PanelRect::new(inner.x, inner.top() - 46.0, inner.w, 46.0)
}

pub(crate) fn tree_toggle_rect(line_rect: PanelRect, depth: usize) -> PanelRect {
    PanelRect::new(
        line_rect.x + 4.0 + depth as f32 * TREE_INDENT,
        line_rect.y,
        12.0,
        line_rect.h,
    )
}

pub(crate) fn panel_toggle_rect(panel: PanelRect) -> PanelRect {
    if panel.w <= PANEL_COLLAPSED_WIDTH + 0.1 && panel.h <= SIDE_PANEL_COLLAPSED_BUTTON_HEIGHT + 0.1
    {
        return panel;
    }

    let size = (panel.h - PANEL_PADDING * 2.0)
        .clamp(16.0, PANEL_TOGGLE_BUTTON_SIZE)
        .min((panel.w - PANEL_PADDING * 2.0).max(16.0));
    PanelRect::new(
        panel.right() - PANEL_PADDING - size,
        panel.top() - PANEL_PADDING - size,
        size,
        size,
    )
}

pub(crate) fn panel_toggle_label(kind: DockPanelKind, open: bool) -> &'static str {
    match kind {
        DockPanelKind::Files | DockPanelKind::Hierarchy => {
            if open {
                "<"
            } else {
                ">"
            }
        }
        DockPanelKind::Inspector => {
            if open {
                ">"
            } else {
                "<"
            }
        }
        DockPanelKind::Bottom => {
            if open {
                "v"
            } else {
                "^"
            }
        }
    }
}

pub(crate) fn collapsed_side_panel_rect(x: f32, content_top: f32, index: usize) -> PanelRect {
    PanelRect::new(
        x,
        content_top
            - SIDE_PANEL_COLLAPSED_BUTTON_HEIGHT
            - index as f32 * (SIDE_PANEL_COLLAPSED_BUTTON_HEIGHT + BUTTON_GAP),
        PANEL_COLLAPSED_WIDTH,
        SIDE_PANEL_COLLAPSED_BUTTON_HEIGHT,
    )
}

pub(crate) fn scroll_max_for_lines(line_count: usize, rect: PanelRect) -> f32 {
    (line_count as f32 * LINE_HEIGHT - rect.h).max(0.0)
}
