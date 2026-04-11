use crate::math::rect::Rect;
use glam::Vec2;

/// Bitmask-based collision layer for filtering which objects can collide.
///
/// Each body has a `layer` (which groups it belongs to) and a `mask` (which
/// groups it interacts with). Two bodies can collide when
/// `a.layer & b.mask != 0 && b.layer & a.mask != 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionLayer {
    /// Which groups this body belongs to.
    pub layer: u32,
    /// Which groups this body can collide with.
    pub mask: u32,
}

impl CollisionLayer {
    pub const NONE: u32 = 0;
    pub const WORLD: u32 = 1 << 0;
    pub const PLAYER: u32 = 1 << 1;
    pub const ENEMY: u32 = 1 << 2;
    pub const PROJECTILE: u32 = 1 << 3;
    pub const TRIGGER: u32 = 1 << 4;
    pub const UI: u32 = 1 << 5;

    /// Create a collision layer that belongs to `layer` groups and collides
    /// with `mask` groups.
    pub const fn new(layer: u32, mask: u32) -> Self {
        Self { layer, mask }
    }

    /// Shorthand: belongs to and collides with the same groups.
    pub const fn symmetric(bits: u32) -> Self {
        Self {
            layer: bits,
            mask: bits,
        }
    }

    /// Returns `true` if these two layers should interact.
    pub const fn interacts_with(&self, other: &CollisionLayer) -> bool {
        ((self.layer & other.mask) != 0) && ((other.layer & self.mask) != 0)
    }
}

impl Default for CollisionLayer {
    fn default() -> Self {
        Self {
            layer: u32::MAX,
            mask: u32::MAX,
        }
    }
}

/// AABB overlap with layer/mask filtering. Returns the MTV only when the two
/// layers interact and the rectangles overlap spatially.
pub fn aabb_overlap_layered(
    a: &Rect,
    a_layer: &CollisionLayer,
    b: &Rect,
    b_layer: &CollisionLayer,
) -> Option<Vec2> {
    if !a_layer.interacts_with(b_layer) {
        return None;
    }
    aabb_overlap(a, b)
}


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
