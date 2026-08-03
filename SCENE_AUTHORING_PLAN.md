# Scene-Authoring Plan — closing the drift between rengine and Formula R

Approved 2026-07-29. Full plan (context, rationale, code-line evidence) lives
at `C:\Users\justi\.claude\plans\okay-since-were-at-indexed-gadget.md`; this
file is the status board so a new session doesn't have to dig through chat
memory to find out what's shipped and what's next.

**North star:** every Formula R screen is a scene document authored in the
rengine editor; game code supplies data and rules only. Two measurements
define the problem: the engine's `ui_*` path can express chrome, not a
screen (6 draw primitives, no binding/lists/layout/text-wrap/images/
clipping), and the editor cannot author `ui_*` at all (no arbitrary
property editor). See the full plan file for the three-measurement
diagnosis and the `docs/UI_AUDIT.md` / `ROADMAP.md` prior art it responds to.

**Critical path:** `E1 → Ed1 → E2 → E10 → E5 → E3 → E4 → E9`. Everything
else parallelizes. Do not start G1 (game migration) before E3/E4 land.

## Status

| ID | Item | Status |
|---|---|---|
| E1 | Unify layout + hit-testing (`resolved_rect`) | ✅ SHIPPED `5796b6e` |
| Ed1 | Arbitrary property editor in inspector | ✅ SHIPPED `de03028` |
| E2 | Data binding (`{key}` placeholders) | ✅ SHIPPED `04364ef` |
| E10 | Text metrics without a `Canvas` | ✅ SHIPPED (engine half pre-existing; game half `b587a30`) |
| E5 | Missing paint kinds (image/text_block/text_spans/polyline/clip) | ✅ SHIPPED `0ba813d` + `80fa472` |
| E3 | Repeater nodes (`ui: "repeat"`) | ✅ SHIPPED `31450c4` |
| E4 | Layout containers (column/row flow) | ⚠️ PARTIAL `aa7fdb2` — core flow only, see below |
| **E9** | **Pixel-grid snapping** | **✅ SHIPPED** (this commit) |
| E6 | Interaction and focus (hover/press/focus states) | not started |
| E7 | Style tokens (theme document) | not started |
| E8 | Node animation beyond bob/sway (tweens) | not started |
| E11 | Harness parity (`run_with_scenes`) | not started |
| Ed2 | Real UI node kinds + typed inspector | not started |
| Ed3 | Viewport preview via engine's own `resolve_ui_rect` | not started |
| Ed4 | Binding picker (`bindings.manifest.json`) | not started |
| Ed5 | Theme document editor | not started |
| Ed6 | Scene hot-reload into running game | not started |
| Ed7 | Rotation/scale on `SceneNode` | not started |
| G0–G3 | Game migration (24 render modules → scene docs) | not started |

## What shipped, briefly

- **E1** — `SceneWorld2D` caches each `ui` node's resolved rect during
  draw; `node_bounds`/`hit_test` read it instead of recomputing layout.
  Public `resolved_rect(handle)`.
- **Ed1** — inspector "Custom Properties" section: free-form key/value rows
  on any node's `properties` map. Also added `--headless` to the editor
  binary and `move`/`click`/`text` steps to `PlayScript`.
- **E2** — any `ui_*` value may contain `{key}` placeholders substituted
  from a `Bindings` scope before parsing. `draw_at_with_bindings`/
  `draw_to_canvas_with_bindings`; old calls pass an empty scope. Also added
  `ui_visible` (binding-aware).
- **E10** — `Engine::font_atlas()` / `FontAtlas::measure_text`/
  `line_height` were already public and `Canvas`-free; game-side deleted
  the forked `ui_line_height`/`LH_14`/`ui_text_width_estimate` guesses.
- **E5** — all 6 kinds: `text_block` + `ui_wrap_w`, `text_spans` via
  numbered `ui_span_<n>_*` properties, `polyline`, `ui_text_align` +
  vertical centering, `image` (paints the node's own `sprites[0]`),
  `ui_clip: true` (scoped to the `draw_to_canvas` path only — the
  `Frame`-based `draw_at` path draws each node's `ui_layer` to a
  potentially different per-layer canvas, so "clip everything under this
  node" is materially harder there; revisit if a screen ever needs it).
- **E4 (partial)** — `ui_layout: "column" | "row"` + `ui_gap` +
  `ui_pad_*`, built on `engine/src/layout.rs`'s `Stack` (peek-one/
  place-one, deliberately not the two-pass `distribute_v/h`/
  `Track::{Even,Weight}`/`Justify::*`). A flow slot is only a reference
  rect — a child still needs `ui_stretch_x`/`ui_stretch_y` to fill it.
  **Deferred, explicitly out of scope for now:** `Track::Weight`/`Even` +
  `distribute_v/h`, `Justify::*`, mixing `ui_w_frac`/stretch main-axis
  children into a flow container, `ui_size: "content"`, `ui_align`/
  `ui_valign`.
- **E3** — `ui: "repeat"` + `ui_repeat_source` treats a node's one
  authored child as a template and materializes real, independently-
  addressable instance clones via `SceneWorld2D::sync_repeaters` — an
  explicit pre-draw step, not something draw itself does. Instances are
  real spawned `NodeHandle2D`s (not redraw-the-template-N-times, which
  would break E1's per-node rect cache). Composes with E4 for free.
- **E9** — `SceneWorld2D::set_pixel_grid(cell)` opts a whole scene into
  quantizing every resolved `ui_*` rect (x/y/w/h) to a grid cell size the
  host supplies (Formula R would pass `sprites::pixel()`). **Off by
  default** (`0.0`) so no existing screen, sample or test changes behavior
  until a host opts in — the plan's own text says "defaulting to on," but
  that reading would silently move every existing fractional-rect test
  and screen the moment this landed, which is exactly the kind of drift
  the plan exists to prevent. "On by default" instead means: once a host
  sets a grid, every node snaps unless it opts out per-node with
  `ui_snap: false` (e.g. a smoothly bobbing/swaying element that shouldn't
  judder to grid steps). Implemented in `resolve_ui_rect`
  (`engine/src/scene/data2d.rs`), threaded through both draw paths in
  `engine/src/scene/world2d.rs`. Tests: `pixel_grid_snaps_a_repeater_
  dividing_rows_into_a_panel` (the exact scenario the plan flags — 22 rows
  into a 217-unit panel, a height that doesn't divide evenly) and
  `pixel_grid_is_off_by_default_and_opt_out_per_node_works`.

## Two proof points (not started)

1. **"First light"** — `difficulty_select` (4 rows, hand-authored, no
   repeater/flow needed). Needs E1+Ed1+E2+E5+E6. Tests whether the loop
   (editor → scene file → runtime → deleted Rust) closes at all.
2. **"Hardest case"** — the race-HUD standings panel (22 bound rows,
   right-anchored, clipped/scrolled, hover, click-to-select). Needs
   E1–E6 + E9/E10. G1 (game migration) starts only after this passes.

Proof 1 is now reachable (E1/Ed1/E2/E5 shipped) except E6 (not started).
Proof 2 additionally needs E3/E4(remaining)/E6, all partial or not started.

## Related

Formula R side: `docs/UI_AUDIT.md` (original diagnosis),
`docs/VISUAL_IDENTITY.md` (Rule 0 — the uniform-pixel-density constraint
E9 protects), `docs/SCENE_DRIVEN_MIGRATION_PLAN.md` (Track 1/2/3 framing
this plan supersedes with concrete E/Ed/G numbers),
`docs/SCENE_MIGRATION_INVENTORY.md` (per-screen ownership).
