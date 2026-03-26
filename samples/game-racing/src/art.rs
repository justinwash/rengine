use rengine::assets::pixelart::PixelCanvas;
use rengine::{Color, Engine, TextureId};

/// Generate a top-down F1-style car sprite (16x16, pointing right).
pub fn car_sprite(engine: &mut Engine, body_color: Color) -> TextureId {
    let mut c = PixelCanvas::new(16, 16);

    let dark = darken(body_color, 0.6);
    let highlight = lighten(body_color, 1.3);
    let black = Color::rgb(0.1, 0.1, 0.1);
    let tire = Color::rgb(0.2, 0.2, 0.2);
    let white = Color::WHITE;

    // Body (elongated, pointing right = +x)
    // Main body rectangle
    c.fill_rect(3, 5, 10, 6, body_color);

    // Nose cone (front)
    c.fill_rect(13, 6, 2, 4, dark);
    c.set(15, 7, body_color);
    c.set(15, 8, body_color);

    // Rear wing
    c.fill_rect(1, 4, 2, 8, dark);
    c.fill_rect(0, 3, 1, 10, black);

    // Cockpit
    c.fill_rect(7, 6, 3, 4, black);
    c.set(8, 7, Color::rgb(0.3, 0.6, 0.9)); // helmet

    // Side pods
    c.fill_rect(5, 4, 4, 1, dark);
    c.fill_rect(5, 11, 4, 1, dark);

    // Front wing elements
    c.fill_rect(12, 4, 2, 1, highlight);
    c.fill_rect(12, 11, 2, 1, highlight);

    // Tires (4 corners)
    c.fill_rect(2, 2, 3, 2, tire);   // rear-left
    c.fill_rect(2, 12, 3, 2, tire);  // rear-right
    c.fill_rect(11, 3, 2, 2, tire);  // front-left
    c.fill_rect(11, 11, 2, 2, tire); // front-right

    // Number plate / accent
    c.set(6, 7, white);
    c.set(6, 8, white);

    let bytes = c.into_bytes();
    engine.create_texture(16, 16, &bytes)
}

/// Create a simple white 1x1 pixel texture (for drawing lines/rects).
pub fn white_pixel(engine: &Engine) -> TextureId {
    engine.white_texture()
}

fn darken(c: Color, factor: f32) -> Color {
    Color::new(c.r * factor, c.g * factor, c.b * factor, c.a)
}

fn lighten(c: Color, factor: f32) -> Color {
    Color::new(
        (c.r * factor).min(1.0),
        (c.g * factor).min(1.0),
        (c.b * factor).min(1.0),
        c.a,
    )
}
