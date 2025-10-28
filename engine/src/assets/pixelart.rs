use super::color::Color;


pub struct PixelCanvas {
    pub width: u32,
    pub height: u32,
    pixels: Vec<[u8; 4]>,
}

impl PixelCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0, 0, 0, 0]; (width * height) as usize],
        }
    }

    pub fn fill(&mut self, color: Color) {
        let c = Self::col8(color);
        self.pixels.fill(c);
    }

    pub fn set(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            self.pixels[(y as u32 * self.width + x as u32) as usize] = Self::col8(color);
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        let c = Self::col8(color);
        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && py >= 0 && (px as u32) < self.width && (py as u32) < self.height {
                    self.pixels[(py as u32 * self.width + px as u32) as usize] = c;
                }
            }
        }
    }


    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color) {
        let c = Self::col8(color);
        let r2 = (radius * radius) as f32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if (dx * dx + dy * dy) as f32 <= r2 {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && py >= 0 && (px as u32) < self.width && (py as u32) < self.height {
                        self.pixels[(py as u32 * self.width + px as u32) as usize] = c;
                    }
                }
            }
        }
    }


    pub fn fill_diamond(&mut self, color: Color) {
        let c = Self::col8(color);
        let hw = self.width as i32 / 2;
        let hh = self.height as i32 / 2;
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {

                let dx = (x - hw).abs() as f32 / hw as f32;
                let dy = (y - hh).abs() as f32 / hh as f32;
                if dx + dy <= 1.0 {
                    self.pixels[(y as u32 * self.width + x as u32) as usize] = c;
                }
            }
        }
    }


    pub fn stroke_diamond(&mut self, color: Color, thickness: f32) {
        let c = Self::col8(color);
        let hw = self.width as f32 / 2.0;
        let hh = self.height as f32 / 2.0;
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let dx = (x as f32 - hw).abs() / hw;
                let dy = (y as f32 - hh).abs() / hh;
                let d = dx + dy;
                if d <= 1.0 && d >= 1.0 - thickness / hw.min(hh) {
                    self.pixels[(y as u32 * self.width + x as u32) as usize] = c;
                }
            }
        }
    }


    pub fn into_bytes(self) -> Vec<u8> {
        self.pixels
            .into_iter()
            .flat_map(|p| p.into_iter())
            .collect()
    }

    fn col8(c: Color) -> [u8; 4] {
        [
            (c.r.clamp(0.0, 1.0) * 255.0) as u8,
            (c.g.clamp(0.0, 1.0) * 255.0) as u8,
            (c.b.clamp(0.0, 1.0) * 255.0) as u8,
            (c.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }
}


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


pub fn darken(c: Color, factor: f32) -> Color {
    Color::new(c.r * factor, c.g * factor, c.b * factor, c.a)
}

pub fn lighten(c: Color, factor: f32) -> Color {
    Color::new(
        (c.r * factor).min(1.0),
        (c.g * factor).min(1.0),
        (c.b * factor).min(1.0),
        c.a,
    )
}


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
