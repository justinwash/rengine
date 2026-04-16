use crate::assets::Color;
use crate::canvas::{Canvas, TextAlign};
use crate::input::InputState;
use crate::text::FontAtlas;
use winit::keyboard::KeyCode;

#[derive(Clone)]
pub struct UiStyle {
    pub text_color: Color,
    pub text_size: f32,
    pub button_bg: Color,
    pub button_focused_bg: Color,
    pub button_pressed_bg: Color,
    pub button_text_color: Color,
    pub button_focused_text_color: Color,
    pub button_padding: f32,
    pub spacing: f32,
    pub panel_bg: Color,
    pub panel_padding: f32,
    pub progress_bg: Color,
    pub progress_fill: Color,
    pub progress_height: f32,
    pub checkbox_size: f32,
    pub checkbox_bg: Color,
    pub checkbox_checked_bg: Color,
    pub slider_track_color: Color,
    pub slider_fill_color: Color,
    pub slider_thumb_color: Color,
    pub slider_height: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            text_color: Color::WHITE,
            text_size: 16.0,
            button_bg: Color::from_rgba8(60, 60, 80, 200),
            button_focused_bg: Color::from_rgba8(80, 100, 180, 240),
            button_pressed_bg: Color::from_rgba8(120, 140, 220, 255),
            button_text_color: Color::from_rgba8(200, 200, 200, 255),
            button_focused_text_color: Color::WHITE,
            button_padding: 8.0,
            spacing: 4.0,
            panel_bg: Color::from_rgba8(20, 20, 35, 200),
            panel_padding: 12.0,
            progress_bg: Color::from_rgba8(40, 40, 55, 200),
            progress_fill: Color::from_rgba8(80, 160, 80, 255),
            progress_height: 20.0,
            checkbox_size: 16.0,
            checkbox_bg: Color::from_rgba8(50, 50, 70, 220),
            checkbox_checked_bg: Color::from_rgba8(80, 140, 200, 255),
            slider_track_color: Color::from_rgba8(50, 50, 70, 220),
            slider_fill_color: Color::from_rgba8(80, 140, 200, 255),
            slider_thumb_color: Color::WHITE,
            slider_height: 16.0,
        }
    }
}

enum Widget {
    Label {
        text: String,
        size: f32,
        color: Color,
        align: TextAlign,
    },
    Button {
        id: usize,
        text: String,
    },
    Separator {
        height: f32,
    },
    Panel {
        color: Color,
        padding: f32,
        children: usize,
    },
    Row {
        spacing: f32,
        children: usize,
    },
    Grid {
        columns: usize,
        spacing: f32,
        children: usize,
    },
    ProgressBar {
        label: String,
        value: f32,
        color: Option<Color>,
    },
    Checkbox {
        id: usize,
        label: String,
        checked: bool,
    },
    Slider {
        id: usize,
        label: String,
        value: f32,
        min: f32,
        max: f32,
    },
    ScrollRegion {
        id: usize,
        height: f32,
        scroll_offset: f32,
        children: usize,
    },
}

pub struct UiResponse {
    pub focused: Option<usize>,
    pub activated: Option<usize>,
    pub hovered: Option<usize>,
    pub toggled: Vec<usize>,
    pub changed_values: Vec<(usize, f32)>,
    pub dragging: Option<usize>,
    pub scroll_offsets: Vec<(usize, f32)>,
}

impl UiResponse {
    pub fn was_activated(&self, id: usize) -> bool {
        self.activated == Some(id)
    }

    pub fn was_toggled(&self, id: usize) -> bool {
        self.toggled.contains(&id)
    }

    pub fn value_for(&self, id: usize) -> Option<f32> {
        self.changed_values
            .iter()
            .find(|(cid, _)| *cid == id)
            .map(|(_, v)| *v)
    }

    pub fn scroll_for(&self, id: usize) -> Option<f32> {
        self.scroll_offsets
            .iter()
            .find(|(sid, _)| *sid == id)
            .map(|(_, v)| *v)
    }
}

pub struct Ui {
    x: f32,
    y: f32,
    width: f32,
    style: UiStyle,
    widgets: Vec<Widget>,
    focusable_ids: Vec<usize>,
    focus_index: usize,
    activated: Option<usize>,
    mouse_focus: bool,
    dragging_slider: Option<usize>,
}

impl Ui {
    pub fn new(x: f32, y: f32, width: f32, _screen_size: (u32, u32)) -> Self {
        Self {
            x,
            y,
            width,
            style: UiStyle::default(),
            widgets: Vec::new(),
            focusable_ids: Vec::new(),
            focus_index: 0,
            activated: None,
            mouse_focus: false,
            dragging_slider: None,
        }
    }

    pub fn with_style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_focus(mut self, focus: usize) -> Self {
        self.focus_index = focus;
        self
    }

    pub fn with_dragging(mut self, dragging: Option<usize>) -> Self {
        self.dragging_slider = dragging;
        self
    }

    pub fn label(&mut self, text: &str, size: f32, color: Color) {
        self.widgets.push(Widget::Label {
            text: text.to_string(),
            size,
            color,
            align: TextAlign::Left,
        });
    }

    pub fn label_centered(&mut self, text: &str, size: f32, color: Color) {
        self.widgets.push(Widget::Label {
            text: text.to_string(),
            size,
            color,
            align: TextAlign::Center,
        });
    }

    pub fn button(&mut self, id: usize, text: &str) {
        self.focusable_ids.push(id);
        self.widgets.push(Widget::Button {
            id,
            text: text.to_string(),
        });
    }

    pub fn separator(&mut self, height: f32) {
        self.widgets.push(Widget::Separator { height });
    }

    pub fn panel(&mut self, children: usize) {
        self.panel_colored(self.style.panel_bg, self.style.panel_padding, children);
    }

    pub fn panel_colored(&mut self, color: Color, padding: f32, children: usize) {
        self.widgets.push(Widget::Panel {
            color,
            padding,
            children,
        });
    }

    pub fn progress_bar(&mut self, label: &str, value: f32) {
        self.widgets.push(Widget::ProgressBar {
            label: label.to_string(),
            value: value.clamp(0.0, 1.0),
            color: None,
        });
    }

    pub fn progress_bar_colored(&mut self, label: &str, value: f32, fill_color: Color) {
        self.widgets.push(Widget::ProgressBar {
            label: label.to_string(),
            value: value.clamp(0.0, 1.0),
            color: Some(fill_color),
        });
    }

    pub fn checkbox(&mut self, id: usize, label: &str, checked: bool) {
        self.focusable_ids.push(id);
        self.widgets.push(Widget::Checkbox {
            id,
            label: label.to_string(),
            checked,
        });
    }

    pub fn slider(&mut self, id: usize, label: &str, value: f32, min: f32, max: f32) {
        self.focusable_ids.push(id);
        self.widgets.push(Widget::Slider {
            id,
            label: label.to_string(),
            value: value.clamp(min, max),
            min,
            max,
        });
    }

    pub fn row(&mut self, children: usize) {
        self.widgets.push(Widget::Row {
            spacing: self.style.spacing,
            children,
        });
    }

    pub fn row_spaced(&mut self, spacing: f32, children: usize) {
        self.widgets.push(Widget::Row { spacing, children });
    }

    pub fn grid(&mut self, columns: usize, children: usize) {
        self.widgets.push(Widget::Grid {
            columns: columns.max(1),
            spacing: self.style.spacing,
            children,
        });
    }

    pub fn grid_spaced(&mut self, columns: usize, spacing: f32, children: usize) {
        self.widgets.push(Widget::Grid {
            columns: columns.max(1),
            spacing,
            children,
        });
    }

    pub fn scroll(&mut self, id: usize, height: f32, scroll_offset: f32, children: usize) {
        self.widgets.push(Widget::ScrollRegion {
            id,
            height,
            scroll_offset,
            children,
        });
    }

    fn compute_widget_height(
        &self,
        widget: &Widget,
        remaining: &[Widget],
        atlas: &FontAtlas,
    ) -> f32 {
        match widget {
            Widget::Label { size, .. } => atlas.line_height(*size) + self.style.spacing,
            Widget::Button { .. } => {
                let lh = atlas.line_height(self.style.text_size);
                lh + self.style.button_padding * 2.0 + self.style.spacing
            }
            Widget::Separator { height } => *height,
            Widget::Panel {
                padding, children, ..
            } => {
                let mut h = padding * 2.0;
                let n = (*children).min(remaining.len());
                let child_slice = &remaining[..n];
                let mut i = 0;
                while i < child_slice.len() {
                    h += self.compute_widget_height(&child_slice[i], &child_slice[i + 1..], atlas);
                    i += 1;
                }
                h + self.style.spacing
            }
            Widget::ProgressBar { .. } => {
                let lh = atlas.line_height(self.style.text_size);
                lh + self.style.progress_height + self.style.spacing * 2.0
            }
            Widget::Checkbox { .. } => {
                let lh = atlas.line_height(self.style.text_size);
                lh.max(self.style.checkbox_size) + self.style.spacing
            }
            Widget::Slider { label, .. } => {
                let mut h = self.style.slider_height + self.style.spacing;
                if !label.is_empty() {
                    h += atlas.line_height(self.style.text_size) + self.style.spacing;
                }
                h
            }
            Widget::Row { children, .. } => {
                let n = (*children).min(remaining.len());
                let child_slice = &remaining[..n];
                let mut max_h: f32 = 0.0;
                let mut ci = 0;
                while ci < child_slice.len() {
                    let ch =
                        self.compute_widget_height(&child_slice[ci], &child_slice[ci + 1..], atlas);
                    max_h = max_h.max(ch);
                    ci += 1;
                }
                max_h + self.style.spacing
            }
            Widget::Grid {
                columns, children, ..
            } => {
                let n = (*children).min(remaining.len());
                let child_slice = &remaining[..n];
                let cols = (*columns).max(1);
                let mut total_h: f32 = 0.0;
                let mut row_max: f32 = 0.0;
                let mut ci = 0;
                while ci < child_slice.len() {
                    let ch =
                        self.compute_widget_height(&child_slice[ci], &child_slice[ci + 1..], atlas);
                    row_max = row_max.max(ch);
                    if (ci + 1) % cols == 0 || ci + 1 == child_slice.len() {
                        total_h += row_max;
                        row_max = 0.0;
                    }
                    ci += 1;
                }
                total_h + self.style.spacing
            }
            Widget::ScrollRegion { height, .. } => *height + self.style.spacing,
        }
    }

    fn compute_focusable_rects(&self, atlas: &FontAtlas) -> Vec<(usize, f32, f32, f32, f32)> {
        let mut rects = Vec::new();
        let mut cursor_y = self.y;
        let mut base_x = self.x;
        let mut current_width = self.width;

        struct Container {
            kind: u8,
            saved_x: f32,
            saved_width: f32,
            remaining: usize,
            padding: f32,
            col_index: usize,
            col_count: usize,
            col_width: f32,
            col_spacing: f32,
            row_start_y: f32,
            row_max_h: f32,
            scroll_clip_top: f32,
            scroll_clip_bottom: f32,
        }
        let mut stack: Vec<Container> = Vec::new();

        let mut i = 0;
        while i < self.widgets.len() {
            let mut pending: Option<Container> = None;

            match &self.widgets[i] {
                Widget::Label { size, .. } => {
                    cursor_y -= atlas.line_height(*size) + self.style.spacing;
                }
                Widget::Button { id, .. } => {
                    let lh = atlas.line_height(self.style.text_size);
                    let pad = self.style.button_padding;
                    let btn_h = lh + pad * 2.0;
                    let btn_y = cursor_y - btn_h + pad;
                    rects.push((*id, base_x, btn_y, current_width, btn_h));
                    cursor_y -= btn_h + self.style.spacing;
                }
                Widget::Separator { height } => {
                    cursor_y -= *height;
                }
                Widget::Panel {
                    padding, children, ..
                } => {
                    pending = Some(Container {
                        kind: 0,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: *padding,
                        col_index: 0,
                        col_count: 0,
                        col_width: 0.0,
                        col_spacing: 0.0,
                        row_start_y: 0.0,
                        row_max_h: 0.0,
                        scroll_clip_top: 0.0,
                        scroll_clip_bottom: 0.0,
                    });
                }
                Widget::Row { spacing, children } => {
                    let cols = (*children).max(1);
                    let total_gap = *spacing * (cols as f32 - 1.0).max(0.0);
                    let cw = (current_width - total_gap) / cols as f32;
                    pending = Some(Container {
                        kind: 1,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: 0.0,
                        col_index: 0,
                        col_count: cols,
                        col_width: cw,
                        col_spacing: *spacing,
                        row_start_y: cursor_y,
                        row_max_h: 0.0,
                        scroll_clip_top: 0.0,
                        scroll_clip_bottom: 0.0,
                    });
                }
                Widget::Grid {
                    columns,
                    spacing,
                    children,
                } => {
                    let cols = (*columns).max(1);
                    let total_gap = *spacing * (cols as f32 - 1.0).max(0.0);
                    let cw = (current_width - total_gap) / cols as f32;
                    pending = Some(Container {
                        kind: 2,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: 0.0,
                        col_index: 0,
                        col_count: cols,
                        col_width: cw,
                        col_spacing: *spacing,
                        row_start_y: cursor_y,
                        row_max_h: 0.0,
                        scroll_clip_top: 0.0,
                        scroll_clip_bottom: 0.0,
                    });
                }
                Widget::ProgressBar { .. } => {
                    let lh = atlas.line_height(self.style.text_size);
                    cursor_y -= lh + self.style.progress_height + self.style.spacing * 2.0;
                }
                Widget::Checkbox { id, .. } => {
                    let lh = atlas.line_height(self.style.text_size);
                    let row_h = lh.max(self.style.checkbox_size);
                    rects.push((*id, base_x, cursor_y - row_h, current_width, row_h));
                    cursor_y -= row_h + self.style.spacing;
                }
                Widget::Slider { id, label, .. } => {
                    let h = self.style.slider_height;
                    if !label.is_empty() {
                        let lh = atlas.line_height(self.style.text_size);
                        cursor_y -= lh + self.style.spacing;
                    }
                    rects.push((*id, base_x, cursor_y - h, current_width, h));
                    cursor_y -= h + self.style.spacing;
                }
                Widget::ScrollRegion {
                    id: _,
                    height,
                    scroll_offset,
                    children,
                } => {
                    let clip_top = cursor_y;
                    let clip_bottom = cursor_y - *height;
                    pending = Some(Container {
                        kind: 3,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: 0.0,
                        col_index: 0,
                        col_count: 0,
                        col_width: 0.0,
                        col_spacing: 0.0,
                        row_start_y: 0.0,
                        row_max_h: 0.0,
                        scroll_clip_top: clip_top,
                        scroll_clip_bottom: clip_bottom,
                    });
                    cursor_y += *scroll_offset;
                }
            }

            let mut pop_idx = None;
            for (si, cont) in stack.iter_mut().enumerate().rev() {
                if cont.remaining > 0 {
                    cont.remaining -= 1;

                    if cont.kind == 1 || cont.kind == 2 {
                        let used_h = cont.row_start_y - cursor_y;
                        if used_h > cont.row_max_h {
                            cont.row_max_h = used_h;
                        }
                        cont.col_index += 1;

                        let end_of_row = cont.col_index >= cont.col_count;
                        let last_child = cont.remaining == 0;

                        if end_of_row || last_child {
                            cursor_y = cont.row_start_y - cont.row_max_h;
                            cont.row_max_h = 0.0;
                            cont.col_index = 0;
                            cont.row_start_y = cursor_y;
                            base_x = cont.saved_x;
                            current_width = cont.col_width;
                        } else {
                            cursor_y = cont.row_start_y;
                            base_x += cont.col_width + cont.col_spacing;
                        }
                    }

                    if cont.remaining == 0 {
                        pop_idx = Some(si);
                    }
                    break;
                }
            }
            if let Some(si) = pop_idx {
                let cont = stack.remove(si);
                match cont.kind {
                    0 => {
                        cursor_y -= cont.padding;
                        base_x = cont.saved_x;
                        current_width = cont.saved_width;
                        cursor_y -= self.style.spacing;
                    }
                    3 => {
                        cursor_y = cont.scroll_clip_bottom;
                        base_x = cont.saved_x;
                        current_width = cont.saved_width;
                        let clip_top = cont.scroll_clip_top;
                        let clip_bot = cont.scroll_clip_bottom;
                        let start = cont.col_index;
                        for r in &mut rects[start..] {
                            let top = r.2 + r.4;
                            let bot = r.2;
                            if top <= clip_bot || bot >= clip_top {
                                r.3 = 0.0;
                                r.4 = 0.0;
                            }
                        }
                        cursor_y -= self.style.spacing;
                    }
                    _ => {
                        base_x = cont.saved_x;
                        current_width = cont.saved_width;
                        cursor_y -= self.style.spacing;
                    }
                }
            }

            if let Some(mut cont) = pending {
                match cont.kind {
                    0 => {
                        cursor_y -= cont.padding;
                        cont.saved_x = base_x;
                        cont.saved_width = current_width;
                        let pad = cont.padding;
                        let old_x = base_x;
                        let old_w = current_width;
                        base_x += pad;
                        current_width -= pad * 2.0;
                        cont.saved_x = old_x;
                        cont.saved_width = old_w;
                        stack.push(cont);
                    }
                    3 => {
                        cont.col_index = rects.len();
                        stack.push(cont);
                    }
                    _ => {
                        cont.row_start_y = cursor_y;
                        base_x = cont.saved_x;
                        current_width = cont.col_width;
                        stack.push(cont);
                    }
                }
            }

            i += 1;
        }
        rects
    }

    pub fn update(&mut self, input: &InputState, atlas: &FontAtlas) -> UiResponse {
        let mut toggled = Vec::new();
        let mut changed_values = Vec::new();
        let mut hovered = None;

        if self.focusable_ids.is_empty() {
            return UiResponse {
                focused: None,
                activated: None,
                hovered: None,
                toggled,
                changed_values,
                dragging: None,
                scroll_offsets: Vec::new(),
            };
        }

        let count = self.focusable_ids.len();
        if self.focus_index >= count {
            self.focus_index = 0;
        }

        let rects = self.compute_focusable_rects(atlas);
        let (mx, my) = input.mouse_position();

        for (rect_idx, &(_, rx, ry, rw, rh)) in rects.iter().enumerate() {
            if mx >= rx && mx <= rx + rw && my >= ry && my <= ry + rh {
                hovered = Some(self.focusable_ids[rect_idx]);
                self.focus_index = rect_idx;
                self.mouse_focus = true;
                break;
            }
        }

        if input.is_key_pressed(KeyCode::ArrowUp) || input.is_key_pressed(KeyCode::KeyW) {
            self.mouse_focus = false;
            if self.focus_index == 0 {
                self.focus_index = count - 1;
            } else {
                self.focus_index -= 1;
            }
        }
        if input.is_key_pressed(KeyCode::ArrowDown) || input.is_key_pressed(KeyCode::KeyS) {
            self.mouse_focus = false;
            self.focus_index = (self.focus_index + 1) % count;
        }

        let focused_id = self.focusable_ids[self.focus_index];

        let keyboard_activate =
            input.is_key_pressed(KeyCode::Enter) || input.is_key_pressed(KeyCode::Space);
        let mouse_activate = input.is_mouse_pressed(0) && hovered.is_some();

        if keyboard_activate || mouse_activate {
            for widget in &self.widgets {
                match widget {
                    Widget::Checkbox { id, checked, .. } if *id == focused_id => {
                        toggled.push(*id);
                        let _ = checked;
                    }
                    Widget::Button { id, .. } if *id == focused_id => {
                        self.activated = Some(*id);
                    }
                    _ => {}
                }
            }
        }

        if input.is_key_pressed(KeyCode::ArrowLeft) || input.is_key_pressed(KeyCode::ArrowRight) {
            let delta = if input.is_key_pressed(KeyCode::ArrowRight) {
                0.05
            } else {
                -0.05
            };
            for widget in &self.widgets {
                if let Widget::Slider {
                    id,
                    value,
                    min,
                    max,
                    ..
                } = widget
                {
                    if *id == focused_id {
                        let range = max - min;
                        let new_val = (value + delta * range).clamp(*min, *max);
                        changed_values.push((*id, new_val));
                    }
                }
            }
        }

        if !input.is_mouse_down(0) {
            self.dragging_slider = None;
        }

        if input.is_mouse_pressed(0) {
            for (rect_idx, &(_, rx, ry, rw, rh)) in rects.iter().enumerate() {
                let wid = self.focusable_ids[rect_idx];
                if mx >= rx && mx <= rx + rw && my >= ry && my <= ry + rh {
                    for widget in &self.widgets {
                        if let Widget::Slider { id, .. } = widget {
                            if *id == wid {
                                self.dragging_slider = Some(*id);
                            }
                        }
                    }
                }
            }
        }

        if let Some(drag_id) = self.dragging_slider {
            if let Some(&(_, rx, _, rw, _)) = rects.iter().find(|(rid, _, _, _, _)| *rid == drag_id)
            {
                for widget in &self.widgets {
                    if let Widget::Slider { id, min, max, .. } = widget {
                        if *id == drag_id {
                            let t = ((mx - rx) / rw).clamp(0.0, 1.0);
                            let new_val = *min + t * (*max - *min);
                            changed_values.push((*id, new_val));
                        }
                    }
                }
            }
        }

        let mut scroll_offsets = Vec::new();
        let (scroll_dx, scroll_dy) = input.scroll_delta();
        if scroll_dy.abs() > 0.0 {
            let mut sy = self.y;
            let mut si = 0;
            while si < self.widgets.len() {
                match &self.widgets[si] {
                    Widget::ScrollRegion {
                        id,
                        height,
                        scroll_offset,
                        children,
                    } => {
                        let region_top = sy;
                        let region_bottom = sy - *height;
                        if mx >= self.x
                            && mx <= self.x + self.width
                            && my <= region_top
                            && my >= region_bottom
                        {
                            let n = (*children).min(self.widgets.len() - si - 1);
                            let child_slice = &self.widgets[si + 1..si + 1 + n];
                            let mut content_h: f32 = 0.0;
                            for (ci, cw) in child_slice.iter().enumerate() {
                                content_h +=
                                    self.compute_widget_height(cw, &child_slice[ci + 1..], atlas);
                            }
                            let max_scroll = (content_h - *height).max(0.0);
                            let new_offset =
                                (*scroll_offset - scroll_dy * 30.0).clamp(0.0, max_scroll);
                            scroll_offsets.push((*id, new_offset));
                        }
                        sy -= *height + self.style.spacing;
                        si += 1 + *children;
                        continue;
                    }
                    other => {
                        sy -= self.compute_widget_height(other, &self.widgets[si + 1..], atlas);
                        let skip = match other {
                            Widget::Panel { children, .. }
                            | Widget::Row { children, .. }
                            | Widget::Grid { children, .. } => *children,
                            _ => 0,
                        };
                        si += skip;
                    }
                }
                si += 1;
            }
            let _ = scroll_dx;
        }

        UiResponse {
            focused: Some(self.focus_index),
            activated: self.activated,
            hovered,
            toggled,
            changed_values,
            dragging: self.dragging_slider,
            scroll_offsets,
        }
    }

    pub fn render(&self, canvas: &mut Canvas, atlas: &FontAtlas) {
        let mut cursor_y = self.y;
        let mut base_x = self.x;
        let mut current_width = self.width;
        let focused_id =
            if !self.focusable_ids.is_empty() && self.focus_index < self.focusable_ids.len() {
                Some(self.focusable_ids[self.focus_index])
            } else {
                None
            };

        struct RenderContainer {
            kind: u8,
            saved_x: f32,
            saved_width: f32,
            remaining: usize,
            padding: f32,
            col_index: usize,
            col_count: usize,
            col_width: f32,
            col_spacing: f32,
            row_start_y: f32,
            row_max_h: f32,
            scroll_height: f32,
        }
        let mut stack: Vec<RenderContainer> = Vec::new();

        let mut i = 0;
        while i < self.widgets.len() {
            let mut pending: Option<RenderContainer> = None;

            match &self.widgets[i] {
                Widget::Label {
                    text,
                    size,
                    color,
                    align,
                } => {
                    let lh = atlas.line_height(*size);
                    let ax = match align {
                        TextAlign::Left => base_x,
                        TextAlign::Center => base_x + current_width / 2.0,
                        TextAlign::Right => base_x + current_width,
                    };
                    canvas.text_aligned(ax, cursor_y, text, *size, *color, *align, atlas);
                    cursor_y -= lh + self.style.spacing;
                }
                Widget::Button { id, text } => {
                    let is_focused = focused_id == Some(*id);
                    let is_pressed = is_focused && self.activated == Some(*id);

                    let bg = if is_pressed {
                        self.style.button_pressed_bg
                    } else if is_focused {
                        self.style.button_focused_bg
                    } else {
                        self.style.button_bg
                    };
                    let fg = if is_focused {
                        self.style.button_focused_text_color
                    } else {
                        self.style.button_text_color
                    };

                    let text_size = self.style.text_size;
                    let pad = self.style.button_padding;
                    let lh = atlas.line_height(text_size);
                    let btn_h = lh + pad * 2.0;

                    canvas.rect(base_x, cursor_y - btn_h + pad, current_width, btn_h, bg);

                    let label = if is_focused && !self.mouse_focus {
                        format!("> {}", text)
                    } else {
                        format!("  {}", text)
                    };
                    canvas.text(base_x + pad, cursor_y, &label, text_size, fg, atlas);

                    cursor_y -= btn_h + self.style.spacing;
                }
                Widget::Separator { height } => {
                    cursor_y -= *height;
                }
                Widget::Panel {
                    color,
                    padding,
                    children,
                } => {
                    let total_h = {
                        let end = (i + 1 + *children).min(self.widgets.len());
                        let child_slice = &self.widgets[i + 1..end];
                        let mut h = 0.0;
                        let mut ci = 0;
                        while ci < child_slice.len() {
                            h += self.compute_widget_height(
                                &child_slice[ci],
                                &child_slice[ci + 1..],
                                atlas,
                            );
                            ci += 1;
                        }
                        h
                    };

                    let panel_h = total_h + *padding * 2.0;
                    canvas.rect(
                        base_x,
                        cursor_y - panel_h + *padding,
                        current_width,
                        panel_h,
                        *color,
                    );

                    pending = Some(RenderContainer {
                        kind: 0,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: *padding,
                        col_index: 0,
                        col_count: 0,
                        col_width: 0.0,
                        col_spacing: 0.0,
                        row_start_y: 0.0,
                        row_max_h: 0.0,
                        scroll_height: 0.0,
                    });
                }
                Widget::Row { spacing, children } => {
                    let cols = (*children).max(1);
                    let total_gap = *spacing * (cols as f32 - 1.0).max(0.0);
                    let cw = (current_width - total_gap) / cols as f32;
                    pending = Some(RenderContainer {
                        kind: 1,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: 0.0,
                        col_index: 0,
                        col_count: cols,
                        col_width: cw,
                        col_spacing: *spacing,
                        row_start_y: cursor_y,
                        row_max_h: 0.0,
                        scroll_height: 0.0,
                    });
                }
                Widget::Grid {
                    columns,
                    spacing,
                    children,
                } => {
                    let cols = (*columns).max(1);
                    let total_gap = *spacing * (cols as f32 - 1.0).max(0.0);
                    let cw = (current_width - total_gap) / cols as f32;
                    pending = Some(RenderContainer {
                        kind: 2,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: 0.0,
                        col_index: 0,
                        col_count: cols,
                        col_width: cw,
                        col_spacing: *spacing,
                        row_start_y: cursor_y,
                        row_max_h: 0.0,
                        scroll_height: 0.0,
                    });
                }
                Widget::ProgressBar {
                    label,
                    value,
                    color,
                } => {
                    let text_size = self.style.text_size;
                    let lh = atlas.line_height(text_size);
                    let bar_h = self.style.progress_height;
                    let fill_color = color.unwrap_or(self.style.progress_fill);

                    if !label.is_empty() {
                        let display = format!("{} ({}%)", label, (*value * 100.0) as u32);
                        canvas.text(
                            base_x,
                            cursor_y,
                            &display,
                            text_size,
                            self.style.text_color,
                            atlas,
                        );
                        cursor_y -= lh + self.style.spacing;
                    }

                    canvas.rect(
                        base_x,
                        cursor_y - bar_h,
                        current_width,
                        bar_h,
                        self.style.progress_bg,
                    );
                    if *value > 0.0 {
                        canvas.rect(
                            base_x,
                            cursor_y - bar_h,
                            current_width * *value,
                            bar_h,
                            fill_color,
                        );
                    }
                    cursor_y -= bar_h + self.style.spacing;
                }
                Widget::Checkbox { id, label, checked } => {
                    let is_focused = focused_id == Some(*id);
                    let text_size = self.style.text_size;
                    let box_size = self.style.checkbox_size;
                    let lh = atlas.line_height(text_size);
                    let row_h = lh.max(box_size);

                    let box_bg = if *checked {
                        self.style.checkbox_checked_bg
                    } else {
                        self.style.checkbox_bg
                    };

                    let bx = base_x;
                    let by = cursor_y - row_h + (row_h - box_size) / 2.0;
                    canvas.rect(bx, by, box_size, box_size, box_bg);

                    if *checked {
                        let inset = box_size * 0.25;
                        canvas.rect(
                            bx + inset,
                            by + inset,
                            box_size - inset * 2.0,
                            box_size - inset * 2.0,
                            Color::WHITE,
                        );
                    }

                    if is_focused {
                        let border = 2.0;
                        let outline = if self.mouse_focus {
                            self.style.button_focused_bg
                        } else {
                            Color::WHITE
                        };
                        canvas.rect(
                            bx - border,
                            by - border,
                            box_size + border * 2.0,
                            border,
                            outline,
                        );
                        canvas.rect(
                            bx - border,
                            by + box_size,
                            box_size + border * 2.0,
                            border,
                            outline,
                        );
                        canvas.rect(bx - border, by, border, box_size, outline);
                        canvas.rect(bx + box_size, by, border, box_size, outline);
                    }

                    let text_x = base_x + box_size + 8.0;
                    let text_y = cursor_y - (row_h - lh) / 2.0;
                    let fg = if is_focused {
                        self.style.button_focused_text_color
                    } else {
                        self.style.text_color
                    };
                    canvas.text(text_x, text_y, label, text_size, fg, atlas);

                    cursor_y -= row_h + self.style.spacing;
                }
                Widget::Slider {
                    id,
                    label,
                    value,
                    min,
                    max,
                } => {
                    let is_focused = focused_id == Some(*id);
                    let text_size = self.style.text_size;
                    let bar_h = self.style.slider_height;

                    if !label.is_empty() {
                        let display = format!("{}: {:.1}", label, value);
                        canvas.text(
                            base_x,
                            cursor_y,
                            &display,
                            text_size,
                            self.style.text_color,
                            atlas,
                        );
                        cursor_y -= atlas.line_height(text_size) + self.style.spacing;
                    }

                    canvas.rect(
                        base_x,
                        cursor_y - bar_h,
                        current_width,
                        bar_h,
                        self.style.slider_track_color,
                    );

                    let range = max - min;
                    let t = if range > 0.0 {
                        (value - min) / range
                    } else {
                        0.0
                    };
                    if t > 0.0 {
                        canvas.rect(
                            base_x,
                            cursor_y - bar_h,
                            current_width * t,
                            bar_h,
                            self.style.slider_fill_color,
                        );
                    }

                    let thumb_w = 8.0;
                    let thumb_x = base_x + current_width * t - thumb_w / 2.0;
                    canvas.rect(
                        thumb_x,
                        cursor_y - bar_h - 2.0,
                        thumb_w,
                        bar_h + 4.0,
                        self.style.slider_thumb_color,
                    );

                    if is_focused {
                        let border = 2.0;
                        let outline = self.style.button_focused_bg;
                        canvas.rect(
                            base_x - border,
                            cursor_y - bar_h - border,
                            current_width + border * 2.0,
                            border,
                            outline,
                        );
                        canvas.rect(
                            base_x - border,
                            cursor_y,
                            current_width + border * 2.0,
                            border,
                            outline,
                        );
                        canvas.rect(base_x - border, cursor_y - bar_h, border, bar_h, outline);
                        canvas.rect(
                            base_x + current_width,
                            cursor_y - bar_h,
                            border,
                            bar_h,
                            outline,
                        );
                    }

                    cursor_y -= bar_h + self.style.spacing;
                }
                Widget::ScrollRegion {
                    height,
                    scroll_offset,
                    children,
                    ..
                } => {
                    canvas.push_clip(base_x, cursor_y - *height, current_width, *height);
                    pending = Some(RenderContainer {
                        kind: 3,
                        saved_x: base_x,
                        saved_width: current_width,
                        remaining: *children,
                        padding: 0.0,
                        col_index: 0,
                        col_count: 0,
                        col_width: 0.0,
                        col_spacing: 0.0,
                        row_start_y: cursor_y,
                        row_max_h: 0.0,
                        scroll_height: *height,
                    });
                    cursor_y += *scroll_offset;
                }
            }

            let mut pop_idx = None;
            for (si, cont) in stack.iter_mut().enumerate().rev() {
                if cont.remaining > 0 {
                    cont.remaining -= 1;

                    if cont.kind == 1 || cont.kind == 2 {
                        let used_h = cont.row_start_y - cursor_y;
                        if used_h > cont.row_max_h {
                            cont.row_max_h = used_h;
                        }
                        cont.col_index += 1;

                        let end_of_row = cont.col_index >= cont.col_count;
                        let last_child = cont.remaining == 0;

                        if end_of_row || last_child {
                            cursor_y = cont.row_start_y - cont.row_max_h;
                            cont.row_max_h = 0.0;
                            cont.col_index = 0;
                            cont.row_start_y = cursor_y;
                            base_x = cont.saved_x;
                            current_width = cont.col_width;
                        } else {
                            cursor_y = cont.row_start_y;
                            base_x += cont.col_width + cont.col_spacing;
                        }
                    }

                    if cont.remaining == 0 {
                        pop_idx = Some(si);
                    }
                    break;
                }
            }
            if let Some(si) = pop_idx {
                let cont = stack.remove(si);
                match cont.kind {
                    0 => {
                        cursor_y -= cont.padding;
                        base_x = cont.saved_x;
                        current_width = cont.saved_width;
                        cursor_y -= self.style.spacing;
                    }
                    3 => {
                        canvas.pop_clip();
                        cursor_y = cont.row_start_y - cont.scroll_height;
                        base_x = cont.saved_x;
                        current_width = cont.saved_width;
                        cursor_y -= self.style.spacing;
                    }
                    _ => {
                        base_x = cont.saved_x;
                        current_width = cont.saved_width;
                        cursor_y -= self.style.spacing;
                    }
                }
            }

            if let Some(mut cont) = pending {
                match cont.kind {
                    0 => {
                        cursor_y -= cont.padding;
                        let pad = cont.padding;
                        let old_x = base_x;
                        let old_w = current_width;
                        base_x += pad;
                        current_width -= pad * 2.0;
                        cont.saved_x = old_x;
                        cont.saved_width = old_w;
                        stack.push(cont);
                    }
                    3 => {
                        stack.push(cont);
                    }
                    _ => {
                        cont.row_start_y = cursor_y;
                        base_x = cont.saved_x;
                        current_width = cont.col_width;
                        stack.push(cont);
                    }
                }
            }

            i += 1;
        }
    }
}
