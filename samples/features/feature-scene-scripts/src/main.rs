//! Feature: Scene-driven scripts, hit-testing, and live-world mutation.
//!
//! This sample wires together the runtime scene stack added in the
//! scene-driven initiative without a single hand-coded hitbox:
//!
//! - the menu buttons are authored as scene nodes (`SceneWorld2D`)
//! - `SceneLayer2D::update` routes pointer input to scripts automatically
//! - clicks are routed to the script bound to that node (`route_pointer_click`)
//! - the button scripts mutate the live world (`spawn`/`despawn`) through their
//!   `SceneScriptContext2D`
//!
//! Rendering reads straight from the world: every visible node draws as a rect
//! plus its `label`, so the on-screen state *is* the scene graph.

use rengine::*;

const SCRIPT_MENU_BUTTON: &str = "scripts/menu_button.rs";
const PREFAB: &str = "ui";

/// A button script: on activation it reads its authored `action` *param* (from
/// the binding, i.e. the node's `param_action` property) and mutates the shared
/// world accordingly. The same script class backs every button; behavior is
/// data-driven by the typed param the editor's inspector writes.
#[derive(Default)]
struct MenuButtonScript;

impl SceneScript2D for MenuButtonScript {
    fn on_event_world(&mut self, ctx: &mut SceneScriptContext2D, event: &SceneScriptEvent2D) {
        let SceneScriptEvent2D::Custom { topic, .. } = event;
        if topic != "activate" {
            return;
        }

        let action = ctx.binding().param("action").map(str::to_string);

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

/// Build the authored menu scene and a fully-wired `SceneLayer2D`. Shared by
/// the runnable app and the headless test so both exercise the exact same wiring.
fn build_menu() -> SceneLayer2D {
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

    let mut registry = SceneScriptRegistry2D::new();
    registry.register_default::<MenuButtonScript>(SCRIPT_MENU_BUTTON);

    let mut layer = SceneLayer2D::from_scene(&scene, &registry);
    layer.enter();
    layer
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
            ("param_action", action.to_string()),
            ("w", "210".to_string()),
            ("h", "46".to_string()),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect(),
    }
}

struct MenuScene {
    layer: SceneLayer2D,
}

impl MenuScene {
    fn new() -> Self {
        Self {
            layer: build_menu(),
        }
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn update(&mut self, engine: &Engine, _globals: &mut Globals, _frame: &mut Frame) -> SceneOp {
        self.layer.update(engine);

        if engine.input().is_key_pressed(KeyCode::Escape) {
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

        let hovered = self.layer.hovered();
        for handle in self.layer.world().visible_draw_order() {
            let Some(bounds) = self.layer.world().node_bounds(handle) else {
                continue;
            };
            let Some(node) = self.layer.world().get(handle) else {
                continue;
            };

            let is_button = node.has_tag("button");
            let color = match (is_button, hovered == Some(handle)) {
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

        let spawned = self.layer.world().by_tag("spawned").len();
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

    fn click(layer: &mut SceneLayer2D, point: [f32; 2]) {
        layer.route_click(point, true);
        layer.route_click(point, false);
    }

    #[test]
    fn clicking_scene_buttons_drives_world_mutation_end_to_end() {
        let mut layer = build_menu();

        // Buttons live at the authored positions; click their centers.
        let spawn_btn = layer.world().find_by_name("spawn_btn").unwrap();
        let clear_btn = layer.world().find_by_name("clear_btn").unwrap();
        let spawn_center = layer.world().node_bounds(spawn_btn).unwrap().center();
        let clear_center = layer.world().node_bounds(clear_btn).unwrap().center();

        assert_eq!(layer.world().by_tag("spawned").len(), 0);

        click(&mut layer, [spawn_center.x, spawn_center.y]);
        click(&mut layer, [spawn_center.x, spawn_center.y]);
        assert_eq!(
            layer.world().by_tag("spawned").len(),
            2,
            "each click should spawn a box via the routed script"
        );

        click(&mut layer, [clear_center.x, clear_center.y]);
        assert_eq!(
            layer.world().by_tag("spawned").len(),
            0,
            "clear should despawn every spawned box"
        );
    }

    #[test]
    fn authored_param_reaches_the_script_binding() {
        // The `param_action` property is collected onto the binding (prefix
        // stripped) and read back through the typed accessor — the same path
        // the editor's typed-param inspector authors.
        let scene = Scene2D::from_definition(
            std::path::Path::new("menu.scene.json"),
            Scene2DDef {
                prefabs: vec![Prefab2DDef {
                    name: PREFAB.to_string(),
                    sprites: vec![],
                }],
                instances: vec![button_instance(
                    1,
                    "spawn_btn",
                    "Spawn Box",
                    "spawn",
                    [-250.0, 110.0],
                )],
            },
            &AssetPack::default(),
        )
        .unwrap();

        let bindings = scene.script_bindings();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].param("action"), Some("spawn"));
    }

    #[test]
    fn clicking_empty_space_activates_nothing() {
        let mut layer = build_menu();
        click(&mut layer, [-380.0, -280.0]);
        assert_eq!(layer.world().by_tag("spawned").len(), 0);
    }

    #[test]
    fn press_on_button_release_off_button_does_not_activate() {
        let mut layer = build_menu();
        let spawn_btn = layer.world().find_by_name("spawn_btn").unwrap();
        let center = layer.world().node_bounds(spawn_btn).unwrap().center();

        layer.route_click([center.x, center.y], true);
        layer.route_click([-380.0, -280.0], false);
        assert_eq!(layer.world().by_tag("spawned").len(), 0);
    }
}
