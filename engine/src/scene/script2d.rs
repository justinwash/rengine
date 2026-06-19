use std::collections::HashMap;
use std::sync::Arc;

use super::{NodeHandle2D, Scene2D, SceneNode2D, SceneScriptBinding2D, SceneWorld2D};
use crate::Vec2;

pub trait SceneScript2D: Send {
    fn on_attach(&mut self, _binding: &SceneScriptBinding2D) {}

    fn on_enter(&mut self, _binding: &SceneScriptBinding2D) {}

    fn on_update(&mut self, _binding: &SceneScriptBinding2D, _dt: f32) {}

    fn on_fixed_update(&mut self, _binding: &SceneScriptBinding2D, _dt: f32) {}

    fn on_input(&mut self, _binding: &SceneScriptBinding2D, _event: &SceneScriptInputEvent2D) {}

    fn on_event(&mut self, _binding: &SceneScriptBinding2D, _event: &SceneScriptEvent2D) {}

    fn on_detach(&mut self, _binding: &SceneScriptBinding2D) {}

    // --- World-aware hooks -------------------------------------------------
    //
    // These receive a `SceneScriptContext2D` granting mutable access to the
    // live runtime `SceneWorld2D`. They default to delegating to the
    // binding-only hooks above so existing scripts keep working unchanged; a
    // script that wants to read or mutate live nodes simply overrides the
    // world-aware variant instead.

    fn on_enter_world(&mut self, ctx: &mut SceneScriptContext2D) {
        self.on_enter(ctx.binding());
    }

    fn on_update_world(&mut self, ctx: &mut SceneScriptContext2D, dt: f32) {
        self.on_update(ctx.binding(), dt);
    }

    fn on_fixed_update_world(&mut self, ctx: &mut SceneScriptContext2D, dt: f32) {
        self.on_fixed_update(ctx.binding(), dt);
    }

    fn on_input_world(&mut self, ctx: &mut SceneScriptContext2D, event: &SceneScriptInputEvent2D) {
        self.on_input(ctx.binding(), event);
    }

    fn on_event_world(&mut self, ctx: &mut SceneScriptContext2D, event: &SceneScriptEvent2D) {
        self.on_event(ctx.binding(), event);
    }
}

/// Mutable context handed to scene scripts during their world-aware callbacks.
///
/// It bundles the script's static [`SceneScriptBinding2D`] with mutable access
/// to the live [`SceneWorld2D`], so a script can move, hide, retag, spawn, or
/// despawn nodes in response to updates, input, and events instead of only
/// inspecting binding metadata. The host constructs one per script per
/// callback; scripts never build it directly.
pub struct SceneScriptContext2D<'a> {
    world: &'a mut SceneWorld2D,
    binding: &'a SceneScriptBinding2D,
}

impl<'a> SceneScriptContext2D<'a> {
    pub fn new(world: &'a mut SceneWorld2D, binding: &'a SceneScriptBinding2D) -> Self {
        Self { world, binding }
    }

    pub fn binding(&self) -> &SceneScriptBinding2D {
        self.binding
    }

    pub fn world(&self) -> &SceneWorld2D {
        self.world
    }

    pub fn world_mut(&mut self) -> &mut SceneWorld2D {
        self.world
    }

    /// Handle of the node this script is attached to, resolved from the
    /// binding's editor node id (falling back to its editor name).
    pub fn node_handle(&self) -> Option<NodeHandle2D> {
        if let Some(id) = self.binding.editor_node_id {
            if let Some(handle) = self.world.find_by_editor_id(id) {
                return Some(handle);
            }
        }
        self.binding
            .editor_name
            .as_deref()
            .and_then(|name| self.world.find_by_name(name))
    }

    /// The node this script is attached to, if it resolves in the world.
    pub fn node(&self) -> Option<&SceneNode2D> {
        self.node_handle().and_then(|handle| self.world.get(handle))
    }

    /// Mutable access to the node this script is attached to, if it resolves.
    pub fn node_mut(&mut self) -> Option<&mut SceneNode2D> {
        match self.node_handle() {
            Some(handle) => self.world.get_mut(handle),
            None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneScriptInputEvent2D {
    PointerMove {
        position: [f32; 2],
    },
    PointerButton {
        button: u8,
        pressed: bool,
        position: [f32; 2],
    },
    Scroll {
        delta: [f32; 2],
    },
    Key {
        key: String,
        pressed: bool,
    },
    Text(String),
    Action(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneScriptEvent2D {
    Custom {
        topic: String,
        payload: HashMap<String, String>,
    },
}

type SceneScriptFactory2D = Arc<dyn Fn() -> Box<dyn SceneScript2D> + Send + Sync>;

#[derive(Default, Clone)]
pub struct SceneScriptRegistry2D {
    factories: HashMap<String, SceneScriptFactory2D>,
}

impl SceneScriptRegistry2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(&mut self, script_path: impl AsRef<str>, factory: F)
    where
        F: Fn() -> Box<dyn SceneScript2D> + Send + Sync + 'static,
    {
        let key = normalize_script_path(script_path.as_ref());
        self.factories.insert(key, Arc::new(factory));
    }

    pub fn register_default<T>(&mut self, script_path: impl AsRef<str>)
    where
        T: SceneScript2D + Default + 'static,
    {
        self.register(script_path, || Box::<T>::default());
    }

    pub fn contains(&self, script_path: &str) -> bool {
        let key = normalize_script_path(script_path);
        self.factories.contains_key(&key)
    }

    pub fn create(&self, script_path: &str) -> Option<Box<dyn SceneScript2D>> {
        let key = normalize_script_path(script_path);
        self.factories.get(&key).map(|factory| factory())
    }
}

struct SceneScriptInstance2D {
    binding: SceneScriptBinding2D,
    script: Box<dyn SceneScript2D>,
}

#[derive(Default)]
pub struct SceneScriptHost2D {
    instances: Vec<SceneScriptInstance2D>,
    warnings: Vec<String>,
    pressed_node: Option<NodeHandle2D>,
}

impl SceneScriptHost2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach_scene(&mut self, scene: &Scene2D, registry: &SceneScriptRegistry2D) {
        self.attach_bindings(scene.script_bindings(), registry);
    }

    pub fn attach_bindings<I>(&mut self, bindings: I, registry: &SceneScriptRegistry2D)
    where
        I: IntoIterator<Item = SceneScriptBinding2D>,
    {
        self.detach_all();
        self.warnings.clear();

        for binding in bindings {
            match registry.create(&binding.script_path) {
                Some(mut script) => {
                    script.on_attach(&binding);
                    self.instances
                        .push(SceneScriptInstance2D { binding, script });
                }
                None => {
                    let label = binding
                        .editor_name
                        .clone()
                        .unwrap_or_else(|| binding.prefab.clone());
                    self.warnings.push(format!(
                        "No SceneScript2D registered for '{}' (instance '{}', index {})",
                        binding.script_path, label, binding.instance_index
                    ));
                }
            }
        }
    }

    pub fn on_enter(&mut self) {
        for instance in &mut self.instances {
            instance.script.on_enter(&instance.binding);
        }
    }

    pub fn on_update(&mut self, dt: f32) {
        for instance in &mut self.instances {
            instance.script.on_update(&instance.binding, dt);
        }
    }

    pub fn on_fixed_update(&mut self, dt: f32) {
        for instance in &mut self.instances {
            instance.script.on_fixed_update(&instance.binding, dt);
        }
    }

    pub fn on_input(&mut self, event: &SceneScriptInputEvent2D) {
        for instance in &mut self.instances {
            instance.script.on_input(&instance.binding, event);
        }
    }

    pub fn on_event(&mut self, event: &SceneScriptEvent2D) {
        for instance in &mut self.instances {
            instance.script.on_event(&instance.binding, event);
        }
    }

    pub fn on_event_for_script_path(&mut self, script_path: &str, event: &SceneScriptEvent2D) {
        let normalized = normalize_script_path(script_path);
        for instance in &mut self.instances {
            if normalize_script_path(&instance.binding.script_path) == normalized {
                instance.script.on_event(&instance.binding, event);
            }
        }
    }

    pub fn on_event_for_editor_name(&mut self, editor_name: &str, event: &SceneScriptEvent2D) {
        for instance in &mut self.instances {
            if instance.binding.editor_name.as_deref() == Some(editor_name) {
                instance.script.on_event(&instance.binding, event);
            }
        }
    }

    pub fn emit_custom_event(
        &mut self,
        topic: impl Into<String>,
        payload: HashMap<String, String>,
    ) {
        self.on_event(&SceneScriptEvent2D::Custom {
            topic: topic.into(),
            payload,
        });
    }

    pub fn emit_custom_event_for_script_path(
        &mut self,
        script_path: &str,
        topic: impl Into<String>,
        payload: HashMap<String, String>,
    ) {
        self.on_event_for_script_path(
            script_path,
            &SceneScriptEvent2D::Custom {
                topic: topic.into(),
                payload,
            },
        );
    }

    pub fn emit_activate_script_path(&mut self, script_path: &str) {
        let mut payload = HashMap::new();
        payload.insert("target".to_string(), script_path.to_string());
        self.emit_custom_event_for_script_path(script_path, "activate", payload);
    }

    // --- World-aware dispatch ---------------------------------------------
    //
    // Mirror the binding-only dispatch above but thread a `&mut SceneWorld2D`
    // through to each script via a `SceneScriptContext2D`. Existing callers can
    // keep using the world-free methods; callers that own a runtime world use
    // these so scripts can mutate live nodes.

    pub fn enter_world(&mut self, world: &mut SceneWorld2D) {
        for instance in &mut self.instances {
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_enter_world(&mut ctx);
        }
    }

    pub fn update_world(&mut self, world: &mut SceneWorld2D, dt: f32) {
        for instance in &mut self.instances {
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_update_world(&mut ctx, dt);
        }
    }

    pub fn fixed_update_world(&mut self, world: &mut SceneWorld2D, dt: f32) {
        for instance in &mut self.instances {
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_fixed_update_world(&mut ctx, dt);
        }
    }

    pub fn input_world(&mut self, world: &mut SceneWorld2D, event: &SceneScriptInputEvent2D) {
        for instance in &mut self.instances {
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_input_world(&mut ctx, event);
        }
    }

    pub fn event_world(&mut self, world: &mut SceneWorld2D, event: &SceneScriptEvent2D) {
        for instance in &mut self.instances {
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_event_world(&mut ctx, event);
        }
    }

    pub fn event_world_for_editor_name(
        &mut self,
        world: &mut SceneWorld2D,
        editor_name: &str,
        event: &SceneScriptEvent2D,
    ) {
        for instance in &mut self.instances {
            if instance.binding.editor_name.as_deref() != Some(editor_name) {
                continue;
            }
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_event_world(&mut ctx, event);
        }
    }

    pub fn event_world_for_script_path(
        &mut self,
        world: &mut SceneWorld2D,
        script_path: &str,
        event: &SceneScriptEvent2D,
    ) {
        let normalized = normalize_script_path(script_path);
        for instance in &mut self.instances {
            if normalize_script_path(&instance.binding.script_path) != normalized {
                continue;
            }
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_event_world(&mut ctx, event);
        }
    }

    pub fn emit_activate_world(&mut self, world: &mut SceneWorld2D, script_path: &str) {
        let mut payload = HashMap::new();
        payload.insert("target".to_string(), script_path.to_string());
        self.event_world_for_script_path(
            world,
            script_path,
            &SceneScriptEvent2D::Custom {
                topic: "activate".to_string(),
                payload,
            },
        );
    }

    // --- Pointer hit-test routing -----------------------------------------
    //
    // These let the engine own picking: the host resolves the node under the
    // pointer via `SceneWorld2D::hit_test` and delivers the event only to the
    // script bound to that node, instead of every game hand-coding hitboxes.

    /// Route a pointer event to the script on the topmost node under the
    /// pointer. Positional events (`PointerMove`/`PointerButton`) reach only the
    /// hit node's script; non-positional events fall back to a broadcast.
    /// Returns the hit node handle, if any.
    pub fn route_input_world(
        &mut self,
        world: &mut SceneWorld2D,
        event: &SceneScriptInputEvent2D,
    ) -> Option<NodeHandle2D> {
        let position = match event {
            SceneScriptInputEvent2D::PointerMove { position } => *position,
            SceneScriptInputEvent2D::PointerButton { position, .. } => *position,
            _ => {
                self.input_world(world, event);
                return None;
            }
        };
        let hit = world.hit_test(Vec2::new(position[0], position[1]))?;
        self.dispatch_input_to_node(world, hit, event);
        Some(hit)
    }

    /// Route a pointer button as a click: remember the topmost hit node on
    /// press, and on release over that same node emit an `activate` event to its
    /// script (payload `target` = the node's script path, matching
    /// [`SceneScriptHost2D::emit_activate_script_path`]). Returns the activated
    /// node handle, if any.
    pub fn route_pointer_click(
        &mut self,
        world: &mut SceneWorld2D,
        position: [f32; 2],
        pressed: bool,
    ) -> Option<NodeHandle2D> {
        let hit = world.hit_test(Vec2::new(position[0], position[1]));
        if pressed {
            self.pressed_node = hit;
            return None;
        }

        let pressed_node = self.pressed_node.take();
        match (pressed_node, hit) {
            (Some(down), Some(up)) if down == up => {
                let mut payload = HashMap::new();
                if let Some(target) = world.get(up).and_then(|node| node.script_path()) {
                    payload.insert("target".to_string(), target.to_string());
                }
                let event = SceneScriptEvent2D::Custom {
                    topic: "activate".to_string(),
                    payload,
                };
                self.dispatch_event_to_node(world, up, &event);
                Some(up)
            }
            _ => None,
        }
    }

    fn dispatch_input_to_node(
        &mut self,
        world: &mut SceneWorld2D,
        handle: NodeHandle2D,
        event: &SceneScriptInputEvent2D,
    ) {
        let Some(editor_id) = world.get(handle).and_then(|node| node.editor_node_id()) else {
            return;
        };
        for instance in &mut self.instances {
            if instance.binding.editor_node_id != Some(editor_id) {
                continue;
            }
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_input_world(&mut ctx, event);
        }
    }

    fn dispatch_event_to_node(
        &mut self,
        world: &mut SceneWorld2D,
        handle: NodeHandle2D,
        event: &SceneScriptEvent2D,
    ) {
        let Some(editor_id) = world.get(handle).and_then(|node| node.editor_node_id()) else {
            return;
        };
        for instance in &mut self.instances {
            if instance.binding.editor_node_id != Some(editor_id) {
                continue;
            }
            let binding = &instance.binding;
            let script = &mut instance.script;
            let mut ctx = SceneScriptContext2D::new(&mut *world, binding);
            script.on_event_world(&mut ctx, event);
        }
    }

    pub fn detach_all(&mut self) {
        for instance in &mut self.instances {
            instance.script.on_detach(&instance.binding);
        }
        self.instances.clear();
    }

    pub fn script_count(&self) -> usize {
        self.instances.len()
    }

    pub fn binding_by_script_path(&self, script_path: &str) -> Option<&SceneScriptBinding2D> {
        let normalized = normalize_script_path(script_path);
        self.instances
            .iter()
            .find(|instance| normalize_script_path(&instance.binding.script_path) == normalized)
            .map(|instance| &instance.binding)
    }

    pub fn binding_by_editor_name(&self, editor_name: &str) -> Option<&SceneScriptBinding2D> {
        self.instances
            .iter()
            .find(|instance| instance.binding.editor_name.as_deref() == Some(editor_name))
            .map(|instance| &instance.binding)
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}

fn normalize_script_path(path: &str) -> String {
    path.trim().replace('\\', "/").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct DummyScript;

    impl SceneScript2D for DummyScript {}

    #[test]
    fn registry_normalizes_paths_for_lookup() {
        let mut registry = SceneScriptRegistry2D::new();
        registry.register_default::<DummyScript>("scripts/Title.rs");

        assert!(registry.contains("scripts/title.rs"));
        assert!(registry.contains("scripts\\TITLE.rs"));
        assert!(registry.create("scripts/title.rs").is_some());
    }

    #[test]
    fn host_attaches_registered_scripts_and_warns_on_missing() {
        let mut registry = SceneScriptRegistry2D::new();
        registry.register_default::<DummyScript>("scripts/title.rs");

        let mut host = SceneScriptHost2D::new();
        host.attach_bindings(
            vec![
                SceneScriptBinding2D {
                    instance_index: 0,
                    prefab: "title_ui".to_string(),
                    script_path: "scripts/title.rs".to_string(),
                    editor_node_id: Some(1),
                    editor_parent_id: None,
                    editor_name: Some("title_root".to_string()),
                },
                SceneScriptBinding2D {
                    instance_index: 1,
                    prefab: "menu".to_string(),
                    script_path: "scripts/missing.rs".to_string(),
                    editor_node_id: Some(2),
                    editor_parent_id: None,
                    editor_name: Some("menu_root".to_string()),
                },
            ],
            &registry,
        );

        assert_eq!(host.script_count(), 1);
        assert_eq!(host.warnings().len(), 1);
        assert!(host.warnings()[0].contains("scripts/missing.rs"));
    }

    #[test]
    fn host_can_lookup_bindings_by_script_path_and_editor_name() {
        let mut registry = SceneScriptRegistry2D::new();
        registry.register_default::<DummyScript>("scripts/title.rs");

        let mut host = SceneScriptHost2D::new();
        host.attach_bindings(
            vec![SceneScriptBinding2D {
                instance_index: 0,
                prefab: "title_ui".to_string(),
                script_path: "scripts/title.rs".to_string(),
                editor_node_id: Some(1),
                editor_parent_id: None,
                editor_name: Some("title_root".to_string()),
            }],
            &registry,
        );

        let by_script = host.binding_by_script_path("scripts\\TITLE.rs");
        assert!(by_script.is_some());
        assert_eq!(
            by_script.and_then(|b| b.editor_name.as_deref()),
            Some("title_root")
        );

        let by_name = host.binding_by_editor_name("title_root");
        assert!(by_name.is_some());
        assert_eq!(
            by_name.map(|b| b.script_path.as_str()),
            Some("scripts/title.rs")
        );
    }

    fn scene_with_scripted_node() -> Scene2D {
        use crate::scene::{Prefab2DDef, Scene2DDef, SceneInstance2DDef};
        let definition = Scene2DDef {
            prefabs: vec![Prefab2DDef {
                name: "marker".to_string(),
                sprites: vec![],
            }],
            instances: vec![SceneInstance2DDef {
                prefab: "marker".to_string(),
                position: [0.0, 0.0],
                scale: [1.0, 1.0],
                properties: [
                    ("editor_node_id", "1"),
                    ("editor_name", "hero"),
                    ("script_path", "scripts/world.rs"),
                ]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            }],
        };
        Scene2D::from_definition(
            std::path::Path::new("t.scene.json"),
            definition,
            &crate::assets::AssetPack::default(),
        )
        .unwrap()
    }

    #[derive(Default)]
    struct WorldMutatingScript;

    impl SceneScript2D for WorldMutatingScript {
        fn on_event_world(&mut self, ctx: &mut SceneScriptContext2D, _event: &SceneScriptEvent2D) {
            if let Some(node) = ctx.node_mut() {
                node.translate(crate::Vec2::new(1.0, 2.0));
            }
            if let Some(handle) = ctx.node_handle() {
                ctx.world_mut()
                    .spawn_child(handle, SceneNode2D::new("bullet").with_name("bullet"));
            }
        }
    }

    #[test]
    fn context_lets_script_read_and_mutate_world() {
        let scene = scene_with_scripted_node();
        let mut world = SceneWorld2D::from_scene(&scene);
        let hero = world.find_by_name("hero").unwrap();
        assert_eq!(world.get(hero).unwrap().position(), crate::Vec2::ZERO);

        let mut registry = SceneScriptRegistry2D::new();
        registry.register_default::<WorldMutatingScript>("scripts/world.rs");
        let mut host = SceneScriptHost2D::new();
        host.attach_scene(&scene, &registry);
        assert_eq!(host.script_count(), 1);

        host.event_world(
            &mut world,
            &SceneScriptEvent2D::Custom {
                topic: "activate".to_string(),
                payload: HashMap::new(),
            },
        );

        // The script moved its own bound node and spawned a child in the world.
        assert_eq!(
            world.get(hero).unwrap().position(),
            crate::Vec2::new(1.0, 2.0)
        );
        let bullet = world.find_by_name("bullet").expect("child was spawned");
        assert_eq!(world.parent(bullet), Some(hero));
    }

    #[test]
    fn legacy_on_event_still_fires_through_world_dispatch() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct LegacyScript {
            hits: Arc<AtomicUsize>,
        }

        // Only the binding-only hook is overridden, exactly like the shipping
        // game scripts. World-aware dispatch must still reach it via the
        // default delegation, so existing content keeps working.
        impl SceneScript2D for LegacyScript {
            fn on_event(&mut self, _binding: &SceneScriptBinding2D, _event: &SceneScriptEvent2D) {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
        }

        let scene = scene_with_scripted_node();
        let mut world = SceneWorld2D::from_scene(&scene);

        let hits = Arc::new(AtomicUsize::new(0));
        let factory_hits = hits.clone();
        let mut registry = SceneScriptRegistry2D::new();
        registry.register("scripts/world.rs", move || {
            Box::new(LegacyScript {
                hits: factory_hits.clone(),
            })
        });

        let mut host = SceneScriptHost2D::new();
        host.attach_scene(&scene, &registry);
        host.event_world(
            &mut world,
            &SceneScriptEvent2D::Custom {
                topic: "x".to_string(),
                payload: HashMap::new(),
            },
        );

        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn pointer_click_routes_activate_to_clicked_node_only() {
        use crate::scene::{Prefab2DDef, Scene2DDef, SceneInstance2DDef};
        use std::sync::Mutex;

        struct ClickScript {
            log: Arc<Mutex<Vec<String>>>,
        }
        impl SceneScript2D for ClickScript {
            fn on_event_world(
                &mut self,
                ctx: &mut SceneScriptContext2D,
                event: &SceneScriptEvent2D,
            ) {
                let SceneScriptEvent2D::Custom { topic, .. } = event;
                if topic == "activate" {
                    if let Some(name) = ctx.binding().editor_name.clone() {
                        self.log.lock().unwrap().push(name);
                    }
                }
            }
        }

        fn button(name: &str, id: u64, x: f32) -> SceneInstance2DDef {
            SceneInstance2DDef {
                prefab: "btn".to_string(),
                position: [x, 0.0],
                scale: [1.0, 1.0],
                properties: [
                    ("editor_node_id", id.to_string()),
                    ("editor_name", name.to_string()),
                    ("script_path", "scripts/btn.rs".to_string()),
                    ("w", "100".to_string()),
                    ("h", "100".to_string()),
                ]
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            }
        }

        let definition = Scene2DDef {
            prefabs: vec![Prefab2DDef {
                name: "btn".to_string(),
                sprites: vec![],
            }],
            instances: vec![button("start_btn", 1, 0.0), button("quit_btn", 2, 200.0)],
        };
        let scene = Scene2D::from_definition(
            std::path::Path::new("t.scene.json"),
            definition,
            &crate::assets::AssetPack::default(),
        )
        .unwrap();
        let mut world = SceneWorld2D::from_scene(&scene);

        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let factory_log = log.clone();
        let mut registry = SceneScriptRegistry2D::new();
        registry.register("scripts/btn.rs", move || {
            Box::new(ClickScript {
                log: factory_log.clone(),
            })
        });
        let mut host = SceneScriptHost2D::new();
        host.attach_scene(&scene, &registry);

        // Press + release over start_btn activates it.
        assert_eq!(
            host.route_pointer_click(&mut world, [50.0, 50.0], true),
            None
        );
        let activated = host.route_pointer_click(&mut world, [50.0, 50.0], false);
        assert_eq!(
            activated.and_then(|h| world.get(h)?.editor_node_id()),
            Some(1)
        );

        // Press + release over quit_btn activates it.
        host.route_pointer_click(&mut world, [250.0, 50.0], true);
        host.route_pointer_click(&mut world, [250.0, 50.0], false);

        // Press on start but release on quit activates neither.
        host.route_pointer_click(&mut world, [50.0, 50.0], true);
        assert_eq!(
            host.route_pointer_click(&mut world, [250.0, 50.0], false),
            None
        );

        let log = log.lock().unwrap();
        assert_eq!(*log, vec!["start_btn".to_string(), "quit_btn".to_string()]);
    }
}
