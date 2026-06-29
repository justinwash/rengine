# Rengine Editor Guide

This document explains what the current editor is, why it is built the way it is, how it works internally, and how to use it as the start of a real workflow for games like the platformer or top-down samples.

## What the Editor Is Right Now

The current editor is an early native shell living in the `editor/` crate.

It is already useful as a proving ground for:

- the overall panel layout
- tabbed scene-document flow
- scene-document structure
- typed scene and node property editing
- selection and inspector flow
- viewport interaction
- context-menu scene authoring
- filesystem browsing
- editor responsiveness and continuity work

It also now has a first runtime bridge for 2D scenes and a real file flow: editor-authored scene JSON can be adapted by the runtime `Scene2D` loader, and the shell can open or save those scene files through native dialogs instead of only writing to the scratch path.

It is not yet a full production editor. The biggest limitation now is not the absence of any bridge, but the narrowness of the current one: the adapter can handle marker nodes, standalone sprite nodes, and grouped multi-sprite prefab compositions, but it is still far short of full prefab authoring, local-space scene editing, and nested-scene workflows.

## Why the Editor Lives in Its Own Crate

The editor is deliberately split from the runtime engine.

That separation matters for a few reasons:

- the runtime crate should stay focused on `rengine::*` gameplay APIs
- desktop-editor concerns such as docking, file trees, inspectors, and background indexing should not leak into runtime engine APIs
- the editor can iterate faster on UI and workflow concerns without destabilizing game code
- it keeps a clean place for editor-only dependencies such as `eframe` and `egui`

## Why the Editor Uses `eframe` and `egui`

The current shell uses immediate-mode UI on purpose.

That choice works well for a first serious pass because:

- iteration speed is high
- panels, inspectors, property editing, and prototype tooling are easy to build quickly
- state stays explicit in Rust instead of being hidden behind a heavyweight retained UI framework
- it lets the editor shell prove its workflows before deeper custom rendering work is justified

The editor may grow custom viewports and heavier runtime bridges later, but immediate-mode UI is a pragmatic starting point.

## How the Current Editor Works

### Shell Layout

The shell is organized into a few stable surfaces:

- top bar: project identity, branch, and file actions
- left panel: filesystem browser
- left-center panel: scene hierarchy
- right panel: grouped properties for scene, node, sprite, camera, and runtime settings
- bottom panel: activity log and scene JSON preview
- center panel: scene-tab strip plus 2D scene viewport

This mirrors the common strengths shared by Godot, Unity, Unreal, and Defold without copying any one tool exactly.

### Scene Document Model

The current editor scene model lives in `editor/src/scene.rs`.

The important ideas are:

- every node has a stable numeric ID
- every node has a parent link so hierarchy can exist without special-case containers
- the scene document carries scene-view settings such as preview window size
- nodes carry a kind, name, position, size, visibility flag, script path, runtime prefab override, sprite asset alias, sprite preview path, and Camera2D preview settings
- the entire document is serializable to pretty JSON

That means the editor already has the beginnings of a diffable authoring format, and the current runtime adapter can translate that document into the engine's `Scene2DDef` shape when the authored nodes fit the current conventions.

### Filesystem Browser Model

The filesystem browser builds a tree of project entries and displays them in the left panel.

The important recent behavior change is that project rescans are no longer done synchronously on the UI thread. Refreshes now run in the background and the UI polls for completion. That reduces visible stalls and avoids the blank-looking frames that show up when the shell stops drawing while it walks the workspace.

### JSON Preview Model

The bottom panel can show the current scene document as JSON.

The editor caches this preview instead of rebuilding it every time the UI asks for it. During live interactions such as dragging a node or resizing panels, the JSON preview is intentionally paused if the scene is dirty. That preserves continuity and reduces flashing caused by expensive preview regeneration during fast pointer-driven changes.

### Runtime Bridge Model

The runtime `Scene2D` loader can now detect editor-authored scene JSON and adapt it into the engine's existing prefab-instance scene format.

The current bridge is intentionally conservative:

- group nodes export as composite prefabs built from descendant sprites until another group boundary is reached
- non-group, non-sprite nodes export as marker prefabs with no visual sprites
- sprite nodes outside a group export as single-sprite prefabs and require a sprite asset alias
- runtime prefab names come from the explicit runtime-prefab override when present, otherwise from the node name
- script path, size, and editor metadata are preserved in instance properties for gameplay code to inspect later

This gives the top-down and platformer samples a usable migration path without pretending the editor already has full prefab authoring.

### Viewport Interaction Model

The current viewport is a simple 2D authoring canvas.

Today it supports:

- rendering the current nodes as colored boxes
- drawing sprite previews from chosen image files and checker placeholders when no image is assigned
- drawing Camera2D screen bounds from the current scene window size or camera override settings
- selecting nodes by clicking them
- double-clicking to create an empty node at the cursor
- right-click add menus in the viewport and scene hierarchy
- dragging nodes to move them

Dragging a node now moves its subtree as well, which makes grouped prefab composition workable even though the editor still does not have full local-transform editing or gizmos.

This is intentionally simple, but it is the right substrate for future snapping, gizmos, guides, and richer scene tools.

## Why the Current Editor Works the Way It Does

### It Optimizes for Diffable Data Early

The shell is already centered around serializable documents because editor workflows become hard to trust if every saved result is opaque or hard to review. Even at prototype stage, readable JSON is the correct direction.

### It Favors Explicit State Over Hidden Magic

The editor state is currently plain Rust data inside the app struct. That is useful because it keeps the important moving pieces visible:

- current project tree
- selected file
- selected node
- open scene tabs and the active scene document
- scene-view settings and typed sprite/camera editor properties
- activity log
- drag state
- JSON preview cache

This makes the shell easier to debug while the workflow is still changing quickly.

### It Protects Visual Continuity During Interaction

The recent continuity work is not cosmetic. It is foundational. If the shell visibly drops or flashes frames during common operations, trust in the editor disappears immediately.

That is why the current shell now:

- uses background rescans for workspace refreshes
- keeps a cached scene JSON preview
- pauses expensive preview regeneration during live interaction
- requests continuous repaint only while the viewport is actively dragging or background work still needs polling

### It Keeps the Runtime Boundary Honest

The current editor does not pretend to already be the final runtime authoring path. That is a strength, not a weakness. The correct next step is to define a clean bridge from editor-authored documents into runtime scene and prefab formats, not to smuggle ad hoc editor assumptions into the engine.

## How to Run the Editor

From the workspace root:

```bash
cargo run -p rengine-editor
```

The binary opens a native desktop window for the editor shell.

## How to Use the Editor Today

### Start a New Scene

- Launch the editor.
- Use `New Scene` in the top bar to open another untitled scene tab.
- Give the scene a name in the Scene panel.

### Switch Between Scenes

- Use the tab strip over the viewport to switch between open scene documents.
- Unsaved tabs show a trailing `*` in their tab label.

### Add Nodes

- Right-click in the viewport to add a Group, Empty, Camera2D, Sprite, Trigger, or UI Root node.
- Right-click a node in the Scene hierarchy when you want to add a child under that node.
- Choosing `Sprite...` will use the selected image file from the Files panel when one is highlighted, otherwise it opens an image picker. If you cancel, the sprite is still created with a visible placeholder preview.
- Alternatively, double-click in the viewport to add an Empty node at the pointer.

### Select and Edit Nodes

- Click a node in the viewport or hierarchy.
- Edit scene window size in the Scene View section of the Properties panel.
- Edit a selected node's name, kind, position, size, visibility, or script path in the Properties panel.
- Sprite nodes expose texture preview selection and placeholder fallback.
- Camera2D nodes expose zoom and view-size settings, and the viewport draws the resulting camera screen rectangle.
- Drag a visible node in the viewport to reposition it.

### Browse the Project

- Use the Files panel to navigate the workspace.
- Use the filter field to narrow the visible tree.
- Use `Refresh` if the filesystem changed outside the editor.

Refresh now runs in the background, so the browser should stay visually stable while the scan completes.

### Save the Scene

- Use `Open Scene`, `Save Scene`, or `Save As` in the top bar.
- `Save Scene` writes back to the current file if one is open, or falls back to the scratch path for a brand-new document.
- `Save As` uses a native file dialog so you can write directly into a sample's asset directory.
- The bottom panel can show the saved JSON structure.

If a runtime sample points `Engine::load_scene2d()` at that editor-authored JSON, the loader can now adapt it directly.

## How to Use the Editor for a Basic Top-Down Game

The top-down sample is the clearest near-term target because it already uses scene data.

The current sample runtime path is:

- asset root set to the sample's `assets/` directory
- asset manifest loaded from `topdown.assets.json`
- scene data loaded from `world.scene.json` through `Engine::load_scene2d()`
- runtime logic reads prefabs such as `player_spawn`, `enemy_spawn`, and `gem_spawn`

You can now use the editor as the authoring front end for the same idea without writing a custom scene adapter first.

A practical workflow looks like this:

1. Create a scene in the editor for the level.
2. Add Empty or Trigger nodes for `player_spawn`, `enemy_spawn`, and `gem_spawn` markers.
3. Add Group nodes when you want a single runtime prefab made from multiple descendant sprites.
4. Add Sprite nodes for decorative props such as trees and fill in the sprite asset alias in the Inspector.
5. Use the runtime-prefab field only when you want the runtime prefab name to differ from the node name.
6. Use `Save As` to write the scene JSON straight into the top-down sample's asset folder.
7. Point `Engine::load_scene2d()` at that file.
8. Keep gameplay logic in Rust while moving placement and spawn layout out of code.

That gets the top-down sample closer to a real editor-driven workflow without waiting for the full prefab/UI/play-mode stack to land.

## How to Use the Editor for a Basic Platformer

The platformer sample is currently more code-authored than the top-down example, so the editor helps in a slightly different way.

A realistic short-term workflow is:

1. Create nodes for player spawn, platforms, checkpoints, hazards, pickups, and camera markers.
2. Use node names or runtime-prefab overrides to define the runtime-facing identifiers that platformer code will query.
3. Use Group nodes when you want a platform chunk or decorative assembly to export as one runtime prefab instance.
4. Save the editor scene JSON and load it through `Engine::load_scene2d()`.
5. Read marker metadata such as position, size, script path, and instance properties from the adapted `Scene2D` instances.
6. Keep movement, collision response, and game rules in Rust while moving placement and layout into editor-authored data.

This still requires platformer-specific gameplay code to interpret those instances, but it no longer requires a separate scene-format adapter just to get the authored data into the runtime.

## What Is Missing Before the Editor Can Replace Hand-Authored Game Layout

The biggest missing pieces are:

- fuller local-space prefab authoring and nested-scene composition beyond the current grouped export adapter
- direct asset drag-and-drop instantiation
- prefab workflows and override handling
- script field editing beyond a raw script path
- richer viewport tools such as snapping, guides, and transform gizmos
- tilemap and collision authoring
- runtime inspection while the game is live (play-in-editor itself has landed: the top-bar **Play** button saves the active scene and launches the project's game target via `cargo run`; **Play** while running is a fast restart, and **Stop** kills it)
- undo or redo
- safe file operations with reference rewriting

Until those exist, the editor is best understood as a serious shell and authoring foundation rather than a complete production tool.

## Recommended Near-Term Workflow

If the goal is to start using the editor on real sample migrations soon, the most sensible path is:

1. Expand the current runtime bridge so grouped export grows into fuller prefab and nested-scene composition.
2. Add prefab/spawn-marker conventions aimed directly at the top-down and platformer samples.
3. Add undo or redo, snapping, and safer file operations.

That path keeps the editor honest, useful, and directly connected to the actual sample games instead of drifting into mock-tool territory.
