# Rengine — Excruciatingly Technical Deep Dive

> **Text-adventure style.**  
> Read the entry-point walkthrough below, then jump to any numbered topic that
> interests you. Each topic is a self-contained chapter on a specific module.

---

## Part 0 — Repository Map

```
rengine/
├── Cargo.toml                  ← workspace root (two members)
├── rengine-lib/                ← the engine library crate
│   └── src/
│       ├── lib.rs              ← public API + run() loop
│       ├── window/mod.rs       ← wgpu device/surface bootstrap
│       ├── graphics/
│       │   └── sprite/
│       │       ├── mod.rs      ← SpriteRenderer (CPU→GPU pipeline)
│       │       ├── sprite.vert.wgsl
│       │       └── sprite.frag.wgsl
│       ├── input/mod.rs        ← key-state machine
│       ├── physics/mod.rs      ← Rapier2D wrapper
│       ├── scene/mod.rs        ← actor container
│       ├── game_object/
│       │   ├── game_object.rs  ← GameObject trait
│       │   └── actor/
│       │       ├── mod.rs      ← Actor trait
│       │       ├── character_actor.rs
│       │       └── rigid_body_actor.rs
│       └── util/mod.rs         ← resource_path helper
└── game/                       ← the game binary crate
    └── src/
        ├── main.rs             ← entry point (analyzed below)
        └── actors/
            └── characters/
                └── player.rs   ← concrete Player actor
```

**Dependencies worth knowing about:**

| Crate | Role |
|---|---|
| `wgpu 25` | Cross-platform GPU API (Vulkan/Metal/DX12/WebGPU) |
| `winit 0.30` | OS window + event loop |
| `pollster 0.4` | Minimal async executor (`block_on`) |
| `bytemuck 1` | Safe `&T → &[u8]` casting for GPU upload |
| `rapier2d 0.29` | Physics engine (rigid bodies, colliders, joints) |
| `image 0.25` | Image decoding (PNG, JPEG, …) |

---

## Part 1 — Entry Point, Line by Line (`game/src/main.rs`)

```rust
mod actors;
```
Declares the local module tree rooted at `game/src/actors/mod.rs`.
Everything under `actors/` becomes part of the `game` binary's private
namespace.

---

```rust
use crate::actors::characters::player::Player;
```
Brings `Player` — the only concrete actor implemented so far — into scope.
`Player` is defined in `game/src/actors/characters/player.rs` and implements
three traits: `GameObject`, `Actor`, and `CharacterActor`.

---

```rust
use rengine_lib::{run, RengineConfig, RengineGame};
```
The three public items the game binary needs from the library:

* **`run`** — the blocking function that owns the event loop for the
  lifetime of the process.
* **`RengineConfig`** — plain data struct: window attributes + optional
  close callback.
* **`RengineGame`** — the trait you implement to give the engine something
  to tick and render.

---

```rust
use winit::keyboard::KeyCode;
use winit::window::WindowAttributes;
```
Re-exported from `winit` (the engine does not re-export these, so the game
crate lists `winit` as its own direct dependency in its `Cargo.toml`).
`KeyCode` is the physical key identifier (layout-independent).

---

```rust
struct Game {
    input_config: Option<rengine_lib::input::InputConfig>,
    should_close: bool,
    scene: rengine_lib::Scene,
    sprites_cache: Vec<rengine_lib::graphics::sprite::Sprite>,
}
```
The game's state. Four fields:

* **`input_config: Option<InputConfig>`** — holds the key-handler map until
  the engine calls `input_config()`, which takes it with `.take()`.
  After that it is `None` for the rest of the run.
* **`should_close: bool`** — checked internally to close the window cleanly.
  (Note: the engine already handles `CloseRequested`; this flag exists for
  Escape-key soft-close if you want to add that logic.)
* **`scene: Scene`** — the actor container; owns all gameplay objects.
* **`sprites_cache: Vec<Sprite>`** — a scratch buffer rebuilt every frame in
  `sprites()`. The engine calls `sprites()` to get the list of things to
  draw; this vec is re-populated by downcast-inspecting each actor.

---

```rust
impl Game {
    fn new() -> Self {
```
Infallible constructor — any failure (e.g., image loading) would panic at
asset-load time, not silently later.

---

```rust
        let input_config = Some(rengine_lib::input::InputConfig::new().on_key(
            KeyCode::Escape,
            || {
                println!("Escape pressed! Closing window...");
            },
        ));
```
`InputConfig::new()` creates an empty `HashMap<KeyCode, Box<dyn FnMut()>>`.
`.on_key(key, handler)` inserts one entry using the builder pattern and
returns `self` so you can chain more calls. The closure here is a side-effect
only (print); actual window closing is done in `update()`.

---

```rust
        let mut scene = rengine_lib::Scene::new();
        let character = Player::load_default();
        scene.add_actor(character);
```
`Scene::new()` allocates an empty `Vec<Box<dyn Actor>>`.
`Player::load_default()` opens `resources/image/mario.png` via the
`resource_path()` helper (which prefers `CARGO_MANIFEST_DIR` for dev
builds), decodes the PNG to get its pixel dimensions, and constructs a
`Sprite` at position `(100.0, 100.0)`.
`scene.add_actor(character)` boxes the `Player` and pushes it onto the vec
as a trait object `Box<dyn Actor>`.

---

```rust
        Self {
            input_config,
            should_close: false,
            scene,
            sprites_cache: Vec::new(),
        }
    }
}
```
Standard struct literal.

---

```rust
impl RengineGame for Game {
```
This is the engine's only coupling point with the game.

---

```rust
    fn input_config(&mut self) -> Option<rengine_lib::input::InputConfig> {
        self.input_config.take()
    }
```
The engine calls this once during `resumed()` (the first window-creation
event). `.take()` moves the value out of the `Option`, leaving `None` in
place. This prevents the engine from re-creating `InputState` on subsequent
resumes (which would wipe keystroke history).

---

```rust
    fn sprites(&mut self) -> &mut Vec<rengine_lib::graphics::sprite::Sprite> {
        if !self.sprites_cache.is_empty() {
            self.sprites_cache.clear();
        }
        for actor in &mut self.scene.actors {
            if let Some(player) = actor.as_any().downcast_ref::<Player>() {
                self.sprites_cache.push(player.sprite.clone());
            }
        }
        &mut self.sprites_cache
    }
```
Called by the engine every tick to get the draw list.

* Clears the cache (`O(n)` drain, `n` = current sprites).
* Iterates every actor as `Box<dyn Actor>`.
* `as_any()` returns `&dyn Any` (each concrete actor implements this by
  returning `self as &dyn Any`).
* `downcast_ref::<Player>()` is a checked type cast: it succeeds only if
  the underlying type is `Player`.
* Each matched player's `Sprite` is cloned (cheaply; `Sprite` is just two
  `f32` pairs and a `String` path) and appended.
* Returns `&mut self.sprites_cache` — the engine then calls `.clone()` on
  this slice before handing it to `SpriteRenderer`.

> **Design note:** This approach ties sprite collection to actor downcasting.
> It will not scale cleanly to many actor types without a more generic
> `Actor::sprites() -> Vec<Sprite>` method. Likely a deliberate simplicity
> trade-off for now.

---

```rust
    fn update(
        &mut self,
        wgpu_ctx: &mut rengine_lib::window::WgpuContext,
        input: &rengine_lib::input::InputState,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    ) {
        self.scene.update(wgpu_ctx, input, event, window);
        if input.is_just_pressed(KeyCode::Escape) {
            self.should_close = true;
        }
    }
```
Called once per fixed-timestep tick (≈60 Hz) from the engine.

* `scene.update(…)` fans out to every `actor.update(…)`, letting each actor
  read input and mutate its own state.
* `is_just_pressed` checks the `just_pressed` `HashSet`; it is only `true`
  for the single tick the key transitions from up→down.

---

```rust
    fn on_close(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("Window close requested!");
        event_loop.exit();
    }
```
Called by the engine when the OS sends `CloseRequested` (user clicks ✕).
`event_loop.exit()` signals winit to break out of its internal loop after
the current event is done.

---

```rust
fn main() {
    let mut attrs = WindowAttributes::default();
    attrs.title = "My Game".to_string();
    let config = RengineConfig {
        window_attributes: attrs,
        on_close: None,
    };
    run(config, Game::new());
}
```
The actual entry point.

* `WindowAttributes::default()` is a winit struct with sane defaults (OS
  picks size, decorations, etc.).
* Title is set directly on the struct field.
* `on_close: None` — the engine's own close path (`event_loop.exit()`) is
  used; no extra callback needed.
* `run(config, Game::new())` is **blocking** — it does not return until the
  event loop exits. The call stack from here is entirely inside winit.

---

## Part 2 — Choose Your Topic

You have just survived the entry point. Now choose where to go next.

```
┌──────────────────────────────────────────────────────────────────────┐
│  RENGINE DEEP DIVE — SELECT A MODULE                                 │
│                                                                      │
│  [1] Engine Core                                                     │
│      lib.rs — run(), RengineApp, the fixed-timestep loop             │
│                                                                      │
│  [2] Window & GPU Bootstrap                                          │
│      window/mod.rs — wgpu instance, adapter, device, surface         │
│                                                                      │
│  [3] Graphics Pipeline                                               │
│      graphics/sprite/mod.rs — SpriteRenderer, render passes,        │
│      GPU buffers, texture caching                                    │
│                                                                      │
│  [4] WGSL Shaders                                                    │
│      sprite.vert.wgsl + sprite.frag.wgsl — NDC math, UV coords      │
│                                                                      │
│  [5] Input System                                                    │
│      input/mod.rs — InputConfig, InputState, frame lifecycle         │
│                                                                      │
│  [6] Scene & Actor System                                            │
│      scene/mod.rs + game_object/ — trait hierarchy, downcasting      │
│                                                                      │
│  [7] Physics World                                                   │
│      physics/mod.rs — Rapier2D integration, pipeline step            │
│                                                                      │
│  [8] Player Actor                                                    │
│      game/src/actors/characters/player.rs — movement, sprite sync   │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

---

## [1] Engine Core — `rengine-lib/src/lib.rs`

### Public surface

```rust
pub mod graphics;   // graphics::sprite::*
pub mod input;      // InputConfig, InputState
pub mod physics;    // PhysicsWorld
pub mod scene;      // Scene
pub mod util;       // resource_path
pub mod window;     // WgpuContext, init_wgpu
pub mod game_object;// Actor, CharacterActor, GameObject

// Convenience re-exports so game code can write rengine_lib::Scene
pub use crate::scene::*;
pub use util::*;
pub use game_object::actor::character_actor::CharacterActor;
pub use game_object::actor::Actor;
pub use game_object::GameObject;
pub use graphics::sprite;
```

### `RengineConfig`

```rust
pub struct RengineConfig {
    pub window_attributes: WindowAttributes,
    pub on_close: Option<Box<dyn FnMut(&ActiveEventLoop) + Send + 'static>>,
}
```

A plain data bag. `WindowAttributes` is cloned into the inner `RengineApp`
struct, so it is consumed on first use. `on_close` is unused by the current
game (it passes `None`) but the slot exists for library consumers that want a
custom teardown callback separate from `RengineGame::on_close`.

### `RengineGame` trait

```rust
pub trait RengineGame {
    fn input_config(&mut self) -> Option<InputConfig> { None }
    fn sprites(&mut self) -> &mut Vec<Sprite>;
    fn update(&mut self, wgpu_ctx, input, event, window);
    fn on_close(&mut self, event_loop) {}
}
```

Two required methods (`sprites`, `update`); two with defaults (`input_config`,
`on_close`). This is the only seam the library exposes to game code.

### `run<G: RengineGame + 'static>(config, game)`

This is the heart of the engine. It:

1. **Defines `RengineApp<G>`** — a private struct local to `run()`, holding
   all mutable state:

   ```rust
   struct RengineApp<G: RengineGame + 'static> {
       config: RengineConfig,
       game: G,
       window: Option<&'static Window>,
       wgpu_ctx: Option<WgpuContext<'static>>,
       input: Option<InputState>,
       sprite_renderer: Option<SpriteRenderer>,
       last_update: Option<Instant>,
       accumulator: Duration,
   }
   ```

   Both `window` and `wgpu_ctx` are `Option` because they do not exist until
   the `resumed` event fires. They use `'static` lifetimes achieved via
   `Box::leak` — an intentional memory leak that keeps the `Window` alive for
   the entire process lifetime.

2. **Implements `ApplicationHandler` for `RengineApp<G>`** — winit's trait
   that receives all OS events. Three methods matter:

   #### `resumed(event_loop)`

   ```
   create_window → Box::leak (make &'static Window) → init_wgpu (blocking)
   → store window + wgpu_ctx → call game.input_config() → create InputState
   → create SpriteRenderer → reset last_update + accumulator
   ```

   `pollster::block_on(window::init_wgpu(…))` runs the async GPU init
   synchronously on the calling thread (main thread). This is safe here
   because `resumed` is called on the main thread and there is no async
   executor running yet.

   #### `about_to_wait(event_loop)` — **the game loop**

   ```
   const DT: Duration = 16_666_667 ns  // ≈ 1/60 second
   
   now = Instant::now()
   Δt = now − last_update
   accumulator += Δt
   
   while accumulator >= DT:
       input.begin_frame()              // clear just_pressed / just_released
       game.update(wgpu_ctx, input, AboutToWait, window)
       renderer.sprites = game.sprites().clone()
       renderer.render(wgpu_ctx)        // GPU draw call
       input.begin_frame()              // clear again (belt-and-suspenders)
       accumulator -= DT
   
   last_update = now
   window.request_redraw()             // ask OS to schedule a repaint
   ```

   This is a **fixed-timestep semi-fixed accumulator loop**. If a frame takes
   longer than 16.67 ms, `accumulator` grows and the inner loop runs multiple
   times — catching the simulation up. Surplus sub-step time is preserved in
   `accumulator` for the next frame. This decouples physics/game-logic speed
   from rendering frequency.

   > **Known quirk:** `input.begin_frame()` is called twice per tick — once
   > before `game.update()` and once after `renderer.render()`. The second
   > call clears `just_pressed` immediately after rendering, which means
   > within a single `about_to_wait` cycle a press is visible to `update`
   > but not to any code after `renderer.render()`. This is mostly harmless
   > but slightly redundant.

   #### `window_event(event_loop, window_id, event)`

   Handles three categories of events:

   * **`CloseRequested`** — calls `game.on_close(event_loop)` then
     `event_loop.exit()`.
   * **`Resized(new_size)`** — updates `wgpu_ctx.config.width/height` and
     calls `surface.configure(…)` to rebuild the swap chain at the new
     dimensions. Guards `width > 0 && height > 0` to avoid zero-size surface
     panics during minimise.
   * **All events** — wraps `WindowEvent` in `Event::WindowEvent{…}` and
     passes it to `input.handle_event(…)`, then calls `game.update(…)` and
     `renderer.render(…)` again. This means `update` + `render` fire once per
     window event *and* once per `about_to_wait`. For input events (key press)
     this is fine because `begin_frame` already cleared stale state.

3. **Creates the event loop and app, starts the loop:**

   ```rust
   let event_loop = EventLoop::new().unwrap();
   let mut app = RengineApp { config, game, window: None, wgpu_ctx: None, … };
   event_loop.run_app(&mut app).unwrap();
   ```

   `run_app` does not return until `event_loop.exit()` is called. The OS owns
   the call stack from this point.

---

## [2] Window & GPU Bootstrap — `rengine-lib/src/window/mod.rs`

### `WgpuContext<'a>`

```rust
pub struct WgpuContext<'a> {
    pub instance: Instance,   // wgpu entry point, chooses backend
    pub surface: Surface<'a>, // the OS window's drawable area
    pub device: Device,       // logical GPU handle
    pub queue: Queue,         // command submission queue
    pub config: SurfaceConfiguration, // swap chain parameters (stored for resize)
}
```

### `init_wgpu(window) -> WgpuContext` — step by step

```
Instance::new(InstanceDescriptor::default())
```
Creates the wgpu runtime. `InstanceDescriptor::default()` auto-selects the
best available backend: Vulkan on Linux, Metal on macOS, DX12 on Windows,
WebGPU/WebGL in the browser. You can override with `backends` field.

```
instance.create_surface(window)
```
Wraps the OS window handle (HWND on Windows, NSView on macOS, XCB/Wayland
handle on Linux) in a wgpu `Surface`. The lifetime `'a` ties the surface to
the window — the surface cannot outlive the window.

```
instance.request_adapter(RequestAdapterOptions {
    power_preference: HighPerformance,
    compatible_surface: Some(&surface),
    force_fallback_adapter: false,
})
```
Enumerates physical GPUs (adapters) and picks the discrete GPU if present
(`HighPerformance`). `compatible_surface` guarantees the chosen adapter can
actually present to the surface (important on multi-GPU systems).
`force_fallback_adapter: false` disables wgpu's software rasteriser fallback.

```
adapter.request_device(DeviceDescriptor {
    required_features: Features::empty(),
    required_limits: Limits::default(),
    …
})
```
Opens a logical connection to the chosen GPU. `Features::empty()` means no
optional GPU features (no geometry shaders, no ray tracing, etc.).
`Limits::default()` enforces conservative resource limits for broad
compatibility. Returns `(Device, Queue)`.

```
SurfaceConfiguration {
    usage: TextureUsages::RENDER_ATTACHMENT,
    format: surface.get_capabilities(&adapter).formats[0],
    width: size.width.max(1),
    height: size.height.max(1),
    present_mode: PresentMode::Fifo,   // VSync
    alpha_mode: capabilities.alpha_modes[0],
    desired_maximum_frame_latency: 2,
}
surface.configure(&device, &config)
```
Configures the swap chain:

* `RENDER_ATTACHMENT` — textures obtained from the swap chain can be used as
  render pass color targets.
* `formats[0]` — picks the first (highest priority) surface format reported
  by the driver. On most platforms this is `Bgra8UnormSrgb`.
* `Fifo` — "first in, first out" = VSync. GPU waits for the display's
  vertical blank before presenting. Prevents tearing, caps FPS to refresh
  rate.
* `desired_maximum_frame_latency: 2` — allows up to 2 frames queued in the
  swap chain pipeline, trading latency for throughput.

---

## [3] Graphics Pipeline — `rengine-lib/src/graphics/sprite/mod.rs`

### `Sprite` (data only)

```rust
pub struct Sprite {
    pub image_path: String,   // file system path to PNG/JPEG
    pub position: (f32, f32), // top-left corner in screen pixels
    pub size: (f32, f32),     // width × height in screen pixels
}
```

No GPU state — deliberately a plain data struct so it can be `Clone`d
cheaply and passed through the `RengineGame::sprites()` boundary without
lifetime complications.

### `SpriteRenderer`

```rust
pub struct SpriteRenderer {
    pub sprites: Vec<Sprite>,
    pub textures: HashMap<String, (Texture, TextureView, Sampler)>,
}
```

`textures` is the GPU texture cache. Keys are image paths. Textures are
uploaded once and reused across frames.

### `render(wgpu_ctx)` — full walkthrough

#### Step 1 — Acquire swap chain texture

```rust
let surface_texture = surface.get_current_texture()?;
let view = surface_texture.texture.create_view(&TextureViewDescriptor::default());
```
`get_current_texture()` blocks until the swap chain has a free slot.
`create_view` wraps it in a `TextureView` that render passes can reference.

#### Step 2 — Create command encoder

```rust
let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
    label: Some("SpriteRenderer Encoder"),
});
```
All GPU commands are recorded into a `CommandEncoder` on the CPU side.
Nothing runs on the GPU yet. This is wgpu's API design: commands are
**recorded then submitted**.

#### Step 3 — Clear pass

```rust
let _rpass = encoder.begin_render_pass(&RenderPassDescriptor {
    color_attachments: &[Some(RenderPassColorAttachment {
        view: &view,
        ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
    })],
    …
});
// _rpass is dropped here, ending the render pass
```
Clears the entire frame buffer to black. `LoadOp::Clear` fills the texture
with the given colour before any draw calls. `StoreOp::Store` ensures the
result is written back (not discarded).

#### Step 4 — Shared geometry (uploaded once per frame)

```rust
let vertices: [[f32; 2]; 4] = [[0,0],[1,0],[1,1],[0,1]]; // unit quad
let indices:  [u16; 6]      = [0,1,2, 0,2,3];            // two triangles
```
All sprites share the same unit-square geometry. Position and scale are
applied in the vertex shader via uniforms. This avoids re-uploading geometry
per sprite — only uniforms change.

`create_buffer_init` uploads the slice to a GPU buffer immediately.
`BufferUsages::VERTEX` / `::INDEX` tell wgpu how the buffer will be used.

#### Step 5 — Shader compilation (per frame — a known bottleneck)

```rust
let vs_src = include_str!("sprite.vert.wgsl");
let fs_src = include_str!("sprite.frag.wgsl");
let vs_module = device.create_shader_module(ShaderModuleDescriptor { source: Wgsl(vs_src) });
let fs_module = device.create_shader_module(…);
```
`include_str!` embeds the WGSL source as a `&'static str` at compile time.
`create_shader_module` compiles WGSL → driver-native IR **every frame**.
This is a significant performance issue in production renderers (shader
compilation is expensive). A proper renderer caches `ShaderModule`,
`RenderPipeline`, `BindGroupLayout`, and `PipelineLayout` across frames.

#### Step 6 — Bind group layout

```rust
BindGroupLayoutEntry { binding: 0, visibility: VERTEX,   ty: Buffer(Uniform) }  // uniforms
BindGroupLayoutEntry { binding: 1, visibility: FRAGMENT, ty: Texture(2D) }      // texture
BindGroupLayoutEntry { binding: 2, visibility: FRAGMENT, ty: Sampler(Filtering)}// sampler
```
Describes the layout of shader resources. Binding 0 (vertex-visible uniform
buffer) carries position/size/screen-size. Bindings 1 and 2
(fragment-visible) carry the texture data and its sampling parameters.

#### Step 7 — Render pipeline

```rust
device.create_render_pipeline(&RenderPipelineDescriptor {
    vertex: VertexState {
        module: &vs_module,
        entry_point: "main",
        buffers: &[VertexBufferLayout {
            array_stride: 8, // 2× f32
            step_mode: Vertex,
            attributes: &[VertexAttribute { format: Float32x2, offset: 0, location: 0 }],
        }],
    },
    fragment: Some(FragmentState {
        module: &fs_module,
        targets: &[Some(ColorTargetState {
            format: Bgra8UnormSrgb,
            blend: Some(BlendState::ALPHA_BLENDING),
        })],
    }),
    primitive: PrimitiveState::default(), // triangles, CCW front face
    …
})
```
`ALPHA_BLENDING` enables standard porter-duff `src_alpha × src + (1 − src_alpha) × dst`
compositing — transparent PNGs render correctly. The pipeline is also created
from scratch each frame (same caveat as above).

#### Step 8 — Per-sprite loop

For each `Sprite`:

1. **Texture upload (cached):** If `sprite.image_path` is not in
   `self.textures`, open the file with `image::open`, convert to RGBA8,
   create a `wgpu::Texture`, upload with `queue.write_texture`, create a
   `TextureView` and a `Sampler`, and store in the HashMap.

2. **Uniform buffer:**
   ```rust
   struct SpriteUniforms {
       sprite_pos:  [f32; 2],  // top-left pixel coords
       sprite_size: [f32; 2],  // width, height in pixels
       screen_size: [f32; 2],  // viewport width, height
   }
   ```
   Uploaded with `create_buffer_init` as a `UNIFORM` buffer. `bytemuck`
   converts the struct to raw bytes without unsafe code because `SpriteUniforms`
   derives `Pod` (plain old data) + `Zeroable`.

3. **Bind group:** Pairs the uniform buffer, texture view, and sampler with
   the layout declared in Step 6.

4. **Draw call:**
   ```rust
   rpass.set_pipeline(&pipeline);
   rpass.set_bind_group(0, &bind_group, &[]);
   rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
   rpass.set_index_buffer(index_buffer.slice(..), Uint16);
   rpass.draw_indexed(0..6, 0, 0..1);  // 6 indices = 2 triangles = 1 quad
   ```

#### Step 9 — Submit and present

```rust
queue.submit(Some(encoder.finish()));
surface_texture.present();
```
`encoder.finish()` seals the command buffer.
`queue.submit` hands it to the GPU driver.
`surface_texture.present()` schedules the swap chain flip at the next VSync.

---

## [4] WGSL Shaders

### Vertex shader — `sprite.vert.wgsl`

```wgsl
struct SpriteUniforms {
    sprite_pos:  vec2<f32>,
    sprite_size: vec2<f32>,
    screen_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: SpriteUniforms;

@vertex
fn main(input: VertexInput) -> VertexOutput {
    let pos = uniforms.sprite_pos + input.position * uniforms.sprite_size;
    let ndc = vec2<f32>(
        (pos.x / uniforms.screen_size.x) * 2.0 - 1.0,
         1.0 - (pos.y / uniforms.screen_size.y) * 2.0
    );
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = input.position;   // unit-square position doubles as UV
}
```

**Coordinate math explained:**

The vertex buffer contains a unit square: `(0,0), (1,0), (1,1), (0,1)`.

Step 1 — scale and translate to pixel space:
```
pos = sprite_pos + vertex_position × sprite_size
```
If `sprite_pos = (100, 100)` and `sprite_size = (64, 64)`, the four vertices
become `(100,100), (164,100), (164,164), (100,164)` in pixel space.

Step 2 — convert pixels → NDC (Normalized Device Coordinates):
```
ndc.x = (pos.x / screen_width ) × 2 − 1    // [-1 .. +1], left→right
ndc.y = 1 − (pos.y / screen_height) × 2    // [-1 .. +1], bottom→top
```
wgpu's NDC has `Y=+1` at the top and `Y=−1` at the bottom. Screen pixels
have `Y=0` at the top and `Y=height` at the bottom, hence the `1 −`
inversion.

`out.uv = input.position` — the unit-square vertex coordinate `[0..1]²`
is passed directly as the texture UV coordinate. `(0,0)` maps to the
texture's top-left; `(1,1)` maps to the bottom-right.

### Fragment shader — `sprite.frag.wgsl`

```wgsl
@group(0) @binding(1) var sprite_tex:     texture_2d<f32>;
@group(0) @binding(2) var sprite_sampler: sampler;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(sprite_tex, sprite_sampler, uv);
}
```

Minimal: sample the texture at the interpolated UV and return the RGBA value.
Alpha blending (configured in the pipeline) then composites it over whatever
was already in the frame buffer.

---

## [5] Input System — `rengine-lib/src/input/mod.rs`

### `InputConfig`

```rust
pub struct InputConfig {
    pub key_handlers: HashMap<KeyCode, Box<dyn FnMut() + Send + 'static>>,
}
```

A map from physical key code → callback. Callbacks fire inside
`InputState::handle_event` immediately when the key press is detected.
`FnMut` allows closures to capture mutable state. `Send + 'static` means
the closure can (in principle) be moved to another thread.

Builder pattern: `.on_key(key, handler) -> Self` inserts one entry and
returns `self`, allowing chaining:
```rust
InputConfig::new()
    .on_key(KeyCode::Escape, || println!("Esc!"))
    .on_key(KeyCode::Space,  || println!("Space!"))
```

### `InputState`

```rust
pub struct InputState {
    pub config: InputConfig,
    held_keys:     HashSet<KeyCode>, // currently depressed keys
    just_pressed:  HashSet<KeyCode>, // pressed this frame (cleared by begin_frame)
    just_released: HashSet<KeyCode>, // released this frame (cleared by begin_frame)
    prev_keys:     HashSet<KeyCode>, // previous frame's held set (updated by end_frame)
}
```

### Frame lifecycle

```
OS event arrives (KeyboardInput)
    → handle_event()
        Pressed:  held_keys.insert(key)  → if new: just_pressed.insert(key)
                  config.key_handlers[key]() if exists
        Released: held_keys.remove(key)
                  just_released.insert(key)

about_to_wait fires
    → begin_frame()
        just_pressed.clear()
        just_released.clear()
    → game.update()          ← query is_held, is_just_pressed, was_just_released
    → begin_frame()          ← second clear (belt-and-suspenders, see §1)
```

`end_frame()` (which copies `held_keys → prev_keys`) is defined but never
called anywhere in the current codebase. It exists for future "was held last
frame but not this frame" queries.

### Query API

| Method | Returns `true` if |
|---|---|
| `is_held(key)` | Key is currently depressed |
| `is_just_pressed(key)` | Key went down this frame |
| `was_just_released(key)` | Key went up this frame |

---

## [6] Scene & Actor System

### Trait hierarchy

```
std::any::Any
    └── GameObject       (position / set_position)
            └── Actor    (update / draw / as_any)
                    └── CharacterActor  (sprite / collision_enabled / health)
```

### `GameObject` — `game_object/game_object.rs`

```rust
pub trait GameObject {
    fn position(&self) -> (f32, f32);
    fn set_position(&mut self, pos: (f32, f32));
}
```

The minimal positional interface. All game objects have a world position.

### `Actor` — `game_object/actor/mod.rs`

```rust
pub trait Actor: GameObject {
    fn update(&mut self, wgpu_ctx, input, event, window);
    fn draw(&mut self, renderer, wgpu_ctx);
    fn as_any(&self) -> &dyn Any;
}
```

`as_any()` returns `self as &dyn Any`. This is the idiomatic Rust pattern for
downcasting trait objects — `dyn Actor` does not implement `Any` directly
because of Rust's object safety rules, so each concrete type must implement
`as_any()` manually.

### `CharacterActor` — `game_object/actor/character_actor.rs`

```rust
pub trait CharacterActor: Actor {
    fn sprite(&self) -> &Sprite;
    fn sprite_mut(&mut self) -> &mut Sprite;
    fn collision_enabled(&self) -> bool { true }
    fn health(&self) -> i32 { 100 }
}
```

Adds sprite access and game-play attributes. `collision_enabled` and `health`
have default implementations — concrete types override them if needed.

### `Scene` — `scene/mod.rs`

```rust
pub struct Scene {
    pub actors: Vec<Box<dyn Actor>>,
}
```

A flat, ordered list of heterogeneous actors behind trait objects.
`Box<dyn Actor>` has the same layout as a fat pointer (data ptr + vtable ptr).

```rust
pub fn add_actor<A: Actor + 'static>(&mut self, actor: A) {
    self.actors.push(Box::new(actor));
}
```

Generic over any concrete `Actor` type. `'static` is required because
`Box<dyn Actor>` (without explicit lifetime annotation) defaults to
`Box<dyn Actor + 'static>`.

```rust
pub fn update(&mut self, …) {
    for actor in self.actors.iter_mut() {
        actor.update(…);  // virtual dispatch through vtable
    }
}
```

`actor.update()` is a virtual call. The compiler emits an indirect function
call through the vtable pointer stored in the fat pointer.

### `RigidBodyActor` — `game_object/actor/rigid_body_actor.rs`

A stub actor holding a `RigidBodyHandle` and `ColliderHandle` from Rapier2D.
All methods are placeholders — the positions return `(0.0, 0.0)` and update
does nothing. This exists as scaffolding for a future physics-driven actor.

---

## [7] Physics World — `rengine-lib/src/physics/mod.rs`

Rengine wraps Rapier2D's pipeline with a convenience struct.

### `PhysicsWorld`

```rust
pub struct PhysicsWorld {
    pub gravity: Vector<f32>,               // e.g. vector![0.0, -9.81]
    pub integration_parameters: IntegrationParameters, // dt, erp, damping, …
    pub physics_pipeline: PhysicsPipeline,  // the top-level stepper
    pub bodies: RigidBodySet,               // all rigid bodies
    pub colliders: ColliderSet,             // all colliders (attached to bodies)
    pub impulse_joints: ImpulseJointSet,    // revolute, prismatic, etc.
    pub multibody_joints: MultibodyJointSet,// articulated chains
    pub island_manager: IslandManager,      // sleeping/waking body groups
    pub narrow_phase: NarrowPhase,          // contact detection + manifolds
    pub ccd_solver: CCDSolver,              // continuous collision detection
    pub broad_phase: BroadPhaseBvh,         // AABB tree for broad-phase cull
}
```

### `step()`

```rust
self.physics_pipeline.step(
    &self.gravity,
    &self.integration_parameters,
    &mut self.island_manager,
    &mut self.broad_phase,   // BVH pass: prune impossible pairs
    &mut self.narrow_phase,  // GJK/EPA pass: find actual contacts
    &mut self.bodies,
    &mut self.colliders,
    &mut self.impulse_joints,
    &mut self.multibody_joints,
    &mut self.ccd_solver,
    &(), // physics hooks (custom forces) — none
    &(), // event handler (collision events) — none
);
```

Each call advances the simulation by `integration_parameters.dt` (default
1/60 s). Rapier's pipeline does:

1. **Island detection** — `IslandManager` groups sleeping bodies (bodies that
   haven't moved recently) to skip their integration entirely.
2. **Broad phase** — `BroadPhaseBvh` uses an AABB bounding-volume hierarchy
   to quickly discard pairs of bodies that are too far apart to touch.
3. **Narrow phase** — `NarrowPhase` runs GJK/EPA on the remaining pairs to
   find exact contact points and normals.
4. **Constraint resolution** — iterative velocity-based solver resolves
   penetrations and joint constraints.
5. **Integration** — updates positions and velocities for all active bodies.
6. **CCD** — `CCDSolver` sub-steps fast-moving bodies to prevent tunnelling
   (passing through thin walls).

### Helper methods

```rust
add_rigid_body(body: RigidBody) -> RigidBodyHandle
remove_rigid_body(handle)        // also removes associated colliders/joints
add_collider(collider, parent_handle) -> ColliderHandle
remove_collider(handle)
```

`RigidBodyHandle` and `ColliderHandle` are typed generational indices into
Rapier's internal arenas (similar to `slotmap` keys). They remain valid until
the body/collider is removed.

> **Note:** `PhysicsWorld` is fully functional but not yet wired into the
> game loop. The `RigidBodyActor` stub shows the intended coupling: an actor
> would hold handles into `PhysicsWorld`, and its `update()` would read back
> the simulated position and apply it to its sprite.

---

## [8] Player Actor — `game/src/actors/characters/player.rs`

The only concrete actor. Demonstrates the full trait implementation stack.

### `Player`

```rust
pub struct Player {
    pub sprite: Sprite,
    pub collision_enabled: bool,
    pub health: i32,
}
```

### `load_default()`

```rust
pub fn load_default() -> Self {
    let image_path = rengine_lib::resource_path("resources/image/mario.png");
    let img = image::open(&image_path).expect("…");
    let (width, height) = img.dimensions();
    let sprite = Sprite::new(&image_path, (100.0, 100.0), (width as f32, height as f32));
    Self::new(sprite)
}
```

`resource_path` resolves the path relative to `CARGO_MANIFEST_DIR` during
development (so `cargo run` finds assets next to `Cargo.toml`) and falls back
to the CWD at runtime. The image is decoded *only* to read its pixel
dimensions; actual GPU texture upload happens lazily inside `SpriteRenderer`.

### `Actor::update()` — WASD movement

```rust
fn update(&mut self, _wgpu_ctx, input, _event, _window) {
    let speed = 1.0;  // pixels per tick
    let mut dx = 0.0;
    let mut dy = 0.0;
    if input.is_held(KeyCode::KeyW) { dy -= speed; }
    if input.is_held(KeyCode::KeyS) { dy += speed; }
    if input.is_held(KeyCode::KeyA) { dx -= speed; }
    if input.is_held(KeyCode::KeyD) { dx += speed; }
    let len = f32::sqrt(dx * dx + dy * dy);
    if len != 0.0 {
        dx = dx / len * speed;
        dy = dy / len * speed;
    }
    let (x, y) = self.position();
    self.set_position((x + dx, y + dy));
}
```

The `if len != 0.0` guard prevents a division-by-zero when no key is held
(`dx` and `dy` both zero → `len == 0`). When at least one direction key is
held, the normalization step (`dx / len * speed`) ensures diagonal movement
does not travel faster than axis-aligned movement (classic "normalize then
scale" trick). Without it, diagonal input `(1,1)` would have magnitude
`√2 ≈ 1.41` and the player would move 41% faster diagonally.

### `Actor::draw()`

```rust
fn draw(&mut self, renderer: &mut SpriteRenderer, _wgpu_ctx) {
    renderer.add_sprite(self.sprite.clone());
}
```

Clones the sprite (three heap values: two `f32` pairs and a `String` path)
and pushes it onto the renderer's list. The actual GPU draw call happens
later inside `SpriteRenderer::render`.

### `CharacterActor` implementation

Accessors to `sprite`, `sprite_mut`, `collision_enabled`, and `health` — all
forwarding to struct fields. Satisfies the trait contract for any system that
queries `CharacterActor` capabilities.

---

## Appendix — Call Graph (one game tick)

```
winit::EventLoop::poll_events
  └── RengineApp::about_to_wait
        ├── input.begin_frame()              // clear just_pressed / just_released
        ├── game.update()
        │     └── scene.update()
        │           └── for actor in actors:
        │                 actor.update()     // vtable dispatch
        │                   └── Player::update()
        │                         ├── input.is_held(W/A/S/D)
        │                         └── self.set_position(pos + delta)
        ├── renderer.sprites = game.sprites().clone()
        │     └── scene.actors.iter()
        │           └── actor.as_any().downcast_ref::<Player>()
        │                 └── push player.sprite.clone()
        └── renderer.render(wgpu_ctx)
              ├── surface.get_current_texture()
              ├── encoder.begin_render_pass()  [clear]
              ├── for sprite in sprites:
              │     ├── textures.get_or_upload(image_path)
              │     ├── create uniform buffer (pos, size, screen_size)
              │     ├── create bind group
              │     └── encoder.begin_render_pass() [draw_indexed 0..6]
              ├── queue.submit(encoder.finish())
              └── surface_texture.present()
```
