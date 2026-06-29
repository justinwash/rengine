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
}
