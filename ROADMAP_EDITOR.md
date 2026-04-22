# Rengine Editor Roadmap

This document tracks the editor as its own product inside the workspace.

The runtime engine roadmap lives in `ROADMAP_ENGINE.md`.

## Current Prototype Status

The current shell already has:

- a dedicated `editor/` crate
- a native desktop window built with `eframe` and `egui`
- a responsive top bar
- scene tabs for switching between in-progress documents
- a project/filesystem browser
- a scene hierarchy panel
- a grouped properties panel with scene-view, node, sprite, and camera settings
- an activity log and JSON preview panel
- a draggable 2D scene canvas
- right-click add-node menus in the viewport and hierarchy instead of the old top-row create-button strip
- sprite-node creation that can browse for image files and falls back to a placeholder preview if no texture is chosen
- Camera2D bounds preview driven by adjustable scene window size, camera zoom, and optional camera-local view size
- background workspace rescans so refreshes do not stall the UI thread
- a paused JSON preview during live drag or resize interactions to reduce visual flashing
- tighter repaint scheduling so the shell only forces continuous redraw during active viewport drags
- native open/save/save-as scene flow through file dialogs
- a `Scene2D` runtime bridge that supports marker export, single-sprite export, and grouped multi-sprite prefab export

That is enough to prove the layout direction, but it is not yet enough for real game production.

## Editor Success Criteria

The editor is successful when a developer can build and iterate on a game like the current platformer or top-down samples without hand-authoring most scene, UI, and authoring data in Rust.

That means the editor must eventually cover:

- project setup and file management
- asset browsing and safe reference-aware file operations
- scene authoring, prefab reuse, and script attachment
- UI authoring
- gameplay markers such as spawns, triggers, collision zones, and paths
- play-in-editor, logs, validation, and recovery
- reliable save formats that stay diffable and merge-friendly

## Complete Feature Set

### Project and Workspace Management

- Recent projects list, create project flow, open project flow, and template-based project bootstrap.
- Per-project editor settings and saved window layouts.
- Crash recovery, autosave recovery, and unsaved-document restore.
- Command palette and shortcut management.
- Task runner integration for build, test, export, and custom project tasks.

### Filesystem and Content Browser

- Tree view, list view, filters, search, and favorites.
- Create, rename, move, duplicate, delete, and reveal-in-explorer/file-manager operations.
- Drag and drop between folders and into authoring documents.
- Safe rename and safe move with reference rewriting.
- Asset labels, bookmarks, collections, and quick filters.
- Dependency viewer for "used by" relationships.
- Background indexing and refresh so the browser never blocks the UI thread.

### Asset Inspection and Import

- Texture preview with dimensions, alpha, filtering, and packing metadata.
- Audio preview with waveform, duration, bus target, and looping metadata.
- Sprite sheet preview with grid slicing, named regions, and animation preview.
- Mesh preview for OBJ and glTF assets.
- Font preview and fallback chain inspection.
- JSON/resource document preview and schema validation.
- Bulk import, reimport, and missing-asset repair tools.

### Scene Documents and Data Contracts

- Stable scene IDs and node IDs.
- Merge-friendly text formats for scenes, prefabs, UI docs, tilemaps, triggers, splines, and metadata.
- Versioned document schemas with upgrade paths.
- Explicit separation between editable authoring state and runtime-instantiated state.
- Save, Save As, duplicate scene, scene templates, and multi-document tabs.
- Strong validation for broken references, duplicate IDs, missing scripts, and invalid nesting.

### Hierarchy, Outliner, and Selection

- Tree hierarchy with nesting, reparenting, folders, visibility toggles, and lock toggles.
- Marquee selection, multi-select, and batch edits.
- Searchable outliner filtering.
- Selection history and frame-selection shortcuts.
- Override indicators for nested scenes and prefab instances.

### 2D Viewport and Spatial Editing

- Pan, zoom, frame selection, and rulers.
- Grid, guides, snapping, and world origin visualization.
- Translate, rotate, scale, pivot, and origin editing.
- Duplicate, align, distribute, and reorder operations.
- Layer visibility toggles and selection masking.
- Camera preview and safe-area overlays.
- Visual continuity under interaction: background tasks, cached previews, and continuous repaint during drags.

### 3D Viewport and Spatial Editing

- Orbit, pan, fly, and focus controls.
- 3D transform gizmos and local/global axes.
- Lighting preview, camera preview, and navigation helpers.
- Selection picking and outliner synchronization.
- View modes for lit, unlit, collision, wireframe, and debug overlays.

### Prefabs, Nested Scenes, and Reuse

- Prefab assets and nested scene assets as first-class documents.
- Apply/revert override workflows.
- Instance override inspection in the hierarchy and inspector.
- Template scenes for common bootstraps such as top-down map, platformer stage, menu, and HUD.

### Gameplay Composition and Script Attachment

- Attach external scripts to nodes, scenes, UI widgets, triggers, and prefabs.
- Exposed-field editing in the inspector.
- Drag and drop reference assignment from scene tree and content browser into script fields.
- Missing-script detection and repair tools.
- Metadata for tags, layers, groups, teams, collision channels, and gameplay categories.
- Searchable add-component or add-behavior workflow.

### UI Authoring

- Dedicated UI document type rather than forcing HUD layout into ad hoc scene nodes.
- Widget palette for labels, buttons, panels, rows, grids, scroll regions, checkboxes, sliders, text inputs, images, tooltips, and animation hooks.
- Visual create, resize, reorder, and reparent workflows.
- Anchors, margins, padding, alignment, spacing, and resolution presets.
- Theme, style, and variant editing.
- UI templates and reusable HUD/menu chunks.
- Runtime preview for keyboard, gamepad, and mouse navigation.

### Shapes, Triggers, Splines, and Tilemaps

- Box, circle, polygon, and spline authoring.
- Trigger authoring with events, filters, layer masks, and metadata.
- Path authoring for AI routes, patrol paths, camera rails, conveyors, and motion guides.
- Tilemap editor with layers, paint/fill tools, metadata, autotiling, and collision views.
- Collision overlay and bounds preview.

### Specialized Editors

- Particle editor with curves, presets, and live preview.
- Animation and timeline editor for clips, transitions, and state machines.
- Cutscene sequencing editor.
- Audio mixer or bus editor.
- Localization table editor.
- Save-data and replay inspection tools once runtime support exists.

### Play-In-Editor and Debugging

- Play, pause, stop, and frame-step inside the editor.
- Preserve authoring state when entering play mode and restore it on exit.
- Console output and log filtering backed by the engine debug system.
- Runtime object inspection while the game is live.
- Clickable errors that jump to the responsible asset, node, or script slot.
- Hot reload for assets and authored documents while the editor is open.
- Overlay toggles for collision, triggers, paths, FPS, and other debug views.

### Validation, Recovery, and Safety

- Autosave and timed backups.
- Corrupted-document diagnostics and recovery flows.
- Project-wide validation passes.
- Broken reference reporting before delete or rename operations complete.
- Safe destructive actions with previews and confirmations.

### Build, Export, and Runtime Integration

- Build profiles surfaced in the editor.
- Launch selected sample or game target from the editor.
- Export/build packaging workflows when the runtime supports them.
- Scene and UI document adapters that map editor-authored data cleanly into runtime formats.
- Asset bundle and manifest integration rather than parallel content systems.

### Source Control and Team Workflows

- Changed-files view.
- Diff and open-file shortcuts.
- Conflict warning surfaces.
- Locking or edit-intent hints where useful.
- Reference-aware operations that behave well in git-based workflows.

### Extension Points and Internal Architecture

- Extension/plugin API for custom inspectors, validators, importers, and tools.
- Internal command stack for undo and redo across every editing operation.
- Clear separation between shell UI, document state, runtime bridge, and background services.
- Background services for indexing, preview generation, validation, and import processing.

### Documentation and Onboarding

- A clear editor guide that explains the shell, why it is structured this way, and how to build a first game with it.
- Project templates for common starter games.
- Better sample-to-editor bridge docs so existing examples can migrate incrementally instead of all at once.

## Basic 2D Game Readiness Checklist

For a platformer or top-down project to feel realistic in the editor, the editor must support:

- player, enemy, pickup, checkpoint, and camera spawn markers
- static prop placement and prefab reuse
- collision shapes and trigger volumes
- patrol paths or motion paths where needed
- level-specific parameters and script-exposed fields
- HUD and pause/menu authoring
- play-in-editor with fast restart
- logs, validation, and missing-reference warnings
- safe save formats that can be committed and reviewed in git

## Minimum Viable Editor Cut Line

If the goal is to stop hardcoding most game content in Rust, the first genuinely useful editor must include:

1. project/file browser
2. scene hierarchy with nesting
3. viewport selection, gizmos, guides, and snapping
4. inspector and exposed-property editing
5. prefab or nested-scene workflow
6. external script attachment
7. trigger, shape, and path tools
8. visual UI builder
9. play-in-editor with logs
10. undo/redo, autosave, and safe reference updates

## Phased Plan

### Phase 0: Data Contracts and Editor Runtime Split

- Keep the editor in its own crate.
- Finalize stable IDs and merge-friendly document formats.
- Build the command stack and undo or redo model.
- Separate editable document state from runtime state.

### Phase 1: Shell, Workspace, and Continuity

- Keep improving shell responsiveness, background work scheduling, and panel stability.
- Add project management, layout persistence, command palette, and autosave.
- Finish safer filesystem workflows.

### Phase 2: Core Scene Authoring

- Add full hierarchy editing, multi-select, reparenting, snapping, and transform workflows.
- Add tabs and multiple documents.
- Expand the current runtime scene bridge beyond grouped sprite composition into richer prefab and nested-scene workflows.

### Phase 3: Prefabs, Scripts, and Gameplay Composition

- Add prefab assets, nested scenes, script attachment, reference assignment, and richer inspector editing.

### Phase 4: Visual UI Authoring

- Add dedicated UI documents and runtime preview.

### Phase 5: Shapes, Triggers, Splines, and Tilemaps

- Add authored geometry, triggers, path tools, and tilemap editing.

### Phase 6: Play, Debug, and Validation

- Add play-in-editor, runtime inspection, validation, and hot reload integration.

### Phase 7: Specialized Editors

- Add particle, animation, cutscene, audio, and localization editors.

### Phase 8: Team Scale and Extensibility

- Add plugin hooks, source control awareness, and stronger project-scale workflows.

## Immediate Next Steps

- Add undo or redo support before editing tools multiply further.
- Add richer viewport tools, snapping, and selection ergonomics.
- Add basic prefab and spawn-marker workflows aimed directly at the top-down and platformer examples.
- Expand the runtime bridge with richer properties, stronger prefab composition, and better nested-scene/export workflows.
