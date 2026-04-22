use super::*;

impl RengineNativeEditor {
    pub(crate) fn draw_shell(&mut self, engine: &Engine, frame: &mut Frame) {
        let layout = ShellLayout::new(engine, &self.panel_layout);
        frame.clear_color = Color::from_rgba8(14, 19, 24, 255);

        let canvas = frame.canvas(0);

        draw_panel(canvas, layout.top_bar, Color::from_rgba8(20, 26, 33, 255));
        draw_panel(canvas, layout.files, Color::from_rgba8(21, 26, 33, 255));
        draw_panel(canvas, layout.hierarchy, Color::from_rgba8(21, 26, 33, 255));
        draw_panel(canvas, layout.inspector, Color::from_rgba8(21, 26, 33, 255));
        draw_panel(canvas, layout.center, Color::from_rgba8(21, 26, 33, 255));
        draw_panel(canvas, layout.bottom, Color::from_rgba8(21, 26, 33, 255));
        draw_panel(canvas, layout.viewport, Color::from_rgba8(17, 22, 28, 255));
        if let Some(rect) = layout.files_resize {
            draw_resize_handle(canvas, rect);
        }
        if let Some(rect) = layout.hierarchy_resize {
            draw_resize_handle(canvas, rect);
        }
        if let Some(rect) = layout.inspector_resize {
            draw_resize_handle(canvas, rect);
        }
        if let Some(rect) = layout.bottom_resize {
            draw_resize_handle(canvas, rect);
        }

        let mut tooltip_targets = Vec::new();
        self.draw_top_bar(canvas, layout.top_bar, &mut tooltip_targets);
        self.draw_scene_tabs(canvas, layout.scene_tabs, &mut tooltip_targets);
        self.draw_project_browser(canvas, layout.files, layout.files_open, &mut tooltip_targets);
        self.draw_scene_hierarchy(
            canvas,
            layout.hierarchy,
            layout.hierarchy_open,
            &mut tooltip_targets,
        );
        self.draw_inspector(
            canvas,
            layout.inspector,
            layout.inspector_open,
            &mut tooltip_targets,
        );
        self.draw_bottom_panel(
            canvas,
            layout.bottom,
            layout.bottom_tabs,
            layout.bottom_content,
            layout.bottom_open,
            &mut tooltip_targets,
        );
        self.draw_viewport(engine, canvas, layout.viewport);
        self.canvas_tooltip_targets = tooltip_targets;
    }

    fn draw_top_bar(
        &self,
        canvas: &mut Canvas,
        rect: PanelRect,
        tooltip_targets: &mut Vec<CanvasTooltipTarget>,
    ) {
        for (label, button_rect) in self.top_bar_buttons(rect) {
            draw_button(canvas, button_rect, label, false, true, tooltip_targets);
        }

        let scene_label = self.active_scene_tab().tab_label();
        let scene_path = self
            .active_scene_tab()
            .scene_path
            .as_ref()
            .map(|path| self.display_path(path))
            .unwrap_or_else(|| "unsaved scene".to_string());
        let header_row = PanelRect::new(rect.x, rect.y + 28.0, rect.w, 18.0);
        let meta_row = PanelRect::new(rect.x, rect.y + 8.0, rect.w, 18.0);

        canvas.text_aligned(
            rect.right() - PANEL_PADDING,
            text_baseline_in_rect(canvas, header_row, 16.0),
            &scene_label,
            16.0,
            Color::from_rgba8(214, 222, 232, 255),
            TextAlign::Right,
        );

        let branch_size = 11.0;
        let branch_label = self.branch_name.as_str();
        let (branch_width, _) = canvas.measure_text(branch_label, branch_size);
        let branch_rect = PanelRect::new(
            rect.right() - PANEL_PADDING - (branch_width + 18.0),
            rect.y + 6.0,
            branch_width + 18.0,
            22.0,
        );
        canvas.rect(
            branch_rect.x,
            branch_rect.y,
            branch_rect.w,
            branch_rect.h,
            Color::from_rgba8(28, 38, 48, 255),
        );
        draw_outline(canvas, branch_rect, Color::from_rgba8(56, 74, 90, 255));
        canvas.text_aligned(
            branch_rect.x + branch_rect.w * 0.5,
            text_baseline_in_rect(canvas, branch_rect, branch_size),
            branch_label,
            branch_size,
            Color::from_rgba8(170, 192, 212, 255),
            TextAlign::Center,
        );
        canvas.text_aligned(
            branch_rect.x - 10.0,
            text_baseline_in_rect(canvas, meta_row, 12.0),
            &scene_path,
            12.0,
            Color::from_rgba8(132, 144, 160, 255),
            TextAlign::Right,
        );
    }

    fn draw_scene_tabs(
        &self,
        canvas: &mut Canvas,
        rect: PanelRect,
        tooltip_targets: &mut Vec<CanvasTooltipTarget>,
    ) {
        for (index, button_rect) in self.scene_tab_buttons(rect) {
            draw_button(
                canvas,
                button_rect,
                &self.scene_tabs[index].tab_label(),
                self.active_scene_tab == index,
                true,
                tooltip_targets,
            );
        }
    }

    fn draw_project_browser(
        &self,
        canvas: &mut Canvas,
        panel: PanelRect,
        open: bool,
        tooltip_targets: &mut Vec<CanvasTooltipTarget>,
    ) {
        let toggle_rect = panel_toggle_rect(panel);
        draw_button(
            canvas,
            toggle_rect,
            panel_toggle_label(DockPanelKind::Files, open),
            false,
            true,
            tooltip_targets,
        );

        if !open {
            return;
        }

        let inner = panel.inset(PANEL_PADDING);
        draw_fitted_text_left(
            canvas,
            PanelRect::new(
                inner.x,
                inner.top() - 24.0,
                (toggle_rect.x - inner.x - 8.0).max(0.0),
                20.0,
            ),
            "Files",
            18.0,
            Color::WHITE,
            tooltip_targets,
        );

        let filter = self.file_browser_form.filter.trim().to_ascii_lowercase();
        let list_rect = project_browser_list_rect(panel);
        let lines = flattened_project_tree(
            &self.project_tree,
            &self.collapsed_project_paths,
            &self.workspace_root,
            &filter,
        );
        canvas.push_clip(list_rect.x, list_rect.y, list_rect.w, list_rect.h);
        if !filter.is_empty() && !project_tree_matches_filter(&self.project_tree, &filter) {
            draw_list_text(
                list_rect.x,
                list_line_rect(list_rect, 0, self.project_scroll),
                canvas,
                "No files match the current filter.",
                12.0,
                Color::from_rgba8(148, 162, 180, 255),
            );
        } else {
            for (index, line) in lines.iter().enumerate() {
                let line_rect = list_line_rect(list_rect, index, self.project_scroll);
                if line_rect.y > list_rect.top() || line_rect.top() < list_rect.y {
                    continue;
                }

                let selected = self
                    .selected_project_path
                    .as_ref()
                    .is_some_and(|path| path == &line.entry.path);
                if selected {
                    canvas.rect(
                        line_rect.x,
                        line_rect.y,
                        (line_rect.w - SCROLLBAR_WIDTH - 4.0).max(0.0),
                        line_rect.h,
                        Color::from_rgba8(44, 82, 122, 220),
                    );
                }

                let toggle_x = line_rect.x + 6.0 + line.depth as f32 * TREE_INDENT;
                let marker = if line.entry.is_dir {
                    if line.entry.children.is_empty() {
                        "-"
                    } else if line.is_collapsed {
                        "+"
                    } else {
                        "-"
                    }
                } else {
                    "-"
                };
                let marker_color = if line.entry.is_dir {
                    Color::from_rgba8(214, 220, 232, 255)
                } else {
                    Color::from_rgba8(120, 130, 142, 255)
                };
                draw_list_text(toggle_x, line_rect, canvas, marker, 12.0, marker_color);
                draw_list_text(
                    toggle_x + 14.0,
                    line_rect,
                    canvas,
                    &line.entry.name,
                    12.0,
                    if line.entry.is_dir {
                        Color::from_rgba8(214, 220, 232, 255)
                    } else {
                        Color::from_rgba8(168, 178, 194, 255)
                    },
                );
            }
        }
        canvas.pop_clip();
        draw_scrollbar(canvas, list_rect, lines.len(), self.project_scroll);
    }

    fn draw_scene_hierarchy(
        &self,
        canvas: &mut Canvas,
        panel: PanelRect,
        open: bool,
        tooltip_targets: &mut Vec<CanvasTooltipTarget>,
    ) {
        let toggle_rect = panel_toggle_rect(panel);
        draw_button(
            canvas,
            toggle_rect,
            panel_toggle_label(DockPanelKind::Hierarchy, open),
            false,
            true,
            tooltip_targets,
        );

        if !open {
            return;
        }

        let inner = panel.inset(PANEL_PADDING);
        let header_rect = scene_hierarchy_header_rect(panel);
        if self.active_scene_tab().selected_node.is_none() {
            canvas.rect(
                header_rect.x,
                header_rect.y,
                (toggle_rect.x - header_rect.x - 8.0).max(0.0),
                header_rect.h,
                Color::from_rgba8(44, 82, 122, 168),
            );
        }
        draw_fitted_text_left(
            canvas,
            PanelRect::new(
                inner.x,
                inner.top() - 24.0,
                (toggle_rect.x - inner.x - 8.0).max(0.0),
                20.0,
            ),
            "Scene",
            18.0,
            Color::WHITE,
            tooltip_targets,
        );

        canvas.text(
            inner.x,
            inner.top() - 38.0,
            &format!("{} node(s)", self.active_scene_tab().scene.nodes.len()),
            12.0,
            Color::from_rgba8(148, 162, 180, 255),
        );

        let list_rect = scene_hierarchy_list_rect(panel);
        let lines = self.scene_node_lines();
        canvas.push_clip(list_rect.x, list_rect.y, list_rect.w, list_rect.h);
        if lines.is_empty() {
            draw_list_text(
                list_rect.x,
                list_line_rect(list_rect, 0, self.hierarchy_scroll),
                canvas,
                "Scene is empty.",
                12.0,
                Color::from_rgba8(148, 162, 180, 255),
            );
        } else {
            for (index, line) in lines.iter().enumerate() {
                let line_rect = list_line_rect(list_rect, index, self.hierarchy_scroll);
                if line_rect.y > list_rect.top() || line_rect.top() < list_rect.y {
                    continue;
                }

                if self.active_scene_tab().selected_node == Some(line.node_id) {
                    canvas.rect(
                        line_rect.x,
                        line_rect.y,
                        (line_rect.w - SCROLLBAR_WIDTH - 4.0).max(0.0),
                        line_rect.h,
                        Color::from_rgba8(44, 82, 122, 220),
                    );
                }

                let toggle_x = line_rect.x + 6.0 + line.depth as f32 * TREE_INDENT;
                let marker = if line.has_children {
                    if line.is_collapsed {
                        "+"
                    } else {
                        "-"
                    }
                } else {
                    "-"
                };
                let marker_color = if line.has_children {
                    Color::from_rgba8(214, 220, 232, 255)
                } else {
                    Color::from_rgba8(120, 130, 142, 255)
                };
                draw_list_text(toggle_x, line_rect, canvas, marker, 12.0, marker_color);
                draw_list_text(
                    toggle_x + 14.0,
                    line_rect,
                    canvas,
                    &line.label,
                    12.0,
                    Color::from_rgba8(214, 220, 232, 255),
                );
            }
        }
        canvas.pop_clip();
        draw_scrollbar(canvas, list_rect, lines.len(), self.hierarchy_scroll);
    }

    fn draw_inspector(
        &self,
        canvas: &mut Canvas,
        panel: PanelRect,
        open: bool,
        tooltip_targets: &mut Vec<CanvasTooltipTarget>,
    ) {
        let toggle_rect = panel_toggle_rect(panel);
        draw_button(
            canvas,
            toggle_rect,
            panel_toggle_label(DockPanelKind::Inspector, open),
            false,
            true,
            tooltip_targets,
        );

        if !open {
            return;
        }

        let inner = panel.inset(PANEL_PADDING);
        draw_fitted_text_left(
            canvas,
            PanelRect::new(
                inner.x,
                inner.top() - 24.0,
                (toggle_rect.x - inner.x - 8.0).max(0.0),
                20.0,
            ),
            "Properties",
            18.0,
            Color::WHITE,
            tooltip_targets,
        );

        canvas.text(
            inner.x,
            inner.top() - 40.0,
            &format!("Scene: {}", self.active_scene_tab().display_name()),
            12.0,
            Color::from_rgba8(214, 220, 232, 255),
        );
        canvas.text(
            inner.x,
            inner.top() - 58.0,
            &format!(
                "Window: {:.0} x {:.0}",
                self.active_scene_tab().scene.view.window_size[0],
                self.active_scene_tab().scene.view.window_size[1]
            ),
            12.0,
            Color::from_rgba8(148, 162, 180, 255),
        );

        let details_top = inner.top() - 94.0;
        if let Some(node_id) = self.active_scene_tab().selected_node {
            if let Some(node) = self.active_scene_tab().scene.node(node_id) {
                let lines = [
                    format!("Selected node {}", node.id),
                    format!(
                        "{}   pos {:.0}, {:.0}   size {:.0} x {:.0}",
                        node.kind.label(),
                        node.position[0],
                        node.position[1],
                        node.size[0],
                        node.size[1]
                    ),
                ];

                for (index, line) in lines.iter().enumerate() {
                    canvas.text(
                        inner.x,
                        details_top - index as f32 * 18.0,
                        line,
                        12.0,
                        if index == 0 {
                            Color::from_rgba8(236, 241, 246, 255)
                        } else {
                            Color::from_rgba8(176, 186, 202, 255)
                        },
                    );
                }
            }
        } else {
            canvas.text(
                inner.x,
                details_top,
                "Scene properties",
                12.0,
                Color::from_rgba8(148, 162, 180, 255),
            );
        }
    }

    fn draw_bottom_panel(
        &mut self,
        canvas: &mut Canvas,
        panel: PanelRect,
        tabs_rect: PanelRect,
        content_rect: PanelRect,
        open: bool,
        tooltip_targets: &mut Vec<CanvasTooltipTarget>,
    ) {
        draw_button(
            canvas,
            panel_toggle_rect(panel),
            panel_toggle_label(DockPanelKind::Bottom, open),
            false,
            true,
            tooltip_targets,
        );

        if !open {
            return;
        }

        for (tab, rect) in self.bottom_tab_buttons(tabs_rect) {
            draw_button(
                canvas,
                rect,
                tab.label(),
                self.bottom_tab == tab,
                true,
                tooltip_targets,
            );
        }

        canvas.push_clip(content_rect.x, content_rect.y, content_rect.w, content_rect.h);

        match self.bottom_tab {
            BottomTab::Activity => {
                for (index, line) in self.activity_log.iter().rev().enumerate() {
                    let line_rect = list_line_rect(content_rect, index, self.bottom_scroll);
                    if line_rect.y > content_rect.top() || line_rect.top() < content_rect.y {
                        continue;
                    }
                    draw_list_text(
                        line_rect.x,
                        line_rect,
                        canvas,
                        line,
                        12.0,
                        Color::from_rgba8(176, 186, 202, 255),
                    );
                }
            }
            BottomTab::SceneJson => {
                let bottom_scroll = self.bottom_scroll;
                let scene_json = self.scene_json_preview_text();
                for (index, line) in scene_json.lines().enumerate() {
                    let line_rect = list_line_rect(content_rect, index, bottom_scroll);
                    if line_rect.y > content_rect.top() || line_rect.top() < content_rect.y {
                        continue;
                    }
                    draw_list_text(
                        line_rect.x,
                        line_rect,
                        canvas,
                        line,
                        12.0,
                        Color::from_rgba8(176, 186, 202, 255),
                    );
                }
            }
        }

        canvas.pop_clip();
        let line_count = match self.bottom_tab {
            BottomTab::Activity => self.activity_log.len(),
            BottomTab::SceneJson => self.scene_json_preview_line_count(),
        };
        draw_scrollbar(canvas, content_rect, line_count, self.bottom_scroll);
    }

    fn draw_viewport(&self, engine: &Engine, canvas: &mut Canvas, viewport: PanelRect) {
        let pan = self.active_scene_tab().viewport_pan;
        canvas.push_clip(viewport.x, viewport.y, viewport.w, viewport.h);
        draw_grid(canvas, viewport, pan);

        for node in self
            .active_scene_tab()
            .scene
            .nodes
            .iter()
            .filter(|node| node.visible)
        {
            if node.kind == SceneNodeKind::Camera2d && node.camera2d.show_bounds {
                let preview_size = if node.camera2d.use_scene_view_size {
                    self.active_scene_tab().scene.view.window_size
                } else {
                    node.camera2d.view_size
                };
                let zoom = node.camera2d.zoom.max(0.1);
                let rect = viewport_node_rect(
                    viewport,
                    node.position,
                    [preview_size[0] / zoom, preview_size[1] / zoom],
                    pan,
                );
                canvas.rect(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    Color::new(0.2, 0.48, 0.52, 0.12),
                );
                draw_outline(
                    canvas,
                    rect,
                    if self.active_scene_tab().selected_node == Some(node.id) {
                        Color::from_rgba8(107, 210, 214, 255)
                    } else {
                        Color::from_rgba8(72, 163, 166, 255)
                    },
                );
            }
        }

        for node in self
            .active_scene_tab()
            .scene
            .nodes
            .iter()
            .filter(|node| node.visible)
        {
            let rect = viewport_node_rect(viewport, node.position, node.size, pan);
            let sprite_texture = if node.kind == SceneNodeKind::Sprite {
                self.sprite_preview_texture(engine, node)
            } else {
                None
            };

            if let Some(texture) = sprite_texture {
                canvas.rect(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    Color::from_rgba8(20, 26, 32, 255),
                );
                canvas.image(texture, rect.x, rect.y, rect.w, rect.h);
            } else {
                canvas.rect(rect.x, rect.y, rect.w, rect.h, node_fill_color(node.kind));
            }

            draw_outline(
                canvas,
                rect,
                if self.active_scene_tab().selected_node == Some(node.id) {
                    Color::from_rgba8(247, 214, 93, 255)
                } else {
                    Color::from_rgba8(36, 44, 52, 255)
                },
            );

            if sprite_texture.is_none() {
                canvas.text_aligned(
                    rect.x + rect.w * 0.5,
                    text_baseline_in_rect(
                        canvas,
                        PanelRect::new(rect.x, rect.y + rect.h * 0.5, rect.w, rect.h * 0.5),
                        12.0,
                    ),
                    node.kind.short_label(),
                    12.0,
                    Color::WHITE,
                    TextAlign::Center,
                );
                canvas.text_aligned(
                    rect.x + rect.w * 0.5,
                    text_baseline_in_rect(
                        canvas,
                        PanelRect::new(rect.x, rect.y, rect.w, rect.h * 0.5),
                        11.0,
                    ),
                    &node.name,
                    11.0,
                    Color::from_rgba8(224, 229, 236, 255),
                    TextAlign::Center,
                );
            } else if self.active_scene_tab().selected_node == Some(node.id) && rect.h >= 18.0 {
                let label_rect = PanelRect::new(rect.x, rect.y, rect.w, 18.0);
                canvas.rect(
                    label_rect.x,
                    label_rect.y,
                    label_rect.w,
                    label_rect.h,
                    Color::new(0.02, 0.03, 0.05, 0.7),
                );
                canvas.text_aligned(
                    label_rect.x + label_rect.w * 0.5,
                    text_baseline_in_rect(canvas, label_rect, 11.0),
                    &node.name,
                    11.0,
                    Color::from_rgba8(238, 242, 246, 255),
                    TextAlign::Center,
                );
            }
        }

        if self.active_scene_tab().scene.nodes.is_empty() {
            canvas.text_aligned(
                viewport.center().x,
                text_baseline_in_rect(
                    canvas,
                    PanelRect::new(viewport.x, viewport.center().y - 16.0, viewport.w, 32.0),
                    20.0,
                ),
                "Empty scene",
                20.0,
                Color::from_rgba8(214, 220, 232, 255),
                TextAlign::Center,
            );
        }

        canvas.pop_clip();
    }

    pub(crate) fn draw_popup_menu(
        &self,
        engine: &Engine,
        canvas: &mut Canvas,
        tooltip_targets: &mut Vec<CanvasTooltipTarget>,
    ) {
        let Some(menu) = self.popup_menu.as_ref() else {
            return;
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
        draw_panel(canvas, rect, Color::from_rgba8(22, 28, 35, 252));
        draw_outline(canvas, rect, Color::from_rgba8(72, 88, 102, 255));

        for (index, (action, label)) in actions.iter().zip(labels.iter()).enumerate() {
            let item_rect = popup_menu_item_rect(rect, index);
            draw_button(
                canvas,
                item_rect,
                label,
                self.popup_action_active(action),
                true,
                tooltip_targets,
            );
        }
    }

    pub(crate) fn draw_canvas_tooltip(
        &mut self,
        canvas: &mut Canvas,
        engine: &Engine,
        tooltip_targets: &[CanvasTooltipTarget],
    ) {
        let mouse = engine.mouse_screen_pos();
        let hovered = tooltip_targets
            .iter()
            .rev()
            .find(|target| target.rect.contains(mouse));

        if let Some(target) = hovered {
            match &mut self.canvas_tooltip_hover {
                Some(state) if state.key == target.key => {
                    state.elapsed += engine.dt();
                }
                Some(state) => {
                    state.key = target.key.clone();
                    state.elapsed = 0.0;
                }
                None => {
                    self.canvas_tooltip_hover = Some(CanvasTooltipHoverState {
                        key: target.key.clone(),
                        elapsed: 0.0,
                    });
                }
            }
        } else {
            self.canvas_tooltip_hover = None;
            return;
        }

        let Some(state) = &self.canvas_tooltip_hover else {
            return;
        };
        if state.elapsed < CANVAS_TOOLTIP_DELAY {
            return;
        }

        let Some(target) = hovered else {
            return;
        };

        let atlas = engine.font_atlas();
        let lines = wrap_text(
            &target.text,
            CANVAS_TOOLTIP_TEXT_SIZE,
            CANVAS_TOOLTIP_MAX_WIDTH - CANVAS_TOOLTIP_PADDING * 2.0,
            atlas,
        );
        let line_height = canvas.line_height(CANVAS_TOOLTIP_TEXT_SIZE);
        let content_width = lines
            .iter()
            .map(|line| canvas.measure_text(line, CANVAS_TOOLTIP_TEXT_SIZE).0)
            .fold(0.0, f32::max);
        let tooltip_width = content_width + CANVAS_TOOLTIP_PADDING * 2.0;
        let tooltip_height = line_height * lines.len() as f32 + CANVAS_TOOLTIP_PADDING * 2.0;
        let (window_width, window_height) = engine.window_size();
        let hw = window_width as f32 * 0.5;
        let hh = window_height as f32 * 0.5;
        let x = (mouse.x + CANVAS_TOOLTIP_OFFSET_X).clamp(-hw + 8.0, hw - tooltip_width - 8.0);
        let y = (mouse.y + CANVAS_TOOLTIP_OFFSET_Y).clamp(-hh + 8.0, hh - tooltip_height - 8.0);
        let tooltip_rect = PanelRect::new(x, y, tooltip_width, tooltip_height);

        canvas.rect(
            tooltip_rect.x,
            tooltip_rect.y,
            tooltip_rect.w,
            tooltip_rect.h,
            Color::from_rgba8(12, 14, 22, 235),
        );
        draw_outline(canvas, tooltip_rect, Color::from_rgba8(58, 72, 88, 255));

        let mut line_y = tooltip_rect.top() - CANVAS_TOOLTIP_PADDING - line_height;
        for line in lines {
            canvas.text(
                tooltip_rect.x + CANVAS_TOOLTIP_PADDING,
                line_y + line_height,
                &line,
                CANVAS_TOOLTIP_TEXT_SIZE,
                Color::from_rgba8(235, 235, 245, 255),
            );
            line_y -= line_height;
        }
    }
}

fn draw_panel(canvas: &mut Canvas, rect: PanelRect, color: Color) {
    canvas.rect(rect.x, rect.y, rect.w, rect.h, color);
}

fn text_baseline_in_rect(canvas: &Canvas, rect: PanelRect, size: f32) -> f32 {
    rect.y + (rect.h + canvas.line_height(size)) * 0.5
}

fn fit_text_to_width(canvas: &Canvas, text: &str, size: f32, max_width: f32) -> (String, bool) {
    if max_width <= 0.0 {
        return (String::new(), !text.is_empty());
    }

    if canvas.measure_text(text, size).0 <= max_width {
        return (text.to_string(), false);
    }

    let ellipsis = "...";
    if canvas.measure_text(ellipsis, size).0 >= max_width {
        return (ellipsis.to_string(), true);
    }

    let mut end = text.len();
    loop {
        let candidate = format!("{}{}", &text[..end], ellipsis);
        if canvas.measure_text(&candidate, size).0 <= max_width {
            return (candidate, true);
        }

        if end == 0 {
            return (ellipsis.to_string(), true);
        }

        end = text[..end]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0);
    }
}

fn draw_fitted_text_left(
    canvas: &mut Canvas,
    rect: PanelRect,
    text: &str,
    size: f32,
    color: Color,
    tooltip_targets: &mut Vec<CanvasTooltipTarget>,
) {
    let (fitted, trimmed) = fit_text_to_width(canvas, text, size, rect.w.max(0.0));
    canvas.push_clip(rect.x, rect.y, rect.w.max(0.0), rect.h.max(0.0));
    canvas.text(
        rect.x,
        text_baseline_in_rect(canvas, rect, size),
        &fitted,
        size,
        color,
    );
    canvas.pop_clip();
    if trimmed {
        tooltip_targets.push(CanvasTooltipTarget::new(rect, text));
    }
}

fn draw_list_text(
    x: f32,
    rect: PanelRect,
    canvas: &mut Canvas,
    text: &str,
    size: f32,
    color: Color,
) {
    canvas.text(
        x,
        text_baseline_in_rect(canvas, rect, size),
        text,
        size,
        color,
    );
}

fn draw_scrollbar(canvas: &mut Canvas, rect: PanelRect, line_count: usize, scroll: f32) {
    let max_scroll = scroll_max_for_lines(line_count, rect);
    if max_scroll <= f32::EPSILON {
        return;
    }

    let content_height = line_count as f32 * LINE_HEIGHT;
    let track_rect = PanelRect::new(rect.right() - SCROLLBAR_WIDTH, rect.y, SCROLLBAR_WIDTH, rect.h);
    let thumb_height = (rect.h * (rect.h / content_height)).clamp(SCROLLBAR_MIN_HEIGHT, rect.h);
    let thumb_travel = (rect.h - thumb_height).max(0.0);
    let thumb_ratio = (scroll / max_scroll).clamp(0.0, 1.0);
    let thumb_top = rect.top() - thumb_travel * thumb_ratio;
    let thumb_rect = PanelRect::new(
        track_rect.x + 1.0,
        thumb_top - thumb_height,
        track_rect.w - 2.0,
        thumb_height,
    );

    canvas.rect(
        track_rect.x,
        track_rect.y,
        track_rect.w,
        track_rect.h,
        Color::from_rgba8(17, 22, 27, 210),
    );
    canvas.rect(
        thumb_rect.x,
        thumb_rect.y,
        thumb_rect.w,
        thumb_rect.h,
        Color::from_rgba8(90, 112, 132, 230),
    );
}

fn draw_button(
    canvas: &mut Canvas,
    rect: PanelRect,
    label: &str,
    active: bool,
    enabled: bool,
    tooltip_targets: &mut Vec<CanvasTooltipTarget>,
) {
    let bg = if !enabled {
        Color::from_rgba8(38, 44, 52, 180)
    } else if active {
        Color::from_rgba8(66, 116, 132, 255)
    } else {
        Color::from_rgba8(36, 44, 54, 240)
    };
    let fg = if enabled {
        Color::from_rgba8(232, 236, 239, 255)
    } else {
        Color::from_rgba8(120, 130, 142, 255)
    };

    canvas.rect(rect.x, rect.y, rect.w, rect.h, bg);
    draw_outline(
        canvas,
        rect,
        if active {
            Color::from_rgba8(120, 190, 204, 255)
        } else {
            Color::from_rgba8(26, 32, 38, 255)
        },
    );
    let (fitted_label, trimmed) = if label.chars().count() <= 1 {
        (label.to_string(), false)
    } else {
        fit_text_to_width(canvas, label, 12.0, (rect.w - 12.0).max(0.0))
    };
    canvas.push_clip(rect.x + 4.0, rect.y, (rect.w - 8.0).max(0.0), rect.h);
    canvas.text_aligned(
        rect.x + rect.w * 0.5,
        text_baseline_in_rect(canvas, rect, 12.0),
        &fitted_label,
        12.0,
        fg,
        TextAlign::Center,
    );
    canvas.pop_clip();
    if trimmed {
        tooltip_targets.push(CanvasTooltipTarget::new(rect, label));
    }
}

fn draw_resize_handle(canvas: &mut Canvas, rect: PanelRect) {
    let color = Color::from_rgba8(62, 78, 92, 220);
    if rect.w >= rect.h {
        canvas.rect(rect.x, rect.y + rect.h * 0.5 - 0.5, rect.w, 1.0, color);
    } else {
        canvas.rect(rect.x + rect.w * 0.5 - 0.5, rect.y, 1.0, rect.h, color);
    }
}

fn draw_outline(canvas: &mut Canvas, rect: PanelRect, color: Color) {
    canvas.line(rect.x, rect.y, rect.x + rect.w, rect.y, 1.0, color);
    canvas.line(
        rect.x + rect.w,
        rect.y,
        rect.x + rect.w,
        rect.y + rect.h,
        1.0,
        color,
    );
    canvas.line(
        rect.x + rect.w,
        rect.y + rect.h,
        rect.x,
        rect.y + rect.h,
        1.0,
        color,
    );
    canvas.line(rect.x, rect.y + rect.h, rect.x, rect.y, 1.0, color);
}

fn draw_grid(canvas: &mut Canvas, viewport: PanelRect, pan: Vec2) {
    let center = viewport.center() + pan;
    let step = 32.0;
    let minor = Color::from_rgba8(28, 34, 42, 255);
    let major = Color::from_rgba8(53, 68, 79, 255);

    let mut x = center.x;
    while x <= viewport.right() {
        canvas.line(x, viewport.y, x, viewport.top(), 1.0, minor);
        x += step;
    }
    let mut x = center.x - step;
    while x >= viewport.x {
        canvas.line(x, viewport.y, x, viewport.top(), 1.0, minor);
        x -= step;
    }

    let mut y = center.y;
    while y <= viewport.top() {
        canvas.line(viewport.x, y, viewport.right(), y, 1.0, minor);
        y += step;
    }
    let mut y = center.y - step;
    while y >= viewport.y {
        canvas.line(viewport.x, y, viewport.right(), y, 1.0, minor);
        y -= step;
    }

    canvas.line(center.x, viewport.y, center.x, viewport.top(), 1.0, major);
    canvas.line(viewport.x, center.y, viewport.right(), center.y, 1.0, major);
}

fn viewport_node_rect(
    viewport: PanelRect,
    position: [f32; 2],
    size: [f32; 2],
    pan: Vec2,
) -> PanelRect {
    let center = scene_to_screen(position, viewport, pan);
    PanelRect::new(
        center.x - size[0] * 0.5,
        center.y - size[1] * 0.5,
        size[0],
        size[1],
    )
}

pub(crate) fn scene_to_screen(position: [f32; 2], viewport: PanelRect, pan: Vec2) -> Vec2 {
    Vec2::new(
        viewport.x + viewport.w * 0.5 + pan.x + position[0],
        viewport.y + viewport.h * 0.5 + pan.y + position[1],
    )
}

pub(crate) fn screen_to_scene(position: Vec2, viewport: PanelRect, pan: Vec2) -> [f32; 2] {
    [
        position.x - (viewport.x + viewport.w * 0.5 + pan.x),
        position.y - (viewport.y + viewport.h * 0.5 + pan.y),
    ]
}

fn node_fill_color(kind: SceneNodeKind) -> Color {
    match kind {
        SceneNodeKind::Group => Color::from_rgba8(67, 79, 89, 255),
        SceneNodeKind::Empty => Color::from_rgba8(92, 103, 112, 255),
        SceneNodeKind::Camera2d => Color::from_rgba8(53, 125, 132, 255),
        SceneNodeKind::Sprite => Color::from_rgba8(64, 114, 176, 255),
        SceneNodeKind::Trigger => Color::from_rgba8(176, 125, 58, 255),
        SceneNodeKind::UiRoot => Color::from_rgba8(70, 142, 104, 255),
    }
}