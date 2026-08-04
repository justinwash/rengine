#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const ORANGE: Color = Color {
        r: 1.0,
        g: 0.498,
        b: 0.0,
        a: 1.0,
    };
    pub const YELLOW: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const INDIGO: Color = Color {
        r: 0.294,
        g: 0.0,
        b: 0.510,
        a: 1.0,
    };
    pub const VIOLET: Color = Color {
        r: 0.580,
        g: 0.0,
        b: 0.827,
        a: 1.0,
    };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Construct a colour from 8-bit **sRGB** (display-space) channels.
    ///
    /// [`Color::from_rgba8`] treats its inputs as already-linear, so dark values
    /// wash out once the renderer applies sRGB gamma on output. This applies the
    /// sRGB→linear transfer first, so the colour appears on screen as authored.
    /// Alpha is treated as linear.
    pub fn from_srgb8(r: u8, g: u8, b: u8, a: u8) -> Self {
        fn to_linear(c: u8) -> f32 {
            let s = c as f32 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        Self {
            r: to_linear(r),
            g: to_linear(g),
            b: to_linear(b),
            a: a as f32 / 255.0,
        }
    }

    /// The inverse of [`from_srgb8`](Self::from_srgb8): 8-bit sRGB channels
    /// back out of a linear colour. Round-trips exactly for any value that came
    /// from `from_srgb8`. Needed anywhere a colour has to be written back as
    /// display-space text or bytes — a scene file's `ui_color`, a captured
    /// frame, a debug readout.
    pub fn to_srgb8(self) -> (u8, u8, u8, u8) {
        fn to_srgb(linear: f32) -> u8 {
            let s = if linear <= 0.003_130_8 {
                linear * 12.92
            } else {
                1.055 * linear.max(0.0).powf(1.0 / 2.4) - 0.055
            };
            (s * 255.0).round().clamp(0.0, 255.0) as u8
        }
        (
            to_srgb(self.r),
            to_srgb(self.g),
            to_srgb(self.b),
            (self.a * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }

    pub fn lerp(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn to_srgb8_round_trips_from_srgb8() {
        // Every byte, including the linear-segment values below 0.04045 where
        // the transfer curve changes shape — those are the darks `from_srgb8`
        // exists to get right, so they are exactly the ones worth pinning.
        for v in 0..=255u8 {
            let (r, g, b, a) = Color::from_srgb8(v, v, v, v).to_srgb8();
            assert_eq!((r, g, b, a), (v, v, v, v), "channel value {v}");
        }
    }
}
