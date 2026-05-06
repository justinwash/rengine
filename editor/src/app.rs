use crate::scene::{SceneDocument, SceneNode, SceneNodeKind, SceneNodeReorderDirection};
use rengine::*;
use rfd::FileDialog;
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const MAX_ACTIVITY_LOG_LINES: usize = 96;
const TOP_BAR_HEIGHT: f32 = 58.0;
const BOTTOM_PANEL_HEIGHT: f32 = 188.0;
const FILES_PANEL_WIDTH: f32 = 268.0;
const HIERARCHY_PANEL_WIDTH: f32 = 272.0;
const INSPECTOR_PANEL_WIDTH: f32 = 316.0;
const PANEL_COLLAPSED_WIDTH: f32 = 32.0;
const PANEL_COLLAPSED_HEIGHT: f32 = 32.0;
const PANEL_GAP: f32 = 10.0;
const PANEL_PADDING: f32 = 12.0;
const PANEL_RESIZE_HANDLE_SIZE: f32 = 8.0;
const PANEL_TOGGLE_BUTTON_SIZE: f32 = 22.0;
const BUTTON_GAP: f32 = 8.0;
const BUTTON_HEIGHT: f32 = 28.0;
const SIDE_PANEL_COLLAPSED_BUTTON_HEIGHT: f32 = 28.0;
const TAB_HEIGHT: f32 = 34.0;
const LINE_HEIGHT: f32 = 20.0;
const TREE_INDENT: f32 = 14.0;
const PROJECT_BROWSER_CONTROLS_HEIGHT: f32 = 92.0;
const POPUP_MENU_MIN_WIDTH: f32 = 176.0;
const POPUP_MENU_ITEM_HEIGHT: f32 = 26.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const SCROLLBAR_MIN_HEIGHT: f32 = 28.0;
const MIN_FILES_PANEL_WIDTH: f32 = 180.0;
const MIN_HIERARCHY_PANEL_WIDTH: f32 = 188.0;
const MIN_INSPECTOR_PANEL_WIDTH: f32 = 220.0;
const MIN_BOTTOM_PANEL_HEIGHT: f32 = 124.0;
const MIN_CENTER_PANEL_WIDTH: f32 = 260.0;
const MIN_CENTER_PANEL_HEIGHT: f32 = 180.0;
const TOP_BAR_BUTTON_MIN_WIDTH: f32 = 72.0;
const SCENE_TAB_BUTTON_MIN_WIDTH: f32 = 104.0;
const BOTTOM_TAB_BUTTON_MIN_WIDTH: f32 = 112.0;
const CANVAS_TOOLTIP_DELAY: f32 = 0.35;
const CANVAS_TOOLTIP_TEXT_SIZE: f32 = 12.0;
const CANVAS_TOOLTIP_PADDING: f32 = 8.0;
const CANVAS_TOOLTIP_MAX_WIDTH: f32 = 280.0;
const CANVAS_TOOLTIP_OFFSET_X: f32 = 18.0;
const CANVAS_TOOLTIP_OFFSET_Y: f32 = 18.0;
const PROJECT_DOUBLE_CLICK_DELAY: f32 = 0.35;
const SCENE_AUTOSAVE_INTERVAL_SECONDS: f32 = 5.0;

const FILE_FILTER_INPUT_ID: usize = 10;

const INSPECTOR_SCENE_NAME_ID: usize = 100;
const INSPECTOR_SCENE_WIDTH_ID: usize = 101;
const INSPECTOR_SCENE_HEIGHT_ID: usize = 102;
const INSPECTOR_NODE_NAME_ID: usize = 110;
const INSPECTOR_NODE_VISIBLE_ID: usize = 111;
const INSPECTOR_NODE_SCRIPT_ID: usize = 112;
const INSPECTOR_NODE_PREFAB_ID: usize = 113;
const INSPECTOR_NODE_POSITION_X_ID: usize = 114;
const INSPECTOR_NODE_POSITION_Y_ID: usize = 115;
const INSPECTOR_NODE_SIZE_WIDTH_ID: usize = 116;
const INSPECTOR_NODE_SIZE_HEIGHT_ID: usize = 117;
const INSPECTOR_NODE_KIND_BUTTON_ID: usize = 118;
const INSPECTOR_CREATE_CHILD_BUTTON_ID: usize = 119;
const INSPECTOR_SPRITE_ALIAS_ID: usize = 120;
const INSPECTOR_SPRITE_TEXTURE_ID: usize = 121;
const INSPECTOR_SPRITE_ASSIGN_SELECTED_ID: usize = 122;
const INSPECTOR_SPRITE_CLEAR_TEXTURE_ID: usize = 123;
const INSPECTOR_SPRITE_BROWSE_IMAGE_ID: usize = 124;
const INSPECTOR_NODE_SCRIPT_PARAM_KEY_ID: usize = 125;
const INSPECTOR_NODE_GEOMETRY_POINTS_ID: usize = 126;
const INSPECTOR_NODE_PATH_POINTS_ID: usize = 127;
const INSPECTOR_TRIGGER_TAG_ID: usize = 128;
const INSPECTOR_NODE_SCRIPT_PARAM_VALUE_ID: usize = 129;
const INSPECTOR_CAMERA_ZOOM_ID: usize = 130;
const INSPECTOR_CAMERA_SHOW_BOUNDS_ID: usize = 131;
const INSPECTOR_CAMERA_USE_SCENE_SIZE_ID: usize = 132;
const INSPECTOR_CAMERA_VIEW_WIDTH_ID: usize = 133;
const INSPECTOR_CAMERA_VIEW_HEIGHT_ID: usize = 134;
const INSPECTOR_NODE_SCRIPT_PARAM_SET_ID: usize = 135;
const INSPECTOR_TRIGGER_ONCE_ID: usize = 136;
const INSPECTOR_TRIGGER_COOLDOWN_ID: usize = 137;
const INSPECTOR_TRIGGER_LAYER_MASK_ID: usize = 138;
const INSPECTOR_SCROLL_REGION_ID: usize = 140;

const NODE_KIND_OPTIONS: [SceneNodeKind; 8] = [
    SceneNodeKind::Group,
    SceneNodeKind::Empty,
    SceneNodeKind::Camera2d,
    SceneNodeKind::Sprite,
    SceneNodeKind::Polygon,
    SceneNodeKind::Path,
    SceneNodeKind::Trigger,
    SceneNodeKind::UiRoot,
];

mod drawing;
mod filesystem;
mod forms;
mod popup;
mod state;
mod windowing;

pub(crate) use drawing::*;
pub(crate) use filesystem::*;
pub(crate) use forms::*;
pub(crate) use popup::*;
pub(crate) use state::*;
pub(crate) use windowing::*;

pub struct RengineNativeEditor {
    workspace_root: PathBuf,
    branch_name: String,
    project_tree: ProjectTreeEntry,
    scene_tabs: Vec<SceneTab>,
    active_scene_tab: usize,
    selected_project_path: Option<PathBuf>,
    collapsed_project_paths: HashSet<PathBuf>,
    activity_log: Vec<String>,
    bottom_tab: BottomTab,
    project_scroll: f32,
    hierarchy_scroll: f32,
    bottom_scroll: f32,
    file_browser_ui: Ui,
    file_browser_form: FileBrowserFormState,
    inspector_ui: Ui,
    inspector_form: InspectorFormState,
    panel_layout: PanelLayoutState,
    panel_resize_drag: Option<PanelResizeDrag>,
    inspector_scroll: f32,
    canvas_tooltip_hover: Option<CanvasTooltipHoverState>,
    canvas_tooltip_targets: Vec<CanvasTooltipTarget>,
    recent_project_click: Option<ProjectEntryClickState>,
    active_text_input_owner: Option<TextInputOwner>,
    automation_log_path: Option<PathBuf>,
    popup_menu: Option<PopupMenuState>,
    /// True when the last left-mouse-down was in the viewport panel.
    viewport_focused: bool,
    quit_requested: bool,
}

impl Game for RengineNativeEditor {
    fn new(_engine: &mut Engine) -> Self {
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project_tree = ProjectTreeEntry::scan(&workspace_root);
        let branch_name = read_git_branch(&workspace_root);
        let automation_log_path =
            std::env::var_os("RENGINE_EDITOR_AUTOMATION_LOG").map(PathBuf::from);

        if let Some(path) = automation_log_path.as_ref() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, "");
        }

        let mut editor = Self {
            workspace_root,
            branch_name,
            project_tree,
            scene_tabs: vec![SceneTab::untitled()],
            active_scene_tab: 0,
            selected_project_path: None,
            collapsed_project_paths: HashSet::new(),
            activity_log: Vec::new(),
            bottom_tab: BottomTab::Activity,
            project_scroll: 0.0,
            hierarchy_scroll: 0.0,
            bottom_scroll: 0.0,
            file_browser_ui: make_file_browser_ui(),
            file_browser_form: FileBrowserFormState::default(),
            inspector_ui: make_inspector_ui(),
            inspector_form: InspectorFormState::default(),
            panel_layout: PanelLayoutState::default(),
            panel_resize_drag: None,
            inspector_scroll: 0.0,
            canvas_tooltip_hover: None,
            canvas_tooltip_targets: Vec::new(),
            recent_project_click: None,
            active_text_input_owner: None,
            automation_log_path,
            popup_menu: None,
            viewport_focused: false,
            quit_requested: false,
        };

        editor.refresh_inspector_form();

        editor.push_log("Booted rengine-native editor shell");
        editor.push_log(format!(
            "Opened workspace {}",
            editor.display_path(&editor.workspace_root)
        ));
        editor.push_log("Started new empty scene");
        editor
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.update_recent_project_click(engine.dt());
        self.clamp_panel_layout(engine);
        let layout = ShellLayout::new(engine, &self.panel_layout);
        self.handle_context_clicks(engine, &layout);
        self.handle_clicks(engine, &layout);
        self.handle_viewport_pan_press(engine, &layout);
        self.update_panel_resize(engine);

        self.clamp_panel_layout(engine);
        let layout = ShellLayout::new(engine, &self.panel_layout);
        self.update_scrolls(engine, &layout);
        self.request_sprite_previews(engine);
        self.update_viewport_drag(engine, &layout);
        self.update_file_browser_ui(engine, &layout);
        self.update_inspector_ui(engine, &layout);
        self.update_scene_autosave(engine.dt());
        self.handle_scene_history_shortcuts(engine);
        self.handle_scene_selection_shortcuts(engine);
        let key_f = engine.input().is_key_pressed(KeyCode::KeyF);
        let key_w = engine.input().is_key_pressed(KeyCode::KeyW);
        let key_e = engine.input().is_key_pressed(KeyCode::KeyE);
        let key_r = engine.input().is_key_pressed(KeyCode::KeyR);
        let key_f5 = engine.input().is_key_pressed(KeyCode::F5);
        let key_n = engine.input().is_key_pressed(KeyCode::KeyN);
        let key_o = engine.input().is_key_pressed(KeyCode::KeyO);
        let key_s = engine.input().is_key_pressed(KeyCode::KeyS);

        if !self.keyboard_captured_by_text_input() && key_f {
            self.frame_active_scene_view();
        }

        if key_w && self.keyboard_captured_by_text_input() {
            self.log_automation_event("shortcut_blocked:KeyW:text_input");
        }
        if !self.keyboard_captured_by_text_input() && self.viewport_focused && key_w {
            self.active_scene_tab_mut().gizmo_mode = GizmoMode::Translate;
            self.log_automation_event("shortcut:gizmo:translate");
        }
        if key_e && self.keyboard_captured_by_text_input() {
            self.log_automation_event("shortcut_blocked:KeyE:text_input");
        }
        if !self.keyboard_captured_by_text_input() && self.viewport_focused && key_e {
            self.active_scene_tab_mut().gizmo_mode = GizmoMode::Rotate;
            self.log_automation_event("shortcut:gizmo:rotate");
        }
        if key_r && self.keyboard_captured_by_text_input() {
            self.log_automation_event("shortcut_blocked:KeyR:text_input");
        }
        if !self.keyboard_captured_by_text_input() && self.viewport_focused && key_r {
            self.active_scene_tab_mut().gizmo_mode = GizmoMode::Scale;
            self.log_automation_event("shortcut:gizmo:scale");
        }

        if !self.keyboard_captured_by_text_input() && key_f5 {
            self.refresh_project_tree();
        }
        if !self.keyboard_captured_by_text_input() && key_n {
            self.new_scene();
        }
        if !self.keyboard_captured_by_text_input() && key_o {
            self.open_scene();
        }
        if !self.keyboard_captured_by_text_input() && key_s {
            self.save_scene();
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        self.draw_shell(engine, frame);
        let canvas = frame.canvas(0);
        let layout = ShellLayout::new(engine, &self.panel_layout);
        if layout.files_open {
            canvas.push_clip(
                layout.files.x,
                layout.files.y,
                layout.files.w,
                layout.files.h,
            );
            self.file_browser_ui.render(canvas, engine);
            canvas.pop_clip();
        }
        if layout.inspector_open {
            canvas.push_clip(
                layout.inspector.x,
                layout.inspector.y,
                layout.inspector.w,
                layout.inspector.h,
            );
            self.inspector_ui.render(canvas, engine);
            canvas.pop_clip();
        }
        let mut tooltip_targets = std::mem::take(&mut self.canvas_tooltip_targets);
        self.draw_popup_menu(engine, canvas, &mut tooltip_targets);
        self.draw_canvas_tooltip(canvas, engine, &tooltip_targets);
        self.canvas_tooltip_targets = tooltip_targets;
    }

    fn should_exit(&self) -> bool {
        self.quit_requested
    }
}
