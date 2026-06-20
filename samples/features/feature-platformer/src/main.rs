//! Feature: 2D platformer physics.
//!
//! A controllable character driven by [`KinematicBody2D`] against a set of
//! static solids: gravity, run, jump, and flush wall/floor/ceiling collisions.
//! This is the worked example for the engine's 2D character-controller physics
//! (`move_and_collide` + `KinematicBody2D`).
//!
//! All coordinates are centered and y-up, matching both the physics module's
//! [`Rect`] convention and the canvas, so bodies and solids render directly.

use rengine::*;

const MOVE_SPEED: f32 = 240.0;
const JUMP_SPEED: f32 = 430.0;
const RESPAWN_BELOW_Y: f32 = -400.0;

/// The player's starting body — placed above the left platform so it visibly
/// falls and lands when the sample boots.
fn initial_player() -> KinematicBody2D {
    KinematicBody2D::new(Rect::new(-120.0, 140.0, 28.0, 28.0))
}

/// The static level geometry: ground, a solid platform, a one-way (drop-through)
/// platform, and side walls.
fn level_solids() -> Vec<Solid2D> {
    vec![
        Solid2D::solid(Rect::new(-360.0, -200.0, 720.0, 30.0)), // ground
        Solid2D::solid(Rect::new(-180.0, -110.0, 160.0, 24.0)), // left platform
        Solid2D::one_way(Rect::new(60.0, -40.0, 160.0, 12.0)),  // right: drop-through
        Solid2D::solid(Rect::new(-360.0, -200.0, 24.0, 380.0)), // left wall
        Solid2D::solid(Rect::new(336.0, -200.0, 24.0, 380.0)),  // right wall
    ]
}

struct PlatformerScene {
    player: KinematicBody2D,
    solids: Vec<Solid2D>,
}

impl PlatformerScene {
    fn new() -> Self {
        Self {
            player: initial_player(),
            solids: level_solids(),
        }
    }
}

impl Scene for PlatformerScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let input = engine.input();
        // Clamp dt so a stall can't produce a step large enough to tunnel
        // through the (non-swept) collider.
        let dt = engine.dt().min(1.0 / 30.0);

        let mut vx = 0.0;
        if input.is_key_down(KeyCode::ArrowLeft) || input.is_key_down(KeyCode::KeyA) {
            vx -= MOVE_SPEED;
        }
        if input.is_key_down(KeyCode::ArrowRight) || input.is_key_down(KeyCode::KeyD) {
            vx += MOVE_SPEED;
        }
        self.player.velocity.x = vx;

        let jump = input.is_key_pressed(KeyCode::Space) || input.is_key_pressed(KeyCode::ArrowUp);
        if jump && self.player.on_ground() {
            self.player.velocity.y = JUMP_SPEED;
        }

        self.player.step_solids(dt, &self.solids);

        if self.player.bounds.y < RESPAWN_BELOW_Y {
            self.player = initial_player();
        }

        if input.is_key_pressed(KeyCode::Escape) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(20, 22, 30, 255);

        let (w, h) = engine.window_size();
        let hw = w as f32 / 2.0;
        let hh = h as f32 / 2.0;
        let canvas = frame.canvas(0);

        canvas.text(
            -hw + 20.0,
            hh - 28.0,
            "Platformer physics: A/D or arrows to move, Space/Up to jump (drop-through the thin platform), Esc to quit",
            15.0,
            Color::WHITE,
        );

        for solid in &self.solids {
            let r = solid.rect;
            let color = if solid.one_way {
                Color::from_rgba8(110, 130, 90, 255)
            } else {
                Color::from_rgba8(70, 80, 100, 255)
            };
            canvas.rect(r.x, r.y, r.width, r.height, color);
        }

        let body = &self.player.bounds;
        let color = if self.player.on_ground() {
            Color::from_rgba8(120, 200, 140, 255)
        } else {
            Color::from_rgba8(230, 180, 90, 255)
        };
        canvas.rect(body.x, body.y, body.width, body.height, color);
    }
}

fn main() {
    rengine::run_with_scenes(
        EngineConfig {
            title: "Feature: Platformer Physics".into(),
            width: 800,
            height: 600,
            show_fps: false,
            ..Default::default()
        },
        |_engine, _globals| Box::new(PlatformerScene::new()),
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = 1.0 / 60.0;

    fn settle(player: &mut KinematicBody2D, solids: &[Solid2D], frames: usize) {
        for _ in 0..frames {
            player.step_solids(STEP, solids);
        }
    }

    #[test]
    fn player_falls_and_lands_on_the_left_platform() {
        let mut player = initial_player();
        let solids = level_solids();
        settle(&mut player, &solids, 240);

        assert!(player.on_ground());
        assert!(player.velocity.y.abs() < 1.0);
        // The left platform's top is at y = -86; the body rests on it.
        assert!((player.bounds.y - (-86.0)).abs() < 1e-2);
    }

    #[test]
    fn jumping_leaves_the_ground() {
        let mut player = initial_player();
        let solids = level_solids();
        settle(&mut player, &solids, 240);
        assert!(player.on_ground());

        player.velocity.y = JUMP_SPEED;
        player.step_solids(STEP, &solids);

        assert!(!player.on_ground());
        assert!(player.velocity.y > 0.0);
    }

    #[test]
    fn can_jump_up_through_and_land_on_the_one_way_platform() {
        let solids = level_solids();
        // Start just below the one-way platform (top at y = -28), rising fast.
        let mut player = KinematicBody2D::new(Rect::new(120.0, -90.0, 28.0, 28.0))
            .with_velocity(Vec2::new(0.0, JUMP_SPEED));

        // Rising: the body passes through the drop-through platform.
        player.step_solids(STEP, &solids);
        assert!(!player.contacts.top);

        // Falling back down: it lands on top of that same platform.
        settle(&mut player, &solids, 240);
        assert!(player.on_ground());
        assert!((player.bounds.y - (-28.0)).abs() < 1e-2);
    }

    #[test]
    fn running_into_the_right_wall_is_blocked() {
        let mut player = initial_player();
        let solids = level_solids();
        settle(&mut player, &solids, 240);

        // Drive right for plenty of time; gravity keeps it grounded en route.
        for _ in 0..1200 {
            player.velocity.x = MOVE_SPEED;
            player.step_solids(STEP, &solids);
        }

        // The right wall's left edge is x = 336; the body cannot pass it.
        assert!(player.bounds.x + player.bounds.width <= 336.0 + 1e-2);
        assert!(player.contacts.right);
    }
}
