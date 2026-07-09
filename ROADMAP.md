# Rengine Roadmaps

Roadmap planning is now split into separate runtime and editor tracks.

- `ROADMAP_ENGINE.md` tracks engine-runtime work such as rendering, assets, simulation, serialization, and platform support.
- `ROADMAP_EDITOR.md` tracks editor-shell, authoring, workflow, validation, and play-in-editor work.
- `EDITOR_GUIDE.md` explains how the current editor works, why it is structured this way, and how to use it as the start of a game-authoring workflow.

Current status: the editor-to-`Scene2D` bridge now supports grouped multi-sprite prefab export, the shell has native open/save-as flow, scene work lives in per-document tabs, and the editor migration to a rengine-native shell now includes a custom canvas-driven layout plus engine-native inspector editing that stays focused on scene or selected-node properties, including a selected-node Create Child Node popup in the properties panel, filtered project browsing with a working filter field, double-click open, and a project-entry context menu whose popup width expands to fit its labels, reliable native right-click context menus across the file browser, hierarchy, and viewport with popup placement clamped into the visible window near top edges, hierarchy-driven node creation via right click on the scene header or existing nodes with explicit root-versus-child add labels in the popup, a proper scene-header selected state when the scene itself is selected, real sprite previews, viewport panning, native scroll indicators, collapsible and draggable-resizable side/bottom panels that collapse into shared stacked side-button strips or slim reopen slivers, a clipped scrollable inspector form, responsive button rows/tab strips and inspector action buttons that only trim when their container actually runs out of space, delayed hover tooltips for trimmed canvas labels, editor-wide single-owner text-input focus running directly on the runtime with click-scoped ownership transfer across the project-browser and inspector `Ui` trees, and a first usability pass toward desktop conventions with explicit top-left app/file/view/theme menus and switchable editor themes. Startup now resolves a project manifest (`.project` / `*.project`) via `--project`, current directory detection, or a native picker fallback so project root and metadata are explicit at boot. The native editor implementation now lives behind `editor/src/app.rs` plus focused `editor/src/app/` modules for windowing/layout, filesystem and scene I/O, popup handling, form state, drawing, and shared editor state.

Current editor priority order: authoring safety now includes undo/redo plus autosave, and the editor now has multi-select, box selection, frame-selection, grid-snapped dragging, selection-center guides, a first axis-constrained translate gizmo, duplicate, reparent, reorder, and selection-history workflows. Play-in-editor with fast restart has landed (top-bar Play/Stop launches the project's game target via `cargo run`; Play while running restarts it). Next up are rotate and scale gizmo follow-up, then gameplay markers, trigger or path tools, prefab reuse, UI documents, and structured data-document workflows so content-heavy 2D games can move out of hand-authored Rust.

This split keeps runtime-system work distinct from authoring-tool work while both are moving forward in parallel.

## Next 10 Most Impactful Tasks (Engine + Editor)

_Prioritized after a full pass over both rengine and its first real client,
Formula R (2026-07-09). rengine exists to make content-heavy 2D games authorable
without hand-writing Rust for everything, and Formula R is a **UI-, data-, and
text-heavy** game — so this list is deliberately weighted toward authoring,
data, UI, and text rather than generic engine features (3D, physics, tilemaps
matter less for the actual client in front of us). Each item is tagged `[engine]`
or `[editor]`; see `ROADMAP_ENGINE.md` / `ROADMAP_EDITOR.md` for the full tracks._

1. **[editor] Structured data-document editor with typed schemas.** Formula R's
   balance and content live in hand-edited `tuning.json` / `progression_tuning.json`
   plus Rust tables (circuits, eras, upgrades, sponsors). A schema-driven table/record
   editor with validation is the single biggest authoring win — it's how balancing and
   content stop requiring code edits or raw-JSON surgery, for this game and any
   data-driven one. Already the editor roadmap's "structured data documents" goal.

2. **[editor] Richer script param kinds — enum + asset reference.** The script
   manifest only supports string/number/bool/color, so Formula R's `param_command`,
   `param_action`, `param_filter`, and `param_index` are stringly-typed and
   unvalidated (an author can type a garbage value that fails silently at runtime).
   Enum dropdowns + asset pickers remove a whole class of errors and are already the
   flagged "Next." Low effort, high daily value for both of us.

3. **[editor] UI document type + widget authoring.** Formula R hand-codes every
   screen and HUD in Rust; the scene-driven migration has spent many PRs laboriously
   moving pixel constants into scene metrics. A dedicated UI document (containers,
   widgets, anchors/margins, styles, live preview) is the structural fix — it makes
   menus, HUD panels, and card layouts authorable instead of hardcoded, which is the
   entire point of the migration initiative.

4. **[engine] Event / signal system.** Gameplay comms are currently ad-hoc per-family
   queues (each scene family owns an `Arc<Mutex<Vec<Action>>>` — we just spent two PRs
   consolidating that boilerplate on the game side). A first-class event/signal bus is
   the runtime primitive that lets scripts, UI, and systems communicate without bespoke
   plumbing; it's a flagged core-runtime gap and would retire a lot of glue.

5. **[editor] Rotate + scale gizmos.** Only a translate gizmo exists, so any real
   layout still falls back to numeric inspector edits for rotation/scale. Completing the
   transform toolset is table-stakes spatial editing and the editor roadmap's stated
   immediate next step.

6. **[editor] Prefab assets + nested scenes.** The runtime already instances nested
   scenes (`instantiate_scene_tree`), but the editor only does grouped export. First-class
   prefabs unlock reuse for repeated setups (trackside props, HUD chunks, spawn packs) —
   the "content-heavy 2D" checklist item that stops content authoring from being
   copy-paste.

7. **[editor] Live hot-reload of scenes & data into the running game.** Play-in-editor
   launches the game, but tuning a value still means stop → edit → rebuild → replay.
   Live reload of scene/data documents into the running target collapses the iteration
   loop to seconds — the "tune balance in minutes, not rebuild cycles" goal, which is
   exactly how Formula R balancing should feel.

8. **[editor] Gameplay marker / trigger / path tools.** Formula R's spatial content
   (route loops, pit entry/exit volumes, camera anchors, encounter/reward nodes) is the
   "content-heavy 2D authoring" checklist. Dedicated marker/trigger/path tools let this
   be placed visually and feed the runtime trigger/scene systems that already exist,
   instead of being hand-authored in Rust.

9. **[engine] Richer 2D text + screen-space debug drawing.** Formula R is text-dense
   (HUD numbers, standings, event log, roll breakdowns) and "strategic clarity /
   readability" is a product pillar, yet engine text lacks the outlines/shadows/bitmap-
   font support that keep dense UI legible over busy backgrounds. Screen-space debug
   draw (lines/rects/labels) also accelerates both engine and game debugging.

10. **[engine] Async / background asset loading.** Synchronous loading is fine today but
    becomes the bottleneck as content scales (more circuits, art, audio for the game's
    content-expansion push) and blocks real loading screens. Flagged in the engine
    roadmap; foundational for larger projects. _(Runner-up, same track: deterministic
    replay recording — it builds on the sim's existing seed/determinism work to enable
    shareable runs and regression-grade repro.)_

_Sequencing note: #1–#3 (data docs, typed params, UI docs) are the authoring
force-multipliers that most directly unblock Formula R's own roadmap (its content
expansion + balance work leans on them); #4–#8 are core authoring ergonomics; #9–#10
are readability/scale investments that can proceed in parallel._
