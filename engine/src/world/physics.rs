use crate::math::rect::Rect;
use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionLayer {
    pub layer: u32,
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

    pub const fn new(layer: u32, mask: u32) -> Self {
        Self { layer, mask }
    }

    pub const fn symmetric(bits: u32) -> Self {
        Self {
            layer: bits,
            mask: bits,
        }
    }

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

/// The faces of a moving body that came into contact with a solid during a
/// [`move_and_collide`] resolution. Coordinates follow [`Rect`]'s y-up
/// convention, so `bottom` is a floor/ground contact and `top` a ceiling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Contacts2D {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Contacts2D {
    pub fn any(&self) -> bool {
        self.left || self.right || self.top || self.bottom
    }
}

/// Result of moving an AABB against a set of static solids.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveResult2D {
    /// The resolved bottom-left position of the body after collision response.
    pub position: Vec2,
    pub contacts: Contacts2D,
}

/// A static collider for [`move_and_collide_solids`].
///
/// A plain solid blocks from every direction. A `one_way` solid (a drop-through
/// platform) only stops a body landing on it from above: the body passes freely
/// through it horizontally and when moving upward from below.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Solid2D {
    pub rect: Rect,
    pub one_way: bool,
}

impl Solid2D {
    /// A fully solid collider that blocks from all sides.
    pub fn solid(rect: Rect) -> Self {
        Self {
            rect,
            one_way: false,
        }
    }

    /// A one-way (drop-through) platform that only stops a downward landing.
    pub fn one_way(rect: Rect) -> Self {
        Self {
            rect,
            one_way: true,
        }
    }
}

impl From<Rect> for Solid2D {
    fn from(rect: Rect) -> Self {
        Self::solid(rect)
    }
}

/// Move an axis-aligned `body` by `motion` against static `solids`, resolving
/// overlaps axis-by-axis (X then Y) and reporting which faces made contact.
///
/// Resolution snaps the body flush against blocking solids using the strict
/// touching semantics of [`Rect::overlaps`], so sliding along a row of tiles
/// does not snag on the seams between them. This is the classic AABB platformer
/// mover; it does not perform swept (time-of-impact) tests, so motion larger
/// than a solid in a single step can tunnel through it — keep per-step motion
/// below your smallest solid (or substep) for very fast bodies.
///
/// See [`move_and_collide_solids`] for one-way (drop-through) platform support.
pub fn move_and_collide(body: Rect, motion: Vec2, solids: &[Rect]) -> MoveResult2D {
    let mut rect = body;
    let mut contacts = Contacts2D::default();

    // X axis first.
    rect.x += motion.x;
    if motion.x != 0.0 {
        for solid in solids {
            if rect.overlaps(solid) {
                if motion.x > 0.0 {
                    rect.x = solid.left() - rect.width;
                    contacts.right = true;
                } else {
                    rect.x = solid.right();
                    contacts.left = true;
                }
            }
        }
    }

    // Then Y axis (y-up: positive motion moves upward toward ceilings).
    rect.y += motion.y;
    if motion.y != 0.0 {
        for solid in solids {
            if rect.overlaps(solid) {
                if motion.y > 0.0 {
                    rect.y = solid.bottom() - rect.height;
                    contacts.top = true;
                } else {
                    rect.y = solid.top();
                    contacts.bottom = true;
                }
            }
        }
    }

    MoveResult2D {
        position: Vec2::new(rect.x, rect.y),
        contacts,
    }
}

/// Like [`move_and_collide`] but against [`Solid2D`] colliders, so some can be
/// one-way (drop-through) platforms.
///
/// One-way solids are skipped during horizontal resolution and when the body is
/// moving upward, and they only stop a downward landing when the body was at or
/// above the platform's top before this step — so a body can jump up through a
/// drop-through platform and land on it, but never get shoved sideways or popped
/// up by one.
pub fn move_and_collide_solids(body: Rect, motion: Vec2, solids: &[Solid2D]) -> MoveResult2D {
    let mut rect = body;
    let mut contacts = Contacts2D::default();

    // X axis first — one-way platforms never block horizontal motion.
    rect.x += motion.x;
    if motion.x != 0.0 {
        for solid in solids {
            if solid.one_way {
                continue;
            }
            if rect.overlaps(&solid.rect) {
                if motion.x > 0.0 {
                    rect.x = solid.rect.left() - rect.width;
                    contacts.right = true;
                } else {
                    rect.x = solid.rect.right();
                    contacts.left = true;
                }
            }
        }
    }

    // Then Y axis. Capture the pre-move bottom so a one-way platform only
    // catches a body that was above it (not one the body is rising through).
    let pre_move_bottom = rect.y;
    rect.y += motion.y;
    if motion.y != 0.0 {
        for solid in solids {
            if solid.one_way && !(motion.y < 0.0 && pre_move_bottom >= solid.rect.top()) {
                continue;
            }
            if rect.overlaps(&solid.rect) {
                if motion.y > 0.0 {
                    rect.y = solid.rect.bottom() - rect.height;
                    contacts.top = true;
                } else {
                    rect.y = solid.rect.top();
                    contacts.bottom = true;
                }
            }
        }
    }

    MoveResult2D {
        position: Vec2::new(rect.x, rect.y),
        contacts,
    }
}

/// A simple kinematic AABB body with gravity, integrated against static solids.
///
/// This is the minimal "character controller" primitive for platformers and
/// top-down games: set [`KinematicBody2D::velocity`] each frame (jump impulses,
/// horizontal input) and call [`KinematicBody2D::step`] to integrate, resolve
/// collisions, and learn about ground/wall/ceiling contacts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinematicBody2D {
    pub bounds: Rect,
    pub velocity: Vec2,
    pub gravity: Vec2,
    pub contacts: Contacts2D,
}

impl KinematicBody2D {
    /// A body with a default downward gravity (y-up world).
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            velocity: Vec2::ZERO,
            gravity: Vec2::new(0.0, -980.0),
            contacts: Contacts2D::default(),
        }
    }

    pub fn with_gravity(mut self, gravity: Vec2) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    /// True when the body's last [`KinematicBody2D::step`] landed on a floor.
    pub fn on_ground(&self) -> bool {
        self.contacts.bottom
    }

    /// Advance one timestep against fully-solid `solids`: apply gravity,
    /// integrate velocity, resolve, and zero the velocity components that ran
    /// into a solid so the body rests (rather than accumulating force).
    pub fn step(&mut self, dt: f32, solids: &[Rect]) {
        let motion = self.integrate(dt);
        let result = move_and_collide(self.bounds, motion, solids);
        self.apply_move_result(result);
    }

    /// Like [`KinematicBody2D::step`] but against [`Solid2D`] colliders, so the
    /// level can include one-way (drop-through) platforms.
    pub fn step_solids(&mut self, dt: f32, solids: &[Solid2D]) {
        let motion = self.integrate(dt);
        let result = move_and_collide_solids(self.bounds, motion, solids);
        self.apply_move_result(result);
    }

    fn integrate(&mut self, dt: f32) -> Vec2 {
        self.velocity += self.gravity * dt;
        self.velocity * dt
    }

    fn apply_move_result(&mut self, result: MoveResult2D) {
        self.bounds.x = result.position.x;
        self.bounds.y = result.position.y;
        self.contacts = result.contacts;

        if result.contacts.left || result.contacts.right {
            self.velocity.x = 0.0;
        }
        if result.contacts.top || result.contacts.bottom {
            self.velocity.y = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(x, y, w, h)
    }

    #[test]
    fn move_into_wall_stops_flush_and_reports_contact() {
        let body = rect(0.0, 0.0, 10.0, 10.0);
        let wall = rect(15.0, 0.0, 10.0, 100.0);

        let result = move_and_collide(body, Vec2::new(20.0, 0.0), &[wall]);

        assert!(result.contacts.right);
        assert!(!result.contacts.left);
        // The body's right edge rests flush against the wall's left edge.
        assert!((result.position.x + 10.0 - wall.left()).abs() < 1e-3);
    }

    #[test]
    fn falling_body_lands_and_rests_on_floor() {
        let floor = rect(-100.0, 0.0, 200.0, 10.0); // top at y = 10
        let mut body = KinematicBody2D::new(rect(0.0, 50.0, 10.0, 10.0));

        for _ in 0..240 {
            body.step(1.0 / 60.0, &[floor]);
        }

        assert!(body.on_ground());
        assert!((body.bounds.y - floor.top()).abs() < 1e-2);
        assert!(body.velocity.y.abs() < 1e-3);
    }

    #[test]
    fn rising_body_stops_under_ceiling() {
        let ceiling = rect(-100.0, 15.0, 200.0, 10.0); // bottom at y = 15
        let mut body = KinematicBody2D::new(rect(0.0, 0.0, 10.0, 10.0))
            .with_gravity(Vec2::ZERO)
            .with_velocity(Vec2::new(0.0, 600.0)); // 10px of motion at 1/60s

        body.step(1.0 / 60.0, &[ceiling]);

        assert!(body.contacts.top);
        // The body's top edge rests flush against the ceiling's bottom edge.
        assert!((body.bounds.y + 10.0 - ceiling.bottom()).abs() < 1e-3);
        assert!(body.velocity.y.abs() < 1e-3);
    }

    #[test]
    fn free_fall_without_solids_accelerates_downward() {
        let mut body = KinematicBody2D::new(rect(0.0, 0.0, 10.0, 10.0));
        let start_y = body.bounds.y;

        body.step(1.0 / 60.0, &[]);

        assert!(body.bounds.y < start_y);
        assert!(!body.on_ground());
        assert!(body.velocity.y < 0.0);
    }

    #[test]
    fn one_way_platform_catches_a_downward_landing() {
        let platform = Solid2D::one_way(rect(-50.0, 0.0, 100.0, 8.0)); // top at y = 8
        let mut body = KinematicBody2D::new(rect(0.0, 40.0, 10.0, 10.0));

        for _ in 0..120 {
            body.step_solids(1.0 / 60.0, &[platform]);
        }

        assert!(body.on_ground());
        assert!((body.bounds.y - platform.rect.top()).abs() < 1e-2);
    }

    #[test]
    fn one_way_platform_is_passable_from_below() {
        let platform = Solid2D::one_way(rect(-50.0, 30.0, 100.0, 8.0));
        // Start below the platform, moving fast upward, no gravity.
        let mut body = KinematicBody2D::new(rect(0.0, 0.0, 10.0, 10.0))
            .with_gravity(Vec2::ZERO)
            .with_velocity(Vec2::new(0.0, 600.0));

        body.step_solids(1.0 / 60.0, &[platform]);

        // It rises through without a ceiling contact and ends above its start.
        assert!(!body.contacts.top);
        assert!(body.bounds.y > 0.0);
    }

    #[test]
    fn one_way_platform_never_blocks_horizontal_motion() {
        let platform = Solid2D::one_way(rect(0.0, 0.0, 40.0, 40.0));
        let body = rect(-20.0, 5.0, 10.0, 10.0);

        let result = move_and_collide_solids(body, Vec2::new(40.0, 0.0), &[platform]);

        assert!(!result.contacts.right);
        assert!((result.position.x - 20.0).abs() < 1e-3); // moved freely through
    }

    #[test]
    fn solid_collider_still_blocks_from_all_sides() {
        let wall = Solid2D::solid(rect(15.0, 0.0, 10.0, 100.0));
        let result =
            move_and_collide_solids(rect(0.0, 0.0, 10.0, 10.0), Vec2::new(20.0, 0.0), &[wall]);

        assert!(result.contacts.right);
        assert!((result.position.x + 10.0 - wall.rect.left()).abs() < 1e-3);
    }
}
