use super::sprite::DrawParams;
use super::texture::TextureId;
use crate::assets::Color;
use glam::Vec2;

#[derive(Debug, Clone)]
pub struct NineSlice {
    pub texture: TextureId,
    pub texture_width: u32,
    pub texture_height: u32,
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
    pub color: Color,
    pub z_order: i32,
}

impl NineSlice {
    pub fn new(
        texture: TextureId,
        texture_width: u32,
        texture_height: u32,
        left: u32,
        right: u32,
        top: u32,
        bottom: u32,
    ) -> Self {
        assert!(texture_width > 0, "NineSlice texture_width must be > 0");
        assert!(texture_height > 0, "NineSlice texture_height must be > 0");
        assert!(
            left + right <= texture_width,
            "NineSlice left + right ({} + {}) exceeds texture_width ({})",
            left,
            right,
            texture_width
        );
        assert!(
            top + bottom <= texture_height,
            "NineSlice top + bottom ({} + {}) exceeds texture_height ({})",
            top,
            bottom,
            texture_height
        );
        Self {
            texture,
            texture_width,
            texture_height,
            left,
            right,
            top,
            bottom,
            color: Color::WHITE,
            z_order: 0,
        }
    }

    pub fn uniform(
        texture: TextureId,
        texture_width: u32,
        texture_height: u32,
        border: u32,
    ) -> Self {
        Self::new(
            texture,
            texture_width,
            texture_height,
            border,
            border,
            border,
            border,
        )
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_z_order(mut self, z: i32) -> Self {
        self.z_order = z;
        self
    }

    pub fn patches(&self, position: Vec2, size: Vec2) -> Vec<DrawParams> {
        let mut out = Vec::with_capacity(9);
        self.patches_into(position, size, &mut out);
        out
    }

    pub fn patches_into(&self, position: Vec2, size: Vec2, out: &mut Vec<DrawParams>) {
        let tw = self.texture_width as f32;
        let th = self.texture_height as f32;
        let l = self.left as f32;
        let r = self.right as f32;
        let t = self.top as f32;
        let b = self.bottom as f32;

        let width = size.x.max(0.0);
        let height = size.y.max(0.0);

        let h_sum = l + r;
        let h_scale = if h_sum > 0.0 && width < h_sum {
            width / h_sum
        } else {
            1.0
        };
        let left_dest = l * h_scale;
        let right_dest = r * h_scale;

        let v_sum = t + b;
        let v_scale = if v_sum > 0.0 && height < v_sum {
            height / v_sum
        } else {
            1.0
        };
        let top_dest = t * v_scale;
        let bottom_dest = b * v_scale;

        let x0 = position.x;
        let x1 = x0 + left_dest;
        let x2 = (x0 + width - right_dest).max(x1);
        let x3 = x0 + width;

        let y0 = position.y;
        let y1 = y0 + bottom_dest;
        let y2 = (y0 + height - top_dest).max(y1);
        let y3 = y0 + height;

        let ul = l / tw;
        let ur = (tw - r) / tw;
        let vt = t / th;
        let vb = (th - b) / th;

        let cells: [(f32, f32, f32, f32, f32, f32, f32, f32); 9] = [
            (x0, y0, x1 - x0, y1 - y0, 0.0, vb, ul, 1.0 - vb),
            (x1, y0, x2 - x1, y1 - y0, ul, vb, ur - ul, 1.0 - vb),
            (x2, y0, x3 - x2, y1 - y0, ur, vb, 1.0 - ur, 1.0 - vb),
            (x0, y1, x1 - x0, y2 - y1, 0.0, vt, ul, vb - vt),
            (x1, y1, x2 - x1, y2 - y1, ul, vt, ur - ul, vb - vt),
            (x2, y1, x3 - x2, y2 - y1, ur, vt, 1.0 - ur, vb - vt),
            (x0, y2, x1 - x0, y3 - y2, 0.0, 0.0, ul, vt),
            (x1, y2, x2 - x1, y3 - y2, ul, 0.0, ur - ul, vt),
            (x2, y2, x3 - x2, y3 - y2, ur, 0.0, 1.0 - ur, vt),
        ];

        for (x, y, w, h, u0, v0, uw, vh) in cells {
            if w > 0.0 && h > 0.0 {
                out.push(
                    DrawParams::new(self.texture, Vec2::new(x, y), Vec2::new(w, h))
                        .with_uv_rect([u0, v0, uw, vh])
                        .with_color(self.color)
                        .with_z_order(self.z_order),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_count() {
        let ns = NineSlice::uniform(TextureId(1), 32, 32, 8);
        let patches = ns.patches(Vec2::ZERO, Vec2::new(100.0, 80.0));
        assert_eq!(patches.len(), 9);
    }

    #[test]
    fn corners_are_fixed_size() {
        let ns = NineSlice::new(TextureId(1), 64, 64, 10, 12, 8, 6);
        let patches = ns.patches(Vec2::new(50.0, 50.0), Vec2::new(200.0, 150.0));

        let bl = &patches[0];
        assert_eq!(bl.position, Vec2::new(50.0, 50.0));
        assert_eq!(bl.size, Vec2::new(10.0, 6.0));

        let br = &patches[2];
        assert_eq!(br.position, Vec2::new(238.0, 50.0));
        assert_eq!(br.size, Vec2::new(12.0, 6.0));

        let tl = &patches[6];
        assert_eq!(tl.position, Vec2::new(50.0, 192.0));
        assert_eq!(tl.size, Vec2::new(10.0, 8.0));

        let tr = &patches[8];
        assert_eq!(tr.position, Vec2::new(238.0, 192.0));
        assert_eq!(tr.size, Vec2::new(12.0, 8.0));
    }

    #[test]
    fn uv_corners_correct() {
        let ns = NineSlice::uniform(TextureId(1), 32, 32, 8);
        let patches = ns.patches(Vec2::ZERO, Vec2::new(100.0, 80.0));

        assert_eq!(patches[0].uv_rect, [0.0, 0.75, 0.25, 0.25]);

        assert_eq!(patches[4].uv_rect, [0.25, 0.25, 0.5, 0.5]);

        assert_eq!(patches[8].uv_rect, [0.75, 0.0, 0.25, 0.25]);
    }

    #[test]
    fn minimum_size_clamps() {
        let ns = NineSlice::uniform(TextureId(1), 32, 32, 8);
        let patches = ns.patches(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(patches.len() <= 9);
        for p in &patches {
            assert!(p.size.x > 0.0);
            assert!(p.size.y > 0.0);
        }
    }

    #[test]
    fn color_and_z_propagate() {
        let ns = NineSlice::uniform(TextureId(1), 32, 32, 8)
            .with_color(Color::new(1.0, 0.0, 0.0, 1.0))
            .with_z_order(5);
        let patches = ns.patches(Vec2::ZERO, Vec2::new(100.0, 80.0));
        for p in &patches {
            assert_eq!(p.color, Color::new(1.0, 0.0, 0.0, 1.0));
            assert_eq!(p.z_order, 5);
        }
    }
}
