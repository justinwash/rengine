use std::collections::HashMap;
use std::sync::Arc;

use super::{Scene2D, SceneScriptBinding2D};

pub trait SceneScript2D: Send {
    fn on_attach(&mut self, _binding: &SceneScriptBinding2D) {}

    fn on_enter(&mut self, _binding: &SceneScriptBinding2D) {}

    fn on_update(&mut self, _binding: &SceneScriptBinding2D, _dt: f32) {}

    fn on_fixed_update(&mut self, _binding: &SceneScriptBinding2D, _dt: f32) {}

    fn on_input(&mut self, _binding: &SceneScriptBinding2D, _event: &SceneScriptInputEvent2D) {}

    fn on_event(&mut self, _binding: &SceneScriptBinding2D, _event: &SceneScriptEvent2D) {}

    fn on_detach(&mut self, _binding: &SceneScriptBinding2D) {}
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
}
