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
}

pub struct UiResponse {
    pub focused: Option<usize>,
    pub activated: Option<usize>,
}

pub struct Ui<'a> {
    x: f32,
    y: f32,
    width: f32,
    screen_size: (u32, u32),
    atlas: &'a FontAtlas,
    style: UiStyle,
    widgets: Vec<Widget>,
    button_ids: Vec<usize>,
    focus_index: usize,
    activated: Option<usize>,
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
            button_ids: Vec::new(),
            focus_index: 0,
            activated: None,
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
        self.button_ids.push(id);
        self.widgets.push(Widget::Button {
            id,
            text: text.to_string(),
        });
    }

    pub fn separator(&mut self, height: f32) {
        self.widgets.push(Widget::Separator { height });
    }

    pub fn update(&mut self, input: &InputState) -> UiResponse {
        if self.button_ids.is_empty() {
            return UiResponse {
                focused: None,
                activated: None,
            };
        }

        let count = self.button_ids.len();
        if self.focus_index >= count {
            self.focus_index = 0;
        }

        if input.is_key_pressed(KeyCode::ArrowUp) || input.is_key_pressed(KeyCode::KeyW) {
            if self.focus_index == 0 {
                self.focus_index = count - 1;
            } else {
                self.focus_index -= 1;
            }
        }
        if input.is_key_pressed(KeyCode::ArrowDown) || input.is_key_pressed(KeyCode::KeyS) {
            self.focus_index = (self.focus_index + 1) % count;
        }

        let focused_id = self.button_ids[self.focus_index];

        if input.is_key_pressed(KeyCode::Enter) || input.is_key_pressed(KeyCode::Space) {
            self.activated = Some(focused_id);
        }

        UiResponse {
            focused: Some(self.focus_index),
            activated: self.activated,
        }
    }

    pub fn render(&self, canvas: &mut Canvas) {
        let mut cursor_y = self.y;
        let focused_id = if !self.button_ids.is_empty() && self.focus_index < self.button_ids.len()
        {
            Some(self.button_ids[self.focus_index])
        } else {
            None
        };

        for widget in &self.widgets {
            match widget {
                Widget::Label {
                    text,
                    size,
                    color,
                    align,
                } => {
                    let lh = self.atlas.line_height(*size);
                    let ax = match align {
                        TextAlign::Left => self.x,
                        TextAlign::Center => self.x + self.width / 2.0,
                        TextAlign::Right => self.x + self.width,
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
                        self.x,
                        cursor_y - btn_h + pad,
                        self.width,
                        btn_h,
                        bg,
                        self.screen_size,
                    );

                    let label = if is_focused {
                        format!("> {}", text)
                    } else {
                        format!("  {}", text)
                    };
                    canvas.text(
                        self.x + pad,
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
            }
        }
    }
}
