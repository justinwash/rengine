use crate::Vec2;
use crate::app::Engine;

use super::{
    NodeHandle2D, Scene2D, SceneScriptHost2D, SceneScriptInputEvent2D, SceneScriptRegistry2D,
    SceneWorld2D,
};

/// Bundles a [`SceneWorld2D`] with its [`SceneScriptHost2D`] and owns all
/// engine-driven input routing so callers don't wire pointer events by hand.
///
/// # Typical usage
///
/// ```ignore
/// // Build once (on_enter):
/// let mut layer = SceneLayer2D::from_scene(&scene, &registry);
/// layer.enter();
///
/// // Every frame (update):
/// layer.update(engine);          // routes pointer input + calls on_update_world
///
/// // Every fixed step (fixed_update):
/// layer.fixed_update(engine);    // calls on_fixed_update_world
///
/// // Render:
/// let hovered = layer.hovered();
/// for handle in layer.world().visible_draw_order() { ... }
/// ```
pub struct SceneLayer2D {
    pub world: SceneWorld2D,
    pub host: SceneScriptHost2D,
    hovered: Option<NodeHandle2D>,
}

impl SceneLayer2D {
    pub fn new(world: SceneWorld2D, host: SceneScriptHost2D) -> Self {
        Self {
            world,
            host,
            hovered: None,
        }
    }

    /// Build a layer from a loaded scene and script registry in one call.
    pub fn from_scene(scene: &Scene2D, registry: &SceneScriptRegistry2D) -> Self {
        let world = SceneWorld2D::from_scene(scene);
        let mut host = SceneScriptHost2D::new();
        host.attach_scene(scene, registry);
        Self::new(world, host)
    }

    /// Call once after construction to fire `on_enter_world` on all scripts.
    pub fn enter(&mut self) {
        self.host.enter_world(&mut self.world);
    }

    /// Route all pointer/scroll/text input from the engine's current frame into
    /// the script host, then call `on_update_world` on every script.
    ///
    /// Call this from your `Scene::update` implementation.
    pub fn update(&mut self, engine: &Engine) {
        let input = engine.input();
        let (mx, my) = input.mouse_position();
        let pos = [mx, my];

        // Update hover state.
        self.hovered = self.world.hit_test(Vec2::new(mx, my));

        // Route pointer move to the topmost hit node.
        self.host.route_input_world(
            &mut self.world,
            &SceneScriptInputEvent2D::PointerMove { position: pos },
        );

        // Left button: route through the click tracker so scripts receive
        // "activate" on a matched press+release over the same node.
        if input.is_mouse_pressed(0) {
            self.host.route_pointer_click(&mut self.world, pos, true);
        }
        if input.is_mouse_released(0) {
            self.host.route_pointer_click(&mut self.world, pos, false);
        }

        // Right and middle buttons: route as raw PointerButton events.
        for button in 1..3usize {
            if input.is_mouse_pressed(button) {
                self.host.route_input_world(
                    &mut self.world,
                    &SceneScriptInputEvent2D::PointerButton {
                        button: button as u8,
                        pressed: true,
                        position: pos,
                    },
                );
            }
            if input.is_mouse_released(button) {
                self.host.route_input_world(
                    &mut self.world,
                    &SceneScriptInputEvent2D::PointerButton {
                        button: button as u8,
                        pressed: false,
                        position: pos,
                    },
                );
            }
        }

        // Scroll: broadcast to the hit node if any, otherwise nothing.
        let (sdx, sdy) = input.scroll_delta();
        if sdx != 0.0 || sdy != 0.0 {
            self.host.route_input_world(
                &mut self.world,
                &SceneScriptInputEvent2D::Scroll { delta: [sdx, sdy] },
            );
        }

        // Text input: broadcast to all scripts.
        let text = input.committed_text();
        if !text.is_empty() {
            self.host.input_world(
                &mut self.world,
                &SceneScriptInputEvent2D::Text(text.to_string()),
            );
        }

        // Script update tick.
        let dt = engine.time().dt();
        self.host.update_world(&mut self.world, dt);
    }

    /// Call this from your `Scene::fixed_update` implementation.
    pub fn fixed_update(&mut self, engine: &Engine) {
        let dt = engine.time().fixed_dt();
        self.host.fixed_update_world(&mut self.world, dt);
    }

    /// The topmost node under the pointer this frame, updated by [`update`].
    pub fn hovered(&self) -> Option<NodeHandle2D> {
        self.hovered
    }

    /// Route a left-button click (press or release) through the host's click
    /// tracker. On a matched press+release over the same node the script bound
    /// to that node receives an `"activate"` event. Returns the activated node
    /// handle on release, if any.
    ///
    /// Useful in tests and for explicit click injection outside of [`update`].
    pub fn route_click(&mut self, position: [f32; 2], pressed: bool) -> Option<NodeHandle2D> {
        self.host.route_pointer_click(&mut self.world, position, pressed)
    }

    pub fn world(&self) -> &SceneWorld2D {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut SceneWorld2D {
        &mut self.world
    }

    pub fn host(&self) -> &SceneScriptHost2D {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut SceneScriptHost2D {
        &mut self.host
    }
}
