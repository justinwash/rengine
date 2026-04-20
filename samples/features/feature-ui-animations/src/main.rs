use rengine::*;

struct UiAnimationDemo {
    ui: Ui,
    badge: TextureId,
    confidence: f32,
    tire_grip: f32,
    aggressive_undercut: bool,
    brake_bias: f32,
    show_strategy_tray: bool,
    pit_call_order: [usize; 3],
    note: String,
}

impl UiAnimationDemo {
    fn create_badge(engine: &mut Engine) -> TextureId {
        let mut icon = pixelart::PixelCanvas::new(40, 40);
        icon.fill(Color::new(0.08, 0.09, 0.14, 0.0));
        let shell = Color::from_rgba8(232, 86, 74, 255);
        let visor = Color::from_rgba8(155, 228, 255, 255);
        let trim = Color::from_rgba8(251, 199, 92, 255);

        for y in 6..34 {
            for x in 6..34 {
                let dx = x as f32 - 19.5;
                let dy = y as f32 - 19.5;
                if dx * dx + dy * dy <= 170.0 {
                    icon.set(x, y, shell);
                }
            }
        }
        for y in 15..23 {
            for x in 13..29 {
                icon.set(x, y, visor);
            }
        }
        for x in 8..33 {
            icon.set(x, 33, trim);
        }

        engine.create_texture(40, 40, &icon.into_bytes())
    }

    fn appear_slide(offset_y: f32) -> UiAnimation {
        UiAnimation::new(0.32)
            .with_easing(Easing::OutBack)
            .with_offset(Vec2::new(0.0, offset_y))
            .with_alpha(0.0)
    }

    fn hover_lift() -> UiAnimation {
        UiAnimation::new(0.14)
            .with_easing(Easing::OutQuad)
            .with_offset(Vec2::new(0.0, 6.0))
            .with_scale(1.03)
    }

    fn focus_bump() -> UiAnimation {
        UiAnimation::new(0.14)
            .with_easing(Easing::OutQuad)
            .with_offset(Vec2::new(0.0, 4.0))
            .with_scale(1.04)
    }

    fn press_snap() -> UiAnimation {
        UiAnimation::new(0.1)
            .with_easing(Easing::OutQuad)
            .with_scale(0.96)
            .with_alpha(0.9)
    }

    fn container_slide(offset_y: f32) -> UiContainerAnimation {
        UiContainerAnimation::new(0.26)
            .with_easing(Easing::OutQuad)
            .with_offset(Vec2::new(0.0, offset_y))
    }

    fn pit_call_label(id: usize) -> &'static str {
        match id {
            10 => "Scout",
            11 => "Defend",
            12 => "Rotate",
            _ => "Unknown Call",
        }
    }

    fn build_ui(&mut self, engine: &Engine) {
        self.ui.style_mut().tooltip_delay = 0.0;

        let label_appear = UiAnimationOptions::new().with_appear(Self::appear_slide(-18.0));
        let badge_animation = UiAnimationOptions::new()
            .with_appear(Self::appear_slide(-22.0))
            .with_hover(Self::hover_lift());
        let bar_animation = UiAnimationOptions::new()
            .with_appear(Self::appear_slide(-14.0))
            .with_hover(Self::hover_lift());
        let interactive_animation = UiAnimationOptions::new()
            .with_appear(Self::appear_slide(-10.0))
            .with_focus(Self::focus_bump())
            .with_press(Self::press_snap());
        let button_animation = UiAnimationOptions::new()
            .with_appear(Self::appear_slide(-10.0))
            .with_hover(Self::hover_lift())
            .with_focus(Self::focus_bump())
            .with_press(Self::press_snap());

        self.ui.begin(engine, -260.0, 52.0, 520.0);
        self.ui
            .label_centered("Widget Animation Hooks", 28.0, Color::WHITE);
        self.ui.animate_with(label_appear);
        self.ui.tooltip_with(
            "Attach hover, focus, press, and appear hooks to the most recently added widget. The hooks reuse the engine's Easing curves.",
            TooltipOptions::new().with_max_width(280.0),
        );
        self.ui.separator(10.0);

        self.ui.panel(8);
        self.ui.image(self.badge, Vec2::new(92.0, 92.0));
        self.ui.animate_with(badge_animation);
        self.ui.tooltip(
            "Non-interactive widgets can animate too. Hover this badge to see the lift + scale hook fire.",
        );
        self.ui.label_centered(
            "Status Briefing",
            18.0,
            Color::from_rgba8(220, 220, 240, 255),
        );
        self.ui.animate_with(label_appear);
        self.ui.progress_bar("Confidence", self.confidence);
        self.ui.animate_with(bar_animation);
        self.ui.tooltip(
            "Hover-only widgets like labels, images, and stat bars can still react without turning into buttons.",
        );
        self.ui.progress_bar_colored(
            "Stability",
            self.tire_grip,
            Color::from_rgba8(236, 174, 72, 255),
        );
        self.ui.animate_with(bar_animation);
        self.ui
            .checkbox(1, "High-risk mode", self.aggressive_undercut);
        self.ui.animate_with(interactive_animation);
        self.ui.tooltip(
            "Arrow keys move focus, Enter or Space triggers the press hook, and the appear hook handles the initial slide-in.",
        );
        self.ui.slider(2, "Balance", self.brake_bias, 45.0, 60.0);
        self.ui.animate_with(interactive_animation);
        self.ui.button(3, "Cycle Values");
        self.ui.animate_with(button_animation);
        self.ui.tooltip(
            "Swap the sample values so the progress bars and footer note do not stay static.",
        );
        self.ui.button(4, "Swap Note");
        self.ui.animate_with(button_animation);

        self.ui.separator(12.0);
        self.ui.button(
            5,
            if self.show_strategy_tray {
                "Hide Call Tray"
            } else {
                "Show Call Tray"
            },
        );
        self.ui.animate_with(button_animation);
        self.ui.tooltip(
            "Container transitions keep a full panel alive while it slides out, instead of forcing game code to special-case the exit frame.",
        );

        if self.ui.animate_container_with(
            200,
            self.show_strategy_tray,
            UiContainerAnimationOptions::new()
                .with_appear(Self::container_slide(-28.0))
                .with_exit(Self::container_slide(28.0)),
        ) {
            self.ui.panel(7);
            self.ui
                .label_centered("Call Tray", 18.0, Color::from_rgba8(220, 220, 240, 255));
            self.ui.label_centered(
                "Drag one call onto another to reorder the tray.",
                12.0,
                Color::from_rgba8(165, 174, 196, 255),
            );
            self.ui.row_spaced(10.0, 3);
            for id in self.pit_call_order {
                self.ui.button(id, Self::pit_call_label(id));
                self.ui.animate_with(button_animation);
                self.ui.draggable();
                self.ui.drop_target();
            }
            self.ui.label_centered(
                "Mouse drag works directly.",
                11.0,
                Color::from_rgba8(132, 142, 160, 255),
            );
            self.ui.label_centered(
                "Keyboard focus can carry a call too.",
                11.0,
                Color::from_rgba8(132, 142, 160, 255),
            );
        }
    }
}

impl Game for UiAnimationDemo {
    fn new(engine: &mut Engine) -> Self {
        Self {
            ui: Ui::default(),
            badge: Self::create_badge(engine),
            confidence: 0.78,
            tire_grip: 0.64,
            aggressive_undercut: false,
            brake_bias: 54.5,
            show_strategy_tray: true,
            pit_call_order: [10, 11, 12],
            note:
                "Hover the badge and bars, then use arrow keys plus Enter on the focusable widgets."
                    .into(),
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        self.build_ui(engine);
        let response = self.ui.update(engine);

        if response.was_toggled(1) {
            self.aggressive_undercut = !self.aggressive_undercut;
            self.note = if self.aggressive_undercut {
                "Focus and press hooks keep keyboard-first flows from feeling static."
            } else {
                "Hover hooks are useful even on passive readouts like badges and stat bars."
            }
            .into();
        }
        if let Some(value) = response.value_for(2) {
            self.brake_bias = value;
        }
        if response.was_activated(3) {
            self.confidence = if self.confidence > 0.7 { 0.56 } else { 0.86 };
            self.tire_grip = if self.tire_grip > 0.5 { 0.31 } else { 0.74 };
            self.note = if self.aggressive_undercut {
                "Press hooks fire on mouse clicks and keyboard confirmation alike."
            } else {
                "Appear hooks handle first-frame slide-in without a separate tween system in game code."
            }
            .into();
        }
        if response.was_activated(4) {
            self.note = if self.note.contains("Hover") {
                "Focus hooks give tabbed or d-pad navigation the same sense of motion as mouse hover."
            } else {
                "Hover the badge and bars, then use arrow keys plus Enter on the focusable widgets."
            }
            .into();
        }
        if response.was_activated(5) {
            self.show_strategy_tray = !self.show_strategy_tray;
            self.note = if self.show_strategy_tray {
                "Container transitions keep the tray alive long enough to animate out and back in cleanly."
            } else {
                "Exit hooks now cover full containers, not just individual widgets."
            }
            .into();
        }
        if let Some((source, target)) = response.dropped {
            if let (Some(source_index), Some(target_index)) = (
                self.pit_call_order.iter().position(|id| *id == source),
                self.pit_call_order.iter().position(|id| *id == target),
            ) {
                self.pit_call_order.swap(source_index, target_index);
                self.note = format!(
                    "Moved '{}' onto '{}'. Drag/drop responses are regular widget ids, so game code can remap them however it wants.",
                    Self::pit_call_label(source),
                    Self::pit_call_label(target),
                );
            }
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(17, 21, 31, 255);
        let (_, hh) = engine.half_size();

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);
        canvas.text_block(
            0.0,
            -hh + 56.0,
            &self.note,
            12.0,
            Color::from_rgba8(184, 192, 208, 255),
            820.0,
            TextAlign::Center,
        );
        canvas.text_block(
            0.0,
            -hh + 26.0,
            "Hover shows passive widget hooks. The lower tray demonstrates container exit motion and drag/drop responses.",
            10.0,
            Color::from_rgba8(132, 142, 160, 255),
            820.0,
            TextAlign::Center,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<UiAnimationDemo>(EngineConfig {
        title: "Feature: UI Animation Hooks".into(),
        width: 1040,
        height: 840,
        ..Default::default()
    })
}
