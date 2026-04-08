use rengine::pixelart::{darken, PixelCanvas};
use rengine::Color;

pub fn fighter_idle(
    body_color: Color,
    skin_color: Color,
    belt_color: Color,
) -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(32, 48);
    let hair = darken(body_color, 0.6);
    let shoe = Color::from_rgba8(60, 40, 20, 255);

    c.fill_circle(16, 8, 5, skin_color);
    c.fill_rect(11, 2, 10, 4, hair);
    c.set(14, 7, Color::BLACK);
    c.set(18, 7, Color::BLACK);

    c.fill_rect(11, 13, 10, 12, body_color);
    c.fill_rect(11, 23, 10, 2, belt_color);

    c.fill_rect(7, 14, 4, 10, skin_color);
    c.fill_rect(21, 14, 4, 10, skin_color);

    c.fill_rect(11, 25, 4, 12, darken(body_color, 0.7));
    c.fill_rect(17, 25, 4, 12, darken(body_color, 0.7));

    c.fill_rect(10, 37, 5, 3, shoe);
    c.fill_rect(17, 37, 5, 3, shoe);

    (32, 48, c.into_bytes())
}

pub fn fighter_punch(
    body_color: Color,
    skin_color: Color,
    belt_color: Color,
) -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(32, 48);
    let hair = darken(body_color, 0.6);
    let shoe = Color::from_rgba8(60, 40, 20, 255);

    c.fill_circle(14, 8, 5, skin_color);
    c.fill_rect(9, 2, 10, 4, hair);
    c.set(12, 7, Color::BLACK);
    c.set(16, 7, Color::BLACK);

    c.fill_rect(10, 13, 10, 12, body_color);
    c.fill_rect(10, 23, 10, 2, belt_color);

    c.fill_rect(6, 15, 4, 8, skin_color);
    c.fill_rect(20, 15, 10, 4, skin_color);
    c.fill_rect(28, 14, 4, 5, skin_color);

    c.fill_rect(10, 25, 4, 12, darken(body_color, 0.7));
    c.fill_rect(16, 25, 4, 12, darken(body_color, 0.7));
    c.fill_rect(9, 37, 5, 3, shoe);
    c.fill_rect(16, 37, 5, 3, shoe);

    (32, 48, c.into_bytes())
}

pub fn fighter_kick(
    body_color: Color,
    skin_color: Color,
    belt_color: Color,
) -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(32, 48);
    let hair = darken(body_color, 0.6);
    let shoe = Color::from_rgba8(60, 40, 20, 255);

    c.fill_circle(14, 8, 5, skin_color);
    c.fill_rect(9, 2, 10, 4, hair);
    c.set(12, 7, Color::BLACK);
    c.set(16, 7, Color::BLACK);

    c.fill_rect(9, 13, 10, 12, body_color);
    c.fill_rect(9, 23, 10, 2, belt_color);

    c.fill_rect(5, 14, 4, 9, skin_color);
    c.fill_rect(19, 14, 4, 9, skin_color);

    c.fill_rect(9, 25, 4, 12, darken(body_color, 0.7));
    c.fill_rect(8, 37, 5, 3, shoe);

    c.fill_rect(15, 28, 12, 4, darken(body_color, 0.7));
    c.fill_rect(26, 27, 5, 4, shoe);

    (32, 48, c.into_bytes())
}

pub fn fighter_block(
    body_color: Color,
    skin_color: Color,
    belt_color: Color,
) -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(32, 48);
    let hair = darken(body_color, 0.6);
    let shoe = Color::from_rgba8(60, 40, 20, 255);

    c.fill_circle(16, 9, 5, skin_color);
    c.fill_rect(11, 3, 10, 4, hair);
    c.set(14, 8, Color::BLACK);
    c.set(18, 8, Color::BLACK);

    c.fill_rect(11, 14, 10, 12, body_color);
    c.fill_rect(11, 24, 10, 2, belt_color);

    c.fill_rect(9, 13, 5, 10, skin_color);
    c.fill_rect(18, 13, 5, 10, skin_color);

    c.fill_rect(9, 26, 4, 12, darken(body_color, 0.7));
    c.fill_rect(19, 26, 4, 12, darken(body_color, 0.7));
    c.fill_rect(8, 38, 5, 3, shoe);
    c.fill_rect(19, 38, 5, 3, shoe);

    (32, 48, c.into_bytes())
}

pub fn fighter_hit(body_color: Color, skin_color: Color, belt_color: Color) -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(32, 48);
    let hair = darken(body_color, 0.6);
    let shoe = Color::from_rgba8(60, 40, 20, 255);

    c.fill_circle(18, 8, 5, skin_color);
    c.fill_rect(13, 2, 10, 4, hair);
    c.set(16, 7, Color::BLACK);
    c.set(20, 7, Color::BLACK);

    c.fill_rect(13, 13, 10, 12, body_color);
    c.fill_rect(13, 23, 10, 2, belt_color);

    c.fill_rect(9, 16, 4, 8, skin_color);
    c.fill_rect(23, 12, 4, 8, skin_color);

    c.fill_rect(13, 25, 4, 12, darken(body_color, 0.7));
    c.fill_rect(19, 25, 4, 12, darken(body_color, 0.7));
    c.fill_rect(12, 37, 5, 3, shoe);
    c.fill_rect(19, 37, 5, 3, shoe);

    (32, 48, c.into_bytes())
}

pub fn dojo_floor_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let base = Color::from_rgba8(90, 70, 50, 255);
    c.fill(base);

    let dark = darken(base, 0.85);
    c.fill_rect(0, 3, 16, 1, dark);
    c.fill_rect(0, 7, 16, 1, dark);
    c.fill_rect(0, 11, 16, 1, dark);
    c.fill_rect(0, 15, 16, 1, dark);
    (16, 16, c.into_bytes())
}
