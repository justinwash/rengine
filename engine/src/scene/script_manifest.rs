//! Project-level script manifest.
//!
//! The editor crate can't link the game's compiled [`SceneScript2D`] registry,
//! so a project ships a `scripts.manifest.json` describing which scripts exist
//! and what typed parameters each accepts. The editor reads it to populate the
//! inspector's script picker, render typed param widgets, and validate scenes
//! (via [`SceneScriptRegistry2D::from_known_paths`]). The runtime ignores it —
//! scripts are still registered in Rust — so the manifest is purely an
//! authoring/tooling contract.
//!
//! [`SceneScript2D`]: super::SceneScript2D
//! [`SceneScriptRegistry2D::from_known_paths`]: super::SceneScriptRegistry2D::from_known_paths

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::AssetError;

/// Type of an authored script parameter — drives which inspector widget the
/// editor renders and how the value is interpreted at read time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScriptParamKind {
    #[default]
    String,
    Number,
    Bool,
    Color,
    /// A fixed choice from `ScriptParamDef::options`. The editor renders a
    /// selector so authors can only pick a declared value, and `is_valid_value`
    /// rejects anything off the list.
    Enum,
}

/// One typed parameter a script accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptParamDef {
    /// Stored on the node as `param_<name>`.
    pub name: String,
    /// Human label for the inspector (falls back to `name` if empty).
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub kind: ScriptParamKind,
    /// Default value seeded when the script is first attached.
    #[serde(default)]
    pub default: String,
    /// Allowed values when `kind` is `Enum`; ignored for other kinds.
    #[serde(default)]
    pub options: Vec<String>,
}

impl ScriptParamDef {
    /// Inspector label, falling back to the param name when none is authored.
    pub fn display_label(&self) -> &str {
        if self.label.trim().is_empty() {
            &self.name
        } else {
            &self.label
        }
    }

    /// Whether `value` is acceptable for this param. Only `Enum` constrains the
    /// value (it must be one of `options`, unless no options are declared);
    /// every other kind accepts any string.
    pub fn is_valid_value(&self, value: &str) -> bool {
        match self.kind {
            ScriptParamKind::Enum => {
                self.options.is_empty() || self.options.iter().any(|opt| opt == value)
            }
            _ => true,
        }
    }

    /// The next option after `current` (wrapping), or the first option when
    /// `current` isn't in the list. `None` if no options are declared. Drives the
    /// editor's cycle-selector for `Enum` params.
    pub fn next_option(&self, current: &str) -> Option<&str> {
        if self.options.is_empty() {
            return None;
        }
        let next_idx = self
            .options
            .iter()
            .position(|opt| opt == current)
            .map(|i| (i + 1) % self.options.len())
            .unwrap_or(0);
        Some(self.options[next_idx].as_str())
    }
}

/// A single script the project exposes to the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptDef {
    /// `script_path` value stored on nodes (matches the runtime registry key).
    pub path: String,
    /// Human label for the picker (falls back to `path` if empty).
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub params: Vec<ScriptParamDef>,
}

impl ScriptDef {
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.path
        } else {
            &self.name
        }
    }
}

/// The parsed `scripts.manifest.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptManifest {
    #[serde(default)]
    pub scripts: Vec<ScriptDef>,
}

impl ScriptManifest {
    pub fn load_from_path(path: &Path) -> Result<Self, AssetError> {
        let text = std::fs::read_to_string(path).map_err(|source| AssetError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| AssetError::Json {
            path: path.to_path_buf(),
            source,
        })
    }

    /// All declared script paths — feed to
    /// [`SceneScriptRegistry2D::from_known_paths`] for validation.
    ///
    /// [`SceneScriptRegistry2D::from_known_paths`]: super::SceneScriptRegistry2D::from_known_paths
    pub fn known_paths(&self) -> impl Iterator<Item = &str> {
        self.scripts.iter().map(|s| s.path.as_str())
    }

    pub fn script(&self, path: &str) -> Option<&ScriptDef> {
        self.scripts.iter().find(|s| s.path == path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_with_typed_params_and_defaults() {
        let json = r#"{
            "scripts": [
                {
                    "path": "scripts/command_button.rs",
                    "name": "Command Button",
                    "params": [
                        { "name": "command", "label": "Command", "kind": "string", "default": "PushPace" },
                        { "name": "tint", "kind": "color", "default": "200,40,40,255" }
                    ]
                },
                { "path": "scripts/menu_action.rs" }
            ]
        }"#;
        let manifest: ScriptManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.scripts.len(), 2);

        let paths: Vec<&str> = manifest.known_paths().collect();
        assert_eq!(
            paths,
            vec!["scripts/command_button.rs", "scripts/menu_action.rs"]
        );

        let cmd = manifest.script("scripts/command_button.rs").unwrap();
        assert_eq!(cmd.display_name(), "Command Button");
        assert_eq!(cmd.params.len(), 2);
        assert_eq!(cmd.params[0].kind, ScriptParamKind::String);
        assert_eq!(cmd.params[0].default, "PushPace");
        // Missing label falls back to the param name.
        assert_eq!(cmd.params[1].kind, ScriptParamKind::Color);
        assert_eq!(cmd.params[1].display_label(), "tint");

        // No `name` falls back to the path.
        let menu = manifest.script("scripts/menu_action.rs").unwrap();
        assert_eq!(menu.display_name(), "scripts/menu_action.rs");
        assert!(menu.params.is_empty());
    }

    #[test]
    fn parses_enum_param_with_options() {
        let json = r#"{
            "scripts": [
                {
                    "path": "scripts/menu_action.rs",
                    "params": [
                        {
                            "name": "action",
                            "kind": "enum",
                            "default": "NewGame",
                            "options": ["NewGame", "Continue", "Settings", "Quit"]
                        }
                    ]
                }
            ]
        }"#;
        let manifest: ScriptManifest = serde_json::from_str(json).unwrap();
        let param = &manifest.script("scripts/menu_action.rs").unwrap().params[0];
        assert_eq!(param.kind, ScriptParamKind::Enum);
        assert_eq!(param.options, ["NewGame", "Continue", "Settings", "Quit"]);
    }

    #[test]
    fn enum_validates_and_cycles_only_declared_options() {
        let param = ScriptParamDef {
            name: "action".into(),
            label: String::new(),
            kind: ScriptParamKind::Enum,
            default: "NewGame".into(),
            options: vec!["NewGame".into(), "Continue".into(), "Quit".into()],
        };
        assert!(param.is_valid_value("Continue"));
        assert!(!param.is_valid_value("Garbage"));

        // Cycles in order and wraps; an unknown current value resets to the first.
        assert_eq!(param.next_option("NewGame"), Some("Continue"));
        assert_eq!(param.next_option("Quit"), Some("NewGame"));
        assert_eq!(param.next_option("Garbage"), Some("NewGame"));

        // A non-enum param accepts anything and never cycles.
        let free = ScriptParamDef {
            name: "label".into(),
            label: String::new(),
            kind: ScriptParamKind::String,
            default: String::new(),
            options: vec![],
        };
        assert!(free.is_valid_value("anything"));
        assert_eq!(free.next_option("anything"), None);
    }
}
