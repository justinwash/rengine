//! Feature: Scene-driven scripts, hit-testing, and live-world mutation.
//!
//! This sample wires together the runtime scene stack added in the
//! scene-driven initiative without a single hand-coded hitbox:
//!
//! - the menu buttons are authored as scene nodes (`SceneWorld2D`)
//! - the engine resolves which button the pointer is over (`hit_test`)
//! - clicks are routed to the script bound to that node (`route_pointer_click`)
//! - the button scripts mutate the live world (`spawn`/`despawn`) through their
//!   `SceneScriptContext2D`
//!
//! Rendering reads straight from the world: every visible node draws as a rect
//! plus its `label`, so the on-screen state *is* the scene graph.

use rengine::*;

const SCRIPT_MENU_BUTTON: &str = "scripts/menu_button.rs";
const PREFAB: &str = "ui";

/// A button script: on activation it reads its node's `action` property and
/// mutates the shared world accordingly. The same script class backs every
/// button; behavior is data-driven by the node it is attached to.
#[derive(Default)]
struct MenuButtonScript;

impl SceneScript2D for MenuButtonScript {
    fn on_event_world(&mut self, ctx: &mut SceneScriptContext2D, event: &SceneScriptEvent2D) {
        let SceneScriptEvent2D::Custom { topic, .. } = event;
        if topic != "activate" {
            return;
        }

        let action = ctx
            .node()
            .and_then(|node| node.property("action"))
            .map(str::to_string);

        match action.as_deref() {
            Some("spawn") => {
                let count = ctx.world().by_tag("spawned").len();
                let col = (count % 6) as f32;
                let row = (count / 6) as f32;
                let position = Vec2::new(-240.0 + col * 84.0, -40.0 - row * 52.0);

                let mut node = SceneNode2D::new("box").with_position(position);
                node.add_tag("spawned");
                node.set_property("w", "72");
                node.set_property("h", "40");
                node.set_property("label", format!("#{}", count + 1));
                ctx.world_mut().spawn(node);
            }
            Some("clear") => {
                for handle in ctx.world().by_tag("spawned") {
                    ctx.world_mut().despawn(handle);
                }
            }
            _ => {}
        }
    }
}

/// Build the authored menu scene, the live world, and a script host with the
/// button scripts attached. Shared by the runnable app and the headless test so
/// both exercise the exact same wiring.
fn build_menu() -> (SceneWorld2D, SceneScriptHost2D) {
    let scene = Scene2D::from_definition(
        std::path::Path::new("menu.scene.json"),
        Scene2DDef {
            prefabs: vec![Prefab2DDef {
                name: PREFAB.to_string(),
                sprites: vec![],
            }],
            instances: vec![
                button_instance(1, "spawn_btn", "Spawn Box", "spawn", [-250.0, 110.0]),
                button_instance(2, "clear_btn", "Clear All", "clear", [40.0, 110.0]),
            ],
        },
        &AssetPack::default(),
    )
    .expect("menu scene definition is valid");

    let world = SceneWorld2D::from_scene(&scene);

    let mut registry = SceneScriptRegistry2D::new();
    registry.register_default::<MenuButtonScript>(SCRIPT_MENU_BUTTON);

    let mut host = SceneScriptHost2D::new();
    host.attach_scene(&scene, &registry);

    (world, host)
}

fn button_instance(
    id: u64,
    name: &str,
    label: &str,
    action: &str,
    position: [f32; 2],
) -> SceneInstance2DDef {
    SceneInstance2DDef {
        prefab: PREFAB.to_string(),
        position,
        scale: [1.0, 1.0],
        properties: [
            ("editor_node_id", id.to_string()),
            ("editor_name", name.to_string()),
            ("kind", "UI Root".to_string()),
            ("script_path", SCRIPT_MENU_BUTTON.to_string()),
            ("tags", "button".to_string()),
            ("label", label.to_string()),
            ("action", action.to_string()),
            ("w", "210".to_string()),
            ("h", "46".to_string()),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect(),
    }
}

struct MenuScene {
    world: SceneWorld2D,
    host: SceneScriptHost2D,
    hovered: Option<NodeHandle2D>,
}

impl MenuScene {
    fn new() -> Self {
        let (world, host) = build_menu();
        Self {
            world,
            host,
            hovered: None,
        }
    }

    /// The engine already reports `mouse_position` in the centered, y-up space
    /// the scene nodes and canvas share, so it is the scene point directly.
    fn to_scene_point(mouse: (f32, f32)) -> Vec2 {
        Vec2::new(mouse.0, mouse.1)
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        let input = engine.input();
        let point = Self::to_scene_point(input.mouse_position());

        self.hovered = self.world.hit_test(point);

        if input.is_mouse_pressed(0) {
            self.host
                .route_pointer_click(&mut self.world, [point.x, point.y], true);
        }
        if input.is_mouse_released(0) {
            self.host
                .route_pointer_click(&mut self.world, [point.x, point.y], false);
        }

        if input.is_key_pressed(KeyCode::Escape) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, _globals: &Globals, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(18, 20, 28, 255);

        let (w, h) = engine.window_size();
        let hw = w as f32 / 2.0;
        let hh = h as f32 / 2.0;
        let canvas = frame.canvas(0);

        canvas.text(
            -hw + 20.0,
            hh - 28.0,
            "Scene-driven menu: buttons are scene nodes, picked + routed by the engine",
            16.0,
            Color::WHITE,
        );
        canvas.text(
            -hw + 20.0,
            hh - 50.0,
            "Click Spawn Box / Clear All. Scripts mutate the live world. [Esc] Quit",
            13.0,
            Color::from_rgba8(170, 180, 200, 255),
        );

        for handle in self.world.visible_draw_order() {
            let Some(bounds) = self.world.node_bounds(handle) else {
                continue;
            };
            let Some(node) = self.world.get(handle) else {
                continue;
            };

            let is_button = node.has_tag("button");
            let hovered = self.hovered == Some(handle);
            let color = match (is_button, hovered) {
                (true, true) => Color::from_rgba8(86, 138, 230, 255),
                (true, false) => Color::from_rgba8(52, 92, 168, 255),
                (false, _) => Color::from_rgba8(120, 96, 64, 255),
            };

            canvas.rect(bounds.x, bounds.y, bounds.width, bounds.height, color);

            if let Some(label) = node.property("label") {
                let center = bounds.center();
                canvas.text_aligned(
                    center.x,
                    center.y - 6.0,
                    label,
                    16.0,
                    Color::WHITE,
                    TextAlign::Center,
                );
            }
        }

        let spawned = self.world.by_tag("spawned").len();
        canvas.text(
            -hw + 20.0,
            -hh + 24.0,
            &format!("spawned boxes: {spawned}"),
            14.0,
            Color::from_rgba8(200, 210, 230, 255),
        );
    }
}

fn main() {
    rengine::run_with_scenes(
        EngineConfig {
            title: "Feature: Scene Scripts + Hit-Testing".into(),
            width: 800,
            height: 600,
            show_fps: false,
            ..Default::default()
        },
        |_engine, _globals| Box::new(MenuScene::new()),
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click(host: &mut SceneScriptHost2D, world: &mut SceneWorld2D, point: [f32; 2]) {
        host.route_pointer_click(world, point, true);
        host.route_pointer_click(world, point, false);
    }

    #[test]
    fn clicking_scene_buttons_drives_world_mutation_end_to_end() {
        let (mut world, mut host) = build_menu();

        // Buttons live at the authored positions; click their centers.
        let spawn_btn = world.find_by_name("spawn_btn").unwrap();
        let clear_btn = world.find_by_name("clear_btn").unwrap();
        let spawn_center = world.node_bounds(spawn_btn).unwrap().center();
        let clear_center = world.node_bounds(clear_btn).unwrap().center();

        assert_eq!(world.by_tag("spawned").len(), 0);

        click(&mut host, &mut world, [spawn_center.x, spawn_center.y]);
        click(&mut host, &mut world, [spawn_center.x, spawn_center.y]);
        assert_eq!(
            world.by_tag("spawned").len(),
            2,
            "each click should spawn a box via the routed script"
        );

        click(&mut host, &mut world, [clear_center.x, clear_center.y]);
        assert_eq!(
            world.by_tag("spawned").len(),
            0,
            "clear should despawn every spawned box"
        );
    }

    #[test]
    fn clicking_empty_space_activates_nothing() {
        let (mut world, mut host) = build_menu();
        // A point well away from any node.
        click(&mut host, &mut world, [-380.0, -280.0]);
        assert_eq!(world.by_tag("spawned").len(), 0);
    }

    #[test]
    fn press_on_button_release_off_button_does_not_activate() {
        let (mut world, mut host) = build_menu();
        let spawn_btn = world.find_by_name("spawn_btn").unwrap();
        let center = world.node_bounds(spawn_btn).unwrap().center();

        host.route_pointer_click(&mut world, [center.x, center.y], true);
        host.route_pointer_click(&mut world, [-380.0, -280.0], false);
        assert_eq!(world.by_tag("spawned").len(), 0);
    }
}
