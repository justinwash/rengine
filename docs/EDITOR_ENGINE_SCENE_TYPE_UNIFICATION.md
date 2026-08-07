# Editor/engine scene-type unification — plan

**Status: plan only, not started.** Recon done 2026-08-06. Read this fully
before touching code; it supersedes re-deriving the same investigation.

## The bug that started this

Formula R's `scenes/*.scene.json` files fail to open in the editor:
`Failed to parse ...settings.scene.json as an editor scene: missing field
'name' at line 596 column 1`. Root cause turned out to be upstream of that
specific file — the editor and the engine each have their own, structurally
similar but incompatible definition of "what is a scene document," and the
editor can only read its own.

## What exists today (three shapes, not two)

1. **`editor::scene::{SceneDocument, SceneNode, SceneNodeKind}`**
   (`editor/src/scene.rs`) — the editor's authoring format. Has typed
   convenience fields the inspector binds to directly:
   `SceneNode.sprite: SpriteNodeSettings { texture_path }` and
   `SceneNode.camera2d: Camera2dNodeSettings { zoom, show_bounds,
   use_scene_view_size, view_size }`. `SceneDocument` also carries the
   authoring-only logic: `add_node`, `duplicate_nodes`, `reparent_nodes`,
   `reorder_nodes`, `remove_nodes`, `translate_subtree`, id allocation, etc.
   `File > Open Scene` (`editor/src/app/filesystem.rs:726`) deserializes
   straight to this type, unconditionally, for anything matching
   `is_scene_path`.

2. **`EditorSceneDocumentDef` / `EditorSceneNodeDef` / `EditorSceneNodeKind`**
   (private to `engine/src/scene/data2d.rs`, lines ~895–946) — the engine's own
   *compiler input* format. Same core fields as `SceneNode`
   (`id`/`parent`/`name`/`kind`/`position`/`size`/`visible`/`script_path`/
   `runtime_prefab`/`asset_alias`) but **no** `sprite`/`camera2d` typed
   fields — those never reach the compiler as typed data. Everything else
   editor-specific is generic: `properties: HashMap<String, String>`.
   `scene_definition_from_json` (line 948) sniffs a JSON value for a
   top-level `"nodes"` key and, if present, deserializes to this type and
   runs it through `scene_definition_from_editor_document` (line 967), which
   compiles it down to `Scene2DDef { prefabs, instances }` — flattening each
   node's `id`/`name`/`kind`/`visible`/`size`/`parent`/`script_path`/
   `asset_alias` into `properties` under `editor_*`-prefixed keys
   (`editor_instance_properties`, line 1328).

3. **Formula R's hand-written files** — written directly in the *compiled
   output* shape (`prefabs` + `instances`), skipping both authoring formats
   above. Out of scope for this change; tracked separately, see "Formula R,
   afterward" below.

**#1 and #2 are the real duplication**, and they've already drifted: the
editor's inspector writes `node.sprite.texture_path`, but the engine's
compiler only ever reads `node.asset_alias` — two fields claiming to hold the
same value, kept in sync by hand (`editor/src/app/filesystem.rs:425`,
`:379-380`). That's a live bug, independent of the scene-opening failure, and
it goes away as a side effect of this change.

There is also a **third, deliberately separate** lenient copy —
`EditorSceneDoc`/`EditorNode` in `engine/src/scene/validation.rs:120-140` —
used only for diagnostics on possibly-malformed files (its doc comment
explains why: strict structs abort on the first bad field; a validator needs
to report every problem in a file, including a missing `kind`). **Leave this
one alone.** It is correct as a deliberately weaker, independent schema for a
different purpose, not accidental drift.

## The fix

Move the canonical type definition into the engine (the crate `editor`
already depends on — `editor/Cargo.toml`: `rengine = { path = "../engine" }` —
so this requires no new dependency and no risk of a cycle). The editor keeps
its authoring-only logic and its editor-only fields, but stops defining its
own copy of the shared shape.

### 1. In `engine/src/scene/data2d.rs`

- Rename `EditorSceneDocumentDef` → `EditorSceneDocument`, `EditorSceneNodeDef`
  → `EditorSceneNode`, `EditorSceneNodeKind` stays named (already unambiguous).
  (Dropping the `Def` suffix since these are becoming the *public* canonical
  type, not a private deserialize-only shadow of something else — matches the
  convention of `CURRENT_EDITOR_SCENE_VERSION` and other `Editor*`-prefixed
  public names already exported from `scene::validation`.)
- Make all three `pub` (currently private to the module).
- Add `Serialize` to each derive list (currently `Deserialize`-only) —
  needed because the editor's `SceneDocument::pretty_json()` calls
  `serde_json::to_string_pretty(self)` for saving.
- `EditorSceneNode` gains one new field: nothing structurally — see the
  `camera2d`/`sprite` handling below, which goes through `properties`, not a
  new typed field. Do **not** add `sprite`/`camera2d` fields to the engine
  type; that would resurrect the duplication this change removes. The whole
  point is that camera/sprite authoring state that the compiler doesn't need
  lives in `properties`.
- Re-export from `engine/src/scene/mod.rs` and `engine/src/lib.rs`, following
  the existing `pub use` list pattern (see `lib.rs:57-66`):
  `EditorSceneDocument, EditorSceneNode, EditorSceneNodeKind`.
- No change to `scene_definition_from_editor_document` or any of its helpers
  — they already operate on this exact shape, just privately. Confirm after
  the rename that the ~90 in-file test call sites (`data2d.rs` tests
  constructing `EditorSceneNodeDef { ... }` literals, lines 1691–2216ish)
  still compile under the new names — pure rename, no behavior change
  expected.

### 2. In `editor/src/scene.rs`

- Delete the local `SceneNode` struct and `SceneNodeKind` enum definitions.
- Add `pub use rengine::{EditorSceneDocument as SceneDocumentData,
  EditorSceneNode as SceneNode, EditorSceneNodeKind as SceneNodeKind};`
  (naming the imported document type distinctly, since `SceneDocument` itself
  stays a *locally defined* wrapper — see next point — so it can keep its
  authoring methods).
- `SceneDocument` cannot become a pure re-export/alias, because it carries
  real inherent methods (`add_node`, `normalize_next_id`, `node`, `node_mut`,
  `root_ids`, `child_ids`, `is_descendant_of`, `selected_root_ids`,
  `subtree_ids`, `duplicate_nodes`, `reparent_nodes`, `reorder_nodes`,
  `remove_nodes`, `translate_subtree`, `pretty_json`, plus private
  `duplicate_subtree`/`alloc_unique_id`/`can_reparent_node`/
  `extract_nodes_by_ids` helpers — none of which exist on the engine's
  `EditorSceneDocument`, and none of which the engine has any reason to know
  about). Two options, pick one during implementation:
  - **(a) Newtype wrapper** — `pub struct SceneDocument(rengine::EditorSceneDocument)`
    with `Deref`/`DerefMut` to `nodes`/`name`/`version`/`view`/`next_id`, and
    the above methods as inherent impls on the wrapper. Cleanest boundary,
    but every field access site (`self.name`, `self.nodes`, `node.id`, etc.)
    needs to keep working through `Deref` — check this actually satisfies
    all 65 call sites found in recon (below) before committing to it.
  - **(b) Extension trait** — keep `EditorSceneDocument` un-wrapped, define
    `trait SceneDocumentExt { fn add_node(...); ... }` implemented for
    `rengine::EditorSceneDocument`, imported wherever the methods are called.
    No wrapper type, but every method becomes non-inherent (trait must be in
    scope), and `SceneDocument::new("untitled_scene")` call sites
    (`app.rs:180`) need to become `EditorSceneDocument::new(...)` or a free
    function, since `new` can't live in the trait cleanly (no `Self` to
    construct from outside the type's own crate — orphan rule).
  - **Recommendation: (a).** `SceneDocument::new(...)` and the tree-editing
    methods read naturally as inherent methods on a document type; a
    `Deref`-based wrapper keeps every existing call site
    (`self.scene.nodes`, `doc.node_mut(id)`, etc.) unchanged, and it makes
    "this is the editor's own type, engine data underneath" explicit at the
    type level rather than implicit via an imported trait. Confirm during
    implementation that `Deref<Target = EditorSceneNode-containing-struct>`
    doesn't fight the borrow checker in `reorder_nodes`/`duplicate_nodes`,
    which mutate `self.nodes` through `std::mem::take` — may need
    `self.0.nodes` directly in those bodies rather than going through
    `Deref` for the mutable paths. Not expected to be a real obstacle, just
    flagging it as the one place worth double-checking.
- `SceneNodeReorderDirection` and `SceneViewSettings` stay exactly as they
  are today — `SceneNodeReorderDirection` is pure editor UI state with no
  engine equivalent and never was duplicated; `SceneViewSettings` (window
  size for the viewport) is editor-only and has no compiler-side reason to
  exist in the engine, so it stays put, referenced from the wrapper.
- **`sprite: SpriteNodeSettings` and `camera2d: Camera2dNodeSettings`
  disappear as typed fields.** Neither reaches the engine compiler as typed
  data today (confirmed via grep — no `sprite.texture_path`/`camera2d.*`
  reference anywhere in `engine/src/scene/data2d.rs`), so unifying the node
  type means this state has to move into `properties: HashMap<String,
  String>`, the same place every other editor-only per-node fact already
  lives (`runtime_prefab`, `asset_alias` are the only two that got promoted
  to real fields, because the compiler reads them directly).
  - `sprite.texture_path` → **delete**. Every read/write site
    (`editor/src/app/filesystem.rs:379-380,441,445,474,494`,
    `editor/src/app/forms.rs:107,1123-1124,1142`) becomes a direct read/write
    of `node.asset_alias` instead. This is not a lossy migration — it fixes
    the existing two-fields-one-value bug, since `asset_alias` was already
    the field the compiler actually reads.
  - `camera2d.{zoom, show_bounds, use_scene_view_size, view_size}` → move
    into `properties` under new keys, e.g. `camera_zoom`, `camera_show_bounds`,
    `camera_use_scene_view_size`, `camera_view_w`, `camera_view_h` (stored as
    strings, parsed on read — same convention every other property already
    uses). Add small helper methods on the `SceneDocument` wrapper (or free
    functions taking `&EditorSceneNode`/`&mut EditorSceneNode`) —
    `camera_zoom(&self) -> f32`, `set_camera_zoom(&mut self, f32)`, etc. — so
    call sites in `editor/src/app/drawing.rs:791-797` and
    `editor/src/app/forms.rs:108-112,1152-1194` read/write through a typed
    accessor rather than parsing `properties.get("camera_zoom")` inline at
    every call site. Defaults (`default_camera_zoom() -> f32` etc., already
    defined at `editor/src/scene.rs:614-628`) become the fallback when the
    key is absent from `properties`, same shape as `unwrap_or_default()`
    already used elsewhere for property lookups.

### 3. Everywhere else in the editor (no logic changes expected)

Recon found these files reference the moved types; all are `use` imports and
field/method access that should keep compiling unchanged once (1) and (2)
land, **except** the `sprite`/`camera2d` sites called out above:

- `editor/src/app.rs` — `SceneNodeKind` variant list (`NODE_KIND_OPTIONS`),
  `SceneDocument`/`SceneNode`/`SceneNodeReorderDirection` imports. No change.
- `editor/src/app/state.rs` — `scene: SceneDocument` field ×2,
  `SceneDocument::new` call (line 180 — becomes
  `SceneDocument::new("untitled_scene")` on the wrapper, unchanged syntax if
  option (a) is used), `SceneNodeReorderDirection` usage. No change.
- `editor/src/app/windowing.rs` — `SceneNodeLine`, `scene_node_lines`,
  test fixture `test_node(...) -> SceneNode` (line 1573) constructing a
  `SceneNode { ... }` literal with `sprite`/`camera2d` fields likely absent
  already (check — if the test literal never sets `sprite`/`camera2d`
  explicitly, it already relies on `#[serde(default)]`-equivalent
  `Default::default()` and needs no change beyond whatever `(a)`'s
  constructor shape ends up being).
- `editor/src/app/popup.rs` — `kind: SceneNodeKind` in two struct fields
  (context-menu "create node of kind X" state). No change.
- `editor/src/app/drawing.rs` — `SceneNodeKind::Camera2d` match arm, plus the
  4 `camera2d.*` reads flagged above (need the new accessor methods).
- `editor/src/app/forms.rs` — inspector form state sync, plus the 18
  `sprite.*`/`camera2d.*` sites flagged above (the bulk of the real edit
  work).
- `editor/src/app/filesystem.rs` — `open_scene_path` (the failing code path
  itself — no change needed to make it *succeed* now, since `SceneDocument`
  now accepts exactly the shape `EditorSceneNode`'s `Deserialize` impl
  already accepts, which is broader/more lenient than before in exactly the
  ways that matter — see "Does this fix the original bug?" below), plus the
  6 `sprite.texture_path` sites flagged above.

## Does this fix the original `settings.scene.json` bug?

**Not by itself — check before assuming it does.** The original failure was
`missing field 'name'`, because Formula R's files are in the *third* shape
(`prefabs`/`instances`), which has no `name` field at all — `SceneDocument`
required `name` (no `#[serde(default)]`). After unification, `SceneDocument`
requires whatever `EditorSceneDocument` requires, which per the recon above
is **only** `nodes` (`#[serde(default)] nodes: Vec<...>` — actually double
check: `EditorSceneDocumentDef` at `data2d.rs:896` has `#[serde(default)]` on
`nodes` too, meaning an empty `{}` would currently parse successfully as an
editor document with zero nodes). Since Formula R's files have `prefabs` and
`instances` keys instead of `nodes`, they will **still fail to open** after
this change — they'll fail differently (probably successfully parse as a
valid-but-empty `SceneDocument` with 0 nodes, silently dropping all the
`prefabs`/`instances` content, rather than erroring — **verify this is not
worse than today's explicit error** before considering this change complete
in isolation).

This unification is the necessary first step (one schema, no drift), but
**loading Formula R's actual files is the follow-up work**, scoped
separately per the user's explicit sequencing ("unification first then we'll
come back to formula r"). Do not consider that follow-up in scope for this
change. When it is picked up: either (a) teach `open_scene_path` to try
`Scene2DDef`-shape (prefabs/instances) as a fallback and offer a one-time
import/flatten into `EditorSceneNode` tree shape, or (b) something else TBD
at that time — not decided here.

## Test/verification plan

1. `cargo build` across the workspace (editor depends on engine; engine
   builds standalone) — catches any missed rename.
2. `cargo test -p rengine` — the ~90 in-file `data2d.rs` tests using
   `EditorSceneNodeDef`/`EditorSceneDocumentDef` literals must still pass
   under the new names (pure rename, but confirm).
3. `cargo test -p rengine-editor` — `editor/src/scene.rs`'s own test module
   (lines 630–818) exercises `add_node`/`duplicate_nodes`/`reparent_nodes`/
   `reorder_nodes`/`remove_nodes` — these must pass unchanged since none of
   that logic touches `sprite`/`camera2d`/`asset_alias`.
4. Manual editor smoke test: open the editor, create a new scene, add a
   Sprite node, set its texture via the inspector, confirm it still shows in
   the viewport preview (exercises the `asset_alias`-only path post-deletion
   of `sprite.texture_path`). Add a Camera2d node, toggle `show_bounds`,
   change zoom, confirm the viewport preview still reflects it (exercises
   the new `properties`-backed camera accessors). Save, reload, confirm both
   round-trip.
5. Confirm a **pre-existing, real editor-authored** `.scene.json` file
   (there should be some under `samples/` — check `samples/games/*/scenes/`
   or similar, if such a corpus exists) still opens correctly — this is the
   regression check that matters most, since Formula R's files were never
   going to open anyway.

## Explicitly out of scope for this change

- Loading Formula R's `prefabs`/`instances`-shaped files into the editor
  (tracked as the immediate follow-up, not started).
- `engine/src/scene/validation.rs`'s `EditorSceneDoc`/`EditorNode` — leave as
  is, it's a deliberate independent lenient schema for diagnostics, not
  drift.
- Any change to the runtime `Scene2D`/`Scene2DDef`/`SceneInstance2D` types or
  `scene_definition_from_editor_document`'s compilation logic — this change
  touches only the *input* shape to that function, not the function itself
  or anything downstream of it.
