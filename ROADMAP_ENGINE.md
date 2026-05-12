# Rengine Engine Roadmap

This document tracks runtime, rendering, simulation, asset, and platform work in the engine itself.

Editor and authoring-tool work now live in `ROADMAP_EDITOR.md`.

## Current Position

Rengine already has a meaningful runtime foundation:

- batched 2D sprite rendering
- immediate-mode 3D rendering
- engine entry points for 2D and 3D games
- canvas and text overlay rendering
- sprite sheets, tweening, particles, and animation state machines
- tilemaps, AABB overlap helpers, collision layers, and trigger volumes
- keyboard, mouse, and gamepad input with action mapping and rebinding hooks
- save/load support
- file-backed asset loading with manifests, sprite sheets, OBJ, glTF, audio, JSON resources, and hot reload
- scene stacks, scene lifecycle hooks, and globals for runtime state sharing
- built-in debug overlay and developer console

What still matters most on the engine side is closing the remaining runtime gaps so the editor has stable systems to target.

## Recently Completed

- asset manifests, resource loading, hot reload, and dependency-aware bundle retention
- improved 2D camera, fixed timestep hooks, RNG helpers, resolution scaling, and render targets
- richer audio controls including bus routing, fades, and crossfades
- immediate-mode UI widgets, layout containers, scroll regions, images, tooltips, styling, text input, and animation hooks
- OBJ and glTF mesh import groundwork for the 3D path
- debug overlay, console, and configurable log capacity
- scene-script host scaffolding now includes targeted dispatch helpers (by script path/editor name) and binding lookups for scene-authored event routing
- Scene2D now exposes typed scalar/tag parsing helpers plus direct lookup helpers by editor node id, editor name, and conventional tags

## Runtime Priorities

### Scene-Driven Game Authoring Initiative

- Promote scene JSON from passive render data into an engine-native gameplay surface.
- Add typed scene instance metadata accessors so game code does not parse string maps ad hoc.
- Add a script host and registry contract that can bind `script_path` values to runtime behavior.
- Add scene-entity lookup and event routing so scripts can react to input and state changes.
- Keep this runtime-first and backend-agnostic so script execution can start with native Rust handlers and later support a VM.

### Core Runtime and Data Model

- Build a coherent object or entity model so games stop reinventing world organization from scratch.
- Add component-style composition or an equally strong runtime composition story.
- Add a general event or signal system for decoupled gameplay communication.
- Formalize script attachment metadata so scenes, prefabs, and editor-authored objects can bind to runtime behavior safely.
- Strengthen stable serialization for scenes, prefabs, resources, and references.
- Expand resource dependency tracking so the engine can answer what depends on what without guesswork.
- Add deterministic simulation helpers for rollback, replays, and stable tests.

### Asset and Content Pipeline

- Keep expanding the asset manager into a true virtualized content layer instead of a convenient loader.
- Improve error reporting, fallback behavior, and diagnostics for bad or missing assets.
- Add async or background asset loading for larger projects and loading screens.
- Strengthen build profiles for debug, release, headless, and future platform-specific variants.
- Add richer import metadata and validation hooks for editor-facing tooling.

### 2D Production Path

- Expand from simple AABB overlap into stronger 2D physics with velocity, gravity, friction, restitution, and moving platforms.
- Add tilemap layers, tile metadata, animated tiles, autotiling, and Tiled import.
- Add custom 2D materials or shader hooks without forcing renderer rewrites per project.
- Improve text further with outlines, shadows, bitmap font support, and richer style variants.
- Add better screen-space debug rendering and deeper performance counters.

### 3D Production Path

- Finish full 3D transforms with rotation and scale per draw.
- Add transform hierarchies for attachments, grouped props, and character rigs.
- Add real 3D material support with textures, normal maps, roughness, metallic, emissive, and transparency.
- Add multiple light types and shadow mapping.
- Add skeletal animation and animation blending.
- Add frustum culling and instanced rendering.
- Add 3D collision queries and a real 3D physics backend.
- Add spatial audio.

### Debugging and Tooling Hooks

- Keep building the debug overlay into a stronger runtime instrumentation surface.
- Add structured logging domains across renderer, assets, audio, gameplay, networking, physics, and tools.
- Add replay recording support.
- Add clearer editor-facing metadata, reflection hooks, and validation seams instead of editor-only hacks.

### Platform and Long-Term Support

- Add navigation and pathfinding support where genre needs justify it.
- Add localization and accessibility support.
- Harden web target behavior and persistence.
- Add mobile-specific hooks like touch, vibration, safe areas, and lifecycle handling.
- Explore client-server multiplayer helpers separately from rollback.
- Add mod support only once the runtime data model is stable enough to expose safely.

## Next Runtime Milestones

0. Land scene binding helpers and script host scaffolding so scene-authored behavior can run without game-specific glue.
   - Status: In progress (scene binding helpers, SceneScript2D registry/host scaffolding, targeted event routing, binding lookups, and Scene2D editor-name/id/tag lookup helpers are landed; remaining work is broader runtime ergonomics and higher-level scripting workflows).
1. Stabilize runtime serialization, IDs, and dependency tracking so the editor has trustworthy data contracts.
2. Add a stronger object or component model that scenes and scripts can target consistently.
3. Finish the missing 3D transform and material basics so imported content stops being fragile.
4. Deepen 2D content workflows with better physics and tilemap authoring support.
5. Add async asset loading and stronger diagnostics so larger projects do not stall on the main thread.

## Guiding Principle

The engine roadmap should optimize for stable, reusable runtime systems.

If a feature is primarily about authoring, workflow, docking, editing, validation UX, or play-in-editor, it belongs in `ROADMAP_EDITOR.md`.
If a feature is primarily about rendering, simulation, assets, runtime APIs, serialization contracts, or cross-platform behavior, it belongs here.
