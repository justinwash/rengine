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

/// Returns the minimum translation vector to push circle A out of circle B,
/// or `None` if the circles do not overlap. The MTV points from B toward A.
pub fn circle_overlap(
    center_a: Vec2,
    radius_a: f32,
    center_b: Vec2,
    radius_b: f32,
) -> Option<Vec2> {
    let diff = center_a - center_b;
    let dist_sq = diff.length_squared();
    let min_dist = radius_a + radius_b;

    if dist_sq >= min_dist * min_dist || dist_sq < 1e-10 {
        return None;
    }

    let dist = dist_sq.sqrt();
    let penetration = min_dist - dist;
    let normal = diff / dist;
    Some(normal * penetration)
}
