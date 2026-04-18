use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use crate::app::Engine;
use crate::assets::Color;
use crate::canvas::{wrap_text, Canvas, TextAlign};
use crate::input::InputState;
use crate::math::Easing;
use crate::text::FontAtlas;
use crate::TextureId;
use glam::Vec2;
use winit::keyboard::KeyCode;

#[derive(Clone, Copy)]
pub enum TooltipPlacement {
    Mouse,
    Widget,
    Screen(Vec2),
}

#[derive(Clone, Copy)]
pub enum TooltipAnimation {
    None,
    Fade { duration: f32 },
    FadeSlide { duration: f32, offset: Vec2 },
}

#[derive(Clone, Copy)]
pub enum TooltipExpandTrigger {
    Shift,
    Key(KeyCode),
}

#[derive(Clone, Copy)]
pub struct UiAnimation {
    pub duration: f32,
    pub easing: Easing,
    pub offset: Vec2,
    pub scale: f32,
    pub alpha: f32,
}

impl Default for UiAnimation {
    fn default() -> Self {
        Self {
            duration: 0.18,
            easing: Easing::OutQuad,
            offset: Vec2::ZERO,
            scale: 1.0,
            alpha: 1.0,
        }
    }
}

impl UiAnimation {
    pub fn new(duration: f32) -> Self {
        Self {
            duration: duration.max(0.0),
            ..Self::default()
        }
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale.max(0.0);
        self
    }

    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.max(0.0);
        self
    }
}

#[derive(Clone, Copy, Default)]
pub struct UiAnimationOptions {
    pub hover: Option<UiAnimation>,
    pub focus: Option<UiAnimation>,
    pub press: Option<UiAnimation>,
    pub appear: Option<UiAnimation>,
}

impl UiAnimationOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_hover(mut self, animation: UiAnimation) -> Self {
        self.hover = Some(animation);
        self
    }

    pub fn with_focus(mut self, animation: UiAnimation) -> Self {
        self.focus = Some(animation);
        self
    }

    pub fn with_press(mut self, animation: UiAnimation) -> Self {
        self.press = Some(animation);
        self
    }

    pub fn with_appear(mut self, animation: UiAnimation) -> Self {
        self.appear = Some(animation);
        self
    }

    fn is_empty(self) -> bool {
        self.hover.is_none()
            && self.focus.is_none()
            && self.press.is_none()
            && self.appear.is_none()
    }
}

#[derive(Clone, Default)]
pub struct TooltipOptions {
    pub max_width: Option<f32>,
    pub fixed_width: Option<f32>,
    pub fixed_height: Option<f32>,
    pub delay: Option<f32>,
    pub placement: Option<TooltipPlacement>,
    pub offset: Option<Vec2>,
    pub animation: Option<TooltipAnimation>,
    pub advanced_text: Option<String>,
    pub expand_trigger: Option<TooltipExpandTrigger>,
}

impl TooltipOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width.max(1.0));
        self
    }

    pub fn with_fixed_width(mut self, width: f32) -> Self {
        self.fixed_width = Some(width.max(1.0));
        self
    }

    pub fn with_fixed_height(mut self, height: f32) -> Self {
        self.fixed_height = Some(height.max(1.0));
        self
    }

    pub fn with_delay(mut self, delay: f32) -> Self {
        self.delay = Some(delay.max(0.0));
        self
    }

    pub fn with_placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = Some(placement);
        self
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_animation(mut self, animation: TooltipAnimation) -> Self {
        self.animation = Some(animation);
        self
    }

    pub fn with_advanced_text(mut self, text: impl Into<String>) -> Self {
        self.advanced_text = Some(text.into());
        self
    }

    pub fn with_expand_trigger(mut self, trigger: TooltipExpandTrigger) -> Self {
        self.expand_trigger = Some(trigger);
        self
    }
}

#[derive(Clone)]
pub struct UiStyle {
    pub text_color: Color,
    pub text_size: f32,
    pub text_input_bg: Color,
    pub text_input_focused_bg: Color,
    pub text_input_text_color: Color,
    pub text_input_placeholder_color: Color,
    pub text_input_caret_color: Color,
    pub text_input_padding: f32,
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
    pub tooltip_delay: f32,
    pub tooltip_width: f32,
    pub tooltip_placement: TooltipPlacement,
    pub tooltip_offset: Vec2,
    pub tooltip_animation: TooltipAnimation,
    pub tooltip_expand_trigger: TooltipExpandTrigger,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            text_color: Color::WHITE,
            text_size: 16.0,
            text_input_bg: Color::from_rgba8(32, 36, 50, 220),
            text_input_focused_bg: Color::from_rgba8(54, 72, 122, 240),
            text_input_text_color: Color::WHITE,
            text_input_placeholder_color: Color::from_rgba8(150, 156, 176, 255),
            text_input_caret_color: Color::WHITE,
            text_input_padding: 8.0,
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
            tooltip_delay: 0.0,
            tooltip_width: 220.0,
            tooltip_placement: TooltipPlacement::Mouse,
            tooltip_offset: Vec2::new(16.0, 16.0),
            tooltip_animation: TooltipAnimation::Fade { duration: 0.12 },
            tooltip_expand_trigger: TooltipExpandTrigger::Shift,
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
    TextInput {
        id: usize,
        text: String,
        placeholder: String,
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
    options: TooltipOptions,
}

struct UiAnimationSpec {
    widget_index: usize,
    options: UiAnimationOptions,
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
    widget_index: usize,
    text: &'a str,
    rect: UiRect,
    hovered: bool,
    mouse_anchor: Vec2,
    max_width: f32,
    fixed_width: Option<f32>,
    fixed_height: Option<f32>,
    delay: f32,
    placement: TooltipPlacement,
    offset: Vec2,
    animation: TooltipAnimation,
}

#[derive(Clone, Copy, Default)]
struct TooltipRuntime {
    widget_index: Option<usize>,
    elapsed: f32,
}

#[derive(Clone, Copy, Default)]
struct WidgetAnimationRuntime {
    hover: f32,
    focus: f32,
    press: f32,
    appear: f32,
    last_seen_frame: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum WidgetRuntimeKey {
    Indexed(usize),
    Button(usize),
    TextInput(usize),
    Checkbox(usize),
    Slider(usize),
    ScrollRegion(usize),
}

#[derive(Clone, Copy)]
struct WidgetRenderAnimation {
    offset: Vec2,
    scale: f32,
    alpha: f32,
}

impl Default for WidgetRenderAnimation {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
            alpha: 1.0,
        }
    }
}

fn widget_supports_tooltip(widget: &Widget) -> bool {
    matches!(
        widget,
        Widget::Label { .. }
            | Widget::Image { .. }
            | Widget::Button { .. }
            | Widget::TextInput { .. }
            | Widget::Panel { .. }
            | Widget::ProgressBar { .. }
            | Widget::Checkbox { .. }
            | Widget::Slider { .. }
            | Widget::ScrollRegion { .. }
    )
}

fn widget_supports_animation(widget: &Widget) -> bool {
    matches!(
        widget,
        Widget::Label { .. }
            | Widget::Image { .. }
            | Widget::Button { .. }
            | Widget::TextInput { .. }
            | Widget::ProgressBar { .. }
            | Widget::Checkbox { .. }
            | Widget::Slider { .. }
    )
}

fn point_visible(point: Vec2, rect: UiRect, clip_stack: &[UiRect]) -> bool {
    rect.contains(point) && clip_stack.iter().all(|clip| clip.contains(point))
}

fn rect_visible(mut rect: UiRect, clip_stack: &[UiRect]) -> bool {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return false;
    }

    for clip in clip_stack {
        let x0 = rect.x.max(clip.x);
        let y0 = rect.y.max(clip.y);
        let x1 = (rect.x + rect.w).min(clip.x + clip.w);
        let y1 = (rect.y + rect.h).min(clip.y + clip.h);
        if x1 <= x0 || y1 <= y0 {
            return false;
        }
        rect = UiRect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        };
    }

    true
}

fn expand_trigger_active(input: &InputState, trigger: TooltipExpandTrigger) -> bool {
    match trigger {
        TooltipExpandTrigger::Shift => {
            input.is_key_down(KeyCode::ShiftLeft) || input.is_key_down(KeyCode::ShiftRight)
        }
        TooltipExpandTrigger::Key(key) => input.is_key_down(key),
    }
}

fn scale_alpha(color: Color, alpha: f32) -> Color {
    Color::new(color.r, color.g, color.b, color.a * alpha.clamp(0.0, 1.0))
}

fn widget_runtime_key(widget: &Widget, widget_index: usize) -> WidgetRuntimeKey {
    match widget {
        Widget::Button { id, .. } => WidgetRuntimeKey::Button(*id),
        Widget::TextInput { id, .. } => WidgetRuntimeKey::TextInput(*id),
        Widget::Checkbox { id, .. } => WidgetRuntimeKey::Checkbox(*id),
        Widget::Slider { id, .. } => WidgetRuntimeKey::Slider(*id),
        Widget::ScrollRegion { id, .. } => WidgetRuntimeKey::ScrollRegion(*id),
        _ => WidgetRuntimeKey::Indexed(widget_index),
    }
}

fn prev_char_boundary(text: &str, index: usize) -> usize {
    if index == 0 {
        return 0;
    }

    text[..index]
        .char_indices()
        .next_back()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }

    match text[index..].chars().next() {
        Some(ch) => index + ch.len_utf8(),
        None => text.len(),
    }
}

fn clamp_char_boundary(text: &str, index: usize) -> usize {
    let clamped = index.min(text.len());
    if text.is_char_boundary(clamped) {
        clamped
    } else {
        prev_char_boundary(text, clamped)
    }
}

fn animation_duration(animation: Option<UiAnimation>) -> f32 {
    animation.map(|animation| animation.duration).unwrap_or(0.0)
}

fn advance_animation_progress(progress: &mut f32, target: f32, duration: f32, dt: f32) {
    if duration <= 0.0 {
        *progress = target;
        return;
    }

    let step = (dt / duration).clamp(0.0, 1.0);
    if target > *progress {
        *progress = (*progress + step).min(target);
    } else {
        *progress = (*progress - step).max(target);
    }
}

fn apply_widget_animation(
    render_animation: &mut WidgetRenderAnimation,
    animation: UiAnimation,
    progress: f32,
) {
    if progress <= 0.0 {
        return;
    }

    let eased = animation.easing.apply(progress);
    render_animation.offset += animation.offset * eased;
    render_animation.scale *= 1.0 + (animation.scale - 1.0) * eased;
    render_animation.alpha *= 1.0 + (animation.alpha - 1.0) * eased;
}

fn resolve_widget_animation(
    animation_options: UiAnimationOptions,
    animation_runtime: &RefCell<HashMap<WidgetRuntimeKey, WidgetAnimationRuntime>>,
    widget: &Widget,
    widget_index: usize,
    rect: UiRect,
    focus_id: Option<usize>,
    focused_id: Option<usize>,
    mouse: Vec2,
    clip_stack: &[UiRect],
    input: &InputState,
    frame_index: u64,
    dt: f32,
) -> WidgetRenderAnimation {
    let hovered = point_visible(mouse, rect, clip_stack);
    let focused = focus_id.is_some() && focus_id == focused_id && rect_visible(rect, clip_stack);
    let pressed = (hovered && input.is_mouse_pressed(0))
        || (focused
            && (input.is_key_pressed(KeyCode::Enter) || input.is_key_pressed(KeyCode::Space)));

    let mut runtimes = animation_runtime.borrow_mut();
    let runtime = runtimes
        .entry(widget_runtime_key(widget, widget_index))
        .or_default();
    let is_new = runtime.last_seen_frame == 0 || runtime.last_seen_frame + 1 != frame_index;
    if is_new {
        runtime.hover = 0.0;
        runtime.focus = 0.0;
        runtime.press = 0.0;
        runtime.appear = 0.0;
    }
    runtime.last_seen_frame = frame_index;

    if animation_options.hover.is_some() {
        advance_animation_progress(
            &mut runtime.hover,
            if hovered { 1.0 } else { 0.0 },
            animation_duration(animation_options.hover),
            dt,
        );
    } else {
        runtime.hover = 0.0;
    }

    if animation_options.focus.is_some() {
        advance_animation_progress(
            &mut runtime.focus,
            if focused { 1.0 } else { 0.0 },
            animation_duration(animation_options.focus),
            dt,
        );
    } else {
        runtime.focus = 0.0;
    }

    if animation_options.press.is_some() {
        if pressed {
            runtime.press = 1.0;
        } else {
            advance_animation_progress(
                &mut runtime.press,
                0.0,
                animation_duration(animation_options.press),
                dt,
            );
        }
    } else {
        runtime.press = 0.0;
    }

    if animation_options.appear.is_some() {
        advance_animation_progress(
            &mut runtime.appear,
            1.0,
            animation_duration(animation_options.appear),
            dt,
        );
    } else {
        runtime.appear = 1.0;
    }

    let runtime = *runtime;
    drop(runtimes);

    let mut render_animation = WidgetRenderAnimation::default();
    if let Some(animation) = animation_options.appear {
        apply_widget_animation(&mut render_animation, animation, 1.0 - runtime.appear);
    }
    if let Some(animation) = animation_options.hover {
        apply_widget_animation(&mut render_animation, animation, runtime.hover);
    }
    if let Some(animation) = animation_options.focus {
        apply_widget_animation(&mut render_animation, animation, runtime.focus);
    }
    if let Some(animation) = animation_options.press {
        apply_widget_animation(&mut render_animation, animation, runtime.press);
    }

    render_animation
}

fn animated_rect(rect: UiRect, anchor: UiRect, animation: WidgetRenderAnimation) -> UiRect {
    let scale = animation.scale.max(0.0);
    let anchor_center = Vec2::new(anchor.x + anchor.w * 0.5, anchor.y + anchor.h * 0.5);
    let rect_center = Vec2::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
    let animated_center = anchor_center + animation.offset + (rect_center - anchor_center) * scale;
    let width = rect.w * scale;
    let height = rect.h * scale;
    UiRect {
        x: animated_center.x - width * 0.5,
        y: animated_center.y - height * 0.5,
        w: width,
        h: height,
    }
}

fn animated_point(point: Vec2, anchor: UiRect, animation: WidgetRenderAnimation) -> Vec2 {
    let scale = animation.scale.max(0.0);
    let anchor_center = Vec2::new(anchor.x + anchor.w * 0.5, anchor.y + anchor.h * 0.5);
    anchor_center + animation.offset + (point - anchor_center) * scale
}

fn animated_size(size: f32, animation: WidgetRenderAnimation) -> f32 {
    (size * animation.scale.max(0.0)).max(1.0)
}

fn resolve_tooltip_text<'a>(
    tooltip: &'a TooltipSpec,
    input: &InputState,
    style: &UiStyle,
) -> &'a str {
    if let Some(advanced) = tooltip.options.advanced_text.as_deref() {
        let trigger = tooltip
            .options
            .expand_trigger
            .unwrap_or(style.tooltip_expand_trigger);
        if expand_trigger_active(input, trigger) {
            return advanced;
        }
    }

    &tooltip.text
}

fn resolve_active_tooltip<'a>(
    tooltip: &'a TooltipSpec,
    rect: UiRect,
    hovered: bool,
    mouse_anchor: Vec2,
    input: &InputState,
    style: &UiStyle,
) -> ActiveTooltip<'a> {
    ActiveTooltip {
        widget_index: tooltip.widget_index,
        text: resolve_tooltip_text(tooltip, input, style),
        rect,
        hovered,
        mouse_anchor,
        max_width: tooltip
            .options
            .max_width
            .unwrap_or(style.tooltip_width)
            .max(1.0),
        fixed_width: tooltip.options.fixed_width.map(|width| width.max(1.0)),
        fixed_height: tooltip.options.fixed_height.map(|height| height.max(1.0)),
        delay: tooltip
            .options
            .delay
            .unwrap_or(style.tooltip_delay)
            .max(0.0),
        placement: tooltip.options.placement.unwrap_or(style.tooltip_placement),
        offset: tooltip.options.offset.unwrap_or(style.tooltip_offset),
        animation: tooltip.options.animation.unwrap_or(style.tooltip_animation),
    }
}

fn capture_tooltip<'a>(
    tooltip: &'a TooltipSpec,
    rect: UiRect,
    focus_id: Option<usize>,
    focused_id: Option<usize>,
    mouse: Vec2,
    clip_stack: &[UiRect],
    mouse_focus: bool,
    input: &InputState,
    style: &UiStyle,
    hovered_tooltip: &mut Option<ActiveTooltip<'a>>,
    focused_tooltip: &mut Option<ActiveTooltip<'a>>,
) {
    if point_visible(mouse, rect, clip_stack) {
        *hovered_tooltip = Some(resolve_active_tooltip(
            tooltip, rect, true, mouse, input, style,
        ));
        return;
    }

    if !rect_visible(rect, clip_stack) {
        return;
    }

    if !mouse_focus && hovered_tooltip.is_none() && focus_id.is_some() && focus_id == focused_id {
        *focused_tooltip = Some(resolve_active_tooltip(
            tooltip, rect, false, mouse, input, style,
        ));
    }
}

fn draw_tooltip(
    canvas: &mut Canvas,
    atlas: &FontAtlas,
    style: &UiStyle,
    tooltip: ActiveTooltip<'_>,
    screen_size: (u32, u32),
    visibility: f32,
) {
    let margin = 8.0;
    let max_box_width = (screen_size.0 as f32 - margin * 2.0).max(1.0);
    let max_box_height = (screen_size.1 as f32 - margin * 2.0).max(1.0);
    let min_box_width = (style.tooltip_padding * 2.0 + 1.0).min(max_box_width);
    let min_box_height = (style.tooltip_padding * 2.0 + 1.0).min(max_box_height);
    let max_text_width = (max_box_width - style.tooltip_padding * 2.0).max(1.0);
    let fixed_box_width = if let Some(width) = tooltip.fixed_width {
        width.min(max_box_width).max(min_box_width)
    } else {
        0.0
    };
    let text_wrap_width = if fixed_box_width > 0.0 {
        (fixed_box_width - style.tooltip_padding * 2.0).max(1.0)
    } else {
        tooltip.max_width.max(1.0).min(max_text_width)
    };
    let lines = wrap_text(
        tooltip.text,
        style.tooltip_text_size,
        text_wrap_width,
        atlas,
    );
    let line_height = atlas.line_height(style.tooltip_text_size);
    let text_width = lines
        .iter()
        .map(|line| atlas.measure_text(line, style.tooltip_text_size).0)
        .fold(0.0, f32::max);
    let content_height = line_height * lines.len() as f32;
    let box_width = if fixed_box_width > 0.0 {
        fixed_box_width
    } else {
        (text_width + style.tooltip_padding * 2.0).min(max_box_width)
    }
    .max(min_box_width);
    let box_height = tooltip
        .fixed_height
        .unwrap_or(content_height + style.tooltip_padding * 2.0)
        .min(max_box_height)
        .max(min_box_height);
    let half_width = screen_size.0 as f32 / 2.0;
    let half_height = screen_size.1 as f32 / 2.0;
    let animation_t = visibility.clamp(0.0, 1.0);
    let (alpha, animation_offset) = match tooltip.animation {
        TooltipAnimation::None => (1.0, Vec2::ZERO),
        TooltipAnimation::Fade { .. } => (animation_t, Vec2::ZERO),
        TooltipAnimation::FadeSlide { offset, .. } => (animation_t, offset * (1.0 - animation_t)),
    };
    let offset = tooltip.offset + animation_offset;
    let widget_anchor = Vec2::new(
        tooltip.rect.x + tooltip.rect.w,
        tooltip.rect.y + tooltip.rect.h,
    );
    let (raw_x, raw_top) = match tooltip.placement {
        TooltipPlacement::Mouse => {
            let anchor = if tooltip.hovered {
                tooltip.mouse_anchor
            } else {
                widget_anchor
            };
            (anchor.x + offset.x, anchor.y + offset.y)
        }
        TooltipPlacement::Widget => (widget_anchor.x + offset.x, widget_anchor.y + offset.y),
        TooltipPlacement::Screen(position) => (position.x + offset.x, position.y + offset.y),
    };
    let min_x = -half_width + margin;
    let max_x = half_width - margin - box_width;
    let min_top = -half_height + margin + box_height;
    let max_top = half_height - margin;
    let x = if min_x <= max_x {
        raw_x.clamp(min_x, max_x)
    } else {
        min_x
    };
    let top = if min_top <= max_top {
        raw_top.clamp(min_top, max_top)
    } else {
        min_top
    };
    let text_clip_height = (box_height - style.tooltip_padding * 2.0).max(1.0);
    let bg = scale_alpha(style.tooltip_bg, alpha);
    let fg = scale_alpha(style.tooltip_text_color, alpha);

    canvas.rect(x, top - box_height, box_width, box_height, bg);
    canvas.push_clip(
        x + style.tooltip_padding,
        top - box_height + style.tooltip_padding,
        (box_width - style.tooltip_padding * 2.0).max(1.0),
        text_clip_height,
    );
    canvas.text_block_lines(
        x + style.tooltip_padding,
        top - style.tooltip_padding,
        &lines,
        style.tooltip_text_size,
        fg,
        TextAlign::Left,
    );
    canvas.pop_clip();
}

pub struct UiResponse {
    pub focused: Option<usize>,
    pub activated: Option<usize>,
    pub hovered: Option<usize>,
    pub toggled: Vec<usize>,
    pub changed_values: Vec<(usize, f32)>,
    pub changed_text: Vec<(usize, String)>,
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

    pub fn text_for(&self, id: usize) -> Option<&str> {
        self.changed_text
            .iter()
            .find(|(cid, _)| *cid == id)
            .map(|(_, text)| text.as_str())
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
    animations: Vec<UiAnimationSpec>,
    focusable_ids: Vec<usize>,
    focus_index: usize,
    activated: Option<usize>,
    mouse_focus: bool,
    dragging_slider: Option<usize>,
    tooltip_runtime: Cell<TooltipRuntime>,
    animation_runtime: RefCell<HashMap<WidgetRuntimeKey, WidgetAnimationRuntime>>,
    animation_frame: Cell<u64>,
    text_input_cursor: RefCell<HashMap<usize, usize>>,
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
            animations: Vec::new(),
            focusable_ids: Vec::new(),
            focus_index: 0,
            activated: None,
            mouse_focus: false,
            dragging_slider: None,
            tooltip_runtime: Cell::new(TooltipRuntime::default()),
            animation_runtime: RefCell::new(HashMap::new()),
            animation_frame: Cell::new(0),
            text_input_cursor: RefCell::new(HashMap::new()),
        }
    }
}

impl Ui {
    pub fn begin(&mut self, engine: &Engine, x: f32, top: f32, width: f32) {
        self.animation_frame
            .set(self.animation_frame.get().wrapping_add(1));
        let (_, sh) = engine.window_size();
        self.x = x;
        self.y = (sh as f32 / 2.0) - top;
        self.width = width;
        self.widgets.clear();
        self.tooltips.clear();
        self.animations.clear();
        self.focusable_ids.clear();
        self.activated = None;
        self.mouse_focus = false;
    }

    pub fn begin_at(&mut self, x: f32, y: f32, width: f32) {
        self.animation_frame
            .set(self.animation_frame.get().wrapping_add(1));
        self.x = x;
        self.y = y;
        self.width = width;
        self.widgets.clear();
        self.tooltips.clear();
        self.animations.clear();
        self.focusable_ids.clear();
        self.activated = None;
        self.mouse_focus = false;
    }

    pub fn with_style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }

    pub fn style(&self) -> &UiStyle {
        &self.style
    }

    pub fn style_mut(&mut self) -> &mut UiStyle {
        &mut self.style
    }

    fn normalize_text_cursor(&self, id: usize, cursor: usize) -> usize {
        self.widgets
            .iter()
            .find_map(|widget| match widget {
                Widget::TextInput {
                    id: widget_id, text, ..
                } if *widget_id == id => Some(clamp_char_boundary(text, cursor)),
                _ => None,
            })
            .unwrap_or(cursor)
    }

    pub fn text_cursor(&self, id: usize) -> Option<usize> {
        self.text_input_cursor
            .borrow()
            .get(&id)
            .copied()
            .map(|cursor| self.normalize_text_cursor(id, cursor))
    }

    pub fn set_text_cursor(&self, id: usize, cursor: usize) {
        let cursor = self.normalize_text_cursor(id, cursor);
        self.text_input_cursor.borrow_mut().insert(id, cursor);
    }

    pub fn with_focus(mut self, focus: usize) -> Self {
        self.focus_index = focus;
        self
    }

    pub fn set_focus(&mut self, focus: usize) {
        self.focus_index = focus;
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
        self.tooltip_with(text, TooltipOptions::new().with_max_width(width));
    }

    pub fn tooltip_with(&mut self, text: &str, options: TooltipOptions) {
        if text.is_empty() {
            return;
        }

        if let Some(widget) = self.widgets.last() {
            if !widget_supports_tooltip(widget) {
                return;
            }
        }

        if let Some(widget_index) = self.widgets.len().checked_sub(1) {
            self.tooltips.push(TooltipSpec {
                widget_index,
                text: text.to_string(),
                options,
            });
        }
    }

    pub fn animate_with(&mut self, options: UiAnimationOptions) {
        if options.is_empty() {
            return;
        }

        if let Some(widget) = self.widgets.last() {
            if !widget_supports_animation(widget) {
                return;
            }
        }

        if let Some(widget_index) = self.widgets.len().checked_sub(1) {
            self.animations.push(UiAnimationSpec {
                widget_index,
                options,
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

    pub fn text_input(&mut self, id: usize, text: &str, placeholder: &str) {
        self.focusable_ids.push(id);
        self.widgets.push(Widget::TextInput {
            id,
            text: text.to_string(),
            placeholder: placeholder.to_string(),
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
            Widget::TextInput { .. } => {
                let lh = atlas.line_height(self.style.text_size);
                lh + self.style.text_input_padding * 2.0 + self.style.spacing
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
                Widget::TextInput { id, .. } => {
                    let lh = atlas.line_height(self.style.text_size);
                    let pad = self.style.text_input_padding;
                    let field_h = lh + pad * 2.0;
                    let field_y = cursor_y - field_h + pad;
                    rects.push((*id, base_x, field_y, current_width, field_h));
                    cursor_y -= field_h + self.style.spacing;
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
        let mut changed_text = Vec::new();
        let mut hovered = None;
        let active_text_input_ids: HashSet<usize> = self
            .widgets
            .iter()
            .filter_map(|widget| match widget {
                Widget::TextInput { id, .. } => Some(*id),
                _ => None,
            })
            .collect();

        self.text_input_cursor
            .borrow_mut()
            .retain(|id, _| active_text_input_ids.contains(id));

        if self.focusable_ids.is_empty() {
            return UiResponse {
                focused: None,
                activated: None,
                hovered: None,
                toggled,
                changed_values,
                changed_text,
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

        let focus_is_text_input = self.widgets.iter().any(|widget| match widget {
            Widget::TextInput { id, .. } => *id == self.focusable_ids[self.focus_index],
            _ => false,
        });

        if input.is_key_pressed(KeyCode::ArrowUp)
            || (!focus_is_text_input && input.is_key_pressed(KeyCode::KeyW))
        {
            self.mouse_focus = false;
            if self.focus_index == 0 {
                self.focus_index = count - 1;
            } else {
                self.focus_index -= 1;
            }
        }
        if input.is_key_pressed(KeyCode::ArrowDown)
            || (!focus_is_text_input && input.is_key_pressed(KeyCode::KeyS))
        {
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

        let focused_text_input = self.widgets.iter().find_map(|widget| match widget {
            Widget::TextInput { id, text, .. } if *id == focused_id => Some((*id, text.as_str())),
            _ => None,
        });

        if focused_text_input.is_none()
            && (input.is_key_pressed(KeyCode::ArrowLeft)
                || input.is_key_pressed(KeyCode::ArrowRight))
        {
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

        if let Some((text_id, text)) = focused_text_input {
            let mut cursor_map = self.text_input_cursor.borrow_mut();
            let cursor = cursor_map.entry(text_id).or_insert(text.len());
            *cursor = clamp_char_boundary(text, *cursor);

            if mouse_activate && hovered == Some(text_id) {
                *cursor = text.len();
            }

            let mut edited = text.to_string();
            let mut text_changed = false;

            if input.is_key_pressed(KeyCode::Home) {
                *cursor = 0;
            }
            if input.is_key_pressed(KeyCode::End) {
                *cursor = edited.len();
            }
            if input.is_key_pressed(KeyCode::ArrowLeft) {
                *cursor = prev_char_boundary(&edited, *cursor);
            }
            if input.is_key_pressed(KeyCode::ArrowRight) {
                *cursor = next_char_boundary(&edited, *cursor);
            }
            if input.is_key_pressed(KeyCode::Backspace) && *cursor > 0 {
                let start = prev_char_boundary(&edited, *cursor);
                edited.replace_range(start..*cursor, "");
                *cursor = start;
                text_changed = true;
            }
            if input.is_key_pressed(KeyCode::Delete) && *cursor < edited.len() {
                let end = next_char_boundary(&edited, *cursor);
                edited.replace_range(*cursor..end, "");
                text_changed = true;
            }
            if !input.committed_text().is_empty() {
                edited.insert_str(*cursor, input.committed_text());
                *cursor += input.committed_text().len();
                text_changed = true;
            }

            if text_changed {
                changed_text.push((text_id, edited));
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
            changed_text,
            dragging: self.dragging_slider,
            scroll_offsets,
        }
    }

    pub fn render(&self, canvas: &mut Canvas, engine: &Engine) {
        let atlas = engine.font_atlas();
        let input = engine.input();
        let dt = engine.dt();
        let mut cursor_y = self.y;
        let mut base_x = self.x;
        let mut current_width = self.width;
        let mouse = engine.mouse_screen_pos();
        let animation_frame = self.animation_frame.get();
        let focused_id =
            if !self.focusable_ids.is_empty() && self.focus_index < self.focusable_ids.len() {
                Some(self.focusable_ids[self.focus_index])
            } else {
                None
            };
        let mut clip_stack: Vec<UiRect> = Vec::new();
        let mut hovered_tooltip: Option<ActiveTooltip<'_>> = None;
        let mut focused_tooltip: Option<ActiveTooltip<'_>> = None;
        let mut tooltip_indices = vec![None; self.widgets.len()];
        for (tooltip_index, tooltip) in self.tooltips.iter().enumerate() {
            if tooltip.widget_index < tooltip_indices.len() {
                tooltip_indices[tooltip.widget_index] = Some(tooltip_index);
            }
        }
        let mut animation_indices = vec![None; self.widgets.len()];
        for animation in &self.animations {
            if animation.widget_index < animation_indices.len() {
                animation_indices[animation.widget_index] = Some(animation.options);
            }
        }

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
            let tooltip = tooltip_indices[i].map(|tooltip_index| &self.tooltips[tooltip_index]);
            let animation = animation_indices[i];
            let has_tooltip = tooltip.is_some();
            let has_animation = animation.is_some();

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
                    let label_rect = if has_tooltip || has_animation {
                        let text_width = atlas.measure_text(text, *size).0;
                        let left = match align {
                            TextAlign::Left => ax,
                            TextAlign::Center => ax - text_width / 2.0,
                            TextAlign::Right => ax - text_width,
                        };
                        Some(UiRect {
                            x: left,
                            y: cursor_y - lh,
                            w: text_width,
                            h: lh,
                        })
                    } else {
                        None
                    };

                    if let (Some(animation_options), Some(label_rect)) = (animation, label_rect) {
                        let render_animation = resolve_widget_animation(
                            animation_options,
                            &self.animation_runtime,
                            &self.widgets[i],
                            i,
                            label_rect,
                            None,
                            focused_id,
                            mouse,
                            &clip_stack,
                            input,
                            animation_frame,
                            dt,
                        );
                        if has_tooltip {
                            tooltip_rect =
                                Some(animated_rect(label_rect, label_rect, render_animation));
                        }
                        let label_pos =
                            animated_point(Vec2::new(ax, cursor_y), label_rect, render_animation);
                        canvas.text_aligned(
                            label_pos.x,
                            label_pos.y,
                            text,
                            animated_size(*size, render_animation),
                            scale_alpha(*color, render_animation.alpha),
                            *align,
                        );
                    } else {
                        if has_tooltip {
                            tooltip_rect = label_rect;
                        }
                        canvas.text_aligned(ax, cursor_y, text, *size, *color, *align);
                    }
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
                    let image_rect = UiRect {
                        x: image_x,
                        y: image_y,
                        w: size.x,
                        h: size.y,
                    };
                    if let Some(animation_options) = animation {
                        let render_animation = resolve_widget_animation(
                            animation_options,
                            &self.animation_runtime,
                            &self.widgets[i],
                            i,
                            image_rect,
                            None,
                            focused_id,
                            mouse,
                            &clip_stack,
                            input,
                            animation_frame,
                            dt,
                        );
                        let render_rect = animated_rect(image_rect, image_rect, render_animation);
                        if has_tooltip {
                            tooltip_rect = Some(render_rect);
                        }
                        canvas.image_region(
                            *texture,
                            render_rect.x,
                            render_rect.y,
                            render_rect.w,
                            render_rect.h,
                            *uv_rect,
                            scale_alpha(*color, render_animation.alpha),
                        );
                    } else {
                        if has_tooltip {
                            tooltip_rect = Some(image_rect);
                        }
                        canvas.image_region(
                            *texture, image_x, image_y, size.x, size.y, *uv_rect, *color,
                        );
                    }
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
                    let button_rect = UiRect {
                        x: base_x,
                        y: btn_y,
                        w: current_width,
                        h: btn_h,
                    };

                    if has_tooltip {
                        tooltip_focus_id = Some(*id);
                    }

                    let label = if is_focused && !self.mouse_focus {
                        format!("> {}", text)
                    } else {
                        format!("  {}", text)
                    };

                    if let Some(animation_options) = animation {
                        let render_animation = resolve_widget_animation(
                            animation_options,
                            &self.animation_runtime,
                            &self.widgets[i],
                            i,
                            button_rect,
                            Some(*id),
                            focused_id,
                            mouse,
                            &clip_stack,
                            input,
                            animation_frame,
                            dt,
                        );
                        let render_rect = animated_rect(button_rect, button_rect, render_animation);
                        if has_tooltip {
                            tooltip_rect = Some(render_rect);
                        }
                        let label_pos = animated_point(
                            Vec2::new(base_x + pad, cursor_y),
                            button_rect,
                            render_animation,
                        );
                        canvas.rect(
                            render_rect.x,
                            render_rect.y,
                            render_rect.w,
                            render_rect.h,
                            scale_alpha(bg, render_animation.alpha),
                        );
                        canvas.text(
                            label_pos.x,
                            label_pos.y,
                            &label,
                            animated_size(text_size, render_animation),
                            scale_alpha(fg, render_animation.alpha),
                        );
                    } else {
                        if has_tooltip {
                            tooltip_rect = Some(button_rect);
                        }
                        canvas.rect(base_x, btn_y, current_width, btn_h, bg);
                        canvas.text(base_x + pad, cursor_y, &label, text_size, fg);
                    }

                    cursor_y -= btn_h + self.style.spacing;
                }
                Widget::TextInput {
                    id,
                    text,
                    placeholder,
                } => {
                    let is_focused = focused_id == Some(*id);
                    let text_size = self.style.text_size;
                    let pad = self.style.text_input_padding;
                    let lh = atlas.line_height(text_size);
                    let field_h = lh + pad * 2.0;
                    let field_y = cursor_y - field_h + pad;
                    let field_rect = UiRect {
                        x: base_x,
                        y: field_y,
                        w: current_width,
                        h: field_h,
                    };
                    let bg = if is_focused {
                        self.style.text_input_focused_bg
                    } else {
                        self.style.text_input_bg
                    };

                    let cursor_index = self
                        .text_input_cursor
                        .borrow()
                        .get(id)
                        .copied()
                        .map(|cursor| clamp_char_boundary(text, cursor))
                        .unwrap_or(text.len());
                    let mut rendered_text = text.clone();
                    let mut render_cursor = cursor_index;
                    if is_focused {
                        if let Some((preedit, cursor)) = input.ime_preedit() {
                            if !preedit.is_empty() {
                                rendered_text.insert_str(cursor_index, preedit);
                                let preedit_cursor = cursor
                                    .map(|(_, end)| clamp_char_boundary(preedit, end))
                                    .unwrap_or(preedit.len());
                                render_cursor = cursor_index
                                    + preedit_cursor.min(preedit.len());
                            }
                        }
                    }
                    let display_text = if rendered_text.is_empty() {
                        placeholder.as_str()
                    } else {
                        rendered_text.as_str()
                    };
                    let fg = if rendered_text.is_empty() {
                        self.style.text_input_placeholder_color
                    } else {
                        self.style.text_input_text_color
                    };

                    if has_tooltip {
                        tooltip_focus_id = Some(*id);
                    }

                    if let Some(animation_options) = animation {
                        let render_animation = resolve_widget_animation(
                            animation_options,
                            &self.animation_runtime,
                            &self.widgets[i],
                            i,
                            field_rect,
                            Some(*id),
                            focused_id,
                            mouse,
                            &clip_stack,
                            input,
                            animation_frame,
                            dt,
                        );
                        let render_rect = animated_rect(field_rect, field_rect, render_animation);
                        let scaled_pad = pad * render_animation.scale.max(0.0);
                        if has_tooltip {
                            tooltip_rect = Some(render_rect);
                        }
                        canvas.rect(
                            render_rect.x,
                            render_rect.y,
                            render_rect.w,
                            render_rect.h,
                            scale_alpha(bg, render_animation.alpha),
                        );
                        canvas.push_clip(
                            render_rect.x + scaled_pad,
                            render_rect.y + scaled_pad,
                            (render_rect.w - scaled_pad * 2.0).max(1.0),
                            (render_rect.h - scaled_pad * 2.0).max(1.0),
                        );
                        let text_pos = animated_point(
                            Vec2::new(base_x + pad, cursor_y),
                            field_rect,
                            render_animation,
                        );
                        canvas.text(
                            text_pos.x,
                            text_pos.y,
                            display_text,
                            animated_size(text_size, render_animation),
                            scale_alpha(fg, render_animation.alpha),
                        );
                        if is_focused {
                            let caret_prefix = atlas
                                .measure_text(&rendered_text[..render_cursor], text_size)
                                .0
                                * render_animation.scale.max(0.0);
                            let caret_w = (2.0 * render_animation.scale.max(0.5)).max(1.0);
                            canvas.rect(
                                text_pos.x + caret_prefix,
                                render_rect.y + scaled_pad,
                                caret_w,
                                (render_rect.h - scaled_pad * 2.0).max(1.0),
                                scale_alpha(
                                    self.style.text_input_caret_color,
                                    render_animation.alpha,
                                ),
                            );
                        }
                        canvas.pop_clip();
                    } else {
                        if has_tooltip {
                            tooltip_rect = Some(field_rect);
                        }
                        canvas.rect(base_x, field_y, current_width, field_h, bg);
                        canvas.push_clip(
                            base_x + pad,
                            field_y + pad,
                            (current_width - pad * 2.0).max(1.0),
                            (field_h - pad * 2.0).max(1.0),
                        );
                        canvas.text(base_x + pad, cursor_y, display_text, text_size, fg);
                        if is_focused {
                            let caret_prefix = atlas
                                .measure_text(&rendered_text[..render_cursor], text_size)
                                .0;
                            canvas.rect(
                                base_x + pad + caret_prefix,
                                field_y + pad,
                                2.0,
                                (field_h - pad * 2.0).max(1.0),
                                self.style.text_input_caret_color,
                            );
                        }
                        canvas.pop_clip();
                    }

                    cursor_y -= field_h + self.style.spacing;
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
                    if has_tooltip {
                        tooltip_rect = Some(UiRect {
                            x: base_x,
                            y: cursor_y - panel_h + *padding,
                            w: current_width,
                            h: panel_h,
                        });
                    }
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
                    let display = if !label.is_empty() {
                        Some(format!("{} ({}%)", label, (*value * 100.0) as u32))
                    } else {
                        None
                    };

                    if let Some(display) = &display {
                        if let Some(animation_options) = animation {
                            let progress_rect = UiRect {
                                x: base_x,
                                y: cursor_y - (lh + self.style.spacing + bar_h),
                                w: current_width,
                                h: lh + self.style.spacing + bar_h,
                            };
                            let bar_rect = UiRect {
                                x: base_x,
                                y: cursor_y - lh - self.style.spacing - bar_h,
                                w: current_width,
                                h: bar_h,
                            };
                            let render_animation = resolve_widget_animation(
                                animation_options,
                                &self.animation_runtime,
                                &self.widgets[i],
                                i,
                                progress_rect,
                                None,
                                focused_id,
                                mouse,
                                &clip_stack,
                                input,
                                animation_frame,
                                dt,
                            );
                            let render_bar =
                                animated_rect(bar_rect, progress_rect, render_animation);
                            if has_tooltip {
                                tooltip_rect = Some(animated_rect(
                                    progress_rect,
                                    progress_rect,
                                    render_animation,
                                ));
                            }
                            let label_pos = animated_point(
                                Vec2::new(base_x, cursor_y),
                                progress_rect,
                                render_animation,
                            );
                            canvas.text(
                                label_pos.x,
                                label_pos.y,
                                display,
                                animated_size(text_size, render_animation),
                                scale_alpha(self.style.text_color, render_animation.alpha),
                            );
                            canvas.rect(
                                render_bar.x,
                                render_bar.y,
                                render_bar.w,
                                render_bar.h,
                                scale_alpha(self.style.progress_bg, render_animation.alpha),
                            );
                            if *value > 0.0 {
                                canvas.rect(
                                    render_bar.x,
                                    render_bar.y,
                                    render_bar.w * *value,
                                    render_bar.h,
                                    scale_alpha(fill_color, render_animation.alpha),
                                );
                            }
                            cursor_y -= lh + self.style.spacing;
                        } else {
                            canvas.text(
                                base_x,
                                cursor_y,
                                display,
                                text_size,
                                self.style.text_color,
                            );
                            cursor_y -= lh + self.style.spacing;
                        }
                    }

                    let progress_rect = UiRect {
                        x: base_x,
                        y: cursor_y - bar_h,
                        w: current_width,
                        h: top_y - (cursor_y - bar_h),
                    };
                    let bar_rect = UiRect {
                        x: base_x,
                        y: cursor_y - bar_h,
                        w: current_width,
                        h: bar_h,
                    };
                    if display.is_none() {
                        if let Some(animation_options) = animation {
                            let render_animation = resolve_widget_animation(
                                animation_options,
                                &self.animation_runtime,
                                &self.widgets[i],
                                i,
                                progress_rect,
                                None,
                                focused_id,
                                mouse,
                                &clip_stack,
                                input,
                                animation_frame,
                                dt,
                            );
                            let render_bar =
                                animated_rect(bar_rect, progress_rect, render_animation);
                            if has_tooltip {
                                tooltip_rect = Some(animated_rect(
                                    progress_rect,
                                    progress_rect,
                                    render_animation,
                                ));
                            }
                            canvas.rect(
                                render_bar.x,
                                render_bar.y,
                                render_bar.w,
                                render_bar.h,
                                scale_alpha(self.style.progress_bg, render_animation.alpha),
                            );
                            if *value > 0.0 {
                                canvas.rect(
                                    render_bar.x,
                                    render_bar.y,
                                    render_bar.w * *value,
                                    render_bar.h,
                                    scale_alpha(fill_color, render_animation.alpha),
                                );
                            }
                        } else {
                            if has_tooltip {
                                tooltip_rect = Some(progress_rect);
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
                        }
                    } else if has_tooltip && animation.is_none() {
                        tooltip_rect = Some(progress_rect);
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
                    let row_rect = UiRect {
                        x: base_x,
                        y: cursor_y - row_h,
                        w: current_width,
                        h: row_h,
                    };
                    let box_rect = UiRect {
                        x: bx,
                        y: by,
                        w: box_size,
                        h: box_size,
                    };
                    if has_tooltip {
                        tooltip_focus_id = Some(*id);
                    }

                    let text_x = base_x + box_size + 8.0;
                    let text_y = cursor_y - (row_h - lh) / 2.0;
                    let fg = if is_focused {
                        self.style.button_focused_text_color
                    } else {
                        self.style.text_color
                    };

                    if let Some(animation_options) = animation {
                        let render_animation = resolve_widget_animation(
                            animation_options,
                            &self.animation_runtime,
                            &self.widgets[i],
                            i,
                            row_rect,
                            Some(*id),
                            focused_id,
                            mouse,
                            &clip_stack,
                            input,
                            animation_frame,
                            dt,
                        );
                        let render_box = animated_rect(box_rect, row_rect, render_animation);
                        if has_tooltip {
                            tooltip_rect =
                                Some(animated_rect(row_rect, row_rect, render_animation));
                        }
                        canvas.rect(
                            render_box.x,
                            render_box.y,
                            render_box.w,
                            render_box.h,
                            scale_alpha(box_bg, render_animation.alpha),
                        );

                        if *checked {
                            let inset = render_box.w.min(render_box.h) * 0.25;
                            canvas.rect(
                                render_box.x + inset,
                                render_box.y + inset,
                                (render_box.w - inset * 2.0).max(0.0),
                                (render_box.h - inset * 2.0).max(0.0),
                                scale_alpha(Color::WHITE, render_animation.alpha),
                            );
                        }

                        if is_focused {
                            let border = (2.0 * render_animation.scale.max(0.5)).max(1.0);
                            let outline = if self.mouse_focus {
                                self.style.button_focused_bg
                            } else {
                                Color::WHITE
                            };
                            let outline = scale_alpha(outline, render_animation.alpha);
                            canvas.rect(
                                render_box.x - border,
                                render_box.y - border,
                                render_box.w + border * 2.0,
                                border,
                                outline,
                            );
                            canvas.rect(
                                render_box.x - border,
                                render_box.y + render_box.h,
                                render_box.w + border * 2.0,
                                border,
                                outline,
                            );
                            canvas.rect(
                                render_box.x - border,
                                render_box.y,
                                border,
                                render_box.h,
                                outline,
                            );
                            canvas.rect(
                                render_box.x + render_box.w,
                                render_box.y,
                                border,
                                render_box.h,
                                outline,
                            );
                        }

                        let text_pos =
                            animated_point(Vec2::new(text_x, text_y), row_rect, render_animation);
                        canvas.text(
                            text_pos.x,
                            text_pos.y,
                            label,
                            animated_size(text_size, render_animation),
                            scale_alpha(fg, render_animation.alpha),
                        );
                    } else {
                        if has_tooltip {
                            tooltip_rect = Some(row_rect);
                        }
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

                        canvas.text(text_x, text_y, label, text_size, fg);
                    }

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
                    let had_label = !label.is_empty();

                    let display = if had_label {
                        Some(format!("{}: {:.1}", label, value))
                    } else {
                        None
                    };

                    if let Some(display) = &display {
                        if animation.is_none() {
                            canvas.text(
                                base_x,
                                cursor_y,
                                display,
                                text_size,
                                self.style.text_color,
                            );
                        }
                        cursor_y -= atlas.line_height(text_size) + self.style.spacing;
                    }

                    let slider_rect = UiRect {
                        x: base_x,
                        y: cursor_y - bar_h - 2.0,
                        w: current_width,
                        h: top_y - (cursor_y - bar_h - 2.0),
                    };
                    let track_rect = UiRect {
                        x: base_x,
                        y: cursor_y - bar_h,
                        w: current_width,
                        h: bar_h,
                    };

                    let range = max - min;
                    let t = if range > 0.0 {
                        (value - min) / range
                    } else {
                        0.0
                    };

                    let thumb_w = 8.0;
                    let thumb_x = base_x + current_width * t - thumb_w / 2.0;
                    let thumb_rect = UiRect {
                        x: thumb_x,
                        y: cursor_y - bar_h - 2.0,
                        w: thumb_w,
                        h: bar_h + 4.0,
                    };

                    if has_tooltip {
                        tooltip_focus_id = Some(*id);
                    }

                    if let Some(animation_options) = animation {
                        let render_animation = resolve_widget_animation(
                            animation_options,
                            &self.animation_runtime,
                            &self.widgets[i],
                            i,
                            slider_rect,
                            Some(*id),
                            focused_id,
                            mouse,
                            &clip_stack,
                            input,
                            animation_frame,
                            dt,
                        );
                        let render_track = animated_rect(track_rect, slider_rect, render_animation);
                        let render_thumb = animated_rect(thumb_rect, slider_rect, render_animation);
                        if has_tooltip {
                            tooltip_rect =
                                Some(animated_rect(slider_rect, slider_rect, render_animation));
                        }
                        if let Some(display) = &display {
                            let label_pos = animated_point(
                                Vec2::new(base_x, top_y),
                                slider_rect,
                                render_animation,
                            );
                            canvas.text(
                                label_pos.x,
                                label_pos.y,
                                display,
                                animated_size(text_size, render_animation),
                                scale_alpha(self.style.text_color, render_animation.alpha),
                            );
                        }
                        canvas.rect(
                            render_track.x,
                            render_track.y,
                            render_track.w,
                            render_track.h,
                            scale_alpha(self.style.slider_track_color, render_animation.alpha),
                        );
                        if t > 0.0 {
                            canvas.rect(
                                render_track.x,
                                render_track.y,
                                render_track.w * t,
                                render_track.h,
                                scale_alpha(self.style.slider_fill_color, render_animation.alpha),
                            );
                        }
                        canvas.rect(
                            render_thumb.x,
                            render_thumb.y,
                            render_thumb.w,
                            render_thumb.h,
                            scale_alpha(self.style.slider_thumb_color, render_animation.alpha),
                        );

                        if is_focused {
                            let border = (2.0 * render_animation.scale.max(0.5)).max(1.0);
                            let outline =
                                scale_alpha(self.style.button_focused_bg, render_animation.alpha);
                            canvas.rect(
                                render_track.x - border,
                                render_track.y - border,
                                render_track.w + border * 2.0,
                                border,
                                outline,
                            );
                            canvas.rect(
                                render_track.x - border,
                                render_track.y + render_track.h,
                                render_track.w + border * 2.0,
                                border,
                                outline,
                            );
                            canvas.rect(
                                render_track.x - border,
                                render_track.y,
                                border,
                                render_track.h,
                                outline,
                            );
                            canvas.rect(
                                render_track.x + render_track.w,
                                render_track.y,
                                border,
                                render_track.h,
                                outline,
                            );
                        }
                    } else {
                        if has_tooltip {
                            tooltip_rect = Some(slider_rect);
                        }
                        canvas.rect(
                            base_x,
                            cursor_y - bar_h,
                            current_width,
                            bar_h,
                            self.style.slider_track_color,
                        );
                        if t > 0.0 {
                            canvas.rect(
                                base_x,
                                cursor_y - bar_h,
                                current_width * t,
                                bar_h,
                                self.style.slider_fill_color,
                            );
                        }
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
                    }
                    cursor_y -= bar_h + self.style.spacing;
                }
                Widget::ScrollRegion {
                    height,
                    scroll_offset,
                    children,
                    ..
                } => {
                    if has_tooltip {
                        tooltip_rect = Some(UiRect {
                            x: base_x,
                            y: cursor_y - *height,
                            w: current_width,
                            h: *height,
                        });
                    }
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

            if let (Some(tooltip), Some(rect)) = (tooltip, tooltip_rect) {
                capture_tooltip(
                    tooltip,
                    rect,
                    tooltip_focus_id,
                    focused_id,
                    mouse,
                    &clip_stack,
                    self.mouse_focus,
                    input,
                    &self.style,
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

        self.animation_runtime
            .borrow_mut()
            .retain(|_, runtime| runtime.last_seen_frame == animation_frame);

        let mut runtime = self.tooltip_runtime.get();
        if let Some(tooltip) = hovered_tooltip.or(focused_tooltip) {
            if runtime.widget_index == Some(tooltip.widget_index) {
                runtime.elapsed += engine.dt();
            } else {
                runtime.widget_index = Some(tooltip.widget_index);
                runtime.elapsed = 0.0;
            }

            let visible_elapsed = (runtime.elapsed - tooltip.delay).max(0.0);
            let visibility = match tooltip.animation {
                TooltipAnimation::None => 1.0,
                TooltipAnimation::Fade { duration }
                | TooltipAnimation::FadeSlide { duration, .. } => {
                    if duration <= 0.0 {
                        1.0
                    } else {
                        (visible_elapsed / duration).clamp(0.0, 1.0)
                    }
                }
            };

            self.tooltip_runtime.set(runtime);
            if runtime.elapsed >= tooltip.delay {
                draw_tooltip(
                    canvas,
                    atlas,
                    &self.style,
                    tooltip,
                    engine.window_size(),
                    visibility,
                );
            }
        } else {
            self.tooltip_runtime.set(TooltipRuntime::default());
        }
    }
}
