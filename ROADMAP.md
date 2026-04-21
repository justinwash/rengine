# Rengine Roadmap

## Current Position

Rengine is already beyond a toy framework. It has:

- A batched 2D sprite renderer with UVs, tinting, and flipping
- A simple immediate-mode 3D renderer with depth and directional lighting
- A basic runtime loop and engine traits for 2D and 3D games
- Canvas and text overlay rendering
- Sprite sheets and frame animation helpers
- Tilemaps and basic AABB collision
- Input handling for keyboard, mouse, and gamepads
- Optional rollback netcode support
- A broader asset pipeline with cached file-backed loading for bytes, text, textures, sprite sheets, OBJ and glTF meshes, audio clips, and JSON asset manifests
- Configurable asset roots on both the 2D and 3D engine entry points
- Hot reload for textures, meshes, manifests, and audio clips during development
- Basic scene and prefab loading for 2D content, with reusable prefab instances and scene-driven spawn points
- Higher-level audio controls for bus routing, master and per-bus volume, and music pause or resume
- Sample coverage for manifest-driven 2D scenes, imported 3D meshes, glTF loading, and richer audio playback

What it still does not have is the broader game-development infrastructure that makes an engine feel complete: higher-level scene switching and lifecycle management, richer serialization beyond the current JSON scene and manifest layer, stronger physics, UI, more complete 3D materials and animation import, and tooling.

This roadmap is ranked by importance to normal game development, informed by the feature sets of engines and frameworks like Godot, Defold, GameMaker, Love2D, and MonoGame.

---

## Progress Update

Recently completed or partially completed:

- Completed: file-based texture loading with caching
- Completed: centralized asset pipeline for bytes, text, textures, sprite sheets, OBJ and glTF meshes, audio clips, and JSON manifests
- Completed: asset root configuration for both Engine and Engine3D
- Completed: hot reload for textures, meshes, manifests, and audio clips
- Completed: basic 2D scene and prefab loading with scene-driven sample content
- Completed: richer audio controls for bus routing, master and per-bus volume, music pause or resume, and one-shot sound effects on buses
- Completed: initial 3D mesh import support via OBJ and glTF, including automatic winding correction against authored normals
- Completed: sample migrations for the platformer, top-down, fight, FPS, and FPS-MP examples to exercise the new asset APIs
- Completed: headless audio mode — mutes output while still exercising decode and playback paths so tests remain representative without audible output
- Completed: dedicated viewmodel render layer for 3D — separate camera, projection, FOV, and depth clear so viewmodels do not clip into world geometry
- Completed: camera-relative viewmodel rendering — viewmodel geometry is authored in local space and transformed through the player camera at render time
- Completed: scene switching and lifecycle management — `Scene`/`Scene3D` traits with `on_enter`, `update`, `render`, `on_pause`, `on_resume`, `on_exit` hooks; `SceneOp`/`SceneOp3D` enums (`Continue`, `Push`, `Switch`, `Pop`, `Quit`); full scene stack with bottom-to-top rendering for transparent overlays
- Completed: `Globals` typed key-value store — `TypeId`-keyed `Any` map shared across the scene stack for persistent cross-scene state
- Completed: `run_with_scenes` / `run3d_with_scenes` entry points — scene-aware alternatives to `run` / `run3d` with init closure, automatic lifecycle dispatch, and stack management
- Completed: `show_fps` config toggle on `EngineConfig` — FPS overlay is now opt-in (defaults to true for backward compat)
- Completed: feature samples convention — `samples/features/feature-<name>/` directory for engine feature demos, separate from game samples
- Completed: `feature-scenes` sample — demonstrates Switch, Push/Pop, pause overlay with transparency, Globals-based persistent counters, and all lifecycle hooks
- Completed: improved 2D camera — rotation, smooth follow with configurable speed, dead zones, screen shake with decay, camera bounds clamping via `CameraBounds`, projection refactored to ortho × view matrix
- Completed: `feature-camera` sample — demonstrates follow, dead zone, bounds, shake, and rotation toggle
- Completed: input action mapping and engine-side rebinding — `ActionMap` with named digital actions and analog axes, `Binding` enum for Key/MouseButton/GamepadButton, `AxisMapping` with positive/negative bindings and optional gamepad stick axis, convenience methods on `Engine` and `Engine3D`, plus runtime `bind`/`unbind`/`clear` and `bind_axis`/`unbind_axis` support
- Completed: `feature-input` sample — demonstrates action binding setup, axis-driven movement, pressed/down/released queries with visual feedback
- Completed: asset pipeline validation and dependency tracking — `validate_manifest()` for pre-load checks, `manifest_dependencies()` for tracking which files each manifest loaded, `loaded_asset_summary()` for debugging, `unload_texture/mesh/data()` for cache eviction
- Completed: serializable resources — `load_resource<T>()` and `load_resource_list<T>()` on Engine and Engine3D for JSON-driven data definitions with serde deserialization
- Completed: fixed-timestep update — `EngineConfig::fixed_dt` (default 1/60), accumulator-based `consume_fixed_step()`, `fixed_update()` hooks on Game, Game3D, Scene, and Scene3D traits wired into all four run functions
- Completed: collision layers and masks — `CollisionLayer` bitmask struct with named constants (WORLD, PLAYER, ENEMY, PROJECTILE, TRIGGER, UI), `interacts_with()` check, `aabb_overlap_layered()` for filtered AABB tests
- Completed: trigger volumes and overlap sensors — `TriggerSystem` with `TriggerZone` (Rect + CollisionLayer), enter/stay/exit events via `OverlapEvent`, per-zone enable/disable, `feature-triggers` sample
- Partial: 3D transforms still only support position-based translation; rotation and scale per draw are not yet supported (caused the recurring door visibility issue)
- Completed: seeded RNG — `Rng` struct (xoshiro256\*\*) with deterministic seeding, `engine.rng()` accessor on Engine and Engine3D, game-dev convenience methods (range, weighted, shuffle, pick, normal distribution, Vec2 helpers, fork), `feature-rng` sample
- Completed: nine-slice rendering — `NineSlice` struct with uniform/asymmetric borders, `frame.draw_nine_slice()`, color tinting, z-order, `feature-nineslice` sample
- Completed: tweening and easing — `Tween` struct with 25 easing curves, `LoopMode` (Once/Loop/PingPong), `lerp()` and `ease()` helpers, `feature-tween` sample
- Completed: text layout — `measure_text`, `line_height`, `TextAlign` (Left/Center/Right), `text_aligned`, word wrapping (`wrap_text`), `text_block`, `feature-text` sample
- Completed: immediate-mode UI widgets — `Ui` builder with `label`, `label_centered`, `button`, `separator`, focus navigation (arrow keys/WASD), activation (Enter/Space), customizable `UiStyle`, `feature-ui` sample
- Completed: save/load — `SaveSystem` with slot-based JSON persistence, platform-appropriate save paths, `feature-saveload` sample
- Completed: resolution scaling — `ScaleMode` enum (Stretch, Letterbox, PixelPerfect), offscreen render target + blit pass, Canvas/HUD at window resolution, runtime switching, `feature-resolution` sample
- Completed: particle system — `EmitterConfig` builder (14+ fields), `ParticleEmitter` pool with O(1) alive count, `EmitShape` (Point/Circle/Rect), `RangeF32` for randomized params, `Color::lerp`, burst/continuous modes, `feature-particles` sample, kitchen-sink integration
- Completed: audio fades — `ActiveFade` with `FadeTarget` (MusicVolume/CrossfadeOut/BusVolume/MasterVolume), fade-in/fade-out/crossfade music, bus and master volume fades, any `Easing` curve, auto-ticked in game loop, `feature-audio` sample, kitchen-sink integration
- Completed: post-processing pipeline — `PostFxChain` with `PostEffect` enum, 8 built-in effects (Vignette, Blur, Bloom, ColorGrade, Crt, Pixelate, ChromaticAberration, Invert), custom WGSL shader support via `PostEffect::Custom`, ping-pong texture chain, lazy pipeline rebuild, `feature-postfx` sample, kitchen-sink integration
- Completed: mouse position tracking — `InputState::mouse_position` in screen-space center-origin coords, `handle_cursor_moved()`, `mouse_position()` accessor, `engine.mouse_screen_pos()` on Engine and Engine3D, `Camera2D::screen_to_world()` for world-space conversion, `MouseInput` handling in 2D event loops
- Completed: canvas drawing primitives — `line()`, `polyline()`, `circle()` (outline), `circle_filled()` on Canvas for immediate-mode shape drawing
- Completed: colored text spans — `text_spans()` and `text_spans_aligned()` on Canvas, accepting `&[(&str, Color)]` for per-substring coloring
- Completed: UI overhaul — mouse hover/click support for all focusable widgets, Panel widget (background rect with padding), ProgressBar widget, Checkbox widget (toggle on Enter/Space/click), Slider widget (arrow keys adjust by 5%, mouse drag), expanded `UiResponse` with `hovered`, `toggled`, `changed_values` fields and `was_activated()`, `was_toggled()`, `value_for()` convenience methods, updated `feature-ui` sample with DemoScene
- Completed: scene transitions — `Transition` struct with `fade()`, `fade_color()`, `fade_white()` constructors, `SceneOp::FadePush`, `FadeSwitch`, `FadePop` variants, transition state machine in `run_with_scenes` (fade out → apply at midpoint → fade in, scene frozen during transition), updated `feature-scenes` sample
- Completed: timer and event queue — `Timer` struct with `once()` and `repeating()` constructors, `tick(dt) -> bool`, `fraction()` progress; `EventQueue<E>` generic delayed event scheduler with `schedule(delay, event)` and `tick(dt) -> Vec<E>`
- Completed: canvas ergonomics — `Canvas` now stores `screen_size` and `FontAtlas` pointer internally. All shape and text methods no longer require `screen_size` or `&FontAtlas` parameters. `Canvas` exposes `measure_text(text, size)` and `line_height(size)` convenience methods. `Frame::begin(screen_size, atlas)` and `Frame3D::new(screen_size, atlas)` propagate both to canvases automatically. `draw_fps()` uses canvas-internal atlas. `wrap_text()` standalone function still accepts `&FontAtlas` for use outside Canvas. `engine.font_atlas()` remains public for edge cases like direct `measure_text()` calls. All samples updated — no game code needs to call `engine.font_atlas()` for text rendering.
- Completed: UI layout containers — `Row` and `Grid` container widgets. `row(children)` distributes N children equally across the available width. `row_spaced(spacing, children)` adds horizontal gaps. `grid(columns, children)` wraps children into rows of N columns. `grid_spaced(columns, spacing, children)` adds gaps. Both containers track per-row max height so mixed-height children align correctly. Generalized internal Container stack (replaces old panel-only stack) handles Panel, Row, and Grid uniformly in both hit-testing and rendering. Updated `feature-ui` sample with LayoutScene demonstrating all variants.
- Completed: scroll regions — `Ui::scroll(id, height, scroll_offset, children)` creates a clipped scrollable container. Canvas `push_clip`/`pop_clip` for GPU scissor-rect clipping with segment-based render pass. `InputState::scroll_delta()` for mouse wheel input wired through all event loops. `UiResponse::scroll_for(id)` returns updated offsets. Focusable rects inside scroll regions are clipped to the visible area. Updated `feature-ui` sample with ScrollScene.
- Completed: multiple font support — `FontId` handle type, `Engine::load_font()` for runtime `.ttf`/`.otf` loading, per-canvas-segment font tracking with bind group switching, `Engine::font(id)` accessor, `FontId::DEFAULT` for the built-in font, backward-compatible `font_atlas()` method, `feature-fonts` sample
- Completed: asset manager follow-through — `AssetManifest` / `AssetPack` support font aliases, retained `AssetBundle` handles carry their manifest path plus tracked dependencies, and `Engine` / `Engine3D` now expose `load_asset_bundle()`, `reload_asset_bundle()`, and `unload_asset_bundle()` with shared path-retention accounting so overlapping bundles can safely release cached manifests, texture/sprite-sheet data, meshes, bytes/text, and audio clips without tearing down assets still retained elsewhere. Font source bytes can now be dropped with the bundle, while the uploaded font atlases themselves still intentionally remain engine-lifetime.
- Completed: screen-space images — `Canvas::image()`, `image_colored()`, and `image_region()` for textured screen-space quads, generalized canvas draw segments that switch between font atlases and texture bind groups, `Ui::image()` / `image_colored()` / `image_region()` widget support, `feature-images` sample, kitchen-sink pause overlay integration, and matching Engine3D texture helpers for HUD canvases
- Completed: tooltip widget — `Ui::tooltip()`, `tooltip_sized()`, and `tooltip_with()` attach explanatory text to the most recently added widget, with engine-level delay, fixed or auto sizing, mouse/widget/screen placement modes, built-in fade/fade-slide animation options, Shift-or-custom-key expansion for advanced text, and a runtime-state fix so tooltips disappear cleanly when no widget is active; includes `feature-tooltips` and kitchen-sink pause overlay coverage
- Completed: widget animation hooks — `Ui::animate_with()` attaches `UiAnimationOptions` to the most recently added widget, with reusable `UiAnimation` builders for hover, focus, press, and appear states built on top of existing `Easing` curves. Hooks currently support labels, images, buttons, text inputs, progress bars, checkboxes, and sliders, compose offset/scale/alpha at render time, keep tooltip hit rects aligned with transformed widgets, and ship with the new `feature-ui-animations` sample plus kitchen-sink pause overlay coverage
- Completed: UI polish follow-through — `Ui::animate_container_with(id, visible, options)` now gives panels, rows, grids, and scroll regions enter/exit slide hooks that keep a container alive until its exit animation finishes, while `Ui::draggable()` / `drop_target()` attach reusable drag/drop metadata to focusable widgets and expand `UiResponse` with `drag_target`, `dropped`, and `drop_for()`. `feature-ui-animations` now demonstrates both the container transition path and drag/drop reordering flow.
- Completed: widget styling variants — `Ui::style_with(UiWidgetStyle)` attaches per-widget overrides after a widget is emitted, so card rarities, warning states, or primary call-to-action buttons can diverge from the shared `UiStyle` without forking a separate theme. Supported overrides currently cover buttons, text inputs, panels, progress bars, checkboxes, sliders, and tooltip colors, and they feed layout, hit-testing, render, and tooltip presentation from the same resolved style data. Includes the new `feature-ui-styling` sample plus kitchen-sink pause overlay variants.
- Completed: UI flow helpers — `Ui::run()` / `run_with()` now collapse the common begin-build-update sequence into one call for static menus, while `Ui::sync()` / `sync_with()` rebuild the widget tree automatically after handling a `UiResponse` so stateful flows can keep labels, button text, panels, and summary widgets in sync on the same frame without duplicating the layout function in game code. Includes the new `feature-ui-flow` sample and kitchen-sink pause overlay usage.
- Completed: render targets and offscreen textures — the 2D renderer now exposes a public `RenderTarget` handle, `Engine::create_render_target()`, `Engine::resize_render_target()`, and `Frame::render_target()` for nested offscreen drawing into a texture that can immediately be reused as a sprite, UI image, or secondary preview surface later in the same frame. Added `feature-render-targets` as the reference sample for monitor/preview composition.
- Completed: text input widget — `InputState` now carries per-frame committed text plus persistent IME preedit state from winit text events, `Ui::text_input()` adds a single-line editable field with caret movement and placeholder rendering, `UiResponse::text_for()` reports changed strings, and `feature-text-input` demonstrates both direct keyboard entry and a game/sample-layer gamepad-friendly on-screen keyboard built from regular Ui buttons
- Completed: animation state machines — `Animation` now supports `Loop`, `Once`, and `PingPong` playback, while `AnimationStateMachine<State, Trigger>` layers named states, trigger-driven transitions, global transitions, and one-shot completion fallthrough on top of sprite-sheet clips. Includes the new `feature-animation-state-machines` sample with car launch, cruise, brake, and spin-out states
- Completed: in-game debug overlay and console — the engine now ships a shared `debug` module with ring-buffer log capture, on-screen overlay stats, target and severity filters, a developer console with command parsing, mouse-accessible overlay controls, and engine-level helpers on both `Engine` and `Engine3D` for toggling the surface and writing trace/debug/info/warn/error entries. Includes the new `feature-debug-overlay` sample and a `--debug-overlay` opt-in path in the kitchen-sink demo.
- Completed: sample presentation polish — feature and game demos now reserve explicit header/footer space, wrap long copy, disable stray FPS overlays by default, and keep generic UI demos visually neutral unless the sample is intentionally game-specific

Tooltip follow-up backlog after the current tooltip PR lands:

- Add richer tooltip anchor presets beyond the current widget top-right and explicit screen-position modes.
- Add a true custom tooltip animation/render hook instead of only the built-in fade and fade-slide options.
- Revisit tooltip support for layout-only widgets if real game UI ends up needing row, grid, or container-level targets.

---

## Tier 1: Highest Priority

These are the features that most directly increase the engine’s usefulness for real game projects.

1. File-based texture loading [done]
   Load PNG and other common image formats directly from disk instead of requiring raw RGBA buffers.

2. Asset manager [partially done]
   Centralized loading, caching, handles, and lifetime management for textures, fonts, sounds, meshes, shaders, tilemaps, and other resources.

3. Scene system [done]
   Scene/Scene3D traits with full lifecycle hooks, SceneOp transition enums, scene stack with bottom-to-top rendering, and Globals typed store for persistent cross-scene state.

4. Scene switching [done]
   First-class support via run_with_scenes/run3d_with_scenes entry points. Push/Pop for overlays, Switch for transitions, Quit for clean exit. Feature sample demonstrates all operations.

5. Prefabs or reusable scene instances [partially done]
   Allow reusable object templates with data overrides for enemies, pickups, UI panels, props, and level chunks.

6. Serializable resources [done]
   `load_resource<T>()` and `load_resource_list<T>()` on Engine and Engine3D load JSON files through the asset pipeline and deserialize them with serde. Any `Deserialize + DeserializeOwned` type works.

7. Better 2D transforms [done]
   Add rotation, scale, and origin support.
8. Improved 2D camera system [done]
   Rotation, smooth follow with dead zones, screen shake (intensity + duration with decay), camera bounds clamping. Projection uses view matrix (translate + rotate) applied to centered orthographic.

9. Audio playback [done]
   Music, sound effects, looping, pause or resume, bus routing, master and per-bus volume control, headless silent mode, fades (fade-in, fade-out, bus volume, master volume), and crossfades are implemented. Still missing: spatial audio.

10. Input action mapping [done]
    Named actions (`"jump"`, `"shoot"`) and axes (`"move_x"`, `"move_y"`) bound to keyboard keys, mouse buttons, and gamepad buttons/sticks. Queries via `engine.action_down()`, `action_pressed()`, `action_released()`, `axis()`. Per-player variants for multiplayer.

11. Rebindable controls [done]
    `ActionMap` supports runtime `bind()`, `unbind()`, `clear()`, `bind_axis()`, and `unbind_axis()`, so the engine side of player/game remapping is already in place. What remains game-specific is the UI for collecting new bindings.

12. Collision layers and masks [done]
    `CollisionLayer` with `layer` and `mask` u32 bitmasks. Named constants for WORLD, PLAYER, ENEMY, PROJECTILE, TRIGGER, UI. `aabb_overlap_layered()` checks layer compatibility before spatial overlap. Default is all-bits so existing code is unaffected.

13. Trigger volumes and overlap sensors [done]
    `TriggerSystem` tracks bodies against `TriggerZone` regions (Rect + CollisionLayer). Produces `OverlapEvent::Enter`, `Stay`, `Exit` each tick. Zones can be enabled/disabled at runtime. `feature-triggers` sample demonstrates checkpoint, pickup, damage, and layer-filtered zones.

14. Stronger 2D physics
    Expand beyond simple AABB overlap into rigid bodies, velocity, gravity, friction, restitution, and moving platforms.

15. Fixed update support [done]
    `EngineConfig::fixed_dt` sets the step size (default 1/60). `TimeState` accumulates frame time and `consume_fixed_step()` drains it. `Game::fixed_update()`, `Game3D::fixed_update()`, `Scene::fixed_update()`, and `Scene3D::fixed_update()` are called N times per frame before the variable `update()`. All four run functions and their headless paths are wired.

16. Save and load support [done]
    `SaveSystem` provides slot-based JSON persistence via `save(slot, &T)` / `load::<T>(slot)` / `delete(slot)` / `exists(slot)` / `list_slots()`. Uses `dirs::data_local_dir()` for platform-appropriate save paths, with `with_dir()` for custom locations. Games derive `Serialize + Deserialize` on save data structs and store `SaveSystem` in `Globals`. Re-exported as `rengine::SaveSystem` and `rengine::SaveError`.

17. Virtual file system or resource path abstraction [partially done]
    Make loading portable across desktop, web, and future mobile targets.

18. Error reporting for assets [partially done]
    Better messages and fallback behavior for missing or invalid textures, sounds, meshes, and data files.

---

## Tier 2: High-Value 2D and General Rendering Features

These features make 2D development substantially more practical.

19. Sprite atlas support
    Pack sprites together and reduce texture switching.

20. Atlas metadata import
    Import data from common atlas generators instead of manually assigning UV rectangles.

21. Render targets and offscreen textures
    Enable minimaps, lighting buffers, portals, masks, compositing, and post-processing pipelines.

22. Post-processing pipeline [done]
    Fullscreen effects like bloom, blur, vignette, color grading, CRT, pixelation, outlines, and distortion.
    Built-in effects with configurable parameters, plus custom user-defined WGSL shaders.

23. Custom 2D materials or shaders
    Allow per-sprite or per-batch custom shader usage without rewriting renderer internals.

24. Particle systems [done]
    Support 2D emitters, bursts, lifetimes, curves, velocity, size-over-time, and color-over-time.

25. Tweening system [done]
    Smoothly animate properties over time with easing functions.

26. Animation state machines [done]
    `AnimationStateMachine<State, Trigger>` now handles named states, explicit triggers, global interrupts, and automatic completion transitions on top of sprite-sheet `Animation` clips.
    `Animation` itself now supports `Loop`, `Once`, and `PingPong` playback modes.

27. Tilemap layers
    Foreground, background, collision-only, decorative, and parallax tile layers.

28. Tile metadata
    Per-tile flags, tags, collision properties, or gameplay data.

29. Animated tiles
    For water, lava, conveyors, signs, or environmental loops.

30. Autotiling
    Greatly improves workflow for terrain and map construction.

31. Tiled map import
    High leverage for normal 2D workflows.

32. Better text rendering [partially done]
    Multiple fonts, outlines, shadows, and bitmap font support.
    Bold, italic, and bold-italic variants via per-style font atlases.
    Fancy text effects: per-letter color shifting, bouncing/wave letters, and rendering text along curves.
    - Completed: measure_text, line_height, TextAlign (Left/Center/Right), text_aligned, word wrapping (wrap_text), text_block
    - Completed: immediate-mode widget system — Ui, UiStyle, UiResponse; label, label_centered, button, separator; focus navigation and activation
    - Completed: single-line text input widget with committed-text / IME plumbing, caret control, placeholder rendering, and `feature-text-input` coverage

33. Resolution scaling modes [done]
    Pixel-perfect, stretch, letterbox, integer scaling, and fit or fill policies.
    - Completed: ScaleMode enum (Stretch, Letterbox, PixelPerfect), EngineConfig render_width/render_height, offscreen render target + blit pass for 2D and 3D, runtime mode switching via set_scale_mode, Canvas/HUD always at window resolution

34. Screen-space debug rendering
    Collision bounds, path nodes, raycasts, contact normals, velocity vectors, and AI state overlays.

35. Debug performance stats
    Partially done: the new in-game debug overlay now reports FPS, frame time, frame count, log volume, filter state, hot reload status, and mode-specific info, while the deeper renderer counters (batch count, texture binds, draw calls, memory trends) still need dedicated instrumentation.

36. Hot reload for assets [done]
    Reload textures, shaders, and data files during development.

37. Nine-slice support [done]
    `NineSlice` struct with uniform/asymmetric borders, `frame.draw_nine_slice()`, color tinting, z-order, `feature-nineslice` sample.

38. Masking and clipping [done]
    Canvas `push_clip`/`pop_clip` for GPU scissor-rect clipping. Segment-based render pass splits draw calls at clip boundaries. Used by `ScrollRegion` UI widget.

39. Scroll regions [done]
    `Ui::scroll(id, height, scroll_offset, children)` creates a clipped, scrollable container. Mouse-wheel input via `InputState::scroll_delta()`. Updated offsets returned in `UiResponse::scroll_for(id)`. `feature-ui` sample demonstrates a scrollable button list.

---

## Tier 3: 3D Features Needed to Become Truly Useful

The 3D renderer exists, but these features are required before it becomes practical for general development.

39. Full 3D transforms [not started]
    Per-draw rotation and scale in addition to position. Lack of rotation caused sample door meshes to be invisible when oriented wrong for the scene.

40. Transform hierarchies
    Parent-child spatial relationships for weapons, bones, cameras, and grouped props.

41. Mesh import [done]
    OBJ and glTF loading are implemented with automatic face-winding correction against authored normals. Next step is richer imported material and texture data.

42. 3D texture loading
    Bring in real textured assets rather than color-only geometry.

43. UV support in 3D meshes
    Necessary for textured models and materials.

44. Material system for 3D
    Base color, normal maps, roughness, metallic, emissive, transparency, and material reuse.

45. Multiple light types
    Directional, point, and spot lights.

46. Shadow mapping
    Directional shadows first, then point or spot shadows if needed.

47. Skeletal animation
    Required for character models and most modern imported content.

48. Animation blending
    Idle-to-run, run-to-jump, upper-body overlays, and smooth transitions.

49. Frustum culling
    Avoid drawing geometry the camera cannot see.

50. Instanced rendering
    Useful for foliage, crowds, bullets, props, and repeated level geometry.

51. 3D collision queries
    Raycasts, overlap checks, shape casts, and filters.

52. 3D physics backend
    Bodies, collisions, character controllers, and static scene geometry.

53. Spatial audio
    Listener positioning, attenuation, panning, and distance falloff.

---

## Tier 4: Engine Architecture and Workflow Improvements

These are the features that shift the engine from framework-like to engine-like.

54. Object or entity model
    A coherent structure for game objects beyond manual per-game patterns.

55. Component-based composition
    Split behavior into reusable chunks like transform, sprite, collider, audio source, or script.

56. Event or signal system
    Decoupled communication between gameplay systems.

57. Script attachment model
    Some way to bind behavior to prefab or scene instances.

58. Inspector-friendly property metadata
    Prepare for future tools and editors by making engine objects introspectable.

59. Stable serialization format
    Save scenes, prefabs, resources, and references cleanly.

60. Resource dependency tracking
    Know what scenes depend on which assets and what needs rebuilding or reloading.

61. Build profiles
    Separate debug, release, headless, and web behavior.

62. Replay recording
    Especially valuable given the existing rollback direction.

63. Plugin or extension API
    Let engine features be added without modifying the core engine every time.

64. In-game developer console
    Useful for scene switching, debug commands, and gameplay iteration.

65. Structured logging
    Rendering, assets, audio, gameplay, networking, physics, and tools should be distinct domains.

66. Better platform abstraction
    Fullscreen, cursor modes, clipboard, drag and drop, and platform-specific behavior.

67. Async or background asset loading
    Important for loading screens and larger games.

68. Deterministic simulation helpers
    Helpful for netcode, replays, and testing.

---

## Tier 5: Tooling and Content Authoring

These dramatically improve iteration speed and team usability.

69. Scene editor
    Place objects, lights, triggers, UI roots, cameras, and spawn points visually.

70. Prefab editor
    Create reusable entities with override workflows.

71. Tilemap editor
    Internal editor or strong external importer support.

72. Particle editor
    Data-driven particle authoring with preview.

73. Animation editor
    Timelines, clip editing, and transitions.

74. Cutscene or timeline system
    Cameras, dialogue, events, animations, and sequencing.

75. Visual profiler
    Frame breakdowns and runtime bottleneck inspection.

76. Remote debugging tools
    Useful later for platform support and larger projects.

77. Asset validation tools
    Detect missing references, bad imports, wrong dimensions, or invalid data.

78. Build packaging and export pipeline
    Better release workflow for distributable builds.

---

## Tier 6: Advanced but Valuable Features

These are important once the core engine has matured.

79. Navigation and pathfinding
    Grid, waypoint, or navmesh systems depending on genre focus.

80. Localization system
    Translation tables, runtime switching, and font fallback support.

81. Accessibility features
    Rebinds, text scaling, high-contrast themes, reduced motion hooks, subtitle support.

82. Video playback
    Good for intros, menu backgrounds, and cutscenes.

83. Mod support
    Load external packaged content and user-defined assets or scripts.

84. Networked client-server model
    Separate from rollback, useful for other multiplayer genres.

85. Lobby and matchmaking helpers
    Session setup, reconnects, version checks, and peer flow.

86. Mobile-specific input and platform hooks
    Touch, vibration, lifecycle handling, safe areas.

87. Web target hardening
    Browser asset loading, persistence, input quirks, and performance tuning.

88. Console-facing abstraction layers
    Only if commercial shipping becomes a target.

89. XR support
    Only worth prioritizing if the engine intentionally targets that space.

---

## Recommended First 10 Milestones

If the goal is to make Rengine a more fully fledged general-purpose indie engine, these are the first ten milestones that likely deliver the most value:

1. Scene system and scene switching
2. Prefabs and serializable resources
3. Input action maps and rebinding
4. Better 2D transforms and camera features
5. Collision layers, masks, triggers, and stronger 2D physics
6. Tweening and animation state support
7. Tilemap pipeline improvements plus Tiled import
8. Render targets and post-processing [done]
9. Expanded 3D asset import beyond OBJ, starting with glTF
10. Sample and content migration onto the asset pipeline across the repo

---

## Suggested Strategy

A practical development order would be:

### Phase 1: Make the 2D path production-usable

- Asset loading [done, continue expanding]
- Scene management
- Prefabs
- Audio [done — playback, buses, fades, crossfades]
- Input maps
- Better transforms
- Stronger collision and physics
- Tilemap import
- Tweening
- UI basics

### Phase 2: Strengthen rendering and tooling

- Render targets
- Post-processing [done]
- Particles
- Debug overlays
- Hot reload
- Profiling
- Serialization and build improvements

### Phase 3: Make 3D serious

- Mesh import [OBJ done, expand to glTF]
- Textures and materials
- Full transforms
- Lighting and shadows
- 3D collision and physics
- Skeletal animation
- Culling and instancing

---

## Guiding Principle

The biggest gap right now is not raw rendering capability. The biggest gap is authoring workflow.

That means the highest-value work is:

- asset pipeline
- scene composition
- prefab reuse
- audio
- input abstraction
- collision filtering
- animation and tweening
- import tools

Those are the features that make an engine pleasant to build games in, rather than merely possible to build games in.

---

## Personal End-Goal: Loop Hero-Inspired Motorsport Simulator

The target game that drives engine development priorities. Everything below describes what the engine must eventually support so a human can sit down and build this game without fighting the tooling.

### Concept

Loop Hero meets the entire history of Formula 1 compressed into one race. Cars drive themselves around a track based on driver/car stats. The player makes management decisions between laps — hiring drivers, allocating R&D, playing cards, building facilities — to win both the Drivers' and Constructors' Championships. Each lap advances 3-4 years through motorsport history (1950s → present), with cars naturally evolving as eras progress.

### Core Loop

1. Create/choose a team (engine supplier, car parts, two drivers from a market)
2. Race starts in the 1950s
3. Cars race autonomously based on stats
4. Each lap = 3-4 years of progress through ~7-10 regulatory eras
5. Between laps: 3-5 decisions (cards, tech tree, staff, strategy)
6. Goal: win Drivers' Championship, then Constructors' Championship

### Key Mechanics

- **Tech Tree**: Permanent progression branches (aero vs engine vs chassis vs reliability). Long-term strategic bets — picking aero early peaks in the ground-effect era but struggles in the turbo era.
- **Cards**: Random tactical per-lap draws (3-5 drawn, play 1-2). Examples: Wind Tunnel Breakthrough (+15% aero for 2 laps), Engine Blow (opponent DNF), Rain Dance (wet lap), Miraculous Save (cancel crash), Poaching (steal rival engineer), Regulation Change (reset a tech branch), Budget Cap (spend limit for 3 laps).
- **Card Rarity**: Common (small stat buffs), Rare (significant advantages), Legendary (game-changers like "Regulation Loophole" — ignore one regulation change).
- **Regulation Changes**: Periodic bans that wipe overspecialized teams and create catch-up opportunities. Scripted per era or card-driven.
- **Drivers**: Two per team. Traits: consistent, aggressive, wet-weather specialist, tyre whisperer, team player vs maverick. Contract length/cost. Hidden "potential" stat — young rookies may bloom into champions or plateau after 2-3 eras.
- **Staff**: Race engineer affects stat-to-laptime translation. Team principal decisions: team orders, pit strategy, risk tolerance.
- **Qualifying**: Mini-decision phase before each era determines grid position (spend tokens for practice, risk crash for speed, or play safe).
- **Points System**: Evolves with era (1950s: 8-6-4-3-2 → modern: 25-18-15-etc).
- **Rival AI Archetypes**: Conservative (reliability), Aggressive (aero), Budget (poach staff). Gives personality to the field.
- **Heritage Bonus**: Staying with the same engine supplier across eras builds a relationship bonus.
- **Pit Crew**: Upgradeable facility — a 1.8-second stop creates drama.
- **Weather Cards**: Affect multiple laps ("Monsoon Season" = 3 wet laps, forces planning).
- **Scandal Events**: Cheating, illegal parts, political interference — narrative chaos.
- **End-of-Era Draft**: When regulations change, worse-performing teams pick first from the new era's tech tree.

### Facilities (3-5 upgrade levels each)

- Factory — manufacturing quality (chassis, engine parts)
- Wind Tunnel / CFD Suite — aero development speed
- Training Center — driver development rate
- Sponsorship Office — passive income

### Race Visualization

- Top-down or isometric track, cars as colored dots/sprites
- Commentary/event log for drama ("Lap 3 (1962): Your driver Stirling takes the lead!")
- Incidents based on reliability + RNG (crashes, mechanical failures, weather, safety cars)
- Abstracted pit stops (push hard / conserve / pit early)

### Win Conditions

- Easy: win Drivers' title
- Normal: win both titles
- Hard: win both with budget constraint
- Nightmare: start in 1970 (miss easy early eras)

### Engine Features Still Needed

Tracked against the build order. Crossed-off items are done.

1. ~~Canvas stores screen_size~~ ✓
2. ~~UI layout containers (Row, Grid)~~ ✓
3. ~~UI scroll region~~ ✓
4. ~~Nine-slice rendering~~ ✓
5. ~~Particles~~ ✓
6. ~~Post-processing (CRT/bloom/vignette for era filters)~~ ✓
7. ~~Resolution scaling~~ ✓
8. ~~Audio fades/crossfades~~ ✓
9. ~~Scene transitions~~ ✓
10. ~~Timer + EventQueue~~ ✓
11. ~~Canvas drawing primitives~~ ✓
12. ~~Colored text spans~~ ✓
13. ~~UI widgets (checkbox, slider, progress bar)~~ ✓
14. ~~Mouse hover/click on widgets~~ ✓
15. ~~UI single-build pattern — stop duplicating widget trees in update() and render()~~ ✓
16. ~~Screen-space sprites / UI image widget — card artwork, driver portraits, facility icons~~ ✓
17. ~~Tooltip widget — card descriptions, stat explanations~~ ✓
18. ~~Widget animation hooks — card flip, slide-in, highlight pulse~~ ✓
19. ~~Multiple font support — headers, body, commentary, HUD~~ ✓
20. ~~Text input widget — team naming~~ ✓
21. ~~Animation state machines — car sprite states~~ ✓
22. ~~Rebindable controls — player key remapping~~ ✓
23. ~~Widget styling variants — per-widget visual overrides for card rarities, warnings, and CTA emphasis~~ ✓

Items 1-23 are done, and the first two motorsport-specific follow-up items after that audit are now done too.

Current priority engine issues for this game:

1. Data-driven UI templates and repeated management panels — driver cards, sponsor offers, facility upgrades, and standings rows still want a cleaner repeated-layout story.
2. Lightweight game-state organization patterns — the engine still lacks an opinionated but simple way to structure teams, drivers, contracts, suppliers, and season state.
3. Form and workflow helpers — team creation, hiring, contracts, and setup screens still require hand-rolled validation and multi-step flow logic.
4. Deferred rollback follow-up — animation/timer/tween/RNG helpers remain the main systemic gap once gameplay justifies them.

Deferred rollback follow-up after those game-facing priorities:

- Add built-in rollback-safe support or snapshot helpers for animation state, timers/tweens, and deterministic RNG progression.
- Keep renderer, audio, and other presentation/runtime caches outside rollback while documenting the game-owned save/load boundary more explicitly.

Unless a more urgent engine bug appears, the next engine work for the motorsport game should stay focused on those remaining UI ergonomics/composition helpers before circling back to rollback.
