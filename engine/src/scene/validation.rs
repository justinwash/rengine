//! Non-fatal scene validation.
//!
//! [`Scene2D::load_from_path`](super::Scene2D::load_from_path) is deliberately
//! fail-fast: it stops at the first structural problem and returns an
//! [`AssetError`](crate::assets::AssetError). That is the right behavior for a
//! runtime that must refuse to boot a broken scene, but it is the wrong shape
//! for tooling. An editor validation panel — and a pre-boot project-wide check
//! — want *every* problem in a scene at once, classified by severity, so the
//! author can fix them in one pass instead of recompiling after each error.
//!
//! [`validate_editor_scene`] provides that: it walks an editor scene document
//! (the `{"nodes": [...]}` authoring format) and returns a
//! [`SceneValidationReport`] collecting all issues. Structural checks (ids,
//! parent references, cycles, node sources) always run; reference checks
//! against an [`AssetPack`](crate::assets::AssetPack) and a
//! [`SceneScriptRegistry2D`] run when those are supplied.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::assets::AssetPack;

use super::SceneScriptRegistry2D;

/// The editor scene schema version this build understands.
///
/// Documents may carry an optional `version` field; a document declaring a
/// newer version than this is flagged (as a warning) so authors get a clear
/// signal instead of silent data loss when an older build opens newer content.
pub const CURRENT_EDITOR_SCENE_VERSION: u32 = 1;

/// Severity of a single [`SceneValidationIssue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneIssueSeverity {
    /// The scene will not load or will misbehave at runtime; must be fixed.
    Error,
    /// The scene loads, but the issue likely indicates an authoring mistake.
    Warning,
}

/// A single problem found while validating a scene document.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneValidationIssue {
    pub severity: SceneIssueSeverity,
    pub message: String,
    /// The editor node id the issue concerns, when it is node-specific.
    pub node_id: Option<u64>,
}

impl SceneValidationIssue {
    fn error(node_id: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            severity: SceneIssueSeverity::Error,
            message: message.into(),
            node_id,
        }
    }

    fn warning(node_id: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            severity: SceneIssueSeverity::Warning,
            message: message.into(),
            node_id,
        }
    }
}

/// The collected result of validating a scene document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneValidationReport {
    pub issues: Vec<SceneValidationIssue>,
}

impl SceneValidationReport {
    pub fn issues(&self) -> &[SceneValidationIssue] {
        &self.issues
    }

    pub fn errors(&self) -> impl Iterator<Item = &SceneValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == SceneIssueSeverity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &SceneValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == SceneIssueSeverity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.errors().count()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings().count()
    }

    pub fn has_errors(&self) -> bool {
        self.errors().next().is_some()
    }

    /// True when there are no error-severity issues (warnings are allowed).
    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }

    fn push(&mut self, issue: SceneValidationIssue) {
        self.issues.push(issue);
    }
}

/// Lightweight, lenient view of an editor scene document used only for
/// diagnostics. It is intentionally independent of the loader's strict structs
/// so validation can report problems (e.g. a missing `kind`) that would
/// otherwise abort deserialization before any check runs.
#[derive(Debug, Deserialize)]
struct EditorSceneDoc {
    #[serde(default)]
    version: Option<u32>,
    #[serde(default)]
    nodes: Vec<EditorNode>,
}

#[derive(Debug, Deserialize)]
struct EditorNode {
    id: u64,
    #[serde(default)]
    parent: Option<u64>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    script_path: String,
    #[serde(default)]
    runtime_prefab: String,
    #[serde(default)]
    asset_alias: String,
}

/// Validate an editor scene document value, collecting every issue found.
///
/// `assets` and `registry` are optional: when present, the validator also
/// flags `Sprite` nodes whose `asset_alias` is missing from the pack and
/// `script_path`s that have no registered [`SceneScript2D`](super::SceneScript2D).
pub fn validate_editor_scene(
    value: &serde_json::Value,
    assets: Option<&AssetPack>,
    registry: Option<&SceneScriptRegistry2D>,
) -> SceneValidationReport {
    let mut report = SceneValidationReport::default();

    if value.get("nodes").is_none() {
        report.push(SceneValidationIssue::error(
            None,
            "scene document has no 'nodes' array (not an editor scene)",
        ));
        return report;
    }

    let doc: EditorSceneDoc = match serde_json::from_value(value.clone()) {
        Ok(doc) => doc,
        Err(error) => {
            report.push(SceneValidationIssue::error(
                None,
                format!("scene document failed to parse: {error}"),
            ));
            return report;
        }
    };

    if let Some(version) = doc.version {
        if version > CURRENT_EDITOR_SCENE_VERSION {
            report.push(SceneValidationIssue::warning(
                None,
                format!(
                    "scene schema version {version} is newer than supported version \
                     {CURRENT_EDITOR_SCENE_VERSION}; some data may not load correctly"
                ),
            ));
        }
    }

    // Index ids (flagging duplicates) and build an id -> parent map the cycle
    // walk can traverse without re-borrowing the node list.
    let mut seen_ids: HashSet<u64> = HashSet::with_capacity(doc.nodes.len());
    let mut id_to_parent: HashMap<u64, Option<u64>> = HashMap::with_capacity(doc.nodes.len());
    for node in &doc.nodes {
        if !seen_ids.insert(node.id) {
            report.push(SceneValidationIssue::error(
                Some(node.id),
                format!("duplicate node id {}", node.id),
            ));
        }
        id_to_parent.insert(node.id, node.parent);
    }

    for node in &doc.nodes {
        validate_parent(node, &id_to_parent, &mut report);
        validate_node_source(node, &mut report);

        if node.kind.is_none() {
            report.push(SceneValidationIssue::warning(
                Some(node.id),
                format!("node {} has no 'kind'", node.id),
            ));
        }

        let script = node.script_path.trim();
        if !script.is_empty() {
            if let Some(registry) = registry {
                if !registry.contains(script) {
                    report.push(SceneValidationIssue::warning(
                        Some(node.id),
                        format!(
                            "node {} references script '{}' with no registered handler",
                            node.id, script
                        ),
                    ));
                }
            }
        }

        let alias = node.asset_alias.trim();
        if !alias.is_empty() {
            if let Some(assets) = assets {
                if assets.texture_id(alias).is_none() {
                    report.push(SceneValidationIssue::error(
                        Some(node.id),
                        format!(
                            "node {} references missing asset alias '{}'",
                            node.id, alias
                        ),
                    ));
                }
            }
        }
    }

    report
}

fn validate_parent(
    node: &EditorNode,
    id_to_parent: &HashMap<u64, Option<u64>>,
    report: &mut SceneValidationReport,
) {
    let Some(parent_id) = node.parent else {
        return;
    };

    if parent_id == node.id {
        report.push(SceneValidationIssue::error(
            Some(node.id),
            format!("node {} is its own parent", node.id),
        ));
        return;
    }

    if !id_to_parent.contains_key(&parent_id) {
        report.push(SceneValidationIssue::error(
            Some(node.id),
            format!("node {} references missing parent {}", node.id, parent_id),
        ));
        return;
    }

    // Walk ancestors. Returning to this node means it sits in a cycle; revisiting
    // any other id means the cycle is elsewhere and will be reported by the node
    // that actually closes it.
    let mut seen: HashSet<u64> = HashSet::new();
    let mut current = Some(parent_id);
    while let Some(id) = current {
        if id == node.id {
            report.push(SceneValidationIssue::error(
                Some(node.id),
                format!("node {} participates in a parent cycle", node.id),
            ));
            return;
        }
        if !seen.insert(id) {
            return;
        }
        current = id_to_parent.get(&id).copied().flatten();
    }
}

fn validate_node_source(node: &EditorNode, report: &mut SceneValidationReport) {
    let is_sprite = node
        .kind
        .as_deref()
        .map(|kind| kind.eq_ignore_ascii_case("sprite"))
        .unwrap_or(false);

    if is_sprite && node.asset_alias.trim().is_empty() && node.runtime_prefab.trim().is_empty() {
        report.push(SceneValidationIssue::error(
            Some(node.id),
            format!(
                "sprite node {} has no source (empty asset_alias and runtime_prefab)",
                node.id
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sprite_node(id: u64, parent: Option<u64>, alias: &str) -> serde_json::Value {
        json!({
            "id": id,
            "parent": parent,
            "name": format!("node{id}"),
            "kind": "Sprite",
            "position": [0.0, 0.0],
            "size": [10.0, 10.0],
            "visible": true,
            "asset_alias": alias,
        })
    }

    #[test]
    fn clean_scene_has_no_errors() {
        let value = json!({
            "nodes": [
                sprite_node(1, None, "hero"),
                sprite_node(2, Some(1), "hero"),
            ]
        });
        let report = validate_editor_scene(&value, None, None);
        assert!(report.is_ok(), "unexpected issues: {:?}", report.issues());
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn non_editor_document_is_flagged() {
        let value = json!({ "prefabs": [], "instances": [] });
        let report = validate_editor_scene(&value, None, None);
        assert!(report.has_errors());
    }

    #[test]
    fn duplicate_ids_are_errors() {
        let value = json!({
            "nodes": [sprite_node(1, None, "hero"), sprite_node(1, None, "hero")]
        });
        let report = validate_editor_scene(&value, None, None);
        assert!(report
            .errors()
            .any(|issue| issue.message.contains("duplicate node id 1")));
    }

    #[test]
    fn dangling_and_self_parents_are_errors() {
        let value = json!({
            "nodes": [
                sprite_node(1, Some(99), "hero"),
                sprite_node(2, Some(2), "hero"),
            ]
        });
        let report = validate_editor_scene(&value, None, None);
        assert!(report
            .errors()
            .any(|issue| issue.message.contains("missing parent 99")));
        assert!(report
            .errors()
            .any(|issue| issue.message.contains("is its own parent")));
    }

    #[test]
    fn parent_cycles_are_errors() {
        // 1 -> 2 -> 3 -> 1
        let value = json!({
            "nodes": [
                sprite_node(1, Some(3), "hero"),
                sprite_node(2, Some(1), "hero"),
                sprite_node(3, Some(2), "hero"),
            ]
        });
        let report = validate_editor_scene(&value, None, None);
        assert!(report
            .errors()
            .any(|issue| issue.message.contains("parent cycle")));
    }

    #[test]
    fn sprite_without_source_is_error_and_missing_kind_is_warning() {
        let value = json!({
            "nodes": [
                { "id": 1, "kind": "Sprite", "asset_alias": "", "runtime_prefab": "" },
                { "id": 2, "name": "no_kind" },
            ]
        });
        let report = validate_editor_scene(&value, None, None);
        assert!(report
            .errors()
            .any(|issue| issue.message.contains("has no source")));
        assert!(report
            .warnings()
            .any(|issue| issue.message.contains("has no 'kind'")));
    }

    #[test]
    fn newer_schema_version_warns_but_is_not_an_error() {
        let value = json!({
            "version": CURRENT_EDITOR_SCENE_VERSION + 1,
            "nodes": [sprite_node(1, None, "hero")],
        });
        let report = validate_editor_scene(&value, None, None);
        assert!(report
            .warnings()
            .any(|issue| issue.message.contains("newer than supported")));
        assert!(report.is_ok());
    }

    #[test]
    fn missing_asset_and_script_refs_are_reported_when_context_supplied() {
        let value = json!({
            "nodes": [{
                "id": 1,
                "kind": "Sprite",
                "asset_alias": "ghost_asset",
                "script_path": "scripts/missing.rs",
            }]
        });

        // Without context, ref checks are skipped.
        let bare = validate_editor_scene(&value, None, None);
        assert_eq!(bare.error_count(), 0);

        let assets = AssetPack::default();
        let registry = SceneScriptRegistry2D::new();
        let report = validate_editor_scene(&value, Some(&assets), Some(&registry));
        assert!(report
            .errors()
            .any(|issue| issue.message.contains("missing asset alias 'ghost_asset'")));
        assert!(report
            .warnings()
            .any(|issue| issue.message.contains("no registered handler")));
    }
}
