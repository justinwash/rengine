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
use std::path::{Path, PathBuf};

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

/// Validate a single scene file on disk, reading and parsing it first.
///
/// IO and JSON parse failures are reported as error issues (with no node id)
/// rather than returned as `Err`, so a project-wide pass can keep going and
/// report every bad file in one sweep instead of stopping at the first.
pub fn validate_scene_file(
    path: &Path,
    assets: Option<&AssetPack>,
    registry: Option<&SceneScriptRegistry2D>,
) -> SceneValidationReport {
    let mut report = SceneValidationReport::default();
    if let Some(value) = read_scene_value(path, &mut report) {
        let file_report = validate_editor_scene(&value, assets, registry);
        report.issues.extend(file_report.issues);
    }
    report
}

/// Validate every `*.scene.json` file under `dir` (recursively), returning one
/// report per file in sorted path order for stable, diff-friendly output.
///
/// In addition to per-file checks, a cross-file pass flags any node id that
/// appears in more than one scene: ids must be unique across the whole project
/// so that script references and prefab links are unambiguous.
pub fn validate_scene_dir(
    dir: &Path,
    assets: Option<&AssetPack>,
    registry: Option<&SceneScriptRegistry2D>,
) -> Vec<(PathBuf, SceneValidationReport)> {
    struct ParsedScene {
        path: PathBuf,
        value: Option<serde_json::Value>,
        report: SceneValidationReport,
    }

    let mut file_paths = Vec::new();
    collect_scene_files(dir, &mut file_paths);
    file_paths.sort();

    let mut scenes: Vec<ParsedScene> = file_paths
        .into_iter()
        .map(|path| {
            let mut report = SceneValidationReport::default();
            let value = read_scene_value(&path, &mut report);
            if let Some(ref v) = value {
                let file_report = validate_editor_scene(v, assets, registry);
                report.issues.extend(file_report.issues);
            }
            ParsedScene {
                path,
                value,
                report,
            }
        })
        .collect();

    // Cross-file check: a node id that appears in more than one scene file is
    // a project error — script/prefab references would be ambiguous.
    let mut id_to_file: HashMap<u64, usize> = HashMap::new();
    let mut collisions: Vec<(u64, usize, usize)> = Vec::new();
    for (file_idx, scene) in scenes.iter().enumerate() {
        let Some(ref value) = scene.value else {
            continue;
        };
        let Some(nodes) = value.get("nodes").and_then(|n| n.as_array()) else {
            continue;
        };
        for node in nodes {
            let Some(id) = node.get("id").and_then(|v| v.as_u64()) else {
                continue;
            };
            match id_to_file.entry(id) {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(file_idx);
                }
                std::collections::hash_map::Entry::Occupied(e) => {
                    collisions.push((id, *e.get(), file_idx));
                }
            }
        }
    }
    for (id, idx_a, idx_b) in collisions {
        let path_a = scenes[idx_a].path.display().to_string();
        let path_b = scenes[idx_b].path.display().to_string();
        scenes[idx_a].report.push(SceneValidationIssue::error(
            Some(id),
            format!(
                "node id {id} also appears in '{path_b}' \
                 — ids must be unique across the project"
            ),
        ));
        scenes[idx_b].report.push(SceneValidationIssue::error(
            Some(id),
            format!(
                "node id {id} also appears in '{path_a}' \
                 — ids must be unique across the project"
            ),
        ));
    }

    scenes.into_iter().map(|s| (s.path, s.report)).collect()
}

fn read_scene_value(path: &Path, report: &mut SceneValidationReport) -> Option<serde_json::Value> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(error) => {
            report.push(SceneValidationIssue::error(
                None,
                format!("failed to read '{}': {error}", path.display()),
            ));
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            report.push(SceneValidationIssue::error(
                None,
                format!("failed to parse '{}': {error}", path.display()),
            ));
            None
        }
    }
}

fn collect_scene_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_scene_files(&path, out);
        } else if is_scene_file(&path) {
            out.push(path);
        }
    }
}

fn is_scene_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with(".scene.json"))
        .unwrap_or(false)
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
    fn cross_scene_id_collision_is_flagged_by_validate_scene_dir() {
        use std::fs;

        let base = std::env::temp_dir().join(format!(
            "rengine_cross_scene_{}_{}",
            std::process::id(),
            CURRENT_EDITOR_SCENE_VERSION
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        // Two scenes sharing node id 1 — cross-scene collision.
        fs::write(
            base.join("a.scene.json"),
            serde_json::to_string(&json!({ "nodes": [sprite_node(1, None, "hero")] })).unwrap(),
        )
        .unwrap();
        fs::write(
            base.join("b.scene.json"),
            serde_json::to_string(&json!({ "nodes": [sprite_node(1, None, "enemy")] })).unwrap(),
        )
        .unwrap();
        // A third scene with a unique id is clean.
        fs::write(
            base.join("c.scene.json"),
            serde_json::to_string(&json!({ "nodes": [sprite_node(2, None, "wall")] })).unwrap(),
        )
        .unwrap();

        let reports = validate_scene_dir(&base, None, None);
        let by_name: HashMap<String, &SceneValidationReport> = reports
            .iter()
            .map(|(p, r)| (p.file_name().unwrap().to_str().unwrap().to_string(), r))
            .collect();

        assert!(
            by_name["a.scene.json"].has_errors(),
            "collision must be an error in scene a"
        );
        assert!(
            by_name["b.scene.json"].has_errors(),
            "collision must be an error in scene b"
        );
        assert!(by_name["c.scene.json"].is_ok(), "unique id must be clean");
        assert!(
            by_name["a.scene.json"]
                .errors()
                .any(|i| i.message.contains("unique across the project")),
            "error message should mention project uniqueness"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn validate_missing_scene_file_is_an_error() {
        let report = validate_scene_file(
            Path::new("definitely/does/not/exist.scene.json"),
            None,
            None,
        );
        assert!(report.has_errors());
    }

    #[test]
    fn validate_scene_dir_reports_each_scene_file() {
        use std::fs;

        let base = std::env::temp_dir().join(format!(
            "rengine_scene_validation_{}_{}",
            std::process::id(),
            CURRENT_EDITOR_SCENE_VERSION
        ));
        let _ = fs::remove_dir_all(&base);
        let nested = base.join("ui");
        fs::create_dir_all(&nested).unwrap();

        let good = base.join("good.scene.json");
        let bad = nested.join("bad.scene.json");
        // Use a unique id (100) that won't collide with the bad scene's ids.
        fs::write(
            &good,
            serde_json::to_string(&json!({ "nodes": [sprite_node(100, None, "hero")] })).unwrap(),
        )
        .unwrap();
        // bad has two nodes with the same id — intra-scene duplicate.
        fs::write(
            &bad,
            serde_json::to_string(
                &json!({ "nodes": [sprite_node(1, None, "hero"), sprite_node(1, None, "hero")] }),
            )
            .unwrap(),
        )
        .unwrap();
        // A non-scene file must be ignored.
        fs::write(base.join("notes.txt"), "ignore me").unwrap();

        let reports = validate_scene_dir(&base, None, None);
        assert_eq!(reports.len(), 2, "should find exactly the two scene files");

        let by_name: HashMap<String, &SceneValidationReport> = reports
            .iter()
            .map(|(path, report)| {
                (
                    path.file_name().unwrap().to_str().unwrap().to_string(),
                    report,
                )
            })
            .collect();
        assert!(by_name["good.scene.json"].is_ok());
        assert!(by_name["bad.scene.json"].has_errors());

        let _ = fs::remove_dir_all(&base);
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
