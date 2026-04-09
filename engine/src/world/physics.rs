use crate::math::rect::Rect;
use glam::Vec2;


pub fn aabb_overlap(a: &Rect, b: &Rect) -> Option<Vec2> {
    let overlap_x = f32::min(a.right(), b.right()) - f32::max(a.left(), b.left());
    let overlap_y = f32::min(a.top(), b.top()) - f32::max(a.bottom(), b.bottom());

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    let center_a = a.center();
    let center_b = b.center();

    if overlap_x < overlap_y {
        let sign = if center_a.x < center_b.x { -1.0 } else { 1.0 };
        Some(Vec2::new(sign * overlap_x, 0.0))
    } else {
        let sign = if center_a.y < center_b.y { -1.0 } else { 1.0 };
        Some(Vec2::new(0.0, sign * overlap_y))
    }
}
