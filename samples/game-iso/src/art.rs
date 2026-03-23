use rengine::pixelart::{darken, lighten, PixelCanvas};
use rengine::Color;

pub fn iso_grass_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(64, 32);
    let base = Color::from_rgba8(76, 153, 0, 255);
    c.fill_diamond(base);

    let light = lighten(base, 1.15);
    for &(x, y) in &[(20, 14), (35, 10), (45, 18), (15, 20)] {
        c.set(x, y, light);
    }
    let dark = darken(base, 0.85);
    c.stroke_diamond(dark, 1.5);

    (64, 32, c.into_bytes())
}

pub fn iso_water_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(64, 32);
    let base = Color::from_rgba8(30, 100, 200, 255);
    c.fill_diamond(base);
    let highlight = Color::from_rgba8(80, 150, 230, 255);
    for &(x, y) in &[(20, 12), (25, 12), (40, 16), (41, 16), (30, 20), (31, 20)] {
        c.set(x, y, highlight);
    }
    let dark = darken(base, 0.8);
    c.stroke_diamond(dark, 1.5);
    (64, 32, c.into_bytes())
}

pub fn iso_stone_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(64, 32);
    let base = Color::from_rgba8(140, 140, 140, 255);
    c.fill_diamond(base);
    let dark = darken(base, 0.8);
    c.stroke_diamond(dark, 2.0);
    (64, 32, c.into_bytes())
}

pub fn iso_dirt_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(64, 32);
    let base = Color::from_rgba8(139, 105, 60, 255);
    c.fill_diamond(base);
    let dark = darken(base, 0.8);
    c.stroke_diamond(dark, 1.5);
    (64, 32, c.into_bytes())
}

pub fn iso_tree() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(32, 48);
    let trunk = Color::from_rgba8(101, 67, 33, 255);
    let foliage = Color::from_rgba8(34, 120, 34, 255);
    let foliage_light = Color::from_rgba8(50, 160, 50, 255);

    c.fill_rect(14, 28, 4, 16, trunk);

    c.fill_circle(16, 18, 10, foliage);
    c.fill_circle(13, 14, 6, foliage_light);
    c.fill_circle(19, 20, 5, foliage_light);

    (32, 48, c.into_bytes())
}

pub fn iso_character(body_color: Color) -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 24);
    let skin = Color::from_rgba8(220, 180, 140, 255);
    let eye = Color::BLACK;

    c.fill_circle(8, 5, 4, skin);
    c.set(6, 4, eye);
    c.set(10, 4, eye);

    c.fill_rect(5, 9, 7, 8, body_color);

    c.fill_rect(5, 17, 3, 5, darken(body_color, 0.7));
    c.fill_rect(9, 17, 3, 5, darken(body_color, 0.7));

    (16, 24, c.into_bytes())
}
