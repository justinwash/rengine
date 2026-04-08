use rengine::pixelart::{darken, lighten, PixelCanvas};
use rengine::Color;

pub fn character_topdown(
    body_color: Color,
    skin_color: Color,
    eye_color: Color,
) -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);

    c.fill_circle(8, 4, 3, skin_color);

    c.set(7, 3, eye_color);
    c.set(9, 3, eye_color);

    c.fill_rect(5, 7, 6, 6, body_color);

    c.fill_rect(3, 7, 2, 5, body_color);
    c.fill_rect(11, 7, 2, 5, body_color);

    c.fill_rect(5, 13, 2, 3, darken(body_color, 0.7));
    c.fill_rect(9, 13, 2, 3, darken(body_color, 0.7));

    (16, 16, c.into_bytes())
}

pub fn grass_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let base = Color::from_rgba8(76, 153, 0, 255);
    c.fill(base);

    let dark = darken(base, 0.8);
    for &(x, y) in &[(2, 3), (7, 1), (12, 5), (4, 10), (9, 12), (14, 8), (1, 14)] {
        c.set(x, y, dark);
        c.set(x + 1, y, dark);
    }

    let light = lighten(base, 1.2);
    for &(x, y) in &[(5, 6), (10, 3), (3, 13), (11, 11), (7, 8)] {
        c.set(x, y, light);
    }
    (16, 16, c.into_bytes())
}

pub fn dirt_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let base = Color::from_rgba8(139, 105, 60, 255);
    c.fill(base);
    let dark = darken(base, 0.85);
    let light = lighten(base, 1.15);
    for &(x, y) in &[(3, 2), (8, 5), (12, 9), (1, 11), (6, 14), (14, 3)] {
        c.set(x, y, dark);
    }
    for &(x, y) in &[(5, 7), (10, 1), (2, 13), (13, 12)] {
        c.set(x, y, light);
    }
    (16, 16, c.into_bytes())
}

pub fn water_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let base = Color::from_rgba8(30, 100, 200, 255);
    c.fill(base);
    let highlight = Color::from_rgba8(80, 150, 230, 255);

    for &(x, y) in &[
        (2, 4),
        (3, 4),
        (8, 3),
        (9, 3),
        (5, 9),
        (6, 9),
        (12, 8),
        (13, 8),
        (1, 13),
        (2, 13),
    ] {
        c.set(x, y, highlight);
    }
    (16, 16, c.into_bytes())
}

pub fn stone_tile() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let base = Color::from_rgba8(128, 128, 128, 255);
    c.fill(base);

    let dark = Color::from_rgba8(90, 90, 90, 255);
    c.fill_rect(0, 7, 16, 1, dark);
    c.fill_rect(0, 15, 16, 1, dark);
    c.fill_rect(4, 0, 1, 8, dark);
    c.fill_rect(12, 0, 1, 8, dark);
    c.fill_rect(0, 8, 1, 8, dark);
    c.fill_rect(8, 8, 1, 8, dark);
    (16, 16, c.into_bytes())
}

pub fn tree_top() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let trunk = Color::from_rgba8(101, 67, 33, 255);
    let foliage = Color::from_rgba8(34, 120, 34, 255);
    let foliage_light = Color::from_rgba8(50, 160, 50, 255);

    c.fill_rect(7, 12, 3, 4, trunk);

    c.fill_circle(8, 7, 6, foliage);
    c.fill_circle(6, 5, 3, foliage_light);

    (16, 16, c.into_bytes())
}

pub fn enemy_topdown() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let body = Color::from_rgba8(200, 50, 50, 255);
    let skin = Color::from_rgba8(180, 130, 100, 255);
    let eye = Color::from_rgba8(255, 255, 0, 255);

    c.fill_circle(8, 4, 3, skin);
    c.set(7, 3, eye);
    c.set(9, 3, eye);
    c.fill_rect(5, 7, 6, 6, body);
    c.fill_rect(3, 7, 2, 5, body);
    c.fill_rect(11, 7, 2, 5, body);
    c.fill_rect(5, 13, 2, 3, darken(body, 0.7));
    c.fill_rect(9, 13, 2, 3, darken(body, 0.7));

    (16, 16, c.into_bytes())
}

pub fn gem_sprite() -> (u32, u32, Vec<u8>) {
    let mut c = PixelCanvas::new(16, 16);
    let gem = Color::from_rgba8(255, 215, 0, 255);
    let highlight = Color::from_rgba8(255, 255, 180, 255);

    c.fill_circle(8, 8, 5, gem);
    c.fill_circle(6, 6, 2, highlight);

    (16, 16, c.into_bytes())
}
