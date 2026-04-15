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
}

pub struct UiResponse {
    pub focused: Option<usize>,
    pub activated: Option<usize>,
    pub hovered: Option<usize>,
    pub toggled: Vec<usize>,
    pub changed_values: Vec<(usize, f32)>,
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
}

pub struct Ui<'a> {
    x: f32,
    y: f32,
    width: f32,
    screen_size: (u32, u32),
    atlas: &'a FontAtlas,
    style: UiStyle,
    widgets: Vec<Widget>,
    focusable_ids: Vec<usize>,
    focus_index: usize,
    activated: Option<usize>,
    mouse_focus: bool,
}

impl<'a> Ui<'a> {
    pub fn new(x: f32, y: f32, width: f32, screen_size: (u32, u32), atlas: &'a FontAtlas) -> Self {
        Self {
            x,
            y,
            width,
            screen_size,
            atlas,
            style: UiStyle::default(),
            widgets: Vec::new(),
            focusable_ids: Vec::new(),
            focus_index: 0,
            activated: None,
            mouse_focus: false,
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

    fn compute_widget_height(&self, widget: &Widget, remaining: &[Widget]) -> f32 {
        match widget {
            Widget::Label { size, .. } => self.atlas.line_height(*size) + self.style.spacing,
            Widget::Button { .. } => {
                let lh = self.atlas.line_height(self.style.text_size);
                lh + self.style.button_padding * 2.0 + self.style.spacing
            }
            Widget::Separator { height } => *height,
            Widget::Panel { padding, children, .. } => {
                let mut h = padding * 2.0;
                let n = (*children).min(remaining.len());
                let child_slice = &remaining[..n];
                let mut i = 0;
                while i < child_slice.len() {
                    h += self.compute_widget_height(&child_slice[i], &child_slice[i + 1..]);
                    i += 1;
                }
                h + self.style.spacing
            }
            Widget::ProgressBar { .. } => {
                let lh = self.atlas.line_height(self.style.text_size);
                lh + self.style.progress_height + self.style.spacing * 2.0
            }
            Widget::Checkbox { .. } => {
                let lh = self.atlas.line_height(self.style.text_size);
                lh.max(self.style.checkbox_size) + self.style.spacing
            }
            Widget::Slider { label, .. } => {
                let mut h = self.style.slider_height + self.style.spacing;
                if !label.is_empty() {
                    h += self.atlas.line_height(self.style.text_size) + self.style.spacing;
                }
                h
            }
        }
    }

    fn compute_focusable_rects(&self) -> Vec<(usize, f32, f32, f32, f32)> {
        let mut rects = Vec::new();
        let mut cursor_y = self.y;
        let mut base_x = self.x;
        let mut current_width = self.width;

        let mut i = 0;
        while i < self.widgets.len() {
            match &self.widgets[i] {
                Widget::Label { size, .. } => {
                    cursor_y -= self.atlas.line_height(*size) + self.style.spacing;
                }
                Widget::Button { id, .. } => {
                    let lh = self.atlas.line_height(self.style.text_size);
                    let pad = self.style.button_padding;
                    let btn_h = lh + pad * 2.0;
                    let btn_y = cursor_y - btn_h + pad;
                    rects.push((*id, base_x, btn_y, current_width, btn_h));
                    cursor_y -= btn_h + self.style.spacing;
                }
                Widget::Separator { height } => {
                    cursor_y -= *height;
                }
                Widget::Panel { padding, children, .. } => {
                    cursor_y -= *padding;
                    base_x += *padding;
                    current_width -= *padding * 2.0;
                    let end = (i + 1 + *children).min(self.widgets.len());
                    let _ = end; // panel children processed inline
                }
                Widget::ProgressBar { .. } => {
                    let lh = self.atlas.line_height(self.style.text_size);
                    cursor_y -= lh + self.style.progress_height + self.style.spacing * 2.0;
                }
                Widget::Checkbox { id, .. } => {
                    let lh = self.atlas.line_height(self.style.text_size);
                    let row_h = lh.max(self.style.checkbox_size);
                    rects.push((*id, base_x, cursor_y - row_h, current_width, row_h));
                    cursor_y -= row_h + self.style.spacing;
                }
                Widget::Slider { id, label, .. } => {
                    let h = self.style.slider_height;
                    if !label.is_empty() {
                        let lh = self.atlas.line_height(self.style.text_size);
                        cursor_y -= lh + self.style.spacing;
                    }
                    rects.push((*id, base_x, cursor_y - h, current_width, h));
                    cursor_y -= h + self.style.spacing;
                }
            }
            i += 1;
        }
        rects
    }

    pub fn update(&mut self, input: &InputState) -> UiResponse {
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
            };
        }

        let count = self.focusable_ids.len();
        if self.focus_index >= count {
            self.focus_index = 0;
        }

        let rects = self.compute_focusable_rects();
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

        if input.is_mouse_down(0) {
            for (rect_idx, &(_, rx, ry, rw, rh)) in rects.iter().enumerate() {
                let wid = self.focusable_ids[rect_idx];
                if mx >= rx && mx <= rx + rw && my >= ry && my <= ry + rh {
                    for widget in &self.widgets {
                        if let Widget::Slider {
                            id,
                            min,
                            max,
                            ..
                        } = widget
                        {
                            if *id == wid {
                                let t = ((mx - rx) / rw).clamp(0.0, 1.0);
                                let new_val = *min + t * (*max - *min);
                                changed_values.push((*id, new_val));
                            }
                        }
                    }
                }
            }
        }

        UiResponse {
            focused: Some(self.focus_index),
            activated: self.activated,
            hovered,
            toggled,
            changed_values,
        }
    }

    pub fn render(&self, canvas: &mut Canvas) {
        let mut cursor_y = self.y;
        let mut base_x = self.x;
        let mut current_width = self.width;
        let focused_id = if !self.focusable_ids.is_empty()
            && self.focus_index < self.focusable_ids.len()
        {
            Some(self.focusable_ids[self.focus_index])
        } else {
            None
        };

        let mut panel_stack: Vec<(f32, f32, f32)> = Vec::new();
        let mut panel_counters: Vec<usize> = Vec::new();

        let mut i = 0;
        while i < self.widgets.len() {
            match &self.widgets[i] {
                Widget::Label {
                    text,
                    size,
                    color,
                    align,
                } => {
                    let lh = self.atlas.line_height(*size);
                    let ax = match align {
                        TextAlign::Left => base_x,
                        TextAlign::Center => base_x + current_width / 2.0,
                        TextAlign::Right => base_x + current_width,
                    };
                    canvas.text_aligned(
                        ax,
                        cursor_y,
                        text,
                        *size,
                        *color,
                        *align,
                        self.screen_size,
                        self.atlas,
                    );
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
                    let lh = self.atlas.line_height(text_size);
                    let btn_h = lh + pad * 2.0;

                    canvas.rect(
                        base_x,
                        cursor_y - btn_h + pad,
                        current_width,
                        btn_h,
                        bg,
                        self.screen_size,
                    );

                    let label = if is_focused && !self.mouse_focus {
                        format!("> {}", text)
                    } else {
                        format!("  {}", text)
                    };
                    canvas.text(
                        base_x + pad,
                        cursor_y,
                        &label,
                        text_size,
                        fg,
                        self.screen_size,
                        self.atlas,
                    );

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
                        self.screen_size,
                    );

                    cursor_y -= *padding;
                    panel_stack.push((base_x, current_width, *padding));
                    base_x += *padding;
                    current_width -= *padding * 2.0;
                    panel_counters.push(*children);
                }
                Widget::ProgressBar {
                    label, value, color,
                } => {
                    let text_size = self.style.text_size;
                    let lh = self.atlas.line_height(text_size);
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
                            self.screen_size,
                            self.atlas,
                        );
                        cursor_y -= lh + self.style.spacing;
                    }

                    canvas.rect(
                        base_x,
                        cursor_y - bar_h,
                        current_width,
                        bar_h,
                        self.style.progress_bg,
                        self.screen_size,
                    );
                    if *value > 0.0 {
                        canvas.rect(
                            base_x,
                            cursor_y - bar_h,
                            current_width * *value,
                            bar_h,
                            fill_color,
                            self.screen_size,
                        );
                    }
                    cursor_y -= bar_h + self.style.spacing;
                }
                Widget::Checkbox { id, label, checked } => {
                    let is_focused = focused_id == Some(*id);
                    let text_size = self.style.text_size;
                    let box_size = self.style.checkbox_size;
                    let lh = self.atlas.line_height(text_size);
                    let row_h = lh.max(box_size);

                    let box_bg = if *checked {
                        self.style.checkbox_checked_bg
                    } else {
                        self.style.checkbox_bg
                    };

                    let bx = base_x;
                    let by = cursor_y - row_h + (row_h - box_size) / 2.0;
                    canvas.rect(bx, by, box_size, box_size, box_bg, self.screen_size);

                    if *checked {
                        let inset = box_size * 0.25;
                        canvas.rect(
                            bx + inset,
                            by + inset,
                            box_size - inset * 2.0,
                            box_size - inset * 2.0,
                            Color::WHITE,
                            self.screen_size,
                        );
                    }

                    if is_focused {
                        let border = 2.0;
                        let outline = if self.mouse_focus {
                            self.style.button_focused_bg
                        } else {
                            Color::WHITE
                        };
                        canvas.rect(bx - border, by - border, box_size + border * 2.0, border, outline, self.screen_size);
                        canvas.rect(bx - border, by + box_size, box_size + border * 2.0, border, outline, self.screen_size);
                        canvas.rect(bx - border, by, border, box_size, outline, self.screen_size);
                        canvas.rect(bx + box_size, by, border, box_size, outline, self.screen_size);
                    }

                    let text_x = base_x + box_size + 8.0;
                    let text_y = cursor_y - (row_h - lh) / 2.0;
                    let fg = if is_focused {
                        self.style.button_focused_text_color
                    } else {
                        self.style.text_color
                    };
                    canvas.text(
                        text_x,
                        text_y,
                        label,
                        text_size,
                        fg,
                        self.screen_size,
                        self.atlas,
                    );

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
                            self.screen_size,
                            self.atlas,
                        );
                        cursor_y -= self.atlas.line_height(text_size) + self.style.spacing;
                    }

                    canvas.rect(
                        base_x,
                        cursor_y - bar_h,
                        current_width,
                        bar_h,
                        self.style.slider_track_color,
                        self.screen_size,
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
                            self.screen_size,
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
                        self.screen_size,
                    );

                    if is_focused {
                        let border = 2.0;
                        let outline = self.style.button_focused_bg;
                        canvas.rect(base_x - border, cursor_y - bar_h - border, current_width + border * 2.0, border, outline, self.screen_size);
                        canvas.rect(base_x - border, cursor_y, current_width + border * 2.0, border, outline, self.screen_size);
                        canvas.rect(base_x - border, cursor_y - bar_h, border, bar_h, outline, self.screen_size);
                        canvas.rect(base_x + current_width, cursor_y - bar_h, border, bar_h, outline, self.screen_size);
                    }

                    cursor_y -= bar_h + self.style.spacing;
                }
            }

            for counter in panel_counters.iter_mut().rev() {
                if *counter > 0 {
                    *counter -= 1;
                    if *counter == 0 {
                        if let Some((old_x, old_w, padding)) = panel_stack.pop() {
                            cursor_y -= padding;
                            base_x = old_x;
                            current_width = old_w;
                            cursor_y -= self.style.spacing;
                        }
                    }
                    break;
                }
            }

            panel_counters.retain(|c| *c > 0);

            i += 1;
        }
    }
}
