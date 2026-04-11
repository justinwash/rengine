# Rengine Architecture: Deep Technical Reference

> This document is an **exhaustive, line-by-line technical deep-dive** into the Rengine game engine as it exists on the `master` branch. It covers every subsystem from boot to shutdown, every GPU pipeline, every data structure, and every interaction surface. A companion "kitchen-sink" game example at the end demonstrates how to exercise as many features as possible in a single coherent project.

---

## Table of Contents

- [Rengine Architecture: Deep Technical Reference](#rengine-architecture-deep-technical-reference)
  - [Table of Contents](#table-of-contents)
  - [1. Crate Layout](#1-crate-layout)
  - [2. Public API Surface (`lib.rs`)](#2-public-api-surface-librs)
  - [3. Entry Points and the Game Loop (`app.rs`)](#3-entry-points-and-the-game-loop-apprs)
    - [3.1 `EngineConfig`](#31-engineconfig)
    - [3.2 The `Engine` struct (2D)](#32-the-engine-struct-2d)
    - [3.3 `run::<G: Game>()` — the 2D trait-based entry point](#33-rung-game--the-2d-trait-based-entry-point)
    - [3.4 The RedrawRequested Frame Cycle](#34-the-redrawrequested-frame-cycle)
    - [3.5 `run_with_scenes()` — the scene-stack entry point](#35-run_with_scenes--the-scene-stack-entry-point)
    - [3.6 `Engine3D` and `run3d::<G: Game3D>()`](#36-engine3d-and-run3dg-game3d)
    - [3.7 Mouse Capture in 3D Mode](#37-mouse-capture-in-3d-mode)
  - [4. The 2D Renderer (`renderer/`)](#4-the-2d-renderer-renderer)
    - [4.1 GPU Initialization](#41-gpu-initialization)
    - [4.2 The Sprite Pipeline (`DrawParams`)](#42-the-sprite-pipeline-drawparams)
    - [4.3 Texture Management (`TextureId`)](#43-texture-management-textureid)
    - [4.4 `Frame` Submission and Batched Rendering](#44-frame-submission-and-batched-rendering)
    - [4.5 `Camera2D` and Projection](#45-camera2d-and-projection)
    - [4.6 The sprite.wgsl Shader](#46-the-spritewgsl-shader)
  - [5. The 3D Renderer (`renderer3d/`)](#5-the-3d-renderer-renderer3d)
    - [5.1 `Renderer3D` Initialization](#51-renderer3d-initialization)
    - [5.2 `Frame3D` and `DrawCmd3D`](#52-frame3d-and-drawcmd3d)
    - [5.3 `Viewmodel3D` Rendering](#53-viewmodel3d-rendering)
    - [5.4 The mesh3d.wgsl Shader](#54-the-mesh3dwgsl-shader)
    - [5.5 Mesh Primitives (`cube_mesh`, `floor_quad`, `wall_quad`)](#55-mesh-primitives-cube_mesh-floor_quad-wall_quad)
    - [`Camera3D`](#camera3d)
  - [6. Canvas and Text Overlay (`canvas/`, `text.rs`)](#6-canvas-and-text-overlay-canvas-textrs)
    - [6.1 `FontAtlas` Construction](#61-fontatlas-construction)
    - [6.2 `Canvas` Drawing](#62-canvas-drawing)
    - [6.3 The canvas.wgsl Shader](#63-the-canvaswgsl-shader)
    - [6.4 The FPS Counter](#64-the-fps-counter)
  - [7. Input System (`input/`)](#7-input-system-input)
    - [7.1 `InputState` — Keyboard State](#71-inputstate--keyboard-state)
    - [7.2 Mouse State](#72-mouse-state)
    - [7.3 `GamepadSystem` and `GamepadState`](#73-gamepadsystem-and-gamepadstate)
    - [7.4 `ActionMap` — Input Action Mapping](#74-actionmap--input-action-mapping)
  - [8. Asset Pipeline (`assets/`)](#8-asset-pipeline-assets)
    - [8.1 `AssetPipeline` (Internal)](#81-assetpipeline-internal)
    - [8.2 `AssetManifest` and `AssetPack`](#82-assetmanifest-and-assetpack)
    - [8.3 Texture Loading](#83-texture-loading)
    - [8.4 `SpriteSheet` and `Animation`](#84-spritesheet-and-animation)
    - [8.5 3D Mesh Loading (OBJ and glTF)](#85-3d-mesh-loading-obj-and-gltf)
    - [8.6 Audio Loading](#86-audio-loading)
    - [8.7 Hot Reload](#87-hot-reload)
    - [8.8 `AssetError`](#88-asseterror)
  - [9. Audio System (`assets/audio.rs`)](#9-audio-system-assetsaudiors)
    - [9.1 `AudioBus` and Volume](#91-audiobus-and-volume)
    - [9.2 Music Playback](#92-music-playback)
    - [9.3 Headless Mode](#93-headless-mode)
  - [10. Color and Pixel Art](#10-color-and-pixel-art)
    - [`Color`](#color)
    - [`PixelCanvas` (Procedural Texture Generation)](#pixelcanvas-procedural-texture-generation)
  - [11. Scene System (`scene/`)](#11-scene-system-scene)
    - [11.1 `Scene` Trait and `SceneOp`](#111-scene-trait-and-sceneop)
    - [11.2 `Globals` — Typed Key-Value Store](#112-globals--typed-key-value-store)
    - [11.3 Scene Stack Dispatch](#113-scene-stack-dispatch)
    - [11.4 2D Scene Data (`Scene2D`, `SceneInstance2D`, Prefabs, Instances)](#114-2d-scene-data-scene2d-sceneinstance2d-prefabs-instances)
  - [12. World Systems (`world/`)](#12-world-systems-world)
    - [12.1 `TileMap` and `TileDef`](#121-tilemap-and-tiledef)
    - [12.2 `aabb_overlap` — AABB Physics](#122-aabb_overlap--aabb-physics)
    - [12.3 `iso_to_screen` / `screen_to_iso` — Isometric Helpers](#123-iso_to_screen--screen_to_iso--isometric-helpers)
  - [13. Math Utilities (`math/`)](#13-math-utilities-math)
    - [13.1 `Rect`](#131-rect)
    - [13.2 `TimeState`](#132-timestate)
  - [14. Rollback Netcode (`netcode/`, feature-gated)](#14-rollback-netcode-netcode-feature-gated)
    - [14.1 Architecture Overview (`Rollbackable`)](#141-architecture-overview-rollbackable)
    - [14.2 `RollbackSession`](#142-rollbacksession)
    - [14.3 `UdpNonBlockingSocket` — UDP Transport](#143-udpnonblockingsocket--udp-transport)
  - [15. Complete Frame Lifecycle: Boot to Pixel](#15-complete-frame-lifecycle-boot-to-pixel)
  - [16. Kitchen-Sink Game Example](#16-kitchen-sink-game-example)
    - [Features Exercised in This Example](#features-exercised-in-this-example)
    - [What This Example Does NOT Cover (and How You Would)](#what-this-example-does-not-cover-and-how-you-would)

---

## 1. Crate Layout

```
rengine/
├── Cargo.toml            # workspace root — lists engine + all samples
├── engine/
│   ├── Cargo.toml        # "rengine" library crate
│   ├── assets/           # embedded font.ttf
│   └── src/
│       ├── lib.rs         # public re-exports
│       ├── app.rs         # Engine, Engine3D, Game, Game3D, run(), run3d(), scene runners
│       ├── text.rs        # FontAtlas — glyph rasterization + GPU atlas
│       ├── canvas/        # Canvas overlay: mod.rs + canvas.wgsl
│       ├── input/         # keyboard.rs, gamepad.rs, action.rs, mod.rs
│       ├── math/          # Rect, TimeState
│       ├── renderer/      # 2D sprite renderer: camera, sprite, texture, mod.rs, sprite.wgsl
│       ├── renderer3d/    # 3D mesh renderer: camera, mesh, mod.rs, mesh3d.wgsl
│       ├── scene/         # Scene trait, Globals, 2D scene data (prefabs/instances)
│       ├── world/         # TileMap, AABB physics, isometric helpers
│       ├── assets/        # AssetPipeline, Color, audio, pixelart, spritesheet
│       └── netcode/       # (feature "rollback") GGRS integration + UDP transport
└── samples/
    ├── features/          # feature-scenes, feature-sprites, feature-camera
    └── games/             # game-platformer, game-topdown, game-iso, game-fps, game-fight, game-fps-mp
```

The `Cargo.toml` workspace root declares all members:

```toml
[workspace]
members = ["engine", "samples/games/game-platformer", "samples/games/game-topdown",
           "samples/games/game-iso", "samples/games/game-fps", "samples/games/game-fight",
           "samples/games/game-fps-mp", "samples/features/feature-scenes",
           "samples/features/feature-sprites"]
resolver = "2"
```

The engine crate itself has one optional feature:

```toml
[features]
default = []
rollback = ["dep:ggrs", "dep:bincode"]
```

When `rollback` is enabled, the `netcode` module is compiled in, exposing [`RollbackSession`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L86), [`Rollbackable`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L73), [`OnlineConfig`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L39), [`SessionMode`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L45), and the [`fletcher64`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L290) checksum function.

**Key dependencies:**
| Dependency | Purpose |
|---|---|
| `winit 0.29` | Window creation + event loop |
| `wgpu 28` | GPU abstraction (Vulkan/Metal/DX12/WebGPU) |
| `glam 0.32` | Math (Vec2, Vec3, Mat4) |
| `bytemuck 1` | Zero-copy GPU buffer casting |
| `pollster 0.4` | Block on async (used to init wgpu synchronously) |
| `image 0.25` | PNG/JPEG/etc decoding |
| `gilrs 0.11` | Gamepad input |
| `rodio 0.17` | Audio playback |
| `fontdue 0.9` | CPU-side font rasterization |
| `tobj 4` | OBJ mesh loading |
| `gltf 1` | glTF mesh loading |
| `serde + serde_json` | Asset manifests, scene definitions |
| `ggrs 0.11` (optional) | Rollback netcode |
| `bincode 1` (optional) | Serialization for rollback transport |

---

## 2. Public API Surface ([`lib.rs`](https://github.com/justinwash/rengine/blob/master/engine/src/lib.rs))

`lib.rs` is purely re-exports. It defines zero logic — its entire job is to flatten the internal module tree into a single `rengine::*` namespace:

```rust
pub mod app;
pub mod math;
pub mod input;
pub mod canvas;
pub mod renderer;
pub mod renderer3d;
pub mod scene;
pub mod text;
pub mod world;
pub mod assets;

#[cfg(feature = "rollback")]
pub mod netcode;
```

Then selective re-exports:

- **Core runtime:** [`run`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L291), [`run_with_scenes`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L409), [`Engine`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L47), [`EngineConfig`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L22), [`Game`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L279)
- **Rendering (2D):** [`Camera2D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/camera.rs#L4), [`DrawParams`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/sprite.rs#L6), [`Frame`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/mod.rs#L21), [`TextureId`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/texture.rs#L2)
- **Rendering (3D):** [`Camera3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L4), [`DrawCmd3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L29), [`Frame3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L57), [`MeshId`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L5), [`Vertex3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L10), [`Viewmodel3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L35), [`cube_mesh`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L54), [`floor_quad`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L107), [`wall_quad`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L123)
- **Input:** [`InputState`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L6), `KeyCode` (from winit), `GamepadButton` (from gilrs), [`GamepadState`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L9), [`GamepadSystem`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L58), [`ActionMap`](https://github.com/justinwash/rengine/blob/master/engine/src/input/action.rs), [`Binding`](https://github.com/justinwash/rengine/blob/master/engine/src/input/action.rs), [`AxisMapping`](https://github.com/justinwash/rengine/blob/master/engine/src/input/action.rs), [`GamepadAxis`](https://github.com/justinwash/rengine/blob/master/engine/src/input/action.rs)
- **Assets:** [`Color`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L2), [`Animation`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L56), [`AssetError`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L15), [`AssetManifest`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L158), [`AssetPack`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L174), [`AudioBus`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L14), [`AudioClip`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L25), [`AudioId`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L22), [`MeshAsset`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L137), [`SpriteSheet`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L5), [`SpriteSheetAssetDef`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L151), [`TextureAsset`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L119)
- **Scene:** [`Globals`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L4), [`Prefab2D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L61)/[`Prefab2DDef`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L26), [`PrefabSprite2D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L50)/[`PrefabSprite2DDef`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L11), [`Scene`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L24), [`Scene2D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L98)/[`Scene2DDef`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L42), [`SceneInstance2D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L67)/[`SceneInstance2DDef`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L32), [`SceneOp`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L16), [`Scene3D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L47), [`SceneOp3D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L39)
- **World:** [`tilemap`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs), [`aabb_overlap`](https://github.com/justinwash/rengine/blob/master/engine/src/world/physics.rs), [`aabb_overlap_layered`](https://github.com/justinwash/rengine/blob/master/engine/src/world/physics.rs), [`CollisionLayer`](https://github.com/justinwash/rengine/blob/master/engine/src/world/physics.rs), [`iso_to_screen`](https://github.com/justinwash/rengine/blob/master/engine/src/world/iso.rs#L4), [`screen_to_iso`](https://github.com/justinwash/rengine/blob/master/engine/src/world/iso.rs#L11), [`TileDef`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L16), [`TileMap`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L6)
- **Canvas/Text:** [`screen_to_ndc`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L145), [`Canvas`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L42), [`CanvasVertex`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L6), [`FontAtlas`](https://github.com/justinwash/rengine/blob/master/engine/src/text.rs#L17)
- **Pixel art:** [`pixelart`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs) (module-level re-export of [`PixelCanvas`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L3), [`darken`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L106), [`lighten`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L110))
- **Math:** [`Rect`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L5), [`TimeState`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L4), `Vec2`, `Vec3` (from glam)

The guiding design philosophy: **a game crate writes `use rengine::*;` and gets everything it needs.**

---

## 3. Entry Points and the Game Loop ([`app.rs`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs))

### 3.1 [`EngineConfig`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L22)

```rust
pub struct EngineConfig {
    pub title: String,      // Window title
    pub width: u32,         // Initial window width in logical pixels
    pub height: u32,        // Initial window height in logical pixels
    pub vsync: bool,        // false → AutoNoVsync; true → AutoVsync
    pub headless: bool,     // Skip window creation visibility + mute audio
    pub hot_reload: bool,   // File-watching for assets at runtime
    pub show_fps: bool,     // Render FPS counter overlay on canvas
    pub fixed_dt: f32,      // Fixed-timestep interval (default 1/60)
}
```

Default: 800×600, no vsync, not headless, hot reload on, FPS shown, fixed_dt 1/60.

The `headless` flag is critical for testing:

- The engine still creates a wgpu surface and device (needed for texture/buffer creation).
- The window is invisible (`with_visible(false)`).
- Audio is muted (master volume set to 0).
- The `run` function uses a tight `loop {}` instead of the platform event loop.

### 3.2 The [`Engine`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L47) struct (2D)

```rust
pub struct Engine {
    pub(crate) renderer: Renderer,        // Owns wgpu surface, device, queue, pipelines, textures
    pub(crate) assets: AssetPipeline,     // File-backed asset loading with caching
    pub(crate) audio: AudioSystem,        // rodio-backed audio playback
    pub(crate) input: InputState,         // Keyboard + mouse state
    pub(crate) time: TimeState,           // Delta-time, total time, frame count
    pub(crate) window_width: u32,
    pub(crate) window_height: u32,
    pub(crate) gamepads: GamepadSystem,   // gilrs-backed gamepad state
    pub(crate) hot_reload_enabled: bool,
}
```

All fields are `pub(crate)` — the game only interacts through accessor methods:

- [`engine.input()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L60) → `&InputState`
- [`engine.time()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L63) / [`engine.dt()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L67) → `&TimeState` / `f32`
- [`engine.window_size()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L70) → `(u32, u32)`
- [`engine.gamepad(player)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L74) → `&GamepadState`
- [`engine.gamepads_connected()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L78) → `usize`
- [`engine.asset_root()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L82) / [`engine.set_asset_root()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L86)
- [`engine.create_texture(w, h, &rgba)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L98) → `TextureId`
- [`engine.create_color_texture(w, h, color)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L256) → `TextureId`
- [`engine.white_texture()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L270) → `TextureId` (1×1 white pixel)
- [`engine.font_atlas()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L274) → `&FontAtlas`
- [`engine.load_texture(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L145) → `Result<TextureAsset, AssetError>`
- [`engine.load_sprite_sheet(path, cell_w, cell_h)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L151) → `Result<SpriteSheet, AssetError>`
- [`engine.load_audio(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L163) → `Result<AudioClip, AssetError>`
- [`engine.load_asset_manifest(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L110) → `Result<AssetPack, AssetError>`
- [`engine.load_bytes(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L102) / [`engine.load_text(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L106)
- [`engine.load_resource::<T>(path)`] → `Result<T, AssetError>` — Load a JSON file and deserialize into any `Deserialize + DeserializeOwned` type.
- [`engine.load_resource_list::<T>(path)`] → `Result<Vec<T>, AssetError>` — Load a JSON array and deserialize into a `Vec<T>`.
- [`engine.load_scene2d(assets, path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L218) → `Result<Scene2D, AssetError>`
- Audio controls: [`play_sound`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L169), [`play_sound_on_bus`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L173), [`play_music`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L182), [`play_music_with_volume`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L186), [`stop_music`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L190), [`pause_music`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L194), [`resume_music`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L198), [`stop_audio_bus`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L202), [`set_master_volume`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L206), [`set_audio_bus_volume`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L210), [`audio_bus_volume`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L214)
- [`engine.reload_assets_if_changed()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L227) — called every frame automatically
- [`engine.hot_reload_enabled()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L90) / [`engine.set_hot_reload_enabled()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L94)

### 3.3 [`run::<G: Game>()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L291) — the 2D trait-based entry point

This is the simplest way to run a 2D game. The type parameter `G` must implement the [`Game`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L279) trait:

```rust
pub trait Game: 'static + Sized {
    fn new(engine: &mut Engine) -> Self;     // Constructor — load assets, init state
    fn update(&mut self, engine: &Engine);   // Logic tick — receives immutable engine
    fn fixed_update(&mut self, _engine: &Engine) {} // Fixed-timestep tick (default empty)
    fn render(&mut self, engine: &Engine, frame: &mut Frame);  // Populate frame for rendering
    fn should_exit(&self) -> bool { false }  // Return true to exit the game loop
}
```

`fixed_update()` is called N times per frame (where N depends on the accumulated time vs `EngineConfig::fixed_dt`) **before** the variable-rate `update()`. The same pattern exists on `Game3D`, `Scene`, and `Scene3D`.

```rust
```

**Line-by-line boot sequence in `run()`:**

1. **`env_logger::init()`** — Initializes the `log` + `env_logger` crate so `log::info!()` etc. print to stderr. The `RUST_LOG` environment variable controls verbosity.

2. **`EventLoop::new()?`** — Creates a winit event loop. This is the OS message pump.

3. **`WindowBuilder::new()...build(&event_loop)?`** — Creates the OS window. `.with_visible(!headless)` hides it for testing. The window is wrapped in `Arc<Window>` because wgpu needs to share ownership.

4. **`PresentMode`** — Selected based on `config.vsync`. `AutoVsync` synchronizes with the display refresh; `AutoNoVsync` runs as fast as possible.

5. **`Renderer::new(window.clone(), present_mode)`** — This is the heavy GPU initialization (see §4.1). It is called via `pollster::block_on()` because wgpu's adapter/device request is async.

6. **Engine construction** — All subsystems are assembled:
   - `AssetPipeline::default()` — roots at the current working directory
   - `AudioSystem::new(config.headless)` — opens rodio output stream (or silences on headless)
   - `InputState::new()` — empty HashSets
   - `TimeState::new()` — starts the clock
   - `GamepadSystem::new()` — initializes gilrs + scans connected gamepads

7. **`G::new(&mut engine)`** — The game's constructor runs. The game gets `&mut Engine` so it can load textures, create meshes, load manifests, etc.

8. **Headless branch** — If `headless`, the engine enters a tight `loop { ... }` instead of the event loop. Each iteration: tick time → update gamepads → hot reload → `game.update()` → check `should_exit()` → `input.end_frame()`.

9. **Event loop** — `event_loop.run(move |event, target| { ... })` enters the platform event loop. Control flow is set to `Poll` (no sleeping — continuously redraws).

### 3.4 The RedrawRequested Frame Cycle

On every `WindowEvent::RedrawRequested`:

```
┌─ engine.time.tick()              // Measure delta-time, accumulate for fixed step
├─ engine.gamepads.update()        // Poll gilrs for gamepad events
├─ engine.reload_assets_if_changed() // Hot-reload textures, manifests, audio
├─ while engine.time.consume_fixed_step():
│   └─ game.fixed_update(&engine)  // Fixed-timestep logic (physics, netcode)
├─ game.update(&engine)            // Variable-rate logic (reads input, modifies game state)
├─ if game.should_exit() → exit
├─ frame.begin()                   // Clear sprites + canvases; camera state persists
├─ game.render(&engine, &mut frame)// Game populates frame with DrawParams + canvases
├─ [if show_fps] draw FPS canvas overlay
├─ engine.renderer.render_frame(&frame) // Submit to GPU
└─ engine.input.end_frame()        // Clear per-frame flags (pressed, released, mouse delta)
```

`Frame` is created once before the event loop so that `Camera2D` state (position, shake, rotation) persists across frames. `frame.begin()` clears only transient per-frame data (sprites, canvases).

Other event handlers:

- **`WindowEvent::Resized`** — Updates engine width/height, calls `renderer.resize()` to reconfigure the surface.
- **`WindowEvent::KeyboardInput`** — Extracts `PhysicalKey::Code(key)` + `state` (pressed/released), passes to `input.handle_key_event()`.
- **`WindowEvent::CloseRequested`** — Calls `target.exit()`.

### 3.5 [`run_with_scenes()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L409) — the scene-stack entry point

```rust
pub fn run_with_scenes<F>(config: EngineConfig, init: F)
where F: FnOnce(&mut Engine, &mut Globals) -> Box<dyn Scene>
```

This is the scene-aware alternative. Instead of a [`Game`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L279) trait, you provide a closure that returns the initial [`Scene`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L24). Key differences from `run()`:

1. A `Globals` is created (`Globals::new()`) — a typed key-value store shared across all scenes.
2. A scene `stack: Vec<Box<dyn Scene>>` is maintained.
3. The `init` closure receives `&mut Engine` and `&mut Globals` and returns `Box<dyn Scene>`.
4. The initial scene's `on_enter()` is called, then it's pushed onto the stack.
5. **Per-frame:** The top scene's `update()` is called, returning a `SceneOp`. The `apply_scene_op()` function processes it (see §11.3).
6. **Rendering:** All scenes in the stack are rendered bottom-to-top: `for scene in stack.iter() { scene.render(...) }`. This allows transparent overlays (e.g. a pause screen rendering on top of the game scene).

### 3.6 [`Engine3D`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L591) and [`run3d::<G: Game3D>()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L778)

`Engine3D` mirrors `Engine` but wraps a `Renderer3D` instead of `Renderer`, and adds `mouse_captured: bool`. It provides the same asset/audio/input API plus 3D-specific methods:

- [`engine.load_obj_mesh(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L669) / [`engine.load_gltf_mesh(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L675) / [`engine.load_mesh(path)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L681) → `Result<MeshAsset, AssetError>`
- [`engine.create_mesh(vertices, indices)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L764) → `MeshId`

[`Game3D`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L769) trait:

```rust
pub trait Game3D: 'static + Sized {
    fn new(engine: &mut Engine3D) -> Self;
    fn update(&mut self, engine: &Engine3D);
    fn render(&mut self, engine: &Engine3D, frame: &mut Frame3D);
    fn should_exit(&self) -> bool { false }
}
```

### 3.7 Mouse Capture in 3D Mode

In `run3d()`, the engine immediately grabs the mouse:

```rust
window.set_cursor_grab(CursorGrabMode::Confined)
    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
window.set_cursor_visible(false);
engine.mouse_captured = true;
```

The `DeviceEvent::MouseMotion` handler accumulates deltas into `input.mouse_delta` **only when** `engine.mouse_captured` is true. Pressing `Escape` releases the mouse; clicking re-captures it. When the window loses focus, the mouse is released; when it regains focus, the mouse is re-captured.

---

## 4. The 2D Renderer ([`renderer/`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/))

### 4.1 GPU Initialization

`Renderer::new()` is an async function (called via `pollster::block_on()`):

1. **`wgpu::Instance::new()`** — Creates a wgpu instance with all backends (Vulkan, Metal, DX12, WebGPU).

2. **`instance.create_surface(window)`** — Creates a surface from the winit window.

3. **`instance.request_adapter()`** — Requests a GPU adapter compatible with the surface. `PowerPreference::default()` lets the system choose.

4. **`adapter.request_device()`** — Requests a logical device and command queue. No special features or limits are required.

5. **Surface configuration:**

   ```rust
   let surface_format = caps.formats.iter().find(|f| f.is_srgb()).copied()
       .unwrap_or(caps.formats[0]);
   ```

   Prefers sRGB format for correct gamma. Configured with the chosen present mode.

6. **Sprite shader** — `include_str!("sprite.wgsl")` compiles the WGSL sprite shader at Rust compile time.

7. **Bind group layouts:**
   - Group 0: `projection` — a single `mat4x4<f32>` uniform buffer (vertex stage).
   - Group 1: `texture` — a 2D float texture + filtering sampler (fragment stage).

8. **Render pipeline** — Triangle list, CCW front face, no culling, no depth test, alpha blending (`ALPHA_BLENDING`), no multisampling.

9. **Vertex buffer** — Pre-allocated for `MAX_SPRITES × 4 = 40,000` vertices.

10. **Index buffer** — Pre-computed quad indices: for each sprite quad i, indices are `[4i, 4i+1, 4i+2, 4i+2, 4i+3, 4i]`.

11. **Projection buffer** — 64 bytes (one `mat4x4<f32>`).

12. **Sampler** — Nearest-neighbor filtering (pixel art friendly), clamp-to-edge addressing.

13. **Canvas pipeline** — Separate pipeline for the Canvas overlay system (see §6).

14. **Font atlas** — Built from the embedded `font.ttf` (see §6.1).

15. **White texture** — A 1×1 white pixel texture created as `create_texture(1, 1, &[255, 255, 255, 255])`. Its `TextureId` is stored as `renderer.white_texture` and used when games want to draw solid-colored rectangles without loading a texture.

### 4.2 The Sprite Pipeline ([`DrawParams`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/sprite.rs#L6))

The sprite pipeline is a standard 2D batcher:

**Vertex layout (`Vertex`):**

```rust
struct Vertex {
    position: [f32; 2],     // World-space position
    tex_coords: [f32; 2],   // UV coordinates
    color: [f32; 4],        // RGBA tint
}
```

Stride: 32 bytes. Attributes at shader locations 0, 1, 2.

### 4.3 Texture Management ([`TextureId`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/texture.rs#L2))

Textures are stored in a `Vec<GpuTexture>`:

```rust
struct GpuTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}
```

`TextureId(usize)` is an index into this vector. [`create_texture()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/mod.rs#L307):

- Asserts `pixels.len() == width × height × 4`
- Creates an `Rgba8UnormSrgb` texture
- Writes pixels via `queue.write_texture()`
- Creates a view and bind group (texture + sampler)
- Pushes to `self.textures` and returns `TextureId(len - 1)`

[`replace_texture()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/mod.rs#L373) follows the same process but writes to an existing slot, enabling hot reload.

### 4.4 [`Frame`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/mod.rs#L21) Submission and Batched Rendering

[`render_frame(&frame)`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/mod.rs#L444) performs the actual GPU work:

1. **Surface acquire** — `self.surface.get_current_texture()`. On `Lost` or `Outdated`, reconfigures and returns early.

2. **Projection upload** — Computes `frame.camera.projection(width, height)` and writes the 4×4 matrix to the projection uniform buffer.

3. **Sort sprites** — `frame.sprites` is sorted by `(z_order, texture_id)`. This ensures correct draw order and minimizes texture bind switches.

4. **Vertex generation** — For each sorted sprite, four vertices are generated:

   ```
   [bottom-left, bottom-right, top-right, top-left]
   ```

   with positions computed from `position`, `size`, `origin`, `rotation`. UV coordinates respect `flip_x`/`flip_y` by swapping U or V ranges.

   **Rotation math:** If `rotation != 0.0`, each corner is rotated around the sprite's position:

   ```rust
   let dx = cx - px;
   let dy = cy - py;
   [px + dx * cos - dy * sin, py + dx * sin + dy * cos]
   ```

5. **Vertex upload** — All vertices are written to the GPU vertex buffer in one `write_buffer` call.

6. **Batching** — Consecutive sprites sharing the same `texture_id` are grouped into batches. Each batch is a `(texture_index, sprite_count)`.

7. **Render pass** — A single render pass with:
   - Clear color from `frame.clear_color`
   - The sprite pipeline bound
   - Projection bind group at group 0
   - For each batch: texture bind group at group 1, `draw_indexed(start..end)`

8. **Canvas pass** — After the sprite pass, `canvas::render_pass()` is called to draw the 2D canvas overlay (text, rectangles) on top of the scene.

9. **Submit** — `queue.submit(encoder.finish())` + `output.present()`.

### 4.5 [`Camera2D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/camera.rs#L3) and Projection

```rust
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
    pub bounds: Option<CameraBounds>,
    // private: follow_target, follow_speed, dead_zone,
    //          shake_intensity, shake_duration, shake_elapsed, shake_offset, shake_seed
}

pub struct CameraBounds {
    pub min: Vec2,
    pub max: Vec2,
}
```

**Smooth follow** — call `cam.follow(target, speed)` each frame. The camera lerps toward the target at the given speed, respecting a configurable dead zone set via `cam.set_dead_zone(half_size)`. Movement inside the dead zone does not move the camera.

**Screen shake** — `cam.shake(intensity, duration)` starts a decaying random offset using a deterministic hash. The offset fades linearly to zero over the duration.

**Bounds clamping** — when `bounds` is `Some`, the camera position is clamped after following.

**`cam.update(dt)`** must be called each frame (typically at the start of `render`) to advance follow interpolation, bounds clamping, and shake.

The [`projection`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/camera.rs#L92) builds an ortho × view matrix:

```rust
fn projection(&self, viewport_width: f32, viewport_height: f32) -> Mat4 {
    let half_w = viewport_width / 2.0 / self.zoom;
    let half_h = viewport_height / 2.0 / self.zoom;
    let pos = self.position + self.shake_offset;
    let ortho = Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, -1.0, 1.0);
    let view = Mat4::from_rotation_z(-self.rotation)
        * Mat4::from_translation(Vec3::new(-pos.x, -pos.y, 0.0));
    ortho * view
}
```

At zoom 1.0, one world unit equals one screen pixel. The camera is centered on `position`. Increasing zoom narrows the view. Rotation is in radians (counter-clockwise). The shader receives the combined matrix unchanged — no shader modifications were needed.

### 4.6 The sprite.wgsl Shader

```wgsl
@group(0) @binding(0) var<uniform> projection: mat4x4<f32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    out.clip_position = projection * vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
}

@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return tex_color * in.color;
}
```

The vertex shader transforms 2D world positions by the orthographic projection. The fragment shader samples the texture and multiplies by the vertex color tint. This enables:

- **Textured sprites** — When using a real texture with white tint
- **Color tinting** — When using a white texture with a colored tint
- **Semi-transparent overlays** — By setting `color.a < 1.0`

---

## 5. The 3D Renderer ([`renderer3d/`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/))

### 5.1 [`Renderer3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L119) Initialization

Similar to the 2D renderer but with key differences:

- **Power preference:** `HighPerformance` (requests discrete GPU).
- **Depth buffer:** A `Depth32Float` texture is created and recreated on resize.
- **Uniform buffer:** Contains a `Uniforms` struct:
  ```rust
  struct Uniforms {
      view_proj: [[f32; 4]; 4],    // Combined view-projection matrix
      light_dir: [f32; 4],         // Directional light direction (w=0)
      light_color: [f32; 4],       // Light color (a = intensity)
      ambient: [f32; 4],           // Ambient color (a = intensity)
  }
  ```
- **Cull mode:** `Some(wgpu::Face::Back)` — back-face culling is enabled.
- **Blend mode:** `REPLACE` (opaque rendering).
- **Vertex/index buffers:** Pre-allocated for 200,000 vertices and 400,000 indices.

### 5.2 [`Frame3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L57) and [`DrawCmd3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L29)

```rust
pub struct Frame3D {
    pub camera: Camera3D,
    pub viewmodel: Viewmodel3D,
    pub clear_color: Color,
    pub light_dir: Vec3,
    pub light_color: Color,
    pub light_intensity: f32,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    draws: Vec<DrawCmd3D>,       // World-space meshes
    raw_verts: Vec<Vertex3D>,    // Inline vertex data
    raw_idxs: Vec<u32>,          // Inline index data
    canvases: Vec<Canvas>,       // 2D overlay
}
```

`DrawCmd3D` is a position + mesh reference:

```rust
pub struct DrawCmd3D {
    pub mesh: MeshId,
    pub position: Vec3,
}
```

The frame is populated via:

- [`frame.draw_mesh(mesh_id, position)`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L92) — World-space mesh
- [`frame.draw_viewmodel_mesh(mesh_id, position)`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L96) — Camera-relative viewmodel mesh
- [`frame.draw_raw(vertices, indices)`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L100) — Inline geometry (no MeshId needed)

**Rendering flow:**

1. Compute view-projection from `frame.camera`.
2. Upload uniforms (VP matrix + lighting).
3. **Build geometry:** `build_draw_geometry()` iterates all `DrawCmd3D`s, copies each mesh's vertices with position offset applied CPU-side, and concatenates all indices with base offsets. Raw vertices/indices are appended after.
4. Upload concatenated vertices + indices to GPU buffers.
5. Render pass with clear + depth attachment.
6. If viewmodel draws exist: a second render pass with the viewmodel camera's VP matrix and depth cleared to 1.0 (viewmodel always renders on top).
7. Canvas overlay pass.

### 5.3 [`Viewmodel3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L35) Rendering

The `Viewmodel3D` has its own `Camera3D` with tight near/far planes (0.01–16.0) and narrow FOV (50°). This prevents viewmodel geometry from clipping into walls.

`build_viewmodel_geometry()` transforms each mesh vertex from camera-local space to world space using the inverse of the viewmodel camera's view matrix:

```rust
let camera_from_view = camera.view_matrix().inverse();
let world_position = camera_from_view.transform_point3(local_position);
let world_normal = camera_from_view.transform_vector3(normal).normalize_or_zero();
```

The viewmodel pass clears depth but **loads** the existing color (preserving the world render), then renders the viewmodel geometry on top.

### 5.4 The mesh3d.wgsl Shader

```wgsl
struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
};

@vertex fn vs_main(in: VertexInput) -> VertexOutput {
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_normal = in.normal;
    out.color = in.color;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let ndotl = max(dot(n, u.light_dir.xyz), 0.0);
    let diffuse = u.light_color.rgb * u.light_color.a * ndotl;
    let ambient = u.ambient.rgb * u.ambient.a;
    let lit = in.color.rgb * (diffuse + ambient);
    return vec4<f32>(lit, in.color.a);
}
```

This is a simple **Lambertian diffuse** lighting model:

- `N · L` gives the cosine of the angle between the surface normal and light direction.
- The diffuse term scales the light color by this factor and the light intensity.
- Ambient is added unconditionally.
- The final color is the vertex color multiplied by (diffuse + ambient).

### 5.5 Mesh Primitives ([`cube_mesh`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L54), [`floor_quad`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L107), [`wall_quad`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L123))

Three procedural mesh generators are provided:

- **`cube_mesh(sx, sy, sz, color)`** — Generates a 24-vertex, 36-index axis-aligned box with outward-facing normals. Each face has 4 unique vertices (no shared normals at edges).

- **`floor_quad(width, depth, y, color)`** — A horizontal quad at height `y`, normal pointing up (+Y).

- **`wall_quad(p0, p1, height, color)`** — A vertical quad between two floor-level points, extruded upward by `height`. Normal is computed as the 2D perpendicular of the base edge.

### [`Camera3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L4)

```rust
pub struct Camera3D {
    pub position: Vec3,
    pub yaw: f32,       // Rotation around Y axis (radians)
    pub pitch: f32,     // Rotation around X axis (radians), clamped to ±89°
    pub fov_y: f32,     // Vertical field of view (radians), default π/3
    pub z_near: f32,
    pub z_far: f32,
}
```

- [`forward()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L31) — Computes the unit direction vector from yaw + pitch using spherical coordinates.
- [`right()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L41) — Cross product of forward and world up.
- [`view_matrix()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L46) — `Mat4::look_at_rh(position, position + forward(), Y)`.
- [`projection_matrix()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L52) — `Mat4::perspective_rh(fov_y, aspect, z_near, z_far)`.
- [`mouse_look(dx, dy, sensitivity)`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L62) — Adds to yaw/pitch from mouse deltas, clamps pitch to ±89°.

---

## 6. Canvas and Text Overlay ([`canvas/`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/), [`text.rs`](https://github.com/justinwash/rengine/blob/master/engine/src/text.rs))

### 6.1 [`FontAtlas`](https://github.com/justinwash/rengine/blob/master/engine/src/text.rs#L17) Construction

The engine embeds `assets/font.ttf` at compile time via `include_bytes!()`. At initialization:

1. Parse the font with `fontdue::Font::from_bytes()`.
2. Allocate a 512×512 single-channel (`R8Unorm`) pixel buffer.
3. Write a 2×2 white pixel block at the top-left corner (used for solid rectangles) → [`white_uv = [1.0/512.0, 1.0/512.0]`](https://github.com/justinwash/rengine/blob/master/engine/src/text.rs#L25).
4. Rasterize ASCII characters 32–126 at 48px using fontdue.
5. Pack glyphs into the atlas using a simple left-to-right, top-to-bottom bin packer with 1px padding.
6. For each glyph, store UV coordinates, pixel dimensions, x/y offsets, and advance width in a `[Option<GlyphEntry>; 128]` array.
7. Upload the atlas to a GPU texture.
8. Create a bind group with the texture + a linear-filtering sampler.

### 6.2 [`Canvas`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L42) Drawing

`Canvas` is an immediate-mode 2D drawing API that operates in **screen space** (pixel coordinates, origin top-left):

```rust
pub struct Canvas {
    pub(crate) verts: Vec<CanvasVertex>,
}
```

Methods:

- **[`canvas.rect(x, y, w, h, color, screen_size)`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L55)** — Draws a solid rectangle. Converts screen coordinates to NDC via `screen_to_ndc()`, uses the `white_uv` from the font atlas so the fragment shader returns a solid color.
- **[`canvas.text(x, y, text, size, color, screen_size, atlas)`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L84)** — Renders text by emitting two triangles per visible glyph. Scales glyphs by `size / FONT_SIZE`. Each quad's UV maps to the glyph's region in the font atlas.
- **[`canvas.shape(triangles)`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L51)** — Accepts raw `CanvasVertex` triangles for custom shapes.

**NDC conversion:**

```rust
pub fn screen_to_ndc(x: f32, y: f32, screen_size: (u32, u32)) -> [f32; 2] {
    [(x / sw) * 2.0 - 1.0, 1.0 - (y / sh) * 2.0]
}
```

Maps screen-space (0,0 = top-left) to NDC (-1,-1 = bottom-left, +1,+1 = top-right).

### 6.3 The canvas.wgsl Shader

```wgsl
@vertex fn vs_main(in: VertexInput) -> VertexOutput {
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);  // Already in NDC!
    out.color = in.color;
    out.uv = in.uv;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(font_texture, font_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
```

Key insight: Canvas vertices are pre-transformed to NDC on the CPU, so the vertex shader is a passthrough. The fragment shader reads the **red channel** from the font atlas as alpha. For solid rectangles (using `white_uv`), alpha ≈ 1.0. For text glyphs, alpha comes from the glyph's rasterized bitmap.

The canvas pipeline uses `ALPHA_BLENDING` and `LoadOp::Load` (draws on top of existing content).

### 6.4 The FPS Counter

When `EngineConfig::show_fps` is true, the engine creates a dedicated canvas, draws a semi-transparent black background rectangle and green FPS text at (8,8) in 16px size. This canvas is appended to `frame.canvases` after the game's render call.

---

## 7. Input System ([`input/`](https://github.com/justinwash/rengine/blob/master/engine/src/input/))

### 7.1 [`InputState`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L6) — Keyboard State

```rust
pub struct InputState {
    keys_down: HashSet<KeyCode>,       // Currently held keys
    keys_pressed: HashSet<KeyCode>,    // Keys pressed THIS frame
    keys_released: HashSet<KeyCode>,   // Keys released THIS frame
    mouse_delta: (f64, f64),           // Accumulated mouse motion this frame
    mouse_buttons: [bool; 3],          // Held: [Left, Right, Middle]
    mouse_buttons_pressed: [bool; 3],  // Pressed this frame
    mouse_buttons_released: [bool; 3], // Released this frame
}
```

**Three-state key model:**

- [`is_key_down(key)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L30) — True every frame the key is held.
- [`is_key_pressed(key)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L35) — True only the first frame of a press (edge trigger).
- [`is_key_released(key)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L40) — True only the frame the key is released.

[`handle_key_event()`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L70) logic:

- On `Pressed`: insert into `keys_down`. If it was newly inserted (not already held), also insert into `keys_pressed`.
- On `Released`: remove from `keys_down`, insert into `keys_released`.

[`end_frame()`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L107) clears `keys_pressed`, `keys_released`, `mouse_delta`, and `mouse_buttons_pressed/released`. This ensures "pressed" and "released" are one-frame events.

### 7.2 Mouse State

Mouse motion is accumulated via [`handle_mouse_motion(dx, dy)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L84):

```rust
self.mouse_delta.0 += dx;
self.mouse_delta.1 += dy;
```

Multiple motion events per frame are summed. The game reads [`input.mouse_delta()`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L45) and the total is reset at [`end_frame()`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L107).

Mouse buttons use the same pressed/down/released model as keys, mapped by index: 0=Left, 1=Right, 2=Middle.

### 7.3 [`GamepadSystem`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L58) and [`GamepadState`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L9)

Built on **gilrs**. Supports up to `MAX_PLAYERS = 4` gamepads.

```rust
pub struct GamepadSystem {
    gilrs: Gilrs,
    slots: Vec<GamepadState>,              // 4 player slots
    id_to_slot: HashMap<GamepadId, usize>, // Maps physical gamepad → slot
}
```

**Auto-assignment:** When a gamepad connects, it's assigned to the first empty slot. When it disconnects, its slot is cleared.

**Per-frame update:**

1. Clear `buttons_pressed` and `buttons_released` for all slots.
2. Drain gilrs events: handle `Connected`, `Disconnected`, `ButtonPressed`, `ButtonReleased`.
3. Read analog axes: `left_stick_x/y` from `Axis::LeftStickX/Y`.
4. **D-pad override:** If D-pad is pressed, override the stick axis to ±1.0.
5. **Dead zone:** Values below 0.15 are clamped to 0.

`GamepadState` provides:

- [`is_button_down(button)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L37), [`is_button_pressed(button)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L42), [`is_button_released(button)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L47)
- `left_stick_x`, `left_stick_y` (public fields)
- [`is_connected()`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L52)

---

### 7.4 [`ActionMap`](https://github.com/justinwash/rengine/blob/master/engine/src/input/action.rs) — Input Action Mapping

Abstracts raw key/button/stick input into named **actions** (digital) and **axes** (analog).

```rust
pub enum Binding {
    Key(KeyCode),
    MouseButton(usize),
    GamepadButton(GamepadButton),
}

pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
}

pub struct AxisMapping {
    pub positive: Vec<Binding>,
    pub negative: Vec<Binding>,
    pub gamepad_axis: Option<GamepadAxis>,
}

pub struct ActionMap {
    actions: HashMap<String, Vec<Binding>>,
    axes: HashMap<String, AxisMapping>,
}
```

**Setup** — call `engine.actions_mut()` during initialization:

```rust
let actions = engine.actions_mut();
actions.bind("jump", Binding::Key(KeyCode::Space));
actions.bind("jump", Binding::GamepadButton(GamepadButton::South));
actions.bind_axis("move_x", AxisMapping {
    positive: vec![Binding::Key(KeyCode::KeyD), Binding::Key(KeyCode::ArrowRight)],
    negative: vec![Binding::Key(KeyCode::KeyA), Binding::Key(KeyCode::ArrowLeft)],
    gamepad_axis: Some(GamepadAxis::LeftStickX),
});
```

**Queries** on `Engine` (default to gamepad player 0):

- `engine.action_down("jump")` — true every frame while any bound input is held.
- `engine.action_pressed("jump")` — true only the first frame (edge trigger).
- `engine.action_released("jump")` — true only the release frame.
- `engine.axis("move_x")` — returns `-1.0..1.0`. Digital bindings contribute ±1; analog stick value is used when its magnitude exceeds the digital sum.

**Multiplayer** — `_player` variants check a specific gamepad slot:

- `engine.action_down_player("jump", 1)` — tests keyboard + player 1’s gamepad.
- `engine.axis_player("move_x", 2)` — uses player 2’s gamepad stick.

Keyboard and mouse bindings always contribute regardless of player index (only one keyboard).

`Engine3D` also has `actions_mut()`, `action_down()`, `action_pressed()`, `action_released()`, and `axis()`. Since `Engine3D` has no gamepad system, gamepad bindings are inert.

`ActionMap` also provides `unbind()`, `unbind_axis()`, and `clear()` for runtime rebinding.

---

## 8. Asset Pipeline ([`assets/`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/))

### 8.1 [`AssetPipeline`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L247) (Internal)

The `AssetPipeline` is the internal caching layer:

```rust
struct AssetPipeline {
    root: PathBuf,
    bytes: HashMap<PathBuf, Arc<[u8]>>,
    text: HashMap<PathBuf, Arc<str>>,
    manifests: HashMap<PathBuf, AssetManifest>,
    textures: HashMap<PathBuf, TextureAsset>,
    sprite_sheets: HashMap<SpriteSheetKey, SpriteSheet>,
    meshes: HashMap<PathBuf, MeshAsset>,
    texture_timestamps: HashMap<PathBuf, SystemTime>,
    mesh_timestamps: HashMap<PathBuf, SystemTime>,
    manifest_timestamps: HashMap<PathBuf, SystemTime>,
    manifest_deps: HashMap<PathBuf, Vec<PathBuf>>,
}
```

**Path resolution:** `resolve_path()` joins relative paths with `self.root` and canonicalizes. Absolute paths are used as-is.

**Caching:** All `load_*` methods check the cache first. This means calling `load_texture("player.png")` twice returns the same `TextureId` without re-uploading.

**Dependency tracking:** When `load_asset_manifest()` is called, the engine records every file path loaded by that manifest in `manifest_deps`. Query with `engine.manifest_dependencies("assets.json")`.

**Manifest validation:** `engine.validate_manifest("assets.json")` parses the manifest JSON and checks that every referenced file exists on disk. Returns `Vec<AssetError>` with all problems found rather than failing on the first. Useful for build-time or startup validation.

**Cache management:** `engine.loaded_asset_summary()` returns an `AssetSummary` with counts and paths. Use `unload_texture()`, `unload_mesh()`, or `unload_data()` to evict cached assets.

### 8.2 [`AssetManifest`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L158) and [`AssetPack`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L174)

An `AssetManifest` is a JSON file declaring assets by alias:

```json
{
  "textures": { "player": "sprites/player.png", "tiles": "sprites/tiles.png" },
  "sprite_sheets": { "walk": { "path": "sprites/walk.png", "cell_width": 32, "cell_height": 32 } },
  "audio": { "jump": "audio/jump.wav", "music": "audio/bgm.ogg" },
  "meshes": { "level": "meshes/level.obj" },
  "bytes": { "config": "data/config.bin" },
  "text": { "dialogue": "data/dialogue.json" }
}
```

`Engine::load_asset_manifest(path)` loads the JSON, then loads each entry through the pipeline, producing an `AssetPack`:

```rust
pub struct AssetPack {
    bytes: HashMap<String, Arc<[u8]>>,
    text: HashMap<String, Arc<str>>,
    textures: HashMap<String, TextureAsset>,
    sprite_sheets: HashMap<String, SpriteSheet>,
    meshes: HashMap<String, MeshAsset>,
    audio: HashMap<String, AudioClip>,
}
```

The `AssetPack` provides typed accessors by alias: [`pack.texture("player")`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L192), [`pack.sprite_sheet("walk")`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L196), [`pack.audio("jump")`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L204), etc. It also provides [`texture_id(alias)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L208) which checks both textures and sprite sheets.

### 8.3 Texture Loading

`load_texture()` flow:

1. Resolve path → check cache → miss: read file from disk.
2. Decode with `image::load_from_memory()` → convert to RGBA8.
3. Call the `create_texture` callback (which calls `renderer.create_texture()`).
4. Store the `TextureAsset { id, width, height, path }` in the cache.
5. Record the file's modification timestamp for hot reload.

### 8.4 [`SpriteSheet`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L5) and [`Animation`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L56)

```rust
pub struct SpriteSheet {
    pub texture: TextureId,
    pub texture_width: u32,
    pub texture_height: u32,
    pub cell_width: u32,
    pub cell_height: u32,
}
```

- [`columns()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L35) → `texture_width / cell_width`
- [`rows()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L40) → `texture_height / cell_height`
- [`uv_rect(col, row)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L45) → `[u, v, w, h]` in 0..1 range for the specified cell

Loading validates that the texture dimensions are evenly divisible by cell dimensions.

**Animation:**

```rust
pub struct Animation {
    pub frames: Vec<(u32, u32)>,  // (col, row) pairs
    pub frame_time: f32,          // Seconds per frame
    elapsed: f32,
    current: usize,
}
```

- [`Animation::new(frames, fps)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L68) — Creates an animation with `frame_time = 1.0 / fps`.
- [`update(dt)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L78) — Advances the timer; when `elapsed >= frame_time`, cycles to the next frame (wrapping). Returns the current `(col, row)`.
- [`current_frame()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L88) — Returns current without advancing.
- [`reset()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L93) — Resets to frame 0.

Usage pattern:

```rust
let (col, row) = animation.update(engine.dt());
let uv = sprite_sheet.uv_rect(col, row);
frame.draw_sprite(DrawParams::new(sprite_sheet.texture, pos, size).with_uv_rect(uv));
```

### 8.5 3D Mesh Loading (OBJ and glTF)

`load_mesh()` dispatches based on file extension:

- `.obj` → `read_obj_mesh()` using `tobj` with triangulation and single-indexing
- `.gltf` / `.glb` → `read_gltf_mesh()` using the `gltf` crate

After loading, two post-processing steps run:

1. **`fix_winding_from_normals()`** — For each triangle, checks if the geometric normal (from cross product) agrees with the average vertex normal. If they disagree, swaps two indices to flip the winding. This corrects meshes where the face winding doesn't match the authored normals.
2. **`compute_flat_normals()`** — If all vertex normals are zero (unset), computes flat-shading normals from face geometry.

### 8.6 Audio Loading

`Engine::load_audio()`:

1. Resolves the path.
2. Loads raw bytes via the asset pipeline.
3. Registers the clip with the audio system: `audio.register_clip(path, bytes)`.
4. Returns an `AudioClip { id: AudioId, path }`.

Audio data is stored as raw bytes and decoded on-demand at playback time using `rodio::Decoder`.

### 8.7 Hot Reload

When `hot_reload_enabled` is true, `engine.reload_assets_if_changed()` is called every frame:

**For 2D Engine:**

1. **Textures:** Iterates all loaded textures, checks file modification time against stored timestamp. If newer, re-reads the image, calls `renderer.replace_texture()` to update the GPU texture in-place (same `TextureId`), and updates sprite sheet dimensions.
2. **Manifests:** Checks modification times and invalidates stale manifests from the cache (does not automatically re-load, just forces a re-parse on next access).
3. **Audio:** Checks modification times and replaces raw bytes in the clip store.

**For 3D Engine:** Same as above but for meshes instead of textures (re-runs winding correction and normal computation).

### 8.8 [`AssetError`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L15)

A comprehensive error enum covering all asset failure modes:

```rust
pub enum AssetError {
    Io { path, source: io::Error },
    Utf8 { path, source: FromUtf8Error },
    Json { path, source: serde_json::Error },
    Image { path, source: image::ImageError },
    Mesh { path, message: String },
    Manifest { path, message: String },
    Scene { path, message: String },
    Audio { path, message: String },
    InvalidSpriteSheet { path, texture_width, texture_height, cell_width, cell_height },
}
```

All variants carry the relevant path for context in error messages.

---

## 9. Audio System ([`assets/audio.rs`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs))

### 9.1 [`AudioBus`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L14) and Volume

Four buses: `Music`, `Effects`, `Ui`, `Ambient`. Each has an independent volume multiplier. The final volume for any sound is:

```
final_volume = master_volume × bus_volume × clip_volume
```

`AudioSystem` uses `RefCell`-based interior mutability for the active sinks and music sink, allowing [`play()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L106) to be called from `&self` contexts.

[`play_on_bus(bus, clip, volume)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L110):

1. Clean up finished sinks.
2. Create a new `rodio::Sink`.
3. Set volume to `final_volume`.
4. Decode the clip bytes and append the audio source.
5. Push to `active_sinks`.

### 9.2 Music Playback

[`play_music_with_volume(clip, volume)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L137):

1. Stops any existing music.
2. Creates a new sink.
3. Decodes the clip and appends it with `.repeat_infinite()` for looping.
4. Stores in `music_sink`.

[`pause_music()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L164) / [`resume_music()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L170) call `.pause()` / `.play()` on the music sink.

### 9.3 Headless Mode

When `headless` is true:

- Master volume is set to 0.
- [`play_on_bus()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L110) still decodes the clip (exercises the decode path for testing) but if no audio handle is available, returns early after decoding.
- [`set_master_volume()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L191) forces 0 if `silent` is true.

---

## 10. Color and Pixel Art

### [`Color`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L2)

```rust
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }
```

Constants: `WHITE`, `BLACK`, `RED`, `ORANGE`, `YELLOW`, `GREEN`, `BLUE`, `INDIGO`, `VIOLET`.

Constructors: [`new(r,g,b,a)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L65), [`rgb(r,g,b)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L69), [`from_rgba8(r,g,b,a)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L73).

Conversions: [`to_array() → [f32; 4]`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L82), [`to_wgpu() → wgpu::Color`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L86).

### [`PixelCanvas`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L3) (Procedural Texture Generation)

`assets::pixelart::PixelCanvas` is a CPU-side pixel buffer for procedural texture creation:

```rust
pub struct PixelCanvas { pub width: u32, pub height: u32, pixels: Vec<[u8; 4]> }
```

Methods: [`fill(color)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L18), [`set(x, y, color)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L23), [`fill_rect(x, y, w, h, color)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L29), [`fill_circle(cx, cy, radius, color)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L42), [`fill_diamond(color)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L58), [`stroke_diamond(color, thickness)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L73), [`into_bytes() → Vec<u8>`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L89).

Usage pattern:

```rust
let mut canvas = pixelart::PixelCanvas::new(16, 16);
canvas.fill(Color::BLUE);
canvas.fill_rect(2, 2, 12, 12, Color::WHITE);
let pixels = canvas.into_bytes();
let tex = engine.create_texture(16, 16, &pixels);
```

[`darken(color, factor)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L106) and [`lighten(color, factor)`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L110) are color utility functions.

---

## 11. Scene System ([`scene/`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/))

### 11.1 [`Scene`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L24) Trait and [`SceneOp`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L16)

```rust
pub trait Scene: 'static {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals);
    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp;
    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame);
    fn on_pause(&mut self, _engine: &Engine, _globals: &Globals) {}
    fn on_resume(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}
    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {}
}
```

`SceneOp` is the return value from `update()`:

```rust
pub enum SceneOp {
    Continue,                   // Do nothing
    Push(Box<dyn Scene>),       // Push new scene (current paused)
    Switch(Box<dyn Scene>),     // Replace current scene
    Pop,                        // Remove current scene (previous resumed)
    Quit,                       // Exit the game
}
```

Lifecycle:

- **`on_enter`** — Called when the scene is first activated (pushed or switched to). `&mut Engine` allows loading assets.
- **`update`** — Called every frame for the top scene only. Returns `SceneOp`.
- **`render`** — Called for **all** scenes in the stack, bottom to top.
- **`on_pause`** — Called on the current scene when a new scene is pushed on top.
- **`on_resume`** — Called when the scene above is popped. `&mut Engine` for potential re-loading.
- **`on_exit`** — Called when the scene is removed from the stack.

### 11.2 [`Globals`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L4) — Typed Key-Value Store

```rust
pub struct Globals {
    data: HashMap<TypeId, Box<dyn Any>>,
}
```

Methods: [`set<T>(value)`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L21), [`get<T>() → Option<&T>`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L25), [`get_mut<T>() → Option<&mut T>`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L29), [`remove<T>() → Option<T>`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L33), [`contains<T>() → bool`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L39).

This uses `TypeId` as keys and `Any` for type-erased storage. Each type `T` can have exactly one value stored. This is a common pattern for cross-scene shared state (scores, player data, settings).

### 11.3 Scene Stack Dispatch

`apply_scene_op()`:

```rust
fn apply_scene_op(stack, op, engine, globals) {
    match op {
        Continue => {},
        Quit => { while let Some(mut scene) = stack.pop() { scene.on_exit(); } },
        Push(new) => { stack.last_mut().on_pause(); new.on_enter(); stack.push(new); },
        Pop => { stack.pop().on_exit(); stack.last_mut().on_resume(); },
        Switch(new) => { stack.pop().on_exit(); new.on_enter(); stack.push(new); },
    }
}
```

**Critical rendering detail:** All scenes render, not just the top:

```rust
for scene in stack.iter() {
    scene.render(&engine, &globals, &mut frame);
}
```

This means a pause overlay scene pushed on top of a game scene will render the game first (with its clear color), then the overlay draws on top (e.g. a semi-transparent dark rectangle + "PAUSED" text).

### 11.4 2D Scene Data ([`Scene2D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L98), [`SceneInstance2D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L67), Prefabs, Instances)

Rengine supports data-driven 2D scenes via JSON:

**Scene2DDef (JSON format):**

```json
{
  "prefabs": [
    {
      "name": "tree",
      "sprites": [{ "asset": "tree_texture", "offset": [0, 0], "size": [32, 48] }]
    }
  ],
  "instances": [{ "prefab": "tree", "position": [100, 200], "scale": [1.5, 1.5], "properties": { "type": "oak" } }]
}
```

[`Scene2D::load_from_path(path, assets)`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L103) parses this JSON, resolves texture aliases against an `AssetPack`, and produces `SceneInstance2D` objects that can be queried and drawn:

- [`scene.instances()`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L144) → slice of all instances
- [`scene.by_prefab("tree")`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L148) → iterator of instances using that prefab
- [`scene.draw(frame)`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L154) — draws all instances
- Each instance has [`property("type")`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L76) for custom key-value metadata

---

## 12. World Systems ([`world/`](https://github.com/justinwash/rengine/blob/master/engine/src/world/))

### 12.1 [`TileMap`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L6) and [`TileDef`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L16)

```rust
pub struct TileMap {
    pub width: usize, pub height: usize, pub tile_size: f32,
    cells: Vec<Option<usize>>,  // Grid of tile IDs (None = empty)
    tiles: Vec<TileDef>,        // Tile definitions
}
```

**Tile definitions:**

```rust
pub struct TileDef {
    pub texture: TextureId,
    pub color: Color,
    pub uv_rect: [f32; 4],
}
```

API:

- [`tilemap.add_tile(def)`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L59) → `usize` (tile ID)
- [`tilemap.set(col, row, Some(tile_id))`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L66) / [`tilemap.get(col, row)`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L73)
- [`tilemap.cell_position(col, row)`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L82) → `Vec2`
- [`tilemap.world_width()`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L87) / [`world_height()`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L92)
- **[`tilemap.collide_rect(rect)`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L97)** → `Option<Vec2>` — Checks a `Rect` against all occupied tiles within range, accumulates AABB minimum translation vectors. Returns the total push-back vector to resolve overlap.
- **[`tilemap.draw(frame)`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L141)** — Frustum-culled tile rendering: only draws tiles visible within a hardcoded 600×400 half-extent around the camera. Each visible tile emits a `DrawParams` with the tile's texture, color, and UV rect.

### 12.2 [`aabb_overlap`](https://github.com/justinwash/rengine/blob/master/engine/src/world/physics.rs) — AABB Physics

```rust
pub fn aabb_overlap(a: &Rect, b: &Rect) -> Option<Vec2>
```

Computes the **Minimum Translation Vector (MTV)** for two overlapping AABBs. Returns `None` if they don't overlap. The MTV is along the axis of least penetration:

- Computes overlap on both X and Y axes.
- Returns the smaller overlap as the push direction, with sign determined by the relative center positions.

This is used by `TileMap::collide_rect()` for tilemap collision.

#### Collision Layers

```rust
pub struct CollisionLayer {
    pub layer: u32,  // which groups this body belongs to
    pub mask: u32,   // which groups this body collides with
}
```

Bitmask-based collision filtering. Two bodies interact when `a.layer & b.mask != 0 && b.layer & a.mask != 0`. Named constants: `WORLD`, `PLAYER`, `ENEMY`, `PROJECTILE`, `TRIGGER`, `UI`. `CollisionLayer::default()` sets all bits so existing code is unaffected.

```rust
pub fn aabb_overlap_layered(a: &Rect, a_layer: &CollisionLayer, b: &Rect, b_layer: &CollisionLayer) -> Option<Vec2>
```

Checks layer compatibility via `interacts_with()` before delegating to `aabb_overlap()`. Returns `None` if layers don't interact or AABBs don't overlap.

### 12.3 [`iso_to_screen`](https://github.com/justinwash/rengine/blob/master/engine/src/world/iso.rs#L4) / [`screen_to_iso`](https://github.com/justinwash/rengine/blob/master/engine/src/world/iso.rs#L11) — Isometric Helpers

```rust
pub fn iso_to_screen(col: i32, row: i32, tile_width: f32, tile_height: f32) -> Vec2
pub fn screen_to_iso(screen: Vec2, tile_width: f32, tile_height: f32) -> (i32, i32)
```

Standard diamond-shaped isometric projection. `iso_to_screen` converts grid coordinates to screen-space positions. `screen_to_iso` converts back.

---

## 13. Math Utilities ([`math/`](https://github.com/justinwash/rengine/blob/master/engine/src/math/))

### 13.1 [`Rect`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L5)

```rust
pub struct Rect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }
```

Methods: [`new()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L13), [`from_pos_size()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L22), [`left()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L31), [`right()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L35), [`bottom()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L39), [`top()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L43), [`center()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L47), [`contains_point()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L51), [`overlaps()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L58).

Note: [`bottom()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L39) returns `y` and [`top()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L43) returns `y + height`, so Y increases upward (matching the world coordinate system).

### 13.2 [`TimeState`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L4)

```rust
pub struct TimeState {
    start_time: Instant,
    last_frame: Instant,
    dt: f32,
    total_time: f32,
    frame_count: u64,
    fixed_dt: f32,       // Fixed-timestep interval (default 1/60)
    accumulator: f32,    // Accumulated time for fixed steps
}
```

- [`dt()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L25) — Seconds since last frame (capped at 0.1 to prevent spiral-of-death).
- [`total_time()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L30) — Seconds since engine start.
- [`frame_count()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L34) — Total frames processed.
- [`fps()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L38) — `1.0 / dt`.
- [`tick()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L46) — Called once per frame by the engine; updates all fields and adds `dt` to `accumulator`.
- [`fixed_dt()`] — Returns the configured fixed-timestep interval.
- [`consume_fixed_step()`] — Returns `true` and subtracts `fixed_dt` from `accumulator` if enough time has accumulated. Called in a `while` loop before `update()` to drive `fixed_update()` N times per frame.

---

## 14. Rollback Netcode ([`netcode/`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/), feature-gated)

### 14.1 Architecture Overview ([`Rollbackable`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L73))

Rengine integrates **GGRS** (Good Game Rollback SDK) for deterministic rollback netcode. This is feature-gated behind `rollback`.

The key trait is:

```rust
pub trait Rollbackable {
    type Input: InputT;
    fn advance(&mut self, inputs: &[Self::Input]);
    fn save(&self) -> Vec<u8>;
    fn load(&mut self, data: &[u8]);
}
```

`InputT` requires: `Copy + Clone + PartialEq + Default + Pod + Zeroable + Serialize + DeserializeOwned + 'static`.

### 14.2 [`RollbackSession`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L86)

```rust
pub struct RollbackSession<I: InputT> {
    variant: SessionVariant<I>,  // Local | SyncTest | P2P
    local_player: usize,
    num_players: usize,
    fixed_dt: f32,               // 1.0 / fps
    accumulator: f32,            // For fixed-timestep accumulation
    frame: u32,
    desync_detected: bool,
    max_frames: Option<u32>,
    headless: bool,
}
```

**Session modes:**

- **`Local`** — No rollback; directly advances the simulation.
- **`SyncTest { check_distance }`** — Runs all players locally and uses GGRS sync testing to validate determinism.
- **`Online(`[`OnlineConfig`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L39)`)`** — Real P2P rollback over UDP.

**[`update(dt, inputs, sim)`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L186)** — The central tick function:

1. Accumulates `dt` into the fixed-timestep accumulator.
2. If not enough time has passed, just polls remote clients (P2P) and returns `false`.
3. Otherwise, subtracts `fixed_dt` from accumulator and processes one tick.
4. For Local: directly calls `sim.advance(inputs)`.
5. For SyncTest/P2P: feeds inputs to GGRS, advances the session, and handles save/load/advance requests via `handle_request()`.
6. Returns `true` if a tick was processed.

**GGRS request handling:**

- `SaveGameState` — Calls `sim.save()`, computes [`fletcher64`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L290) checksum, stores in GGRS cell.
- `LoadGameState` — Calls `sim.load()` with the stored state.
- `AdvanceFrame` — Calls `sim.advance()` with the GGRS-provided inputs.

### 14.3 [`UdpNonBlockingSocket`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/transport.rs#L5) — UDP Transport

`UdpNonBlockingSocket` implements `ggrs::NonBlockingSocket<String>`:

- Binds a non-blocking UDP socket.
- `send_to()` serializes GGRS messages with bincode and sends via UDP.
- `receive_all_messages()` drains all pending UDP packets, deserializes them.
- Address strings are parsed and cached for efficiency.

---

## 15. Complete Frame Lifecycle: Boot to Pixel

Here is the complete sequence from program start to a pixel appearing on screen:

```
main()
 └─ rengine::run::<MyGame>(config)
     ├─ env_logger::init()
     ├─ EventLoop::new()
     ├─ WindowBuilder → Arc<Window>
     ├─ Renderer::new(window, present_mode)   ← async, blocked by pollster
     │   ├─ wgpu::Instance::new(all backends)
     │   ├─ instance.create_surface(window)
     │   ├─ instance.request_adapter()
     │   ├─ adapter.request_device()
     │   ├─ surface.configure(sRGB, present_mode)
     │   ├─ create_shader_module(sprite.wgsl)
     │   ├─ create bind group layouts (projection + texture)
     │   ├─ create_render_pipeline(sprite pipeline)
     │   ├─ create vertex_buffer (40K vertices)
     │   ├─ create + fill index_buffer (60K indices)
     │   ├─ create projection_buffer (64 bytes)
     │   ├─ create sampler (nearest-neighbor)
     │   ├─ create canvas pipeline + canvas vertex buffer
     │   ├─ rasterize font → create font_atlas texture + bind group
     │   └─ create white_texture (1×1 white pixel)
     ├─ Engine { renderer, assets, audio, input, time, gamepads, ... }
     ├─ MyGame::new(&mut engine)              ← Game loads assets
     │   ├─ engine.set_asset_root(...)
     │   ├─ engine.load_texture("player.png") → TextureId
     │   ├─ engine.load_sprite_sheet("walk.png", 32, 32) → SpriteSheet
     │   ├─ engine.load_audio("jump.wav") → AudioClip
     │   └─ engine.load_asset_manifest("assets.json") → AssetPack
     │
     └─ event_loop.run(|event, target| { ... })
         │
         ├─ Event::WindowEvent::KeyboardInput → input.handle_key_event(key, state)
         ├─ Event::WindowEvent::Resized → renderer.resize(w, h)
         ├─ Event::WindowEvent::CloseRequested → target.exit()
         │
         ├─ Event::AboutToWait → window.request_redraw()
         │
         └─ Event::WindowEvent::RedrawRequested
             ├─ time.tick()                   // Measure dt
             ├─ gamepads.update()             // Poll gamepad events
             ├─ reload_assets_if_changed()    // Hot reload
             ├─ game.update(&engine)          // GAME LOGIC
             │   ├─ engine.input().is_key_pressed(KeyCode::Space) → jump
             │   ├─ engine.dt() → apply physics
             │   └─ engine.play_sound(&jump_clip) → rodio playback
             ├─ [should_exit check]
             ├─ Frame::new()                  // Empty draw list
             ├─ game.render(&engine, &mut frame)  // GAME RENDERING
             │   ├─ frame.camera.position = player_pos
             │   ├─ frame.clear_color = Color::rgb(0.5, 0.8, 0.9)
             │   ├─ frame.draw_sprite(DrawParams::new(tex, pos, size)
             │   │       .with_uv_rect(sheet.uv_rect(col, row))
             │   │       .with_flip_x(!facing_right))
             │   └─ frame.canvas(0).text(...)  // HUD text
             ├─ [FPS overlay if enabled]
             ├─ renderer.render_frame(&frame)
             │   ├─ surface.get_current_texture()
             │   ├─ camera.projection() → write to projection_buffer
             │   ├─ sort sprites by (z_order, texture)
             │   ├─ generate vertices (with rotation, flip, origin)
             │   ├─ write_buffer(vertices)
             │   ├─ batch by texture
             │   ├─ begin_render_pass(clear_color)
             │   ├─ for each batch: set_bind_group(texture), draw_indexed(range)
             │   ├─ canvas::render_pass(canvases)  // Text + rects on top
             │   │   ├─ collect all canvas vertices
             │   │   ├─ write_buffer(canvas_verts)
             │   │   ├─ begin_render_pass(LoadOp::Load)
             │   │   └─ draw(0..count)
             │   ├─ queue.submit(encoder.finish())
             │   └─ output.present()          // SWAP BUFFERS → PIXEL ON SCREEN
             └─ input.end_frame()             // Clear per-frame state
```

---

## 16. Kitchen-Sink Game Example

The following is a **single, cohesive game** that exercises as many Rengine features as possible. It's a 2D platformer with scene switching, audio, tilemaps, sprite sheets, animations, canvas HUD, camera control, pixel art generation, collision physics, gamepad support, and data-driven scenes.

To use this example, create a new crate in the workspace and add it to the `Cargo.toml` members.

```rust
// main.rs — Kitchen-Sink Rengine Demo
//
// Demonstrates: EngineConfig, Game trait, run(), run_with_scenes(), Scene trait,
// SceneOp (Switch, Push, Pop, Quit), Globals, Engine, Frame, Camera2D,
// DrawParams (position, size, color, uv_rect, flip_x, flip_y, rotation, origin, z_order),
// TextureId, SpriteSheet, Animation, TileMap, TileDef, aabb_overlap, Rect,
// Canvas (rect, text, shape), FontAtlas, Color, pixelart::PixelCanvas,
// InputState (key_down, key_pressed, key_released), mouse input,
// GamepadState, AudioClip, AudioBus, play_sound, play_music, set_master_volume,
// TimeState (dt, total_time, fps), hot reload, asset manifest loading,
// create_texture, create_color_texture, white_texture, screen_to_ndc, Vec2.

use rengine::*;
use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────
// Shared data stored in Globals
// ──────────────────────────────────────────────────────────────

/// Tracks total scene transitions across the entire application.
struct TransitionCounter(u32);

/// Persistent player stats shared across scenes.
struct PlayerStats {
    coins: u32,
    best_height: f32,
}

// ──────────────────────────────────────────────────────────────
// Title Scene — demonstrates Switch and canvas text
// ──────────────────────────────────────────────────────────────

struct TitleScene {
    blink_timer: f32,
}

impl Scene for TitleScene {
    fn on_enter(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        // Increment the global transition counter
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        self.blink_timer += engine.dt();

        // Press Enter or Space to start the game → Switch to GameScene
        if engine.input().is_key_pressed(KeyCode::Enter)
            || engine.input().is_key_pressed(KeyCode::Space)
        {
            return SceneOp::Switch(Box::new(GameScene::default()));
        }

        // Escape quits
        if engine.input().is_key_pressed(KeyCode::Escape) {
            return SceneOp::Quit;
        }

        // Gamepad support: South button starts
        let pad = engine.gamepad(0);
        if pad.is_button_pressed(GamepadButton::South) {
            return SceneOp::Switch(Box::new(GameScene::default()));
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        frame.clear_color = Color::new(0.1, 0.05, 0.2, 1.0);

        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);

        // Title
        canvas.text(200.0, 100.0, "RENGINE KITCHEN SINK", 32.0, Color::YELLOW, (sw, sh), atlas);

        // Blinking prompt
        if (self.blink_timer * 2.0).sin() > 0.0 {
            canvas.text(
                220.0, 250.0, "Press ENTER to start", 18.0, Color::WHITE, (sw, sh), atlas,
            );
        }

        // Show stats from Globals
        let transitions = globals.get::<TransitionCounter>().map_or(0, |c| c.0);
        canvas.text(
            10.0, 550.0,
            &format!("Scene transitions: {}", transitions),
            12.0, Color::GREEN, (sw, sh), atlas,
        );

        // Gamepad hint
        if engine.gamepads_connected() > 0 {
            canvas.text(
                220.0, 300.0, "(Gamepad detected: press A)", 14.0, Color::ORANGE, (sw, sh), atlas,
            );
        }
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[TitleScene] on_exit");
    }
}

// ──────────────────────────────────────────────────────────────
// Game Scene — the main gameplay
// ──────────────────────────────────────────────────────────────

struct GameScene {
    // Textures (created procedurally via PixelCanvas)
    player_tex: Option<TextureId>,
    coin_tex: Option<TextureId>,
    ground_tex: Option<TextureId>,
    bg_tex: Option<TextureId>,

    // Sprite sheet for animated coins (procedurally generated)
    coin_sheet: Option<SpriteSheet>,
    coin_anim: Animation,

    // Tilemap
    tilemap: Option<TileMap>,

    // Player state
    player_pos: Vec2,
    player_vel: Vec2,
    player_on_ground: bool,
    facing_right: bool,
    player_rotation: f32,

    // Coins
    coins: Vec<Vec2>,
    score: u32,

    // Camera
    cam_pos: Vec2,
    cam_zoom: f32,

    // Time tracking
    play_time: f32,
}

impl Default for GameScene {
    fn default() -> Self {
        Self {
            player_tex: None,
            coin_tex: None,
            ground_tex: None,
            bg_tex: None,
            coin_sheet: None,
            coin_anim: Animation::new(vec![(0, 0), (1, 0), (2, 0), (3, 0)], 8.0),
            tilemap: None,
            player_pos: Vec2::new(100.0, 200.0),
            player_vel: Vec2::ZERO,
            player_on_ground: false,
            facing_right: true,
            player_rotation: 0.0,
            coins: Vec::new(),
            score: 0,
            cam_pos: Vec2::ZERO,
            cam_zoom: 1.0,
            play_time: 0.0,
        }
    }
}

impl Scene for GameScene {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals) {
        // Increment transition counter
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }

        // ── Procedural textures via PixelCanvas ──

        // Player texture: a small character sprite
        let mut player_canvas = pixelart::PixelCanvas::new(16, 16);
        player_canvas.fill(Color::new(0.0, 0.0, 0.0, 0.0)); // transparent
        player_canvas.fill_rect(4, 0, 8, 12, Color::new(0.2, 0.5, 1.0, 1.0)); // body
        player_canvas.fill_rect(5, 12, 2, 4, Color::new(0.2, 0.5, 1.0, 1.0)); // left leg
        player_canvas.fill_rect(9, 12, 2, 4, Color::new(0.2, 0.5, 1.0, 1.0)); // right leg
        player_canvas.fill_rect(6, 2, 4, 4, Color::new(1.0, 0.85, 0.7, 1.0)); // face
        player_canvas.set(7, 3, Color::BLACK); // left eye
        player_canvas.set(9, 3, Color::BLACK); // right eye
        self.player_tex = Some(engine.create_texture(16, 16, &player_canvas.into_bytes()));

        // Coin sprite sheet: 4 frames side by side (64×16 total, 16×16 per cell)
        let mut coin_canvas = pixelart::PixelCanvas::new(64, 16);
        coin_canvas.fill(Color::new(0.0, 0.0, 0.0, 0.0));
        for i in 0..4 {
            let ox = (i * 16 + 3) as i32;
            let widths = [10, 8, 4, 8]; // animation: shrink and grow
            let w = widths[i];
            let x_offset = (10 - w) / 2;
            coin_canvas.fill_rect(ox + x_offset, 3, w, 10, Color::YELLOW);
            coin_canvas.fill_rect(
                ox + x_offset + 1, 4, (w - 2).max(1), 8,
                pixelart::lighten(Color::YELLOW, 1.3),
            );
        }
        let coin_pixels = coin_canvas.into_bytes();
        let coin_tex_id = engine.create_texture(64, 16, &coin_pixels);
        self.coin_tex = Some(coin_tex_id);
        self.coin_sheet = Some(SpriteSheet::new(coin_tex_id, 64, 16, 16, 16));

        // Ground tile texture
        let mut ground_canvas = pixelart::PixelCanvas::new(16, 16);
        ground_canvas.fill(Color::new(0.4, 0.25, 0.1, 1.0)); // brown
        // Top grass strip
        ground_canvas.fill_rect(0, 0, 16, 3, Color::new(0.2, 0.7, 0.2, 1.0));
        // Some texture variation
        ground_canvas.set(3, 5, pixelart::darken(Color::new(0.4, 0.25, 0.1, 1.0), 0.7));
        ground_canvas.set(10, 8, pixelart::darken(Color::new(0.4, 0.25, 0.1, 1.0), 0.7));
        ground_canvas.set(7, 12, pixelart::darken(Color::new(0.4, 0.25, 0.1, 1.0), 0.7));
        self.ground_tex = Some(engine.create_texture(16, 16, &ground_canvas.into_bytes()));

        // Background texture: simple gradient sky
        let mut bg_canvas = pixelart::PixelCanvas::new(1, 64);
        for y in 0..64 {
            let t = y as f32 / 63.0;
            let r = 0.4 * (1.0 - t) + 0.1 * t;
            let g = 0.6 * (1.0 - t) + 0.2 * t;
            let b = 1.0 * (1.0 - t) + 0.5 * t;
            bg_canvas.set(0, y, Color::rgb(r, g, b));
        }
        self.bg_tex = Some(engine.create_texture(1, 64, &bg_canvas.into_bytes()));

        // ── Build tilemap ──
        let ground = self.ground_tex.unwrap();
        let mut tilemap = TileMap::new(50, 30, 32.0);
        let ground_tile = tilemap.add_tile(TileDef::solid(ground));

        // Ground floor
        for col in 0..50 {
            tilemap.set(col, 0, Some(ground_tile));
            tilemap.set(col, 1, Some(ground_tile));
        }

        // Platforms
        for col in 5..10 {
            tilemap.set(col, 5, Some(ground_tile));
        }
        for col in 15..22 {
            tilemap.set(col, 8, Some(ground_tile));
        }
        for col in 25..30 {
            tilemap.set(col, 5, Some(ground_tile));
        }
        for col in 8..14 {
            tilemap.set(col, 12, Some(ground_tile));
        }
        for col in 30..40 {
            tilemap.set(col, 10, Some(ground_tile));
        }
        // Vertical wall
        for row in 2..8 {
            tilemap.set(40, row, Some(ground_tile));
        }

        self.tilemap = Some(tilemap);

        // ── Place coins ──
        self.coins = vec![
            Vec2::new(200.0, 200.0),
            Vec2::new(300.0, 200.0),
            Vec2::new(550.0, 300.0),
            Vec2::new(600.0, 300.0),
            Vec2::new(250.0, 420.0),
            Vec2::new(350.0, 420.0),
            Vec2::new(850.0, 370.0),
            Vec2::new(1050.0, 380.0),
        ];

        // Reset state
        self.player_pos = Vec2::new(100.0, 100.0);
        self.player_vel = Vec2::ZERO;
        self.score = 0;
        self.play_time = 0.0;

        // Initialize player stats in globals if not present
        if !globals.contains::<PlayerStats>() {
            globals.set(PlayerStats {
                coins: 0,
                best_height: 0.0,
            });
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp {
        let dt = engine.dt();
        let input = engine.input();
        self.play_time += dt;

        // ── Input ──
        let mut move_x: f32 = 0.0;

        // Keyboard
        if input.is_key_down(KeyCode::ArrowLeft) || input.is_key_down(KeyCode::KeyA) {
            move_x -= 1.0;
        }
        if input.is_key_down(KeyCode::ArrowRight) || input.is_key_down(KeyCode::KeyD) {
            move_x += 1.0;
        }

        // Gamepad
        let pad = engine.gamepad(0);
        if pad.is_connected() {
            move_x += pad.left_stick_x;
        }

        // Apply horizontal movement
        let speed = 250.0;
        self.player_vel.x = move_x * speed;

        // Update facing direction
        if move_x > 0.1 {
            self.facing_right = true;
        } else if move_x < -0.1 {
            self.facing_right = false;
        }

        // Jump
        let jump_pressed = input.is_key_pressed(KeyCode::Space)
            || input.is_key_pressed(KeyCode::ArrowUp)
            || input.is_key_pressed(KeyCode::KeyW)
            || pad.is_button_pressed(GamepadButton::South);
        if jump_pressed && self.player_on_ground {
            self.player_vel.y = 500.0;
            self.player_on_ground = false;
        }

        // Gravity
        let gravity = -980.0;
        self.player_vel.y += gravity * dt;

        // Move and collide with tilemap
        self.player_pos += self.player_vel * dt;

        if let Some(tilemap) = &self.tilemap {
            let player_rect = Rect::new(
                self.player_pos.x, self.player_pos.y, 28.0, 44.0,
            );
            if let Some(mtv) = tilemap.collide_rect(&player_rect) {
                self.player_pos += mtv;
                if mtv.y > 0.0 {
                    self.player_vel.y = 0.0;
                    self.player_on_ground = true;
                } else if mtv.y < 0.0 {
                    self.player_vel.y = 0.0;
                }
                if mtv.x != 0.0 {
                    self.player_vel.x = 0.0;
                }
            } else {
                self.player_on_ground = false;
            }
        }

        // Small rotation when airborne (visual flair)
        if !self.player_on_ground {
            self.player_rotation += dt * 5.0 * if self.facing_right { 1.0 } else { -1.0 };
        } else {
            // Smoothly return to upright
            self.player_rotation *= (1.0 - dt * 10.0).max(0.0);
        }

        // ── Coin collection ──
        let player_center = self.player_pos + Vec2::new(14.0, 22.0);
        self.coins.retain(|coin| {
            let dist = (*coin + Vec2::new(8.0, 8.0) - player_center).length();
            if dist < 24.0 {
                self.score += 1;
                false // remove coin
            } else {
                true
            }
        });

        // Update player stats in globals
        if let Some(stats) = globals.get_mut::<PlayerStats>() {
            stats.coins = self.score;
            if self.player_pos.y > stats.best_height {
                stats.best_height = self.player_pos.y;
            }
        }

        // ── Coin animation ──
        self.coin_anim.update(dt);

        // ── Camera ──
        let target_cam = self.player_pos + Vec2::new(14.0, 22.0);
        self.cam_pos += (target_cam - self.cam_pos) * (5.0 * dt).min(1.0);

        // Zoom controls
        if input.is_key_down(KeyCode::Equal) {
            self.cam_zoom *= 1.0 + dt;
        }
        if input.is_key_down(KeyCode::Minus) {
            self.cam_zoom *= 1.0 - dt;
        }
        self.cam_zoom = self.cam_zoom.clamp(0.3, 3.0);

        // ── Scene management ──
        // P pushes pause overlay
        if input.is_key_pressed(KeyCode::KeyP) {
            return SceneOp::Push(Box::new(PauseOverlay));
        }

        // T switches back to title
        if input.is_key_pressed(KeyCode::KeyT) {
            return SceneOp::Switch(Box::new(TitleScene { blink_timer: 0.0 }));
        }

        // Escape quits
        if input.is_key_pressed(KeyCode::Escape) {
            return SceneOp::Quit;
        }

        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        frame.clear_color = Color::new(0.4, 0.6, 1.0, 1.0); // sky blue
        frame.camera.position = self.cam_pos;
        frame.camera.zoom = self.cam_zoom;

        // ── Background (parallax-like: draw a large sprite behind everything) ──
        if let Some(bg_tex) = self.bg_tex {
            frame.draw_sprite(
                DrawParams::new(
                    bg_tex,
                    Vec2::new(self.cam_pos.x - 500.0, -100.0),
                    Vec2::new(1000.0, 800.0),
                )
                .with_z_order(-10),
            );
        }

        // ── Tilemap ──
        if let Some(tilemap) = &self.tilemap {
            tilemap.draw(frame);
        }

        // ── Coins ──
        if let Some(sheet) = &self.coin_sheet {
            let (col, row) = self.coin_anim.current_frame();
            let uv = sheet.uv_rect(col, row);
            for coin_pos in &self.coins {
                frame.draw_sprite(
                    DrawParams::new(sheet.texture, *coin_pos, Vec2::new(16.0, 16.0))
                        .with_uv_rect(uv)
                        .with_z_order(5),
                );
            }
        }

        // ── Player ──
        if let Some(player_tex) = self.player_tex {
            frame.draw_sprite(
                DrawParams::new(player_tex, self.player_pos, Vec2::new(28.0, 44.0))
                    .with_flip_x(!self.facing_right)
                    .with_rotation(self.player_rotation * 0.05) // subtle tilt
                    .with_origin(Vec2::new(14.0, 22.0)) // center origin
                    .with_z_order(10),
            );
        }

        // ── HUD via Canvas ──
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let hud = frame.canvas(0);

        // Score
        hud.rect(5.0, 30.0, 180.0, 60.0, Color::new(0.0, 0.0, 0.0, 0.5), (sw, sh));
        hud.text(
            10.0, 35.0,
            &format!("Coins: {}", self.score),
            18.0, Color::YELLOW, (sw, sh), atlas,
        );
        hud.text(
            10.0, 55.0,
            &format!("Height: {:.0}", self.player_pos.y),
            14.0, Color::WHITE, (sw, sh), atlas,
        );
        hud.text(
            10.0, 72.0,
            &format!("Time: {:.1}s", self.play_time),
            14.0, Color::WHITE, (sw, sh), atlas,
        );

        // Controls hint
        hud.text(
            sw as f32 - 350.0, sh as f32 - 20.0,
            "WASD/Arrows: Move | Space: Jump | P: Pause | T: Title | +/-: Zoom",
            10.0, Color::new(1.0, 1.0, 1.0, 0.6), (sw, sh), atlas,
        );

        // Show persistent stats from Globals
        if let Some(stats) = globals.get::<PlayerStats>() {
            hud.text(
                sw as f32 - 200.0, 35.0,
                &format!("Best height: {:.0}", stats.best_height),
                12.0, Color::GREEN, (sw, sh), atlas,
            );
        }
    }

    fn on_pause(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[GameScene] on_pause — game paused");
    }

    fn on_resume(&mut self, _engine: &mut Engine, _globals: &mut Globals) {
        println!("[GameScene] on_resume — game resumed");
    }

    fn on_exit(&mut self, _engine: &Engine, globals: &Globals) {
        println!("[GameScene] on_exit — final score: {}", self.score);
        if let Some(stats) = globals.get::<PlayerStats>() {
            println!("  Total coins collected: {}, Best height: {:.0}", stats.coins, stats.best_height);
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Pause Overlay — demonstrates Push/Pop and transparent overlay
// ──────────────────────────────────────────────────────────────

struct PauseOverlay;

impl Scene for PauseOverlay {
    fn on_enter(&mut self, _engine: &mut Engine, globals: &mut Globals) {
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }
        println!("[PauseOverlay] on_enter");
    }

    fn update(&mut self, engine: &Engine, _globals: &mut Globals) -> SceneOp {
        let input = engine.input();
        if input.is_key_pressed(KeyCode::KeyP) || input.is_key_pressed(KeyCode::Escape) {
            return SceneOp::Pop;
        }
        // Gamepad: Start button unpauses
        if engine.gamepad(0).is_button_pressed(GamepadButton::Start) {
            return SceneOp::Pop;
        }
        SceneOp::Continue
    }

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame) {
        // Semi-transparent overlay (renders on top of GameScene because stack renders bottom-up)
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let overlay = frame.canvas(1); // Use canvas index 1 to layer above HUD

        // Dark overlay
        overlay.rect(0.0, 0.0, sw as f32, sh as f32, Color::new(0.0, 0.0, 0.0, 0.65), (sw, sh));

        // Pause text
        overlay.text(
            sw as f32 / 2.0 - 80.0, sh as f32 / 2.0 - 30.0,
            "PAUSED", 40.0, Color::WHITE, (sw, sh), atlas,
        );
        overlay.text(
            sw as f32 / 2.0 - 120.0, sh as f32 / 2.0 + 20.0,
            "Press P or Esc to resume", 16.0, Color::new(0.8, 0.8, 0.8, 1.0), (sw, sh), atlas,
        );

        // Show current stats
        if let Some(stats) = globals.get::<PlayerStats>() {
            overlay.text(
                sw as f32 / 2.0 - 100.0, sh as f32 / 2.0 + 60.0,
                &format!("Coins: {} | Best Height: {:.0}", stats.coins, stats.best_height),
                14.0, Color::YELLOW, (sw, sh), atlas,
            );
        }
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[PauseOverlay] on_exit");
    }
}

// ──────────────────────────────────────────────────────────────
// Entry Point
// ──────────────────────────────────────────────────────────────

fn main() {
    // Using run_with_scenes for full scene management
    rengine::run_with_scenes(
        EngineConfig {
            title: "Rengine Kitchen Sink".into(),
            width: 960,
            height: 720,
            vsync: false,      // Run as fast as possible
            headless: false,   // Real window
            hot_reload: true,  // Watch for asset changes
            show_fps: true,    // FPS counter overlay
        },
        |_engine, globals| {
            // Initialize shared globals
            globals.set(TransitionCounter(0));
            globals.set(PlayerStats {
                coins: 0,
                best_height: 0.0,
            });

            // Start with the title scene
            Box::new(TitleScene { blink_timer: 0.0 })
        },
    )
    .unwrap();
}
```

### Features Exercised in This Example

| Feature                                                                                                             | Where Used                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`EngineConfig`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L22) (all fields)              | `main()` — title, width, height, vsync, headless, hot_reload, show_fps                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| [`run_with_scenes()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L409)                     | `main()` — scene-stack entry point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| [`Scene`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L24) trait (all hooks)          | `TitleScene`, `GameScene`, `PauseOverlay` — on_enter, update, render, on_pause, on_resume, on_exit                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| [`SceneOp::Switch`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L16)                  | Title → Game, Game → Title                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| [`SceneOp::Push`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L16) / `Pop`            | Game → PauseOverlay, PauseOverlay → Game                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| [`SceneOp::Quit`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/mod.rs#L16)                    | Escape from title                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| [`Globals`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L4) typed store           | `TransitionCounter`, `PlayerStats` — [`set`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L21), [`get`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L25), [`get_mut`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L29), [`contains`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/globals.rs#L39)                                                                                                              |
| [`pixelart::PixelCanvas`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L3)       | Player, coin, ground, sky textures — [`fill`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L18), [`fill_rect`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L29), [`set`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L23), [`darken`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L106), [`lighten`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pixelart.rs#L110) |
| [`Engine::create_texture()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L98)               | Procedural texture upload                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| [`SpriteSheet`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L5)              | Coin animation sprite sheet (4 frames)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| [`Animation`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L56)               | `coin_anim` — [`new`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L68), [`update`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L78), [`current_frame`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/spritesheet.rs#L88)                                                                                                                                                                                                              |
| [`TileMap`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L6)                       | Level geometry — [`new`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L48), [`add_tile`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L59), [`set`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L66), [`collide_rect`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L97), [`draw`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L141)                               |
| [`TileDef::solid()`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L24)             | Ground tile                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| [`TileMap::collide_rect()`](https://github.com/justinwash/rengine/blob/master/engine/src/world/tilemap.rs#L97)      | Player vs tilemap collision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| [`Rect`](https://github.com/justinwash/rengine/blob/master/engine/src/math/rect.rs#L5)                              | Player collision box                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| [`DrawParams`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/sprite.rs#L6) builder          | position, size, color, uv_rect, flip_x, rotation, origin, z_order                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| [`Frame::draw_sprite()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/mod.rs#L39)          | Player, coins, background                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| [`Frame::canvas()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/mod.rs#L58)               | HUD text, pause overlay                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| [`Canvas::rect()`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L55)                  | HUD background, pause darkening                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| [`Canvas::text()`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L84)                  | Score, controls, stats                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| [`Camera2D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer/camera.rs#L4)                    | position, zoom — smooth follow with lerp                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| [`InputState`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L6)                   | [`is_key_down`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L30), [`is_key_pressed`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L35) for movement/jump/menu                                                                                                                                                                                                                                                                                                        |
| [`GamepadState`](https://github.com/justinwash/rengine/blob/master/engine/src/input/gamepad.rs#L9)                  | Left stick for movement, South for jump, Start for unpause                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| [`TimeState`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L4)                         | [`dt()`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L25) for physics, [`total_time`](https://github.com/justinwash/rengine/blob/master/engine/src/math/time.rs#L30) tracking                                                                                                                                                                                                                                                                                                                               |
| [`Color`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L2) constants + constructors | WHITE, BLACK, YELLOW, GREEN, ORANGE, [`rgb()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L69), [`new()`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/color.rs#L65)                                                                                                                                                                                                                                                                                                             |
| [`FontAtlas`](https://github.com/justinwash/rengine/blob/master/engine/src/text.rs#L17)                             | Passed to canvas.text() calls                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| [`screen_to_ndc`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L145) (implicit)       | Used by Canvas internally                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| [`aabb_overlap`](https://github.com/justinwash/rengine/blob/master/engine/src/world/physics.rs#L5) (implicit)       | Used by TileMap::collide_rect internally                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `Vec2`                                                                                                              | All positions, velocities, sizes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Hot reload                                                                                                          | Enabled via config — textures would auto-reload if files change                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| FPS overlay                                                                                                         | Enabled via config — rendered by engine after game render                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Bottom-to-top stack rendering                                                                                       | GameScene renders world, PauseOverlay renders on top                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |

### What This Example Does NOT Cover (and How You Would)

| Feature                                                                                                                                                                                                       | How to Use                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`run::<G: Game>()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L291)                                                                                                                | Implement [`Game`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L279) trait directly instead of using scenes                                                                                                                                                                                                                                                                                                                                                                                            |
| [`AssetManifest`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L158) / [`AssetPack`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/pipeline.rs#L174) | Create a `manifest.json` file, call [`engine.load_asset_manifest("manifest.json")`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L110)                                                                                                                                                                                                                                                                                                                                                                  |
| [`Scene2D`](https://github.com/justinwash/rengine/blob/master/engine/src/scene/data2d.rs#L98) / Prefabs                                                                                                       | Create a scene JSON with prefabs + instances, call [`engine.load_scene2d(assets, "level.json")`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L218)                                                                                                                                                                                                                                                                                                                                                     |
| File-based textures                                                                                                                                                                                           | [`engine.load_texture("player.png")`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L145) instead of procedural generation                                                                                                                                                                                                                                                                                                                                                                               |
| [`AudioClip`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L25) / [`play_sound`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L169)                    | `let clip = engine.load_audio("jump.wav"); engine.play_sound(&clip);`                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| [`play_music`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L182) / [`stop_music`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L190)                           | `engine.play_music(&bgm); engine.pause_music(); engine.resume_music();`                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| [`AudioBus`](https://github.com/justinwash/rengine/blob/master/engine/src/assets/audio.rs#L14) / volume                                                                                                       | [`engine.play_sound_on_bus(AudioBus::Effects, &clip, 0.5)`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L173)                                                                                                                                                                                                                                                                                                                                                                                          |
| [`set_master_volume`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L206)                                                                                                               | `engine.set_master_volume(0.8)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 3D rendering                                                                                                                                                                                                  | Use [`run3d::<G: Game3D>()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L778) with [`Frame3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L57), [`Camera3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/camera.rs#L4), [`DrawCmd3D`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mod.rs#L29), [`cube_mesh()`](https://github.com/justinwash/rengine/blob/master/engine/src/renderer3d/mesh.rs#L54), etc. |
| Rollback netcode                                                                                                                                                                                              | Enable `rollback` feature, implement [`Rollbackable`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L73), create [`RollbackSession`](https://github.com/justinwash/rengine/blob/master/engine/src/netcode/mod.rs#L86)                                                                                                                                                                                                                                                                            |
| [`iso_to_screen`](https://github.com/justinwash/rengine/blob/master/engine/src/world/iso.rs#L4) / [`screen_to_iso`](https://github.com/justinwash/rengine/blob/master/engine/src/world/iso.rs#L11)            | Use in an isometric game for tile placement                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| [`Canvas::shape()`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L51)                                                                                                           | Pass raw [`CanvasVertex`](https://github.com/justinwash/rengine/blob/master/engine/src/canvas/mod.rs#L6) triangles for custom shapes                                                                                                                                                                                                                                                                                                                                                                                           |
| [`create_color_texture`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L256)                                                                                                            | `engine.create_color_texture(32, 32, Color::RED)` for solid-color textures                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| [`white_texture()`](https://github.com/justinwash/rengine/blob/master/engine/src/app.rs#L270)                                                                                                                 | `engine.white_texture()` for drawing solid rectangles without a texture file                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Mouse input                                                                                                                                                                                                   | [`engine.input().mouse_delta()`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L45), [`is_mouse_down(0)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L50), [`is_mouse_pressed(1)`](https://github.com/justinwash/rengine/blob/master/engine/src/input/keyboard.rs#L55)                                                                                                                                                                                    |

---

_This document was generated from the `master` branch of the Rengine repository. All line references, struct definitions, and API signatures are current as of that branch._
