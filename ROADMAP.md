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
- Completed: input action mapping — `ActionMap` with named digital actions and analog axes, `Binding` enum for Key/MouseButton/GamepadButton, `AxisMapping` with positive/negative bindings and optional gamepad stick axis, convenience methods on `Engine` and `Engine3D`
- Completed: `feature-input` sample — demonstrates action binding setup, axis-driven movement, pressed/down/released queries with visual feedback
- Partial: broader asset pipeline coverage still needs validation tooling, dependency tracking, and additional import formats beyond OBJ and glTF
- Partial: 3D transforms still only support position-based translation; rotation and scale per draw are not yet supported (caused the recurring door visibility issue)

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

6. Serializable resources
   Data-driven definitions for entities, items, attacks, animation clips, tile sets, dialogue, and configuration.

7. Better 2D transforms [done]
   Add rotation, scale, and origin support.
8. Improved 2D camera system [done]
   Rotation, smooth follow with dead zones, screen shake (intensity + duration with decay), camera bounds clamping. Projection uses view matrix (translate + rotate) applied to centered orthographic.

9. Audio playback [mostly done]
   Music, sound effects, looping, pause or resume, bus routing, master and per-bus volume control, and headless silent mode are implemented. Still missing: fades, crossfades, and spatial audio.

10. Input action mapping [done]
    Named actions (`"jump"`, `"shoot"`) and axes (`"move_x"`, `"move_y"`) bound to keyboard keys, mouse buttons, and gamepad buttons/sticks. Queries via `engine.action_down()`, `action_pressed()`, `action_released()`, `axis()`. Per-player variants for multiplayer.

11. Rebindable controls
    Let players or games remap keyboard and gamepad actions.

12. Collision layers and masks
    Support filtering between world, player, enemy, trigger, projectile, and UI collision groups.

13. Trigger volumes and overlap sensors
    Needed for pickups, checkpoints, dialogue zones, scripted events, and hurtboxes.

14. Stronger 2D physics
    Expand beyond simple AABB overlap into rigid bodies, velocity, gravity, friction, restitution, and moving platforms.

15. Fixed update support
    Make simulation-friendly fixed stepping explicit and ergonomic.

16. Save and load support
    Profiles, settings, progress, keybindings, and serialized game state.

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

22. Post-processing pipeline
    Fullscreen effects like bloom, blur, vignette, color grading, CRT, pixelation, outlines, and distortion.

23. Custom 2D materials or shaders
    Allow per-sprite or per-batch custom shader usage without rewriting renderer internals.

24. Particle systems
    Support 2D emitters, bursts, lifetimes, curves, velocity, size-over-time, and color-over-time.

25. Tweening system
    Smoothly animate properties over time with easing functions.

26. Animation state machines
    Move beyond raw frame cycling into walk, idle, attack, hit, death, and transition logic.

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

32. Better text rendering
    Multiple fonts, wrapping, alignment, outlines, shadows, and bitmap font support.

33. Resolution scaling modes
    Pixel-perfect, stretch, letterbox, integer scaling, and fit or fill policies.

34. Screen-space debug rendering
    Collision bounds, path nodes, raycasts, contact normals, velocity vectors, and AI state overlays.

35. Debug performance stats
    Show frame time, batch count, texture binds, draw calls, and memory trends.

36. Hot reload for assets [done]
    Reload textures, shaders, and data files during development.

37. Nine-slice support
    Important for UI panels, windows, and scalable decorative frames.

38. Masking and clipping
    Essential for UI panels, scroll regions, and some gameplay effects.

---

## Tier 3: 3D Features Needed to Become Truly Useful

The 3D renderer exists, but these features are required before it becomes practical for general development.

39. Full 3D transforms [not started — needed]
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
8. Render targets and post-processing
9. Expanded 3D asset import beyond OBJ, starting with glTF
10. Sample and content migration onto the asset pipeline across the repo

---

## Suggested Strategy

A practical development order would be:

### Phase 1: Make the 2D path production-usable

- Asset loading [done, continue expanding]
- Scene management
- Prefabs
- Audio [basic playback done, expand controls and mixing]
- Input maps
- Better transforms
- Stronger collision and physics
- Tilemap import
- Tweening
- UI basics

### Phase 2: Strengthen rendering and tooling

- Render targets
- Post-processing
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
