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
