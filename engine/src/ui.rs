use crate::app::Engine;
use crate::assets::Color;
use crate::canvas::{wrap_text, Canvas, TextAlign};
use crate::text::FontAtlas;
use crate::TextureId;
use glam::Vec2;
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
    pub tooltip_bg: Color,
    pub tooltip_text_color: Color,
    pub tooltip_text_size: f32,
    pub tooltip_padding: f32,
    pub tooltip_width: f32,
    pub tooltip_offset: Vec2,
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
            tooltip_bg: Color::from_rgba8(12, 14, 22, 235),
            tooltip_text_color: Color::from_rgba8(235, 235, 245, 255),
            tooltip_text_size: 14.0,
            tooltip_padding: 8.0,
            tooltip_width: 220.0,
            tooltip_offset: Vec2::new(16.0, 16.0),
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
    Image {
        texture: TextureId,
        size: Vec2,
        color: Color,
        uv_rect: [f32; 4],
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

struct TooltipSpec {
    widget_index: usize,
    text: String,
    width: f32,
}

#[derive(Clone, Copy)]
struct UiRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl UiRect {
    fn contains(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.w
            && point.y >= self.y
            && point.y <= self.y + self.h
    }
}

struct ActiveTooltip<'a> {
    text: &'a str,
    width: f32,
    anchor: Vec2,
}

fn tooltip_for_widget(tooltips: &[TooltipSpec], widget_index: usize) -> Option<&TooltipSpec> {
    tooltips
        .iter()
        .rev()
        .find(|tooltip| tooltip.widget_index == widget_index)
}

fn point_visible(point: Vec2, rect: UiRect, clip_stack: &[UiRect]) -> bool {
    rect.contains(point) && clip_stack.iter().all(|clip| clip.contains(point))
}

fn capture_tooltip<'a>(
    tooltips: &'a [TooltipSpec],
    widget_index: usize,
    rect: UiRect,
    focus_id: Option<usize>,
    focused_id: Option<usize>,
    mouse: Vec2,
    clip_stack: &[UiRect],
    mouse_focus: bool,
    hovered_tooltip: &mut Option<ActiveTooltip<'a>>,
    focused_tooltip: &mut Option<ActiveTooltip<'a>>,
) {
    let Some(tooltip) = tooltip_for_widget(tooltips, widget_index) else {
        return;
    };

    if point_visible(mouse, rect, clip_stack) {
        *hovered_tooltip = Some(ActiveTooltip {
            text: &tooltip.text,
            width: tooltip.width,
            anchor: mouse,
        });
        return;
    }

    if !mouse_focus && hovered_tooltip.is_none() && focus_id.is_some() && focus_id == focused_id {
        *focused_tooltip = Some(ActiveTooltip {
            text: &tooltip.text,
            width: tooltip.width,
            anchor: Vec2::new(rect.x + rect.w, rect.y + rect.h),
        });
    }
}

fn draw_tooltip(
    canvas: &mut Canvas,
    atlas: &FontAtlas,
    style: &UiStyle,
    tooltip: ActiveTooltip<'_>,
    screen_size: (u32, u32),
) {
    let margin = 8.0;
    let max_text_width = style.tooltip_width.max(32.0).min(
        screen_size.0 as f32 - margin * 2.0 - style.tooltip_padding * 2.0,
    );
    let max_text_width = tooltip.width.max(32.0).min(max_text_width.max(32.0));
    let lines = wrap_text(tooltip.text, style.tooltip_text_size, max_text_width, atlas);
    let line_height = atlas.line_height(style.tooltip_text_size);
    let text_width = lines
        .iter()
        .map(|line| atlas.measure_text(line, style.tooltip_text_size).0)
        .fold(0.0, f32::max);
    let box_width = text_width + style.tooltip_padding * 2.0;
    let box_height = line_height * lines.len() as f32 + style.tooltip_padding * 2.0;
    let half_width = screen_size.0 as f32 / 2.0;
    let half_height = screen_size.1 as f32 / 2.0;
    let x = (tooltip.anchor.x + style.tooltip_offset.x)
        .clamp(-half_width + margin, half_width - margin - box_width);
    let top = (tooltip.anchor.y + style.tooltip_offset.y)
        .clamp(-half_height + margin + box_height, half_height - margin);

    canvas.rect(x, top - box_height, box_width, box_height, style.tooltip_bg);
    canvas.text_block(
        x + style.tooltip_padding,
        top - style.tooltip_padding,
        tooltip.text,
        style.tooltip_text_size,
        style.tooltip_text_color,
        max_text_width,
        TextAlign::Left,
    );
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
    tooltips: Vec<TooltipSpec>,
    focusable_ids: Vec<usize>,
    focus_index: usize,
    activated: Option<usize>,
    mouse_focus: bool,
    dragging_slider: Option<usize>,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            style: UiStyle::default(),
            widgets: Vec::new(),
            tooltips: Vec::new(),
            focusable_ids: Vec::new(),
            focus_index: 0,
            activated: None,
            mouse_focus: false,
            dragging_slider: None,
        }
    }
}

impl Ui {
    pub fn begin(&mut self, engine: &Engine, x: f32, top: f32, width: f32) {
        let (_, sh) = engine.window_size();
        self.x = x;
        self.y = (sh as f32 / 2.0) - top;
        self.width = width;
        self.widgets.clear();
        self.tooltips.clear();
        self.focusable_ids.clear();
        self.activated = None;
        self.mouse_focus = false;
    }

    pub fn begin_at(&mut self, x: f32, y: f32, width: f32) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.widgets.clear();
        self.tooltips.clear();
        self.focusable_ids.clear();
        self.activated = None;
        self.mouse_focus = false;
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

    pub fn image(&mut self, texture: TextureId, size: Vec2) {
        self.widgets.push(Widget::Image {
            texture,
            size,
            color: Color::WHITE,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        });
    }

    pub fn image_colored(&mut self, texture: TextureId, size: Vec2, color: Color) {
        self.widgets.push(Widget::Image {
            texture,
            size,
            color,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        });
    }

    pub fn image_region(&mut self, texture: TextureId, size: Vec2, uv_rect: [f32; 4]) {
        self.widgets.push(Widget::Image {
            texture,
            size,
            color: Color::WHITE,
            uv_rect,
        });
    }

    pub fn tooltip(&mut self, text: &str) {
        self.tooltip_sized(text, self.style.tooltip_width);
    }

    pub fn tooltip_sized(&mut self, text: &str, width: f32) {
        if text.is_empty() {
            return;
        }

        if let Some(widget_index) = self.widgets.len().checked_sub(1) {
            self.tooltips.push(TooltipSpec {
                widget_index,
                text: text.to_string(),
                width: width.max(1.0),
            });
        }
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
            Widget::Image { size, .. } => size.y + self.style.spacing,
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
                Widget::Image { size, .. } => {
                    cursor_y -= size.y + self.style.spacing;
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

    pub fn update(&mut self, engine: &Engine) -> UiResponse {
        let input = engine.input();
        let atlas = engine.font_atlas();
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

    pub fn render(&self, canvas: &mut Canvas, engine: &Engine) {
        let atlas = engine.font_atlas();
        let mut cursor_y = self.y;
        let mut base_x = self.x;
        let mut current_width = self.width;
        let mouse = engine.mouse_screen_pos();
        let focused_id =
            if !self.focusable_ids.is_empty() && self.focus_index < self.focusable_ids.len() {
                Some(self.focusable_ids[self.focus_index])
            } else {
                None
            };
        let mut clip_stack: Vec<UiRect> = Vec::new();
        let mut hovered_tooltip: Option<ActiveTooltip<'_>> = None;
        let mut focused_tooltip: Option<ActiveTooltip<'_>> = None;

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
            let mut tooltip_rect = None;
            let mut tooltip_focus_id = None;

            match &self.widgets[i] {
                Widget::Label {
                    text,
                    size,
                    color,
                    align,
                } => {
                    let lh = atlas.line_height(*size);
                    let text_width = atlas.measure_text(text, *size).0;
                    let ax = match align {
                        TextAlign::Left => base_x,
                        TextAlign::Center => base_x + current_width / 2.0,
                        TextAlign::Right => base_x + current_width,
                    };
                    let left = match align {
                        TextAlign::Left => ax,
                        TextAlign::Center => ax - text_width / 2.0,
                        TextAlign::Right => ax - text_width,
                    };
                    tooltip_rect = Some(UiRect {
                        x: left,
                        y: cursor_y - lh,
                        w: text_width,
                        h: lh,
                    });
                    canvas.text_aligned(ax, cursor_y, text, *size, *color, *align);
                    cursor_y -= lh + self.style.spacing;
                }
                Widget::Image {
                    texture,
                    size,
                    color,
                    uv_rect,
                } => {
                    let image_x = base_x + (current_width - size.x).max(0.0) * 0.5;
                    let image_y = cursor_y - size.y;
                    tooltip_rect = Some(UiRect {
                        x: image_x,
                        y: image_y,
                        w: size.x,
                        h: size.y,
                    });
                    canvas.image_region(*texture, image_x, image_y, size.x, size.y, *uv_rect, *color);
                    cursor_y -= size.y + self.style.spacing;
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
                    let btn_y = cursor_y - btn_h + pad;

                    tooltip_rect = Some(UiRect {
                        x: base_x,
                        y: btn_y,
                        w: current_width,
                        h: btn_h,
                    });
                    tooltip_focus_id = Some(*id);

                    canvas.rect(base_x, btn_y, current_width, btn_h, bg);

                    let label = if is_focused && !self.mouse_focus {
                        format!("> {}", text)
                    } else {
                        format!("  {}", text)
                    };
                    canvas.text(base_x + pad, cursor_y, &label, text_size, fg);

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
                    tooltip_rect = Some(UiRect {
                        x: base_x,
                        y: cursor_y - panel_h + *padding,
                        w: current_width,
                        h: panel_h,
                    });
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
                    let top_y = cursor_y;
                    let text_size = self.style.text_size;
                    let lh = atlas.line_height(text_size);
                    let bar_h = self.style.progress_height;
                    let fill_color = color.unwrap_or(self.style.progress_fill);

                    if !label.is_empty() {
                        let display = format!("{} ({}%)", label, (*value * 100.0) as u32);
                        canvas.text(base_x, cursor_y, &display, text_size, self.style.text_color);
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
                    tooltip_rect = Some(UiRect {
                        x: base_x,
                        y: cursor_y - bar_h,
                        w: current_width,
                        h: top_y - (cursor_y - bar_h),
                    });
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
                    tooltip_rect = Some(UiRect {
                        x: base_x,
                        y: cursor_y - row_h,
                        w: current_width,
                        h: row_h,
                    });
                    tooltip_focus_id = Some(*id);
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
                    canvas.text(text_x, text_y, label, text_size, fg);

                    cursor_y -= row_h + self.style.spacing;
                }
                Widget::Slider {
                    id,
                    label,
                    value,
                    min,
                    max,
                } => {
                    let top_y = cursor_y;
                    let is_focused = focused_id == Some(*id);
                    let text_size = self.style.text_size;
                    let bar_h = self.style.slider_height;

                    if !label.is_empty() {
                        let display = format!("{}: {:.1}", label, value);
                        canvas.text(base_x, cursor_y, &display, text_size, self.style.text_color);
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

                    tooltip_rect = Some(UiRect {
                        x: base_x,
                        y: cursor_y - bar_h - 2.0,
                        w: current_width,
                        h: top_y - (cursor_y - bar_h - 2.0),
                    });
                    tooltip_focus_id = Some(*id);
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

            if let Some(rect) = tooltip_rect {
                capture_tooltip(
                    &self.tooltips,
                    i,
                    rect,
                    tooltip_focus_id,
                    focused_id,
                    mouse,
                    &clip_stack,
                    self.mouse_focus,
                    &mut hovered_tooltip,
                    &mut focused_tooltip,
                );
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
                        clip_stack.pop();
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
                        clip_stack.push(UiRect {
                            x: cont.saved_x,
                            y: cont.row_start_y - cont.scroll_height,
                            w: cont.saved_width,
                            h: cont.scroll_height,
                        });
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

        if let Some(tooltip) = hovered_tooltip.or(focused_tooltip) {
            draw_tooltip(canvas, atlas, &self.style, tooltip, engine.window_size());
        }
    }
}
