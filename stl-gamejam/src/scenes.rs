use crate::gameplay::{ArmoryDefenseGame, GamePhase};
use crate::state::{
    back_pressed, back_quit_hint, confirm_pressed, menu_footer_hint, options_footer_hint,
    quit_pressed, scene_footer_hint, sync_gamepad_pairing, JamOptions, SessionState,
};
use rengine::*;

const MENU_PANEL_WIDTH: f32 = 260.0;
const ACTION_PANEL_WIDTH: f32 = 210.0;
const OPTIONS_PANEL_WIDTH: f32 = 420.0;
const THANKS_PANEL_WIDTH: f32 = 360.0;
const TITLE_HEADER_WIDTH: f32 = 660.0;
const TITLE_HEADER_HEIGHT: f32 = 86.0;
const TITLE_DOCK_WIDTH: f32 = 700.0;
const TITLE_DOCK_HEIGHT: f32 = 156.0;
const SCREEN_MARGIN: f32 = 24.0;
const PLAYFIELD_PADDING: f32 = 22.0;
const PLAYFIELD_TOP_MARGIN: f32 = 24.0;
const STATUS_BAR_HEIGHT: f32 = 122.0;
const TITLE_PIXEL: f32 = 5.0;

struct GameLayout {
    playfield_rect: Rect,
    world_origin: Vec2,
    scale: f32,
    status_rect: Rect,
    action_left: f32,
}

fn centered_panel_left(width: f32) -> f32 {
    -width * 0.5
}

fn title_header_rect(engine: &Engine) -> Rect {
    let (_, hh) = engine.half_size();
    Rect::new(
        -TITLE_HEADER_WIDTH * 0.5,
        hh - 150.0,
        TITLE_HEADER_WIDTH,
        TITLE_HEADER_HEIGHT,
    )
}

fn title_dock_rect(engine: &Engine) -> Rect {
    let (_, hh) = engine.half_size();
    Rect::new(
        -TITLE_DOCK_WIDTH * 0.5,
        -hh + 94.0,
        TITLE_DOCK_WIDTH,
        TITLE_DOCK_HEIGHT,
    )
}

fn title_info_rect(engine: &Engine) -> Rect {
    let dock = title_dock_rect(engine);
    Rect::new(dock.x + 18.0, dock.y + 18.0, 272.0, dock.height - 36.0)
}

fn title_menu_rect(engine: &Engine) -> Rect {
    let dock = title_dock_rect(engine);
    Rect::new(
        dock.right() - MENU_PANEL_WIDTH - 28.0,
        dock.y + 18.0,
        MENU_PANEL_WIDTH + 20.0,
        dock.height - 36.0,
    )
}

fn title_menu_button_rects(engine: &Engine) -> [Rect; 3] {
    let menu_rect = title_menu_rect(engine);
    let button_x = menu_rect.x + 14.0;
    let button_width = menu_rect.width - 28.0;
    [
        Rect::new(button_x, menu_rect.y + 92.0, button_width, 30.0),
        Rect::new(button_x, menu_rect.y + 56.0, button_width, 30.0),
        Rect::new(button_x, menu_rect.y + 20.0, button_width, 30.0),
    ]
}

fn title_menu_hovered_button(engine: &Engine) -> Option<usize> {
    let mouse = engine.mouse_screen_pos();
    title_menu_button_rects(engine)
        .iter()
        .position(|rect| rect.contains_point(mouse))
}

fn session_options(globals: &Globals) -> JamOptions {
    globals
        .get::<SessionState>()
        .map_or(JamOptions::default(), |session| session.options)
}

fn menu_style() -> UiStyle {
    UiStyle {
        button_bg: Color::from_rgba8(156, 48, 38, 238),
        button_focused_bg: Color::from_rgba8(230, 140, 52, 255),
        button_pressed_bg: Color::from_rgba8(101, 28, 22, 255),
        ..UiStyle::default()
    }
}

fn thanks_style() -> UiStyle {
    UiStyle {
        button_bg: Color::from_rgba8(35, 82, 156, 230),
        button_focused_bg: Color::from_rgba8(91, 172, 244, 255),
        button_pressed_bg: Color::from_rgba8(23, 49, 97, 255),
        ..UiStyle::default()
    }
}

fn draw_skyline(canvas: &mut Canvas, hw: f32, hh: f32) {
    for (x, width, height, color) in [
        (-hw + 24.0, 58.0, 88.0, Color::from_rgba8(20, 31, 53, 255)),
        (-hw + 96.0, 42.0, 132.0, Color::from_rgba8(26, 39, 63, 255)),
        (-hw + 150.0, 66.0, 96.0, Color::from_rgba8(31, 47, 74, 255)),
        (-56.0, 74.0, 126.0, Color::from_rgba8(28, 45, 69, 255)),
        (34.0, 48.0, 152.0, Color::from_rgba8(23, 37, 57, 255)),
        (92.0, 66.0, 116.0, Color::from_rgba8(29, 43, 67, 255)),
        (170.0, 52.0, 102.0, Color::from_rgba8(21, 33, 52, 255)),
        (236.0, 38.0, 72.0, Color::from_rgba8(28, 41, 65, 255)),
    ] {
        canvas.rect(x, -hh + 42.0, width, height, color);
    }

    for (x, y, width, height) in [
        (-74.0, -hh + 52.0, 8.0, 134.0),
        (-66.0, -hh + 64.0, 8.0, 122.0),
        (-58.0, -hh + 82.0, 8.0, 104.0),
        (-50.0, -hh + 102.0, 8.0, 84.0),
        (-42.0, -hh + 120.0, 8.0, 66.0),
        (-34.0, -hh + 136.0, 8.0, 50.0),
        (26.0, -hh + 136.0, 8.0, 50.0),
        (34.0, -hh + 120.0, 8.0, 66.0),
        (42.0, -hh + 102.0, 8.0, 84.0),
        (50.0, -hh + 82.0, 8.0, 104.0),
        (58.0, -hh + 64.0, 8.0, 122.0),
        (66.0, -hh + 52.0, 8.0, 134.0),
        (-26.0, -hh + 150.0, 52.0, 12.0),
    ] {
        canvas.rect(x, y, width, height, Color::from_rgba8(238, 201, 126, 255));
    }
}

fn draw_background(engine: &Engine, frame: &mut Frame, river_color: Color) {
    let (hw, hh) = engine.half_size();
    frame.clear_color = Color::from_rgba8(29, 53, 110, 255);

    let canvas = frame.canvas(0);
    canvas.rect(-hw, -hh, hw * 2.0, hh * 0.60, river_color);
    canvas.rect(
        -hw,
        hh * 0.18,
        hw * 2.0,
        hh * 0.82,
        Color::from_rgba8(84, 153, 228, 255),
    );
    canvas.rect(-hw, -hh, hw * 2.0, 34.0, Color::from_rgba8(98, 37, 32, 255));
    draw_skyline(canvas, hw, hh);
}

fn draw_footer(canvas: &mut Canvas, hh: f32, hint: &str) {
    canvas.text_aligned(
        0.0,
        -hh + 22.0,
        hint,
        10.0,
        Color::from_rgba8(252, 235, 208, 255),
        TextAlign::Center,
    );
}

fn title_pixel_rect(canvas: &mut Canvas, x: f32, y: f32, w: i32, h: i32, color: Color) {
    canvas.rect(
        x * TITLE_PIXEL,
        y * TITLE_PIXEL,
        w as f32 * TITLE_PIXEL,
        h as f32 * TITLE_PIXEL,
        color,
    );
}

fn estimated_text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.58
}

fn fitted_text_size(text: &str, size: f32, max_width: f32, min_size: f32) -> f32 {
    let mut fitted = size;
    while fitted > min_size && estimated_text_width(text, fitted) > max_width {
        fitted -= 1.0;
    }
    fitted.max(min_size)
}

fn draw_fitted_centered_text(
    canvas: &mut Canvas,
    center_x: f32,
    y: f32,
    max_width: f32,
    text: &str,
    size: f32,
    min_size: f32,
    color: Color,
) {
    let fitted = fitted_text_size(text, size, max_width, min_size);
    canvas.text_aligned(center_x, y, text, fitted, color, TextAlign::Center);
}

fn draw_title_cloud(canvas: &mut Canvas, x: f32, y: f32, scale: f32, color: Color, shadow: Color) {
    canvas.rect(
        x - 30.0 * scale,
        y - 2.0 * scale,
        60.0 * scale,
        8.0 * scale,
        shadow,
    );

    for (ox, oy, width, height) in [
        (-28.0, 2.0, 22.0, 10.0),
        (-12.0, 8.0, 26.0, 12.0),
        (8.0, 6.0, 24.0, 10.0),
        (-4.0, 0.0, 34.0, 11.0),
    ] {
        canvas.rect(
            x + ox * scale,
            y + oy * scale,
            width * scale,
            height * scale,
            color,
        );
    }
}

fn draw_title_tree(
    canvas: &mut Canvas,
    x: f32,
    ground_y: f32,
    scale: f32,
    leaf: Color,
    leaf_dark: Color,
) {
    canvas.rect(
        x - 3.0 * scale,
        ground_y,
        6.0 * scale,
        20.0 * scale,
        Color::from_rgba8(108, 58, 33, 255),
    );
    canvas.rect(
        x - 18.0 * scale,
        ground_y + 10.0 * scale,
        36.0 * scale,
        10.0 * scale,
        leaf_dark,
    );
    canvas.rect(
        x - 14.0 * scale,
        ground_y + 20.0 * scale,
        28.0 * scale,
        10.0 * scale,
        leaf,
    );
    canvas.rect(
        x - 22.0 * scale,
        ground_y + 28.0 * scale,
        44.0 * scale,
        10.0 * scale,
        leaf,
    );
    canvas.rect(
        x - 12.0 * scale,
        ground_y + 38.0 * scale,
        24.0 * scale,
        8.0 * scale,
        leaf_dark,
    );
}

fn title_sign_glyph(ch: char) -> [&'static str; 5] {
    match ch.to_ascii_uppercase() {
        'A' => [" ### ", "#   #", "#####", "#   #", "#   #"],
        'M' => ["#   #", "## ##", "# # #", "#   #", "#   #"],
        'O' => [" ### ", "#   #", "#   #", "#   #", " ### "],
        'R' => ["#### ", "#   #", "#### ", "#  # ", "#   #"],
        'Y' => ["#   #", " # # ", "  #  ", "  #  ", "  #  "],
        ' ' => ["     ", "     ", "     ", "     ", "     "],
        _ => ["     ", "     ", "     ", "     ", "     "],
    }
}

fn draw_title_sign_text(
    canvas: &mut Canvas,
    center_x: f32,
    y: f32,
    text: &str,
    color: Color,
    shadow: Color,
) {
    let glyph_width = 5.0;
    let spacing = 1.0;
    let total_width = if text.is_empty() {
        0.0
    } else {
        text.chars().count() as f32 * (glyph_width + spacing) - spacing
    };
    let start_x = center_x - total_width * 0.5;

    for (index, ch) in text.chars().enumerate() {
        let glyph = title_sign_glyph(ch);
        let glyph_x = start_x + index as f32 * (glyph_width + spacing);

        for (row, pattern) in glyph.iter().enumerate() {
            for (col, pixel) in pattern.chars().enumerate() {
                if pixel == ' ' {
                    continue;
                }

                let px = glyph_x + col as f32;
                let py = y + (4 - row) as f32;
                title_pixel_rect(canvas, px + 0.35, py - 0.35, 1, 1, shadow);
                title_pixel_rect(canvas, px, py, 1, 1, color);
            }
        }
    }
}

fn draw_title_background(engine: &Engine, frame: &mut Frame, options: JamOptions) {
    let (hw, hh) = engine.half_size();
    frame.clear_color = Color::from_rgba8(251, 145, 96, 255);
    let header = title_header_rect(engine);

    let canvas = frame.canvas(0);
    canvas.rect(
        -hw,
        -hh,
        hw * 2.0,
        hh * 2.0,
        Color::from_rgba8(248, 227, 197, 255),
    );
    canvas.rect(
        -hw,
        -8.0,
        hw * 2.0,
        hh + 8.0,
        Color::from_rgba8(255, 154, 96, 255),
    );
    canvas.rect(
        -hw,
        68.0,
        hw * 2.0,
        hh - 68.0,
        Color::from_rgba8(76, 104, 214, 255),
    );
    canvas.rect(
        -hw,
        22.0,
        hw * 2.0,
        22.0,
        Color::from_rgba8(255, 215, 128, 210),
    );
    canvas.rect(
        -hw,
        -hh,
        hw * 2.0,
        164.0,
        Color::from_rgba8(150, 106, 96, 255),
    );
    canvas.rect(
        -hw,
        -hh + 148.0,
        hw * 2.0,
        22.0,
        Color::from_rgba8(236, 193, 92, 255),
    );

    if options.show_title_atmosphere {
        draw_title_cloud(
            canvas,
            -208.0,
            116.0,
            1.0,
            Color::from_rgba8(255, 244, 230, 255),
            Color::from_rgba8(255, 188, 146, 140),
        );
        draw_title_cloud(
            canvas,
            194.0,
            134.0,
            0.9,
            Color::from_rgba8(255, 246, 235, 255),
            Color::from_rgba8(255, 196, 152, 135),
        );
        draw_title_cloud(
            canvas,
            18.0,
            92.0,
            1.15,
            Color::from_rgba8(255, 242, 224, 255),
            Color::from_rgba8(255, 179, 133, 145),
        );

        for (x, scale) in [(-270.0, 1.15), (-230.0, 0.95), (-192.0, 1.0)] {
            draw_title_tree(
                canvas,
                x,
                -58.0,
                scale,
                Color::from_rgba8(61, 135, 72, 255),
                Color::from_rgba8(38, 96, 54, 255),
            );
        }
        for (x, scale) in [(192.0, 1.0), (230.0, 0.95), (272.0, 1.15)] {
            draw_title_tree(
                canvas,
                x,
                -58.0,
                scale,
                Color::from_rgba8(61, 135, 72, 255),
                Color::from_rgba8(38, 96, 54, 255),
            );
        }
    }

    for index in 0..44 {
        let t = index as f32 / 43.0;
        let x = -350.0 + t * 700.0;
        let center = t * 2.0 - 1.0;
        let height = 80.0 + (1.0 - center.abs().powf(1.55)) * 210.0;
        canvas.rect(
            x,
            -hh + 194.0,
            6.0,
            height,
            Color::from_rgba8(255, 239, 221, 150),
        );
        canvas.rect(
            x + 1.0,
            -hh + 194.0,
            2.0,
            height - 16.0,
            Color::from_rgba8(255, 200, 151, 90),
        );
    }

    let bx = 0.0;
    let by = -8.0;
    let brick = Color::from_rgba8(145, 54, 43, 255);
    let brick_dark = Color::from_rgba8(105, 35, 29, 255);
    let brick_light = Color::from_rgba8(176, 80, 61, 255);
    let trim = Color::from_rgba8(239, 229, 214, 255);
    let roof = Color::from_rgba8(84, 28, 24, 255);
    let glass = Color::from_rgba8(45, 144, 211, 255);

    canvas.rect(
        -326.0,
        -236.0,
        652.0,
        28.0,
        Color::from_rgba8(96, 36, 34, 70),
    );

    title_pixel_rect(canvas, bx - 58.0, by - 32.0, 116, 34, brick);
    title_pixel_rect(canvas, bx - 58.0, by - 22.0, 116, 3, brick_light);
    title_pixel_rect(canvas, bx - 58.0, by - 2.0, 116, 2, brick_dark);
    title_pixel_rect(canvas, bx - 58.0, by + 12.0, 116, 3, brick_light);
    title_pixel_rect(canvas, bx - 60.0, by - 34.0, 120, 3, trim);
    title_pixel_rect(canvas, bx - 61.0, by + 1.0, 122, 3, trim);

    for row in 0..16 {
        let width = 72 - row * 4;
        title_pixel_rect(
            canvas,
            bx - width as f32 * 0.5,
            by + 2.0 + row as f32,
            width,
            1,
            roof,
        );
    }
    for row in 0..13 {
        let width = 66 - row * 4;
        title_pixel_rect(
            canvas,
            bx - width as f32 * 0.5,
            by + 4.0 + row as f32,
            width,
            1,
            brick_dark,
        );
    }
    for row in 0..15 {
        let width = 76 - row * 4;
        title_pixel_rect(
            canvas,
            bx - width as f32 * 0.5,
            by + 1.0 + row as f32,
            width,
            1,
            trim,
        );
    }

    title_pixel_rect(canvas, bx - 34.0, by - 4.0, 68, 11, glass);
    for offset in [-25.0, -13.0, -1.0, 11.0, 23.0] {
        title_pixel_rect(canvas, bx + offset, by - 3.0, 3, 10, trim);
    }
    title_pixel_rect(canvas, bx - 34.0, by + 6.0, 68, 2, trim);

    for x in [-48.0, -36.0, -24.0, -12.0, 0.0, 12.0, 24.0, 36.0] {
        title_pixel_rect(canvas, bx + x, by - 28.0, 4, 30, brick_dark);
        title_pixel_rect(canvas, bx + x + 1.0, by - 10.0, 2, 9, glass);
        title_pixel_rect(canvas, bx + x - 1.0, by - 1.0, 6, 2, trim);
    }

    title_pixel_rect(
        canvas,
        bx - 9.0,
        by - 28.0,
        18,
        8,
        Color::from_rgba8(117, 88, 64, 255),
    );
    title_pixel_rect(
        canvas,
        bx - 25.0,
        by + 17.0,
        50,
        7,
        Color::from_rgba8(230, 218, 199, 255),
    );
    title_pixel_rect(
        canvas,
        bx - 22.0,
        by + 18.0,
        44,
        5,
        Color::from_rgba8(215, 201, 181, 255),
    );
    draw_title_sign_text(
        canvas,
        bx,
        by + 18.0,
        "ARMORY",
        Color::from_rgba8(163, 120, 87, 255),
        Color::from_rgba8(120, 81, 55, 255),
    );

    canvas.rect(
        header.x,
        header.y,
        header.width,
        header.height,
        Color::from_rgba8(97, 26, 54, 232),
    );
    canvas.rect(
        header.x,
        header.y,
        header.width,
        6.0,
        Color::from_rgba8(255, 193, 79, 255),
    );
    draw_fitted_centered_text(
        canvas,
        header.center().x,
        header.top() - 24.0,
        header.width - 36.0,
        "ST. LOUIS ARMORY DEFENSE",
        22.0,
        16.0,
        Color::from_rgba8(255, 244, 220, 255),
    );
    draw_fitted_centered_text(
        canvas,
        header.center().x,
        header.top() - 50.0,
        header.width - 48.0,
        "Stop the server rollout before the crews lock down the floor.",
        10.0,
        7.0,
        Color::from_rgba8(235, 239, 251, 255),
    );
}

fn draw_title_panel(
    canvas: &mut Canvas,
    engine: &Engine,
    completed_runs: u32,
    hovered_button: Option<usize>,
) {
    let dock = title_dock_rect(engine);
    let info_rect = title_info_rect(engine);
    let menu_rect = title_menu_rect(engine);
    let [start_rect, options_rect, quit_rect] = title_menu_button_rects(engine);
    let divider_x = (info_rect.right() + menu_rect.x) * 0.5;

    canvas.rect(
        dock.x,
        dock.y,
        dock.width,
        dock.height,
        Color::from_rgba8(114, 43, 29, 236),
    );
    canvas.rect(
        dock.x,
        dock.top() - 4.0,
        dock.width,
        4.0,
        Color::from_rgba8(255, 193, 79, 255),
    );
    canvas.rect(
        info_rect.x,
        info_rect.y,
        info_rect.width,
        info_rect.height,
        Color::from_rgba8(156, 58, 45, 255),
    );
    canvas.rect(
        menu_rect.x,
        menu_rect.y,
        menu_rect.width,
        menu_rect.height,
        Color::from_rgba8(38, 119, 89, 255),
    );
    canvas.rect(
        divider_x,
        dock.y + 24.0,
        2.0,
        dock.height - 48.0,
        Color::from_rgba8(255, 193, 79, 255),
    );
    draw_fitted_centered_text(
        canvas,
        info_rect.center().x,
        info_rect.top() - 18.0,
        info_rect.width - 22.0,
        "ARMORY STATUS",
        10.0,
        7.0,
        Color::from_rgba8(255, 223, 175, 255),
    );
    draw_fitted_centered_text(
        canvas,
        info_rect.center().x,
        info_rect.top() - 42.0,
        info_rect.width - 22.0,
        &format!("Runs held: {completed_runs}"),
        16.0,
        10.0,
        Color::from_rgba8(255, 244, 228, 255),
    );
    draw_fitted_centered_text(
        canvas,
        info_rect.center().x,
        info_rect.top() - 62.0,
        info_rect.width - 24.0,
        "Crews staged inside.",
        8.0,
        6.0,
        Color::from_rgba8(255, 225, 205, 255),
    );
    draw_fitted_centered_text(
        canvas,
        info_rect.center().x,
        info_rect.top() - 74.0,
        info_rect.width - 24.0,
        "Set traps. Hold the floor.",
        8.0,
        6.0,
        Color::from_rgba8(255, 225, 205, 255),
    );
    draw_fitted_centered_text(
        canvas,
        menu_rect.center().x,
        menu_rect.top() - 18.0,
        menu_rect.width - 20.0,
        "MAIN MENU",
        10.0,
        7.0,
        Color::from_rgba8(219, 235, 255, 255),
    );

    for (index, (rect, label, fill, accent, hover_fill)) in [
        (
            start_rect,
            "START DEFENSE",
            Color::from_rgba8(180, 58, 37, 255),
            Color::from_rgba8(255, 196, 90, 255),
            Color::from_rgba8(225, 90, 53, 255),
        ),
        (
            options_rect,
            "OPTIONS",
            Color::from_rgba8(199, 117, 39, 255),
            Color::from_rgba8(255, 222, 131, 255),
            Color::from_rgba8(232, 151, 58, 255),
        ),
        (
            quit_rect,
            "QUIT",
            Color::from_rgba8(87, 30, 28, 255),
            Color::from_rgba8(255, 163, 105, 255),
            Color::from_rgba8(130, 48, 45, 255),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let is_hovered = hovered_button == Some(index);
        let button_fill = if is_hovered { hover_fill } else { fill };

        if is_hovered {
            canvas.rect(
                rect.x - 4.0,
                rect.y - 4.0,
                rect.width + 8.0,
                rect.height + 8.0,
                Color::from_rgba8(255, 233, 179, 200),
            );
        }

        canvas.rect(rect.x, rect.y, rect.width, rect.height, button_fill);
        canvas.rect(rect.x, rect.top() - 3.0, rect.width, 3.0, accent);
        canvas.rect(rect.x, rect.y, 3.0, rect.height, accent);
        draw_fitted_centered_text(
            canvas,
            rect.center().x,
            rect.y + 16.0,
            rect.width - 20.0,
            label,
            11.0,
            8.0,
            Color::from_rgba8(255, 242, 226, 255),
        );
    }
}

fn game_layout(engine: &Engine) -> GameLayout {
    let (hw, hh) = engine.half_size();
    let status_rect = Rect::new(
        -hw + SCREEN_MARGIN,
        -hh + SCREEN_MARGIN,
        hw * 2.0 - SCREEN_MARGIN * 2.0,
        STATUS_BAR_HEIGHT,
    );
    let playfield_y = status_rect.top() + SCREEN_MARGIN;
    let playfield_rect = Rect::new(
        -hw + SCREEN_MARGIN,
        playfield_y,
        hw * 2.0 - SCREEN_MARGIN * 2.0,
        hh - PLAYFIELD_TOP_MARGIN - playfield_y,
    );

    let map_bounds = ArmoryDefenseGame::world_bounds();
    let usable_width = (playfield_rect.width - PLAYFIELD_PADDING * 2.0).max(200.0);
    let usable_height = (playfield_rect.height - PLAYFIELD_PADDING * 2.0).max(200.0);
    let scale =
        ((usable_width / map_bounds.width).min(usable_height / map_bounds.height)).clamp(0.45, 1.8);
    let extra_x = (usable_width - map_bounds.width * scale) * 0.5;
    let extra_y = (usable_height - map_bounds.height * scale) * 0.5;

    GameLayout {
        playfield_rect,
        world_origin: Vec2::new(
            playfield_rect.x + PLAYFIELD_PADDING + extra_x - map_bounds.x * scale,
            playfield_rect.y + PLAYFIELD_PADDING + extra_y - map_bounds.y * scale,
        ),
        scale,
        status_rect,
        action_left: status_rect.right() - ACTION_PANEL_WIDTH - 18.0,
    }
}

fn screen_to_playfield(layout: &GameLayout, mouse: Vec2) -> Option<Vec2> {
    if !layout.playfield_rect.contains_point(mouse) {
        return None;
    }

    Some((mouse - layout.world_origin) / layout.scale)
}

fn draw_game_chrome(canvas: &mut Canvas, layout: &GameLayout) {
    canvas.rect(
        layout.playfield_rect.x - 10.0,
        layout.playfield_rect.y - 10.0,
        layout.playfield_rect.width + 20.0,
        layout.playfield_rect.height + 20.0,
        Color::from_rgba8(85, 30, 34, 188),
    );
    canvas.rect(
        layout.playfield_rect.x,
        layout.playfield_rect.y,
        layout.playfield_rect.width,
        layout.playfield_rect.height,
        Color::from_rgba8(46, 24, 36, 238),
    );
    canvas.rect(
        layout.status_rect.x,
        layout.status_rect.y,
        layout.status_rect.width,
        layout.status_rect.height,
        Color::from_rgba8(27, 55, 111, 232),
    );
    canvas.rect(
        layout.status_rect.x,
        layout.status_rect.top() - 2.0,
        layout.status_rect.width,
        2.0,
        Color::from_rgba8(234, 169, 69, 255),
    );
    canvas.text_aligned(
        layout.playfield_rect.x + 6.0,
        layout.playfield_rect.top() + 14.0,
        "TACTICAL FLOOR",
        11.0,
        Color::from_rgba8(255, 230, 191, 255),
        TextAlign::Left,
    );
}

fn draw_status_bar(
    canvas: &mut Canvas,
    layout: &GameLayout,
    game: &ArmoryDefenseGame,
    detail: &str,
    visits: u32,
    show_footer_hints: bool,
) {
    let left = layout.status_rect.x + 18.0;
    let top = layout.status_rect.top() - 20.0;
    let phase_color = match game.phase() {
        GamePhase::Build => Color::from_rgba8(255, 215, 105, 255),
        GamePhase::Wave => Color::from_rgba8(255, 142, 72, 255),
        GamePhase::Victory => Color::from_rgba8(113, 233, 179, 255),
        GamePhase::Defeat => Color::from_rgba8(255, 109, 92, 255),
    };

    canvas.text_aligned(
        left,
        top,
        "HOLD THE ARMORY",
        17.0,
        Color::from_rgba8(255, 244, 222, 255),
        TextAlign::Left,
    );
    canvas.text_aligned(
        left,
        top - 20.0,
        &format!(
            "{}   Wave {}/{}   Scrap {}   Conversion {:.0}%",
            game.phase_label(),
            game.displayed_wave(),
            game.total_waves(),
            game.scrap(),
            game.conversion()
        ),
        12.0,
        phase_color,
        TextAlign::Left,
    );
    canvas.text_aligned(
        left,
        top - 38.0,
        &format!(
            "Stopped {} contractor crews   Session deployments {}",
            game.contractors_stopped(),
            visits
        ),
        11.0,
        Color::from_rgba8(224, 233, 255, 255),
        TextAlign::Left,
    );
    canvas.text_aligned(
        left,
        top - 59.0,
        detail,
        11.0,
        Color::from_rgba8(255, 232, 185, 255),
        TextAlign::Left,
    );
    canvas.text_aligned(
        left,
        top - 80.0,
        "Mouse: build pads on the floor.  Builds: Event Fence $20, Pop Box $28.",
        10.0,
        Color::from_rgba8(214, 226, 250, 255),
        TextAlign::Left,
    );
    if show_footer_hints {
        canvas.text_aligned(
            layout.status_rect.x + layout.status_rect.width * 0.5,
            layout.status_rect.y + 16.0,
            scene_footer_hint(),
            10.0,
            Color::from_rgba8(238, 227, 207, 255),
            TextAlign::Center,
        );
    }
}

pub struct MenuScene {
    ui: Ui,
}

impl MenuScene {
    pub fn new() -> Self {
        Self {
            ui: Ui::default().with_style(menu_style()),
        }
    }

    fn build_menu(ui: &mut Ui) {
        ui.button(0, "Start Defense");
        ui.button(1, "Options");
        ui.button(2, "Quit");
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        sync_gamepad_pairing(engine);
    }

    fn on_resume(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        sync_gamepad_pairing(engine);
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let menu_rect = title_menu_rect(engine);
        let hovered_button = title_menu_hovered_button(engine);
        let response = self.ui.run(
            engine,
            menu_rect.x + 14.0,
            menu_rect.y + 116.0,
            menu_rect.width - 28.0,
            |ui| Self::build_menu(ui),
        );

        let action = hovered_button
            .filter(|_| engine.input().is_mouse_pressed(0))
            .or(response.activated);

        if let Some(id) = action {
            match id {
                0 => {
                    return SceneOp::FadeSwitch(Box::new(GameScene::new()), Transition::fade(0.35));
                }
                1 => {
                    return SceneOp::FadeSwitch(
                        Box::new(OptionsScene::new()),
                        Transition::fade_color(Color::from_rgba8(255, 193, 79, 255), 0.3),
                    );
                }
                2 => return SceneOp::Quit,
                _ => {}
            }
        }

        if quit_pressed(engine) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        let options = session_options(globals);
        let hovered_button = title_menu_hovered_button(engine);
        draw_title_background(engine, frame, options);

        let completed_runs = globals
            .get::<SessionState>()
            .map_or(0, |session| session.completed_runs);

        let canvas = frame.canvas(0);
        draw_title_panel(canvas, engine, completed_runs, hovered_button);
        if options.show_footer_hints {
            let (_, hh) = engine.half_size();
            draw_footer(canvas, hh, menu_footer_hint());
        }
    }
}

pub struct OptionsScene {
    ui: Ui,
}

impl OptionsScene {
    pub fn new() -> Self {
        Self {
            ui: Ui::default().with_style(menu_style()),
        }
    }

    fn build_menu(ui: &mut Ui, options: JamOptions) {
        let route = if options.show_route_overlay {
            "ON"
        } else {
            "OFF"
        };
        let grid = if options.show_floor_grid { "ON" } else { "OFF" };
        let footer = if options.show_footer_hints {
            "ON"
        } else {
            "OFF"
        };
        let sky = if options.show_title_atmosphere {
            "ON"
        } else {
            "OFF"
        };

        ui.button(0, &format!("Route Overlay: {route}"));
        ui.button(1, &format!("Floor Grid: {grid}"));
        ui.button(2, &format!("Footer Hints: {footer}"));
        ui.button(3, &format!("Title Atmosphere: {sky}"));
        ui.button(4, "Reset Defaults");
        ui.button(5, "Back");
    }
}

impl Scene for OptionsScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        sync_gamepad_pairing(engine);
    }

    fn on_resume(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        sync_gamepad_pairing(engine);
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let options = session_options(globals);
        let response = self.ui.run(
            engine,
            centered_panel_left(OPTIONS_PANEL_WIDTH),
            112.0,
            OPTIONS_PANEL_WIDTH,
            |ui| Self::build_menu(ui, options),
        );

        if let Some(id) = response.activated {
            if let Some(session) = globals.get_mut::<SessionState>() {
                match id {
                    0 => session.options.show_route_overlay = !session.options.show_route_overlay,
                    1 => session.options.show_floor_grid = !session.options.show_floor_grid,
                    2 => session.options.show_footer_hints = !session.options.show_footer_hints,
                    3 => {
                        session.options.show_title_atmosphere =
                            !session.options.show_title_atmosphere
                    }
                    4 => session.options = JamOptions::default(),
                    5 => {
                        return SceneOp::FadeSwitch(
                            Box::new(MenuScene::new()),
                            Transition::fade_color(Color::from_rgba8(255, 193, 79, 255), 0.25),
                        );
                    }
                    _ => {}
                }
            }
        }

        if back_pressed(engine) {
            return SceneOp::FadeSwitch(
                Box::new(MenuScene::new()),
                Transition::fade_color(Color::from_rgba8(255, 193, 79, 255), 0.25),
            );
        }
        if quit_pressed(engine) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        let options = session_options(globals);
        let (_, hh) = engine.half_size();
        draw_background(engine, frame, Color::from_rgba8(28, 56, 112, 255));

        let canvas = frame.canvas(0);
        canvas.rect(
            -OPTIONS_PANEL_WIDTH * 0.5 - 20.0,
            -188.0,
            OPTIONS_PANEL_WIDTH + 40.0,
            328.0,
            Color::from_rgba8(81, 30, 53, 226),
        );
        canvas.rect(
            -OPTIONS_PANEL_WIDTH * 0.5 - 20.0,
            136.0,
            OPTIONS_PANEL_WIDTH + 40.0,
            4.0,
            Color::from_rgba8(255, 193, 79, 255),
        );
        canvas.text_aligned(
            0.0,
            110.0,
            "OPTIONS",
            26.0,
            Color::from_rgba8(255, 243, 220, 255),
            TextAlign::Center,
        );
        canvas.text_aligned(
            0.0,
            86.0,
            "Tune readability, overlays, and title atmosphere.",
            11.0,
            Color::from_rgba8(240, 228, 207, 255),
            TextAlign::Center,
        );
        canvas.text_aligned(
            0.0,
            -150.0,
            &format!(
                "Route overlay {} | Floor grid {} | Footer hints {} | Title atmosphere {}",
                if options.show_route_overlay {
                    "on"
                } else {
                    "off"
                },
                if options.show_floor_grid { "on" } else { "off" },
                if options.show_footer_hints {
                    "on"
                } else {
                    "off"
                },
                if options.show_title_atmosphere {
                    "on"
                } else {
                    "off"
                }
            ),
            10.0,
            Color::from_rgba8(255, 215, 170, 255),
            TextAlign::Center,
        );
        self.ui.render(canvas, engine);
        if options.show_footer_hints {
            draw_footer(canvas, hh, options_footer_hint());
        }
    }
}

pub struct GameScene {
    ui: Ui,
    game: ArmoryDefenseGame,
}

impl GameScene {
    pub fn new() -> Self {
        Self {
            ui: Ui::default().with_style(menu_style()),
            game: ArmoryDefenseGame::new(),
        }
    }

    fn build_action_panel(ui: &mut Ui, phase: GamePhase) {
        match phase {
            GamePhase::Build => ui.button(0, "Start Wave"),
            GamePhase::Victory | GamePhase::Defeat => ui.button(1, "Continue"),
            GamePhase::Wave => {
                ui.label_centered("Wave active", 11.0, Color::from_rgba8(205, 219, 239, 255))
            }
        }
    }
}

impl Scene for GameScene {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals) {
        sync_gamepad_pairing(engine);
        if let Some(session) = globals.get_mut::<SessionState>() {
            session.main_scene_visits += 1;
        }
    }

    fn on_resume(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        sync_gamepad_pairing(engine);
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let layout = game_layout(engine);
        let mouse_playfield = screen_to_playfield(&layout, engine.mouse_screen_pos());

        let response = self.ui.run(
            engine,
            layout.action_left,
            layout.status_rect.top() - 16.0,
            ACTION_PANEL_WIDTH,
            |ui| Self::build_action_panel(ui, self.game.phase()),
        );

        if self.game.phase() == GamePhase::Build {
            if response.was_activated(0) {
                self.game.start_wave();
            }

            if engine.input().is_mouse_pressed(0) {
                if let Some(mouse) = mouse_playfield {
                    self.game.handle_build_click(mouse);
                }
            }
        }

        self.game.update(engine);

        if matches!(self.game.phase(), GamePhase::Victory | GamePhase::Defeat)
            && (response.was_activated(1) || confirm_pressed(engine))
        {
            if let Some(session) = globals.get_mut::<SessionState>() {
                if self.game.phase() == GamePhase::Victory {
                    session.completed_runs += 1;
                    session.last_stop =
                        format!("The Armory held through {} waves.", self.game.total_waves());
                } else {
                    session.last_stop = "Contractors converted the install node.".to_string();
                }
                session.last_conversion = self.game.conversion();
                session.last_contractors_stopped = self.game.contractors_stopped();
            }

            return SceneOp::FadeSwitch(
                Box::new(ThanksScene::new()),
                Transition::fade_color(Color::from_rgba8(232, 189, 101, 255), 0.45),
            );
        }

        if back_pressed(engine) {
            return SceneOp::FadeSwitch(Box::new(MenuScene::new()), Transition::fade(0.25));
        }
        if quit_pressed(engine) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        let layout = game_layout(engine);
        let options = session_options(globals);
        let mouse_playfield = screen_to_playfield(&layout, engine.mouse_screen_pos());
        let hovered_slot = mouse_playfield.and_then(|mouse| self.game.hovered_slot_index(mouse));
        let detail = mouse_playfield
            .and_then(|mouse| self.game.hover_message(mouse))
            .unwrap_or_else(|| self.game.message().to_string());
        let visits = globals
            .get::<SessionState>()
            .map_or(0, |session| session.main_scene_visits);

        draw_background(engine, frame, Color::from_rgba8(11, 24, 58, 255));

        let canvas = frame.canvas(0);
        draw_game_chrome(canvas, &layout);
        self.game.render(
            canvas,
            layout.world_origin,
            layout.scale,
            hovered_slot,
            mouse_playfield,
            options.show_route_overlay,
            options.show_floor_grid,
        );
        draw_status_bar(
            canvas,
            &layout,
            &self.game,
            &detail,
            visits,
            options.show_footer_hints,
        );
        self.ui.render(canvas, engine);
    }
}

pub struct ThanksScene {
    ui: Ui,
}

impl ThanksScene {
    pub fn new() -> Self {
        Self {
            ui: Ui::default().with_style(thanks_style()),
        }
    }

    fn build_menu(ui: &mut Ui) {
        ui.label_centered(
            "Thanks For Playing",
            26.0,
            Color::from_rgba8(255, 234, 198, 255),
        );
        ui.separator(6.0);
        ui.label_centered(
            "Loop scaffold complete.",
            12.0,
            Color::from_rgba8(245, 223, 191, 255),
        );
        ui.separator(10.0);
        ui.button(0, "Play Again");
        ui.button(1, "Return to Menu");
        ui.button(2, "Quit");
    }
}

impl Scene for ThanksScene {
    fn on_enter(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        sync_gamepad_pairing(engine);
    }

    fn on_resume(&mut self, engine: &mut Engine, _globals: &mut Globals) {
        sync_gamepad_pairing(engine);
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let response = self.ui.run(
            engine,
            centered_panel_left(THANKS_PANEL_WIDTH),
            44.0,
            THANKS_PANEL_WIDTH,
            |ui| Self::build_menu(ui),
        );

        if let Some(id) = response.activated {
            match id {
                0 => {
                    return SceneOp::FadeSwitch(Box::new(GameScene::new()), Transition::fade(0.35));
                }
                1 => {
                    return SceneOp::FadeSwitch(Box::new(MenuScene::new()), Transition::fade(0.35));
                }
                2 => return SceneOp::Quit,
                _ => {}
            }
        }

        if back_pressed(engine) {
            return SceneOp::FadeSwitch(Box::new(MenuScene::new()), Transition::fade(0.25));
        }
        if quit_pressed(engine) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        let (_, hh) = engine.half_size();
        let options = session_options(globals);
        draw_background(engine, frame, Color::from_rgba8(35, 21, 43, 255));

        let canvas = frame.canvas(0);
        self.ui.render(canvas, engine);

        let (completed_runs, last_stop) = globals
            .get::<SessionState>()
            .map(|session| (session.completed_runs, session.last_stop.as_str()))
            .unwrap_or((0, ""));
        let (last_conversion, contractors_stopped) = globals
            .get::<SessionState>()
            .map(|session| (session.last_conversion, session.last_contractors_stopped))
            .unwrap_or((0.0, 0));

        canvas.text_aligned(
            0.0,
            -hh + 58.0,
            &format!("Armory runs held: {completed_runs}"),
            12.0,
            Color::from_rgba8(255, 234, 198, 255),
            TextAlign::Center,
        );
        if !last_stop.is_empty() {
            canvas.text_aligned(
                0.0,
                -hh + 40.0,
                &format!("Last stop: {last_stop}"),
                11.0,
                Color::from_rgba8(230, 192, 130, 255),
                TextAlign::Center,
            );
        }
        canvas.text_aligned(
            0.0,
            -hh + 22.0,
            &format!(
                "Last run: {:.0}% conversion | {} contractors stopped",
                last_conversion, contractors_stopped
            ),
            10.0,
            Color::from_rgba8(245, 223, 191, 255),
            TextAlign::Center,
        );
        if options.show_footer_hints {
            draw_footer(canvas, hh, back_quit_hint());
        }
    }
}
